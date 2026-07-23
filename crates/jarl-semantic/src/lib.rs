//! Per-file semantic info for jarl lint rules.
//!
//! `SemanticInfo` is computed once over a parsed file and exposes the
//! information lint rules need to answer "is this definition used?", without
//! every rule reimplementing the AST passes (NSE detection, string
//! interpolation, closure escape analysis, ...) on top of oak's
//! `SemanticIndex`.
//!
//! Mirrors ruff's `Binding::is_unused()` style: rules ask
//! `info.is_definition_used(scope, def_id, def)` rather than walking the
//! semantic index themselves.

pub mod strings;

use std::collections::HashSet;

use air_r_parser::RParserOptions;
use air_r_syntax::{
    AnyRArgumentName, AnyRExpression, RArgument, RArgumentList, RBinaryExpression, RCall,
    RExtractExpression, RNamespaceExpression, RStringValue, RSyntaxKind, RSyntaxNode,
};
use biome_rowan::{AstNode, AstSeparatedList, SyntaxNodeCast, TextRange, TextSize};
use oak_core::syntax_ext::{RIdentifierExt, RStringValueExt};
use oak_semantic::DefinitionId;
use oak_semantic::semantic_index::{Definition, DefinitionKind, ScopeId, SemanticIndex};

/// Per-file semantic info derived from oak's [`SemanticIndex`] plus AST
/// passes over the syntax tree. Computed once per file; consumed by lints.
pub struct SemanticInfo<'a> {
    index: &'a SemanticIndex,
    /// Root syntax node of the analyzed file. Needed to resolve
    /// `AstPtr` references stored in [`DefinitionKind`] back to nodes.
    root: RSyntaxNode,
    /// Path of the file being analyzed. Used to resolve `source("path")`
    /// arguments against the current file's directory.
    file: &'a std::path::Path,
    /// Names that have a synthetic use from AST passes (`do.call("f", …)`,
    /// `..cols`, `on.exit` bodies, loop assignment LHSes, short-circuit
    /// assignment LHSes, custom infix operators). A definition whose symbol
    /// name is in this set is treated as used.
    synthetic_used_names: HashSet<String>,
    /// String-interpolation reads (`glue("{x}")`, cli markup, custom
    /// delimiters) as `(name, read range)` pairs. Unlike `synthetic_used_names`,
    /// these are resolved position-aware in
    /// [`Self::precompute_interpolation_uses`]: a read resolves to the
    /// definition it actually sees, so a *later* same-scope reassignment of the
    /// name is not kept alive by it.
    interpolation_uses: Vec<(String, TextRange)>,
    /// Identifier `Use` ranges that should be ignored because they sit inside
    /// an NSE call argument (`quote(x)`, `substitute(…)`, …).
    nse_ranges: Vec<TextRange>,
    /// Ranges inside an NSE argument that are nonetheless evaluated and so
    /// carve a hole back out of [`Self::nse_ranges`]: the `.()` operands of
    /// `bquote(...)`, plus `bquote`'s `where`/`splice` arguments (which are
    /// evaluated normally). A use inside one of these counts as a real use.
    unquote_ranges: Vec<TextRange>,
    /// Bodies of `local({...})` calls.
    local_body_ranges: Vec<TextRange>,
    /// Ranges of formula RHSes (`~ rhs`).
    formula_ranges: Vec<TextRange>,
    /// Definitions reached by some non-NSE use anywhere in the file. Computed
    /// from oak's `reaching_definitions`, which resolves both local uses and
    /// free-variable uses in nested closures (via enclosing snapshots).
    reaching_used: HashSet<(ScopeId, DefinitionId)>,
}

impl<'a> SemanticInfo<'a> {
    /// Build the info table. Runs the AST pass (collecting synthetic uses,
    /// NSE ranges, formula ranges, local body ranges) and then the
    /// reaching-use precomputation over oak's use-def maps.
    pub fn build(
        root: &RSyntaxNode,
        expressions: &[RSyntaxNode],
        index: &'a SemanticIndex,
        file: &'a std::path::Path,
    ) -> Self {
        let mut this = Self {
            index,
            root: root.clone(),
            file,
            synthetic_used_names: HashSet::new(),
            interpolation_uses: Vec::new(),
            nse_ranges: Vec::new(),
            unquote_ranges: Vec::new(),
            local_body_ranges: Vec::new(),
            formula_ranges: Vec::new(),
            reaching_used: HashSet::new(),
        };
        this.collect_ast_passes(expressions);
        let scopes = this.scope_ids();
        this.precompute_reaching_uses(&scopes);
        this.precompute_interpolation_uses();
        this
    }

    pub fn index(&self) -> &SemanticIndex {
        self.index
    }

    pub fn root(&self) -> &RSyntaxNode {
        &self.root
    }

    /// Walk all scopes (root + descendants) in arbitrary order.
    pub fn scope_ids(&self) -> Vec<ScopeId> {
        self.index.scope_ids().collect()
    }

    // ── High-level queries ────────────────────────────────────────────

    /// True if any of the supported "is used" conditions hold for this
    /// definition: synthetic AST-derived use, a reaching use (local or via a
    /// nested closure), or the `local({…})` body shortcut.
    pub fn is_definition_used(
        &self,
        scope_id: ScopeId,
        def_id: DefinitionId,
        def: &Definition,
    ) -> bool {
        let symbol_name = self.index.symbols(scope_id).symbol(def.symbol()).name();
        if self.synthetic_used_names.contains(symbol_name) {
            return true;
        }
        if self.reaching_used.contains(&(scope_id, def_id)) {
            return true;
        }
        if self.is_used_inside_local_body(scope_id, def) {
            return true;
        }
        false
    }

