//! Per-file semantic info for jarl lint rules.
//!
//! `SemanticInfo` is computed once over a parsed file and exposes the
//! information lint rules need to answer "is this definition used?", without
//! every rule walking oak's `SemanticIndex` themselves.
//!
//! Mirrors ruff's `Binding::is_unused()` style: rules ask
//! `info.is_definition_used(scope, def_id, def)` rather than walking the
//! semantic index themselves.

pub mod strings;

use std::collections::HashSet;

use air_r_parser::RParserOptions;
use air_r_syntax::{
    AnyRArgumentName, AnyRExpression, RArgument, RArgumentList, RBinaryExpression, RCall,
    RExtractExpression, RNamespaceExpression, RSyntaxKind, RSyntaxNode,
};
use biome_rowan::{AstNode, AstSeparatedList, SyntaxNodeCast, TextRange, TextSize};
use oak_core::syntax_ext::RIdentifierExt;
use oak_semantic::DefinitionId;
use oak_semantic::semantic_index::{Definition, DefinitionKind, ScopeId, SemanticIndex};

/// Per-file semantic info derived from oak's [`SemanticIndex`] plus AST
/// passes over the syntax tree. Computed once per file; consumed by lints.
pub struct SemanticInfo<'a> {
    index: &'a SemanticIndex,
    /// Root syntax node of the analyzed file. Needed to resolve
    /// `AstPtr` references stored in [`DefinitionKind`] back to nodes.
    root: RSyntaxNode,
    /// Position-aware reads collected by the AST pass: string interpolation
    /// (`glue("{x}")`, cli markup, custom delimiters). Stored as
    /// `(name, read range)` pairs and resolved in
    /// [`Self::precompute_positional_uses`]: a read resolves to the definition
    /// it actually sees, so a *later* same-scope reassignment of the name is
    /// not kept alive by it.
    positional_uses: Vec<(String, TextRange)>,
    /// Identifier `Use` ranges that should be ignored because they sit inside
    /// a quoting call oak's effects registry doesn't cover (`substitute(…)`,
    /// `Quote(…)`, `expression(…)`, `alist(…)`). `quote()` and `bquote()` are
    /// modeled by oak itself (their quoted arguments produce no uses or
    /// definitions in the index), so they don't need ranges here.
    nse_ranges: Vec<TextRange>,
    /// Ranges of formula RHSes (`~ rhs`).
    formula_ranges: Vec<TextRange>,
    /// Definitions reached by some non-NSE use anywhere in the file. Computed
    /// from oak's `reaching_definitions`, which resolves both local uses and
    /// free-variable uses in nested closures (via enclosing snapshots).
    reaching_used: HashSet<(ScopeId, DefinitionId)>,
}

impl<'a> SemanticInfo<'a> {
    /// Build the info table. Runs the AST pass (collecting interpolation
    /// reads, NSE ranges, formula ranges) and then the reaching-use
    /// precomputation over oak's use-def maps. `_file` is unused for now; it
    /// feeds the sourced-file reads pass once that lands.
    pub fn build(
        root: &RSyntaxNode,
        expressions: &[RSyntaxNode],
        index: &'a SemanticIndex,
        _file: &std::path::Path,
    ) -> Self {
        let mut this = Self {
            index,
            root: root.clone(),
            positional_uses: Vec::new(),
            nse_ranges: Vec::new(),
            formula_ranges: Vec::new(),
            reaching_used: HashSet::new(),
        };
        this.collect_ast_passes(expressions);
        let scopes = this.scope_ids();
        this.precompute_reaching_uses(&scopes);
        this.precompute_positional_uses();
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

    /// True if a reaching use (local or via a nested closure) consumes this
    /// definition.
    pub fn is_definition_used(
        &self,
        scope_id: ScopeId,
        def_id: DefinitionId,
        _def: &Definition,
    ) -> bool {
        if self.reaching_used.contains(&(scope_id, def_id)) {
            return true;
        }
        false
    }

    // ── Low-level predicates (compose for new rules) ──────────────────

    pub fn is_in_formula(&self, range: TextRange) -> bool {
        in_any_range(range, &self.formula_ranges)
    }

    /// True when `range` sits in a quoted NSE context (`substitute(...)`,
    /// `expression(...)`, …) where code is captured rather than evaluated, so
    /// neither an assignment nor a read there touches the live binding. Only
    /// covers the quoting calls oak's effects registry doesn't model —
    /// `quote()`/`bquote()` arguments never enter the index in the first
    /// place.
    pub fn is_in_nse(&self, range: TextRange) -> bool {
        in_any_range(range, &self.nse_ranges)
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
            RSyntaxKind::R_BINARY_EXPRESSION => {
                if let Some(bin) = node.clone().cast::<RBinaryExpression>() {
                    self.visit_binary(&bin);
                }
            }
            _ => {}
        }
    }