    // ── Low-level predicates (compose for new rules) ──────────────────

    pub fn is_in_formula(&self, range: TextRange) -> bool {
        in_any_range(range, &self.formula_ranges)
    }

    /// True when `range` sits in a quoted NSE context (`quote(...)`,
    /// `substitute(...)`, `bquote(...)`, …) where code is captured rather than
    /// evaluated, so neither an assignment nor a read there touches the live
    /// binding. The `.()` unquote holes inside `bquote` are evaluated, so
    /// ranges within them are excluded.
    pub fn is_in_nse(&self, range: TextRange) -> bool {
        in_any_range(range, &self.nse_ranges) && !in_any_range(range, &self.unquote_ranges)
    }

    pub fn has_synthetic_use(&self, name: &str) -> bool {
        self.synthetic_used_names.contains(name)
    }

    /// True if `name` is referenced as a use in any scope nested under
    /// `scope_id`. Keeps a binding alive when a nested closure captures it.
    /// Needed for `%<>%`, which oak doesn't model as a definition, so its
    /// reaching-use set can't answer the closure-capture question directly.
    pub fn is_used_in_nested_scope(&self, scope_id: ScopeId, name: &str) -> bool {
        for descendant in self.scope_descendants(scope_id) {
            if descendant == scope_id {
                continue;
            }
            let Some(symbol) = self.index.symbols(descendant).id(name) else {
                continue;
            };
            if self
                .index
                .uses(descendant)
                .iter()
                .any(|(_, u)| u.symbol() == symbol)
            {
                return true;
            }
        }
        false
    }

    /// Answer "is this name used?" for an assignment-pipe target (`x %<>% f()`),
    /// which oak doesn't model as a definition (so `is_definition_used` can't
    /// apply). Mirrors the definition check, keyed by name + the rebinding's end
    /// position instead of a `DefinitionId`:
    /// - a real (oak) read later in `scope_id`, or one captured by a nested
    ///   scope;
    /// - a synthetic use (name-based: `..x`, `do.call("x", …)`, custom infix, …);
    /// - a string-interpolation read (`glue("{x}")`, cli, custom delimiters)
    ///   later in `scope_id`, or one in a nested scope.
    pub fn is_pipe_target_used(&self, scope_id: ScopeId, name: &str, rebind_end: TextSize) -> bool {
        if let Some(sym) = self.index.symbols(scope_id).id(name)
            && self
                .index
                .uses(scope_id)
                .iter()
                .any(|(_, u)| u.symbol() == sym && u.range().start() >= rebind_end)
        {
            return true;
        }
        if self.is_used_in_nested_scope(scope_id, name) {
            return true;
        }
        // Synthetic uses carry no position, matching the main loop's check.
        if self.synthetic_used_names.contains(name) {
            return true;
        }
        self.interpolation_use_reaches(scope_id, name, rebind_end)
    }

    /// True if a string-interpolation read of `name` sees the binding created at
    /// `rebind_end`: a read later in `scope_id`, or one in a nested scope
    /// (evaluated later, so textual order is irrelevant — the same same-scope /
    /// nested-scope split as the oak-use checks in [`Self::is_pipe_target_used`]).
    fn interpolation_use_reaches(
        &self,
        scope_id: ScopeId,
        name: &str,
        rebind_end: TextSize,
    ) -> bool {
        if self.interpolation_uses.is_empty() {
            return false;
        }
        let descendants = self.scope_descendants(scope_id);
        self.interpolation_uses
            .iter()
            .any(|(use_name, read_range)| {
                if use_name != name {
                    return false;
                }
                let (read_scope, _) = self.index.scope_at(read_range.start());
                if read_scope == scope_id {
                    read_range.start() >= rebind_end
                } else {
                    descendants.contains(&read_scope)
                }
            })
    }

    // ── Internal: AST pass ────────────────────────────────────────────

    fn collect_ast_passes(&mut self, expressions: &[RSyntaxNode]) {
        for expr in expressions {
            for node in expr.descendants() {
                self.visit_node(&node);
            }
        }
    }

    fn visit_node(&mut self, node: &RSyntaxNode) {
        match node.kind() {
            RSyntaxKind::R_STRING_VALUE => self.collect_string_interpolation(node),
            RSyntaxKind::R_CALL => {
                if let Some(call) = node.clone().cast::<RCall>() {
                    self.visit_call(&call);
                }
            }
            RSyntaxKind::R_DOT_DOT_I => self.collect_dotdot_identifier(node),
            RSyntaxKind::R_IDENTIFIER => self.collect_dotdot_identifier(node),
            RSyntaxKind::R_BINARY_EXPRESSION => {
                if let Some(bin) = node.clone().cast::<RBinaryExpression>() {
                    self.visit_binary(&bin);
                }
            }
            RSyntaxKind::R_WHILE_STATEMENT
            | RSyntaxKind::R_FOR_STATEMENT
            | RSyntaxKind::R_REPEAT_STATEMENT => {
                self.collect_loop_assignment_names(node);
            }
            _ => {}
        }
    }

    /// Workaround for oak not retroactively connecting loop-body defs to
    /// loop-condition uses. Coarse but matches the test contract.
    fn collect_loop_assignment_names(&mut self, loop_node: &RSyntaxNode) {
        for descendant in loop_node.descendants() {
            if descendant.kind() != RSyntaxKind::R_BINARY_EXPRESSION {
                continue;
            }
            if let Some(name) = assignment_lhs_name(&descendant) {
                self.synthetic_used_names.insert(name);
            }
        }
    }

    fn collect_string_interpolation(&mut self, node: &RSyntaxNode) {
        let text = node.text_trimmed().to_string();
        // The read happens where the string sits, so identifiers inside it
        // resolve against the definitions live at this position.
        let read_range = node.text_trimmed_range();
        // cli's inline markup (`{.field {x}}`) interleaves styling with
        // interpolation, so strings inside a cli call need a markup-aware scan
        // rather than the plain glue scan.
        if node_in_cli_markup_call(node) {
            if let Some(content) = strings::get_string_literal_contents(&text) {
                self.collect_cli_interpolation(&content, read_range);
            }
            return;
        }
        // Scanned with the default glue delimiters regardless of the wrapping
        // call: any `{x}` in any string keeps `x` alive. Calls that override
        // the delimiters via `.open`/`.close` are handled separately in
        // `collect_custom_glue_interpolation`.
        for segment in scan_interpolation_segments(&text, "{", "}") {
            self.collect_identifiers_in_interpolation(segment, read_range);
        }
    }

    /// Collect identifier uses from a cli-formatted string.
    ///
    /// cli reuses glue's `{...}` interpolation but adds inline markup spans of
    /// the form `{.class content}`, where `.class` and the literal `content`
    /// are styling — not R code — yet any nested `{...}` inside the content is
    /// still interpolated. So `{.field {x}}` uses `x`, but `{.field x}` does
    /// not. Markup spans recurse into their content; plain segments are parsed
    /// as R code.
    fn collect_cli_interpolation(&mut self, content: &str, read_range: TextRange) {
        for segment in scan_interpolation_segments(content, "{", "}") {
            if let Some(inner) = cli_markup_content(segment) {
                self.collect_cli_interpolation(inner, read_range);
            } else {
                self.collect_identifiers_in_interpolation(segment, read_range);
            }
        }
    }

    /// glue-family calls can override the interpolation delimiters with
    /// `.open` / `.close` (e.g. `glue("<x>", .open = "<", .close = ">")`). The
    /// default-`{}` scan in [`Self::collect_string_interpolation`] can't see
    /// those, so when a call sets custom delimiters, rescan its unnamed string
    /// arguments with them and record the identifiers as synthetic uses.
    ///
    /// Operates on the *unquoted* string contents, not the raw token text: a
    /// custom delimiter like `(`/`)` would otherwise collide with the
    /// `r"(...)"` raw-string wrapper.
    fn collect_custom_glue_interpolation(&mut self, args: &[(Option<String>, RSyntaxNode)]) {
        let open = named_string_arg(args, ".open");
        let close = named_string_arg(args, ".close");
        // Nothing to do unless a delimiter is actually customised; the default
        // case is already covered by `collect_string_interpolation`.
        if open.is_none() && close.is_none() {
            return;
        }
        let open = open.unwrap_or_else(|| "{".to_string());
        let close = close.unwrap_or_else(|| "}".to_string());
        if open == "{" && close == "}" {
            return;
        }
        for (name, value) in args {
            if name.is_some() || value.kind() != RSyntaxKind::R_STRING_VALUE {
                continue;
            }
            let Some(content) =
                strings::get_string_literal_contents(&value.text_trimmed().to_string())
            else {
                continue;
            };
            let read_range = value.text_trimmed_range();
            for segment in scan_interpolation_segments(&content, &open, &close) {
                self.collect_identifiers_in_interpolation(segment, read_range);
            }
        }
    }

    /// Parse a glue-style `{...}` interpolation as R code and collect every
    /// identifier reference as an interpolation use at `read_range`. Skips the
    /// field side of `x$a` / `x@a` and the namespace side of `pkg::name` —
    /// those name members, not bindings.
    fn collect_identifiers_in_interpolation(&mut self, src: &str, read_range: TextRange) {
        let parsed = air_r_parser::parse(src, RParserOptions::default());
        if parsed.has_error() {
            return;
        }
        for node in parsed.syntax().descendants() {
            if node.kind() != RSyntaxKind::R_IDENTIFIER {
                continue;
            }
            if is_member_name(&node) {
                continue;
            }
            if let Some(token) = node.first_token() {
                self.interpolation_uses
                    .push((token.text_trimmed().to_string(), read_range));
            }
        }
    }