    fn collect_string_interpolation(&mut self, node: &RSyntaxNode) {
        // Only strings passed to an interpolating call are code: `{x}` in
        // `message("{x}")` is literal text and reads nothing.
        let Some(flavor) = interpolation_flavor(node) else {
            return;
        };
        let Some(content) = strings::get_string_literal_contents(&node.text_trimmed().to_string())
        else {
            return;
        };
        // The read happens where the string sits, so identifiers inside it
        // resolve against the definitions live at this position.
        let read_range = node.text_trimmed_range();
        match flavor {
            // cli's inline markup (`{.field {x}}`) interleaves styling with
            // interpolation, so cli strings need a markup-aware scan rather
            // than the plain glue scan.
            InterpolationFlavor::CliMarkup => self.collect_cli_interpolation(&content, read_range),
            // Scanned with the default glue delimiters; calls that override
            // them via `.open`/`.close` are handled separately in
            // `collect_custom_glue_interpolation`.
            InterpolationFlavor::Glue => {
                for segment in scan_interpolation_segments(&content, "{", "}") {
                    self.collect_identifiers_in_interpolation(segment, read_range);
                }
            }
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
    /// arguments with them and record the identifiers as positional uses.
    ///
    /// Operates on the *unquoted* string contents, not the raw token text: a
    /// custom delimiter like `(`/`)` would otherwise collide with the
    /// `r"(...)"` raw-string wrapper.
    fn collect_custom_glue_interpolation(
        &mut self,
        call_name: &str,
        args: &[(Option<String>, RSyntaxNode)],
    ) {
        if !is_glue_interpolating_function(call_name) {
            return;
        }
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
                self.positional_uses
                    .push((token.text_trimmed().to_string(), read_range));
            }
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
        }
    }