    fn collect_dotdot_identifier(&mut self, node: &RSyntaxNode) {
        let Some(token) = node.first_token() else {
            return;
        };
        let text = token.text_trimmed();
        if let Some(stripped) = text.strip_prefix("..")
            && !stripped.is_empty()
            && stripped
                .chars()
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '.')
        {
            self.synthetic_used_names.insert(stripped.to_string());
        }
    }

    fn visit_binary(&mut self, bin: &RBinaryExpression) {
        let Ok(op) = bin.operator() else {
            return;
        };
        let op_text = op.text_trimmed();
        // Formulas are `R_BINARY_EXPRESSION` with a `~` operator. Only an `=`
        // inside a formula is non-standard (it's named-arg syntax, not an
        // assignment), so the formula range is recorded to suppress those
        // definitions. Identifier *reads* in a formula still consume bindings:
        // `X <- 2; lm(1 ~ X)` looks `X` up at evaluation time, so the formula
        // is deliberately not added to `nse_ranges`.
        if op_text == "~" {
            self.formula_ranges.push(bin.syntax().text_trimmed_range());
            return;
        }

        // Custom infix operators (`a %op% b`): oak doesn't model the operator
        // as a use of the `%op%` binding, so an operator whose only reference
        // is at a call site would look unused. Record the operator name as a
        // synthetic use. Only user-defined `%...%` bindings can match; R's
        // built-in operators have no local definition to keep alive.
        if op_text.starts_with('%') && op_text.ends_with('%') {
            self.synthetic_used_names.insert(op_text.to_string());
        }

        // Short-circuit operators: `cond || (x <- 2)` may skip the
        // assignment entirely, so prior defs of `x` should remain alive.
        // Oak walks linearly and shadows them. Workaround: any LHS
        // assigned inside a `||`/`&&` is considered synthetically used.
        if op_text == "||" || op_text == "&&" || op_text == "|" || op_text == "&" {
            for descendant in bin.syntax().descendants() {
                if descendant.kind() == RSyntaxKind::R_BINARY_EXPRESSION
                    && let Some(name) = assignment_lhs_name(&descendant)
                {
                    self.synthetic_used_names.insert(name);
                }
            }
        }
    }

    fn visit_call(&mut self, call: &RCall) {
        let Some(name) = call_name(call) else {
            return;
        };

        let arg_values: Vec<(Option<String>, RSyntaxNode)> = call_args(call);

        self.collect_custom_glue_interpolation(&arg_values);

        match name.as_str() {
            // Only the quoted `expr` argument is NSE. Other arguments are
            // evaluated normally — e.g. `substitute(x, env = env)` reads
            // `env` — so their identifiers stay real uses.
            "quote" | "substitute" | "Quote" => {
                if let Some(expr) = nse_expr_arg(&arg_values) {
                    self.nse_ranges.push(expr.text_trimmed_range());
                }
            }
            // `expression(...)` and `alist(...)` quote every argument: their
            // values are stored unevaluated, so an assignment like
            // `alist(x <- 1)` is captured code, not a real definition of `x`.
            "expression" | "alist" => {
                for (_, value) in &arg_values {
                    self.nse_ranges.push(value.text_trimmed_range());
                }
            }
            // `bquote` quotes its `expr` argument, but `.()` unquotes
            // (evaluates) the wrapped expression. So `expr` is NSE —
            // `bquote(x)` does not use `x` — except for identifiers inside
            // `.()`, which are real uses: `bquote(.(x))` does use `x`. The
            // `where`/`splice` arguments are evaluated normally, so they are
            // carved out of any enclosing NSE range — e.g. in
            // `substitute(bquote(.(x), env))`, `env` is a real use even though
            // the whole `bquote(...)` sits inside `substitute`'s NSE range.
            "bquote" => {
                let expr = nse_expr_arg(&arg_values);
                if let Some(expr) = expr {
                    self.nse_ranges.push(expr.text_trimmed_range());
                    self.collect_bquote_unquoted_uses(expr);
                }
                let expr_range = expr.map(|e| e.text_trimmed_range());
                for (_, value) in &arg_values {
                    if Some(value.text_trimmed_range()) != expr_range {
                        self.unquote_ranges.push(value.text_trimmed_range());
                    }
                }
            }
            "do.call" | "match.fun" | "Recall" | "getFunction" => {
                if let Some((_, first)) = arg_values.first()
                    && let Some(s) = string_literal_value(first)
                {
                    self.synthetic_used_names.insert(s);
                }
            }
            "local" => {
                if let Some((_, body)) = arg_values.first() {
                    self.local_body_ranges.push(body.text_trimmed_range());
                }
            }
            "on.exit" => {
                if let Some((_, body)) = arg_values.first() {
                    self.collect_on_exit_uses(body);
                }
            }
            "source" => {
                if let Some((_, first)) = arg_values.first()
                    && let Some(path) = string_literal_value(first)
                {
                    self.import_uses_from_sourced_file(&path);
                }
            }
            _ => {}
        }
    }

    /// Resolves a `source("path")` argument against the current file, parses
    /// the target, and harvests every identifier appearing in it as a
    /// synthetic use. R's `source()` runs its argument in the caller's
    /// environment, so any name read by the sourced script consumes a
    /// binding in this file.
    fn import_uses_from_sourced_file(&mut self, path: &str) {
        let Some(target) = resolve_sourced_path(self.file, path) else {
            return;
        };
        let Ok(contents) = std::fs::read_to_string(&target) else {
            return;
        };
        let parsed = air_r_parser::parse(&contents, RParserOptions::default());
        if parsed.has_error() {
            return;
        }
        for node in parsed.syntax().descendants() {
            if node.kind() == RSyntaxKind::R_IDENTIFIER
                && let Some(token) = node.first_token()
            {
                self.synthetic_used_names
                    .insert(token.text_trimmed().to_string());
            }
        }
    }

    fn collect_on_exit_uses(&mut self, body: &RSyntaxNode) {
        for node in body.descendants() {
            if node.kind() == RSyntaxKind::R_IDENTIFIER
                && let Some(token) = node.first_token()
            {
                self.synthetic_used_names
                    .insert(token.text_trimmed().to_string());
            }
        }
    }

    /// Within a `bquote()` argument, operands wrapped in `.()` are unquoted
    /// (evaluated), so uses inside them are real. Record each `.()` operand
    /// range as a hole in the surrounding NSE range pushed for the argument.
    fn collect_bquote_unquoted_uses(&mut self, arg: &RSyntaxNode) {
        for node in arg.descendants() {
            if node.kind() != RSyntaxKind::R_CALL {
                continue;
            }
            let Some(call) = node.clone().cast::<RCall>() else {
                continue;
            };
            if call_name(&call).as_deref() != Some(".") {
                continue;
            }
            for (_, value) in call_args(&call) {
                self.unquote_ranges.push(value.text_trimmed_range());
            }
        }
    }

    // ── Internal: reach / closure analysis ────────────────────────────

    /// Collect every definition reached by a non-NSE use, in every scope.
    ///
    /// `reaching_definitions` returns both local reaching definitions and, for
    /// a free variable in a nested closure, the enclosing-scope definitions
    /// captured by oak's enclosing snapshots. So a single pass over all uses
    /// covers in-scope reads and closure captures alike. Uses sitting inside an
    /// NSE argument (`quote(x)`, …) are skipped: they don't consume a binding.
    fn precompute_reaching_uses(&mut self, scopes: &[ScopeId]) {
        let index = self.index;
        for &scope_id in scopes {
            for (use_id, u) in index.uses(scope_id).iter() {
                if self.is_in_nse(u.range()) {
                    continue;
                }
                for (def_scope, def_id) in index.reaching_definitions(scope_id, use_id) {
                    self.mark_reaching_definition_used(def_scope, def_id);
                }
            }
        }
    }

    /// Record a definition reached by a real read as used.
    ///
    /// An NSE assignment (`substitute(x <- 2)`) is quoted code, not an
    /// executed assignment, but oak — which doesn't model NSE — still lets it
    /// shadow a prior real definition in its dataflow. So a real read after
    /// such an assignment resolves to the NSE definition instead of the live
    /// binding it actually reads. When that happens, walk back to the nearest
    /// preceding real definition of the same symbol and mark it used instead.
    fn mark_reaching_definition_used(&mut self, def_scope: ScopeId, def_id: DefinitionId) {
        let def = &self.index.definitions(def_scope)[def_id];
        if !self.is_in_nse(def.range()) {
            self.reaching_used.insert((def_scope, def_id));
            return;
        }
        if let Some(real_id) = self.preceding_real_definition(def_scope, def) {
            self.reaching_used.insert((def_scope, real_id));
        }
    }

    /// The nearest definition of `target`'s symbol in `scope` that starts
    /// before `target` and is not itself an NSE (quoted) assignment.
    fn preceding_real_definition(
        &self,
        scope: ScopeId,
        target: &Definition,
    ) -> Option<DefinitionId> {
        let symbol = target.symbol();
        let cutoff = target.range().start();
        let mut best: Option<(DefinitionId, TextRange)> = None;
        for (id, def) in self.index.definitions(scope).iter() {
            if def.symbol() != symbol || def.range().start() >= cutoff {
                continue;
            }
            if self.is_in_nse(def.range()) {
                continue;
            }
            if best.is_none_or(|(_, best_range)| def.range().start() > best_range.start()) {
                best = Some((id, def.range()));
            }
        }
        best.map(|(id, _)| id)
    }

    /// Resolve each string-interpolation read to the definition(s) it uses and
    /// record them in `reaching_used`, mirroring the real-use pass. Runs after
    /// the AST pass so the NSE ranges it consults are already collected.
    fn precompute_interpolation_uses(&mut self) {
        // Keep `interpolation_uses` populated (clone rather than take): pipe
        // targets, which oak doesn't model as definitions, query it later via
        // `interpolation_use_reaches`.
        let uses = self.interpolation_uses.clone();
        for (name, read_range) in &uses {
            self.mark_interpolation_use(name, read_range.start());
        }
    }

    /// Mark the definition(s) an interpolated read of `name` at `pos` resolves
    /// to.
    ///
    /// Interpolation reads are position-aware, so — unlike `synthetic_used_names`
    /// — a *later* same-scope reassignment of the name isn't kept alive. Walk
    /// outward from the reading scope to the scope that binds `name`:
    /// - if that's the reading scope, mark every definition that precedes the
    ///   read: more than one can reach it through branching control flow (e.g.
    ///   an `if`/`else` assigning in both arms), but a later reassignment is
    ///   excluded so it stays reported;
    /// - if it's an enclosing scope, the read is a closure capture evaluated
    ///   later, so textual order is irrelevant and every definition of the name
    ///   there is kept alive.
    fn mark_interpolation_use(&mut self, name: &str, pos: TextSize) {
        let index = self.index;
        let (read_scope, _) = index.scope_at(pos);
        for owner in index.ancestor_scope_ids(read_scope) {
            let Some(symbol_id) = index.symbols(owner).id(name) else {
                continue;
            };
            let defs: Vec<(DefinitionId, TextSize)> = index
                .definitions(owner)
                .iter()
                .filter(|(_, def)| def.symbol() == symbol_id && !self.is_in_nse(def.range()))
                .map(|(id, def)| (id, def.range().start()))
                .collect();
            // `name` may be referenced but not bound in this scope; if so, keep
            // walking outward to the scope that actually binds it.
            if defs.is_empty() {
                continue;
            }
            let captured = owner != read_scope;
            for (def_id, start) in defs {
                if captured || start < pos {
                    self.reaching_used.insert((owner, def_id));
                }
            }
            return;
        }
    }

    fn is_used_inside_local_body(&self, scope_id: ScopeId, def: &Definition) -> bool {
        if self.local_body_ranges.is_empty() {
            return false;
        }
        let Some(body_range) = self
            .local_body_ranges
            .iter()
            .find(|r| r.contains_range(def.range()))
            .copied()
        else {
            return false;
        };
        for (_, u) in self.index.uses(scope_id).iter() {
            if u.symbol() != def.symbol() {
                continue;
            }
            if body_range.contains_range(u.range()) {
                return true;
            }
        }
        false
    }

    fn scope_descendants(&self, scope_id: ScopeId) -> Vec<ScopeId> {
        let mut out = vec![scope_id];
        let mut stack = vec![scope_id];
        while let Some(s) = stack.pop() {
            for child in self.index.child_scope_ids(s) {
                out.push(child);
                stack.push(child);
            }
        }
        out
    }
}

// ── Free helpers (also used by rule policy) ──────────────────────────────

fn in_any_range(target: TextRange, ranges: &[TextRange]) -> bool {
    ranges.iter().any(|r| r.contains_range(target))
}

/// Extract glue-style interpolation segments delimited by `open`/`close`.
/// Doubled delimiters (`{{`/`}}` for the default case) are glue escapes and
/// are skipped. Nested delimiters are tracked so `{f({x})}` yields the whole
/// inner expression — except when `open == close` (e.g. `.open`/`.close`
/// both `|`), where the delimiters are indistinguishable so nesting is
/// impossible and the first delimiter after an opener always closes it.
/// Returns the source slices between the outermost delimiter pairs.
fn scan_interpolation_segments<'t>(text: &'t str, open: &str, close: &str) -> Vec<&'t str> {
    let mut segments = Vec::new();
    if open.is_empty() || close.is_empty() {
        return segments;
    }
    let escaped_open = format!("{open}{open}");
    let escaped_close = format!("{close}{close}");
    let mut i = 0;
    while i < text.len() {
        let slice = &text[i..];
        // Doubled delimiters are glue escape sequences for literal characters.
        if slice.starts_with(&escaped_open) {
            i += escaped_open.len();
            continue;
        }
        if slice.starts_with(&escaped_close) {
            i += escaped_close.len();
            continue;
        }
        if slice.starts_with(open) {
            let start = i + open.len();
            let mut depth = 1usize;
            let mut end = start;
            while end < text.len() && depth > 0 {
                let rest = &text[end..];
                // When `open == close` a delimiter can only close the current
                // segment; treating it as a nested opener would never balance.
                if open != close && rest.starts_with(open) {
                    depth += 1;
                    end += open.len();
                } else if rest.starts_with(close) {
                    depth -= 1;
                    if depth > 0 {
                        end += close.len();
                    }
                } else {
                    end += next_char_len(text, end);
                }
            }
            if depth == 0 && end > start {
                segments.push(&text[start..end]);
            }
            // Skip past the closing delimiter (`end` points at its start).
            i = end + close.len();
        } else {
            i += next_char_len(text, i);
        }
    }
    segments
}