    fn visit_call(&mut self, call: &RCall) {
        let Some(name) = call_name(call) else {
            return;
        };

        let arg_values: Vec<(Option<String>, RSyntaxNode)> = call_args(call);

        self.collect_custom_glue_interpolation(&name, &arg_values);

        match name.as_str() {
            // Quoting calls oak's effects registry doesn't cover. (`quote` and
            // `bquote` are covered: oak drops their quoted arguments from the
            // index entirely, and walks `bquote`'s `.()` unquote holes as real
            // uses.)
            //
            // Only the quoted `expr` argument is NSE. Other arguments are
            // evaluated normally — e.g. `substitute(x, env = env)` reads
            // `env` — so their identifiers stay real uses.
            "substitute" | "Quote" => {
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
            _ => {}
        }
    }

    // ── Internal: reach / closure analysis ────────────────────────────

    /// Collect every definition reached by a non-NSE use, in every scope.
    ///
    /// `reaching_definitions` returns both local reaching definitions and, for
    /// a free variable in a nested closure, the enclosing-scope definitions
    /// captured by oak's enclosing snapshots. So a single pass over all uses
    /// covers in-scope reads and closure captures alike. Uses sitting inside an
    /// NSE argument (`substitute(x)`, …) are skipped: they don't consume a
    /// binding.
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
    /// For the quoting calls oak doesn't model, an NSE assignment
    /// (`substitute(x <- 2)`) is quoted code, not an executed assignment, but
    /// oak still lets it shadow a prior real definition in its dataflow. So a
    /// real read after such an assignment resolves to the NSE definition
    /// instead of the live binding it actually reads. When that happens, walk
    /// back to the nearest preceding real definition of the same symbol and
    /// mark it used instead.
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

    /// Resolve each position-aware read (interpolation) to the definition(s)
    /// it uses and record them in `reaching_used`, mirroring the real-use
    /// pass. Runs after the AST pass so the NSE ranges it consults are
    /// already collected.
    fn precompute_positional_uses(&mut self) {
        let uses = std::mem::take(&mut self.positional_uses);
        for (name, read_range) in &uses {
            self.mark_positional_use(name, read_range.start());
        }
    }

    /// Mark the definition(s) a position-aware read of `name` at `pos`
    /// resolves to.
    ///
    /// These reads are position-aware, so — unlike a blanket "name is used"
    /// marker — a *later* same-scope reassignment of the name isn't kept
    /// alive. Walk outward from the reading scope to the scope that binds
    /// `name` before the read:
    /// - in the reading scope itself, mark every definition that precedes the
    ///   read: more than one can reach it through branching control flow (e.g.
    ///   an `if`/`else` assigning in both arms), but a later reassignment is
    ///   excluded so it stays reported. If no definition precedes the read,
    ///   the read falls through to the enclosing scope, so keep walking;
    /// - in an enclosing scope, the read is a closure capture evaluated
    ///   later, so textual order is irrelevant and every definition of the
    ///   name there is kept alive.
    fn mark_positional_use(&mut self, name: &str, pos: TextSize) {
        let index = self.index;
        let (read_scope, _) = index.scope_at(pos);
        for owner in index.ancestor_scope_ids(read_scope) {
            let Some(symbol_id) = index.symbols(owner).id(name) else {
                continue;
            };
            let captured = owner != read_scope;
            let reached: Vec<DefinitionId> = index
                .definitions(owner)
                .iter()
                .filter(|(_, def)| def.symbol() == symbol_id && !self.is_in_nse(def.range()))
                .filter(|(_, def)| captured || def.range().start() < pos)
                .map(|(id, _)| id)
                .collect();
            // `name` may be referenced but not bound (before `pos`) in this
            // scope; if so, keep walking outward to the scope whose binding
            // the read actually consumes.
            if reached.is_empty() {
                continue;
            }
            for def_id in reached {
                self.reaching_used.insert((owner, def_id));
            }
            return;
        }
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

/// The interpolation dialect a string literal is written in.
#[derive(Clone, Copy)]
enum InterpolationFlavor {
    /// glue's plain `{...}` interpolation.
    Glue,
    /// glue interpolation plus cli's inline markup spans (`{.field {x}}`).
    CliMarkup,
}

/// The interpolation dialect applying to `node`, or `None` when no enclosing
/// call interpolates its arguments — a string is only code when it is handed
/// to a glue, stringr or cli function. Walks all ancestors (not just the
/// immediate call) so message strings nested in a `c(...)` bullets vector
/// still count.
fn interpolation_flavor(node: &RSyntaxNode) -> Option<InterpolationFlavor> {
    node.ancestors()
        .filter_map(|ancestor| ancestor.cast::<RCall>())
        .find_map(|call| {
            let name = call_name(&call)?;
            if is_cli_markup_function(&name) {
                Some(InterpolationFlavor::CliMarkup)
            } else if is_glue_interpolating_function(&name) {
                Some(InterpolationFlavor::Glue)
            } else {
                None
            }
        })
}

/// glue and stringr functions that glue-interpolate their string arguments.
/// Excludes the non-interpolating helpers of both packages (`as_glue`,
/// `glue_collapse`, `str_c`, …). `str_interp()` uses `${...}` rather than
/// `{...}`, but scanning for `{`/`}` still finds its expressions.
/// Namespaced calls (`glue::glue`) resolve to the bare name via [`call_name`].
fn is_glue_interpolating_function(name: &str) -> bool {
    matches!(
        name,
        "glue"
            | "glue_data"
            | "glue_col"
            | "glue_data_col"
            | "glue_safe"
            | "glue_data_safe"
            | "glue_sql"
            | "glue_data_sql"
            | "str_glue"
            | "str_glue_data"
            | "str_interp"
    )
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

/// The value node of the quoted-expression argument (`expr`) of a quote-like
/// call: the argument named `expr =` if present, otherwise the first
/// positional (unnamed) argument. Other arguments — `substitute`'s `env` —
/// are evaluated normally, so their reads must not be swallowed as NSE.
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

fn argument_name(arg: &RArgument) -> Option<String> {
    let clause = arg.name_clause()?;
    let name = clause.name().ok()?;
    match name {
        AnyRArgumentName::RIdentifier(ident) => Some(ident.name_text()),
        AnyRArgumentName::RDots(_) => Some("...".to_string()),
        _ => None,
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
        // A call-created binding (`assign("x", …)`, `x %<>% f()`): the name
        // expression is the goto/report anchor.
        DefinitionKind::Assign { name, .. } => {
            return Some(name.to_node(root).syntax().text_trimmed_range());
        }
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

/// `ImportsResolver` impl that plugs `source("path")` injection and effect
/// resolution into oak's builder.
///
/// The resolver parses the target file, builds its `SemanticIndex` with
/// another `JarlImportsResolver` (so `source()` chains resolve
/// transitively), and reports its top-level definitions — own and
/// forwarded — as `SourceResolution.names`. Oak then materialises
/// `DefinitionKind::Import` entries at the `source()` call site in the
/// calling file's index.
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

/// Packages whose effect annotations jarl resolves even without a `library()`
/// call in the file, in shadowing order (base last). Effects power oak's NSE
/// model — `quote()` dropping its argument from the index, `local()` opening a
/// scope, `x %<>% f()` binding `x` — and jarl is deliberately lenient about
/// attachment, mirroring how it treats e.g. `test_that()` without requiring
/// `library(testthat)`. S7 is deliberately absent: resolving its `:=` operator
/// would turn data.table's column assignment (`DT[, x := y]`) into a variable
/// binding; S7 users still get it through an explicit `library(S7)` entering
/// the attached set.
const DEFAULT_EFFECT_PACKAGES: &[&str] = &["magrittr", "rlang", "testthat", "shiny", "base"];

impl oak_semantic::ImportsResolver for JarlImportsResolver {
    fn resolve_effects(
        &mut self,
        name: &str,
        attached: &[String],
    ) -> Option<oak_semantic::EffectsHandlers> {
        // Packages attached in the file (flow order, latest last) shadow the
        // lenient defaults, so walk them LIFO first.
        attached
            .iter()
            .rev()
            .map(String::as_str)
            .chain(DEFAULT_EFFECT_PACKAGES.iter().copied())
            .find_map(|pkg| oak_semantic::effects::lookup(pkg, name))
            .copied()
    }

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