/// Byte length of the UTF-8 character starting at `i` (which must be a char
/// boundary). Used to advance scanning without splitting multi-byte chars.
fn next_char_len(text: &str, i: usize) -> usize {
    text[i..].chars().next().map_or(1, |c| c.len_utf8())
}

/// Unquoted contents of a named string-literal argument (e.g. `.open = "<"`),
/// or `None` if absent or not a string literal.
fn named_string_arg(args: &[(Option<String>, RSyntaxNode)], name: &str) -> Option<String> {
    let (_, value) = args.iter().find(|(n, _)| n.as_deref() == Some(name))?;
    if value.kind() != RSyntaxKind::R_STRING_VALUE {
        return None;
    }
    strings::get_string_literal_contents(&value.text_trimmed().to_string())
}

/// True if `node` sits inside a cli call that glue-interpolates its arguments
/// with inline markup support. Walks all ancestors (not just the immediate
/// call) so message strings nested in a `c(...)` bullets vector still count.
fn node_in_cli_markup_call(node: &RSyntaxNode) -> bool {
    node.ancestors().any(|ancestor| {
        ancestor.kind() == RSyntaxKind::R_CALL
            && ancestor
                .cast::<RCall>()
                .and_then(|call| call_name(&call))
                .is_some_and(|name| is_cli_markup_function(&name))
    })
}

/// cli functions that glue-interpolate their text arguments with inline markup.
/// Excludes non-interpolating ones (`cli_verbatim`, `cli_code`,
/// `cli_bullets_raw`). Namespaced calls (`cli::cli_abort`) resolve to the bare
/// name via [`call_name`].
fn is_cli_markup_function(name: &str) -> bool {
    matches!(
        name,
        "cli_abort"
            | "cli_warn"
            | "cli_inform"
            | "cli_alert"
            | "cli_alert_success"
            | "cli_alert_info"
            | "cli_alert_warning"
            | "cli_alert_danger"
            | "cli_text"
            | "cli_h1"
            | "cli_h2"
            | "cli_h3"
            | "cli_li"
            | "cli_ul"
            | "cli_ol"
            | "cli_dl"
            | "cli_bullets"
            | "cli_par"
            | "cli_progress_message"
            | "cli_progress_step"
            | "format_inline"
            | "format_error"
            | "format_warning"
            | "format_message"
    )
}

/// If `segment` is a cli inline-markup span (`.class content`), return the
/// `content` part, which is itself glue-interpolated. The leading `.class` and
/// any literal text are styling, not R code. Returns `None` for plain
/// interpolation segments (`x`, `mean(x)`, `.x` with no following space).
fn cli_markup_content(segment: &str) -> Option<&str> {
    let rest = segment.strip_prefix('.')?;
    let class_len = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    if class_len == 0 {
        return None;
    }
    // A markup span separates the class from its content with whitespace.
    let after_class = rest[class_len..].strip_prefix(|c: char| c.is_whitespace())?;
    Some(after_class.trim_start())
}

fn is_member_name(node: &RSyntaxNode) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    match parent.kind() {
        RSyntaxKind::R_EXTRACT_EXPRESSION => parent
            .cast::<RExtractExpression>()
            .and_then(|e| e.right().ok())
            .is_some_and(|r| r.syntax() == node),
        RSyntaxKind::R_NAMESPACE_EXPRESSION => parent
            .cast::<RNamespaceExpression>()
            .and_then(|e| e.right().ok())
            .is_some_and(|r| r.syntax() == node),
        _ => false,
    }
}

/// True if the value assigned by this binary assignment is a function
/// definition, following chained assignments (`x <- y <- function() {}`) down
/// to the innermost value so every name in the chain is treated as a function
/// binding.
pub fn assignment_rhs_is_function_def(bin: &RBinaryExpression) -> bool {
    for child in bin.syntax().children() {
        match child.kind() {
            RSyntaxKind::R_FUNCTION_DEFINITION => return true,
            // The value is itself an assignment (`x <- y <- function() {}`), so
            // follow the chain down to its own value.
            RSyntaxKind::R_BINARY_EXPRESSION => {
                if let Some(inner) = child.cast::<RBinaryExpression>()
                    && assignment_lhs_name(inner.syntax()).is_some()
                    && assignment_rhs_is_function_def(&inner)
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// True if the LHS of a binary assignment is anything other than a bare
/// identifier (e.g. `names(x)`, `x[1]`, `x$a`).
pub fn assignment_lhs_is_complex(bin: &RBinaryExpression) -> bool {
    let Ok(op) = bin.operator() else {
        return false;
    };
    let lhs = if op.text_trimmed() == "->" || op.text_trimmed() == "->>" {
        bin.right().ok().map(|n| n.syntax().clone())
    } else {
        bin.left().ok().map(|n| n.syntax().clone())
    };
    match lhs {
        Some(node) => !matches!(node.kind(), RSyntaxKind::R_IDENTIFIER),
        None => false,
    }
}

/// The text range of the bare-identifier LHS of an assignment, if any.
pub fn lhs_range_for_definition(def: &Definition, root: &RSyntaxNode) -> Option<TextRange> {
    let bin = match def.kind() {
        DefinitionKind::Assignment(ptr) | DefinitionKind::SuperAssignment(ptr) => ptr.to_node(root),
        DefinitionKind::Parameter(_) | DefinitionKind::ForVariable(_) => {
            return Some(def.range());
        }
        DefinitionKind::Import { .. } => return None,
    };
    let op = bin.operator().ok()?;
    let lhs = if op.text_trimmed() == "->" || op.text_trimmed() == "->>" {
        bin.right().ok()?
    } else {
        bin.left().ok()?
    };
    let lhs_node = lhs.syntax();
    if lhs_node.kind() == RSyntaxKind::R_IDENTIFIER {
        Some(lhs_node.text_trimmed_range())
    } else {
        None
    }
}

/// The value node of the quoted-expression argument (`expr`) of a quote-like
/// call: the argument named `expr =` if present, otherwise the first
/// positional (unnamed) argument. Other arguments — `substitute`'s `env`,
/// `bquote`'s `where`/`splice` — are evaluated normally, so their reads must
/// not be swallowed as NSE.
fn nse_expr_arg(args: &[(Option<String>, RSyntaxNode)]) -> Option<&RSyntaxNode> {
    if let Some((_, value)) = args
        .iter()
        .find(|(name, _)| name.as_deref() == Some("expr"))
    {
        return Some(value);
    }
    args.iter()
        .find(|(name, _)| name.is_none())
        .map(|(_, value)| value)
}

fn call_name(call: &RCall) -> Option<String> {
    let func = call.function().ok()?;
    match func {
        AnyRExpression::RIdentifier(ident) => Some(ident.name_text()),
        AnyRExpression::RNamespaceExpression(ns) => ns
            .right()
            .ok()
            .and_then(|r| r.syntax().first_token())
            .map(|t| t.text_trimmed().to_string()),
        _ => None,
    }
}

fn call_args(call: &RCall) -> Vec<(Option<String>, RSyntaxNode)> {
    let Ok(arguments) = call.arguments() else {
        return Vec::new();
    };
    let items = arguments.items();
    args_iter(&items)
}

fn args_iter(list: &RArgumentList) -> Vec<(Option<String>, RSyntaxNode)> {
    let mut out = Vec::new();
    for item in list.iter() {
        let Ok(arg) = item else { continue };
        let name = argument_name(&arg);
        let value = arg.value().map(|v| v.syntax().clone());
        if let Some(value) = value {
            out.push((name, value));
        }
    }
    out
}

/// LHS identifier name of `x <- …` / `x = …` / `… -> x` / `x <<- …` /
/// `… ->> x`. None for any other binary expression.
pub fn assignment_lhs_name(node: &RSyntaxNode) -> Option<String> {
    let bin = node.clone().cast::<RBinaryExpression>()?;
    let op = bin.operator().ok()?;
    let op_text = op.text_trimmed();
    let lhs = match op_text {
        "<-" | "<<-" | "=" => bin.left().ok()?,
        "->" | "->>" => bin.right().ok()?,
        _ => return None,
    };
    let node = lhs.syntax();
    if node.kind() == RSyntaxKind::R_IDENTIFIER {
        let ident = node.clone().cast::<air_r_syntax::RIdentifier>()?;
        Some(ident.name_text())
    } else {
        None
    }
}

fn argument_name(arg: &RArgument) -> Option<String> {
    let clause = arg.name_clause()?;
    let name = clause.name().ok()?;
    match name {
        AnyRArgumentName::RIdentifier(ident) => Some(ident.name_text()),
        AnyRArgumentName::RDots(_) => Some("...".to_string()),
        _ => None,
    }
}

fn string_literal_value(node: &RSyntaxNode) -> Option<String> {
    node.clone().cast::<RStringValue>()?.string_text()
}

/// Resolve a `source("path")` argument against the currently-analyzed file.
///
/// Absolute paths are taken as-is. Relative paths are tried against a chain
/// of anchors, nearest first: the analyzed file's directory (helpers usually
/// sit next to the script), then each ancestor up to the process CWD. R
/// itself resolves `source()` against `getwd()`, and the project root a
/// script is run from sits somewhere between the file and where jarl was
/// invoked — trying every level catches layouts like `jarl check foo` where
/// `foo/sub/a.R` sources a file at `foo/` (the same reason oak's salsa
/// resolver anchors at the workspace root). Lint paths are CWD-relative, so
/// walking their ancestors down to `""` is exactly that chain and never
/// escapes the CWD.
fn resolve_sourced_path(current_file: &std::path::Path, path: &str) -> Option<std::path::PathBuf> {
    let candidate = std::path::Path::new(path);
    if candidate.is_absolute() {
        return Some(candidate.to_path_buf());
    }
    let file_dir = current_file.parent().unwrap_or(std::path::Path::new(""));
    let fallback = file_dir.join(candidate);

    if current_file.is_absolute() {
        // An absolute analyzed path has no CWD-bounded ancestor chain to
        // walk (it would climb toward the filesystem root); anchor next to
        // the file, then at the CWD.
        if fallback.is_file() {
            return Some(fallback);
        }
        if candidate.is_file() {
            return Some(candidate.to_path_buf());
        }
        return Some(fallback);
    }

    let mut dir = file_dir;
    loop {
        let resolved = dir.join(candidate);
        if resolved.is_file() {
            return Some(resolved);
        }
        if dir.as_os_str().is_empty() {
            // Every anchor between the file and the CWD missed; keep the
            // file-relative guess so callers fail uniformly when reading it.
            return Some(fallback);
        }
        dir = dir.parent().unwrap_or(std::path::Path::new(""));
    }
}

/// `ImportsResolver` impl that plugs `source("path")` injection into oak's
/// builder.
///
/// The resolver parses the target file, builds its `SemanticIndex` with
/// another `JarlImportsResolver` (so `source()` chains resolve
/// transitively), and reports its top-level definitions — own and
/// forwarded — as `SourceResolution.names`. Oak then materialises
/// `DefinitionKind::Import` entries at the `source()` call site in the
/// calling file's index.
///
/// This handles the *defined-by-source* side of `source()` semantics.
/// The complementary *used-by-source* side — names *read* by the sourced
/// file consume bindings in the calling file — is still handled
/// separately by [`SemanticInfo::import_uses_from_sourced_file`] because
/// oak's [`oak_semantic::SourceResolution`] only carries defined names.
pub struct JarlImportsResolver {
    current_file: std::path::PathBuf,
    /// Files already resolved along this `source()` chain (absolutized),
    /// the analyzed file included. Shared across the whole chain so cyclic
    /// `source()` graphs terminate: a file is resolved at most once per
    /// chain, and a repeat contributes no names (mirroring oak_db's
    /// cycle-recovery on `File::exports`).
    visited: std::rc::Rc<std::cell::RefCell<HashSet<std::path::PathBuf>>>,
}

impl JarlImportsResolver {
    pub fn new(current_file: impl Into<std::path::PathBuf>) -> Self {
        let current_file = current_file.into();
        let mut visited = HashSet::new();
        visited.insert(absolutize_path(&current_file));
        Self {
            current_file,
            visited: std::rc::Rc::new(std::cell::RefCell::new(visited)),
        }
    }
}

impl oak_semantic::ImportsResolver for JarlImportsResolver {
    fn resolve_source(&mut self, path: &str) -> Option<oak_semantic::SourceResolution> {
        let target = resolve_sourced_path(&self.current_file, path)?;
        let target_key = absolutize_path(&target);
        if !self.visited.borrow_mut().insert(target_key.clone()) {
            return None;
        }
        let contents = std::fs::read_to_string(&target).ok()?;
        let parsed = air_r_parser::parse(&contents, RParserOptions::default());
        if parsed.has_error() {
            return None;
        }
        // The URL is built from the absolutized path so consumers (e.g. the
        // cross-file pre-pass) can round-trip it back to a filesystem path.
        // `Url::from_file_path` rejects relative paths; fall back to a
        // synthetic `file:///` URL so exotic paths still index.
        let url = url::Url::from_file_path(&target_key)
            .ok()
            .or_else(|| url::Url::parse(&format!("file:///{}", target_key.display())).ok())?;
        // Recurse with the chain's visited set: the target's own `source()`
        // calls inject Import entries into its index, so its exports below
        // include names it forwards from deeper files.
        let sub_resolver = JarlImportsResolver {
            current_file: target,
            visited: std::rc::Rc::clone(&self.visited),
        };
        let sub_index = oak_semantic::build_index(&parsed.tree(), sub_resolver);
        let names: Vec<String> = sub_index.exports().keys().map(|s| s.to_string()).collect();
        Some(oak_semantic::SourceResolution { url, names, packages: Vec::new() })
    }
}

/// Absolutize `path` against the process CWD, without touching the
/// filesystem. Gives `source()` targets a canonical key so cycle detection
/// and URL construction agree regardless of how the path was spelled.
fn absolutize_path(path: &std::path::Path) -> std::path::PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}
