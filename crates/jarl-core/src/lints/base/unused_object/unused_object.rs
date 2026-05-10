use std::collections::{HashMap, HashSet};

use air_r_parser::RParserOptions;
use air_r_syntax::{
    AnyRArgumentName, AnyRExpression, RArgument, RArgumentList, RBinaryExpression, RCall,
    RExtractExpression, RNamespaceExpression, RSyntaxKind, RSyntaxNode,
};
use biome_rowan::{AstNode, AstSeparatedList, SyntaxNodeCast, TextRange, TextSize};
use oak_core::syntax_ext::RIdentifierExt;
use oak_index::DefinitionId;
use oak_index::semantic_index::{Definition, DefinitionKind, ScopeId, SemanticIndex, SymbolId};

use crate::checker::Checker;
use crate::diagnostic::{Diagnostic, Fix, ViolationData};

/// Version added: 0.6.0
///
/// ## What it does
///
/// Detects local variables assigned a value that is never read. Operates on
/// oak's per-file `SemanticIndex`: walks every scope, looks at each
/// definition, and emits a warning when no `Use` reaches it (directly or
/// through a closure).
///
/// ## Why is this bad?
///
/// Unused assignments are usually a sign of dead code or a bug. Removing them
/// reduces noise.
///
/// ## Examples
///
/// ```r
/// x <- 1   # unused
/// print(y)
/// ```
pub fn unused_object(
    expressions: &[RSyntaxNode],
    semantic: &SemanticIndex,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    let mut ctx = Ctx::new(semantic, &checker.namespace_exports);
    ctx.collect_ast_passes(expressions);

    let scopes = scope_ids(semantic);
    ctx.precompute_closure_uses(&scopes);

    let mut diagnostics = ctx.collect_diagnostics(&scopes);
    diagnostics.extend(ctx.collect_assignment_pipe_diagnostics(expressions));

    for d in diagnostics {
        checker.report_diagnostic(Some(d));
    }

    Ok(())
}

fn scope_ids(index: &SemanticIndex) -> Vec<ScopeId> {
    let mut ids = Vec::new();
    let mut stack = vec![ScopeId::from(0)];
    while let Some(s) = stack.pop() {
        ids.push(s);
        stack.extend(index.child_scopes(s));
    }
    ids
}

struct Ctx<'a> {
    index: &'a SemanticIndex,
    /// Names exported by the package's NAMESPACE. A top-level definition with
    /// one of these names is "used" by external callers and must not be
    /// flagged, even when nothing in the file reads it.
    exports: &'a HashSet<String>,
    /// Names that have a synthetic use from AST passes (string interpolation,
    /// `do.call("f", …)`, `..cols`, etc.). These short-circuit reports.
    synthetic_used_names: HashSet<String>,
    /// Identifier `Use` ranges that should be ignored because they sit inside
    /// an NSE call argument (`quote(x)`, `substitute(…)`, `bquote(…)`, …).
    nse_ranges: Vec<TextRange>,
    /// Bodies of `local({...})` calls. A definition inside one body that's
    /// also read inside the same body counts as used (oak doesn't model
    /// `local()` as an eager scope yet).
    local_body_ranges: Vec<TextRange>,
    /// Ranges of formula RHSes (`~ rhs`). `=` inside a formula is named-arg
    /// syntax, not assignment.
    formula_ranges: Vec<TextRange>,
    /// Definitions reachable through call-site analysis of nested closures.
    /// Populated by `precompute_closure_uses`. If `(scope_id, def_id)` is
    /// in this set, treat the definition as used by a closure call site.
    closure_used_defs: HashSet<(ScopeId, DefinitionId)>,
    /// Definitions in scopes containing an "escaped" closure: any closure
    /// referenced as something other than a direct call (returned, passed
    /// as argument, anonymous), forces all enclosing-symbol defs to be
    /// considered used.
    closure_escaped_symbols: HashSet<(ScopeId, SymbolId)>,
    /// Identifier ranges that appear as the callee of an `R_CALL`. Computed
    /// from the AST walk; consulted by call-site analysis.
    callee_ranges: Vec<TextRange>,
}

impl<'a> Ctx<'a> {
    fn new(index: &'a SemanticIndex, exports: &'a HashSet<String>) -> Self {
        Self {
            index,
            exports,
            synthetic_used_names: HashSet::new(),
            nse_ranges: Vec::new(),
            local_body_ranges: Vec::new(),
            formula_ranges: Vec::new(),
            closure_used_defs: HashSet::new(),
            closure_escaped_symbols: HashSet::new(),
            callee_ranges: Vec::new(),
        }
    }

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
    /// loop-condition uses. For any `<-`/`=`/`->` inside a loop body or
    /// condition, mark the LHS name as a synthetic use so the def isn't
    /// flagged. Coarse but matches the test contract.
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
        let Some(token) = node.first_token() else {
            return;
        };
        let text = token.text_trimmed();
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            // glue's `{{` / `}}` are escaped literals, not interpolations.
            if bytes[i] == b'{' && bytes.get(i + 1) == Some(&b'{') {
                i += 2;
                continue;
            }
            if bytes[i] == b'}' && bytes.get(i + 1) == Some(&b'}') {
                i += 2;
                continue;
            }
            if bytes[i] == b'{' {
                let start = i + 1;
                let mut depth = 1usize;
                let mut end = start;
                while end < bytes.len() && depth > 0 {
                    match bytes[end] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        _ => {}
                    }
                    if depth > 0 {
                        end += 1;
                    }
                }
                if depth == 0 && end > start {
                    self.collect_identifiers_in_interpolation(&text[start..end]);
                }
                i = end + 1;
            } else {
                i += 1;
            }
        }
    }

    /// Parse a glue-style `{...}` interpolation as R code and collect every
    /// identifier reference. Skips the field side of `x$a` / `x@a` and the
    /// namespace side of `pkg::name` — those name members, not bindings.
    fn collect_identifiers_in_interpolation(&mut self, src: &str) {
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
                self.synthetic_used_names
                    .insert(token.text_trimmed().to_string());
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
        // Formulas are `R_BINARY_EXPRESSION` with a `~` operator.
        if op_text == "~" {
            self.formula_ranges.push(bin.syntax().text_trimmed_range());
            self.nse_ranges.push(bin.syntax().text_trimmed_range());
            return;
        }

        // Short-circuit operators: `cond || (x <- 2)` may skip the
        // assignment entirely, so prior defs of `x` should remain alive.
        // Oak walks linearly and shadows them. Workaround: any LHS
        // assigned inside a `||`/`&&` is considered synthetically used,
        // which stops us from flagging earlier defs of the same name.
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
        // Record the callee position so call-site analysis can distinguish
        // `f` (call) from `f` (read/return).
        if let Ok(func) = call.function() {
            self.callee_ranges.push(func.syntax().text_trimmed_range());
        }

        let Some(name) = call_name(call) else {
            return;
        };

        let arg_values: Vec<(Option<String>, RSyntaxNode)> = call_args(call);

        match name.as_str() {
            "quote" | "substitute" | "bquote" | "enquote" | "expression" | "Quote" => {
                for (_, value) in &arg_values {
                    self.nse_ranges.push(value.text_trimmed_range());
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
            // 4.3 `on.exit(body)` is a deferred body. The simplest
            // approximation that satisfies the tests: any identifier read
            // inside an `on.exit` body counts as a use, regardless of source
            // order, because the body runs at function exit. Implement this
            // by NOT marking those ranges NSE — oak already records them as
            // uses, and the use-def map already considers them as such.
            // The remaining gap is that source-order use-def can mark a
            // definition before `on.exit` as dead even though the on.exit
            // body reads it later. Patch: when an identifier is read inside
            // an on.exit body, ALSO record it as a synthetic_used_names
            // entry, so a same-name definition anywhere in the function is
            // considered used.
            "on.exit" => {
                if let Some((_, body)) = arg_values.first() {
                    self.collect_on_exit_uses(body);
                }
            }
            _ => {}
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

    fn collect_diagnostics(&self, scopes: &[ScopeId]) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        let top_level = ScopeId::from(0);
        for &scope_id in scopes {
            for (def_id, def) in self.index.definitions(scope_id).iter() {
                if !self.should_lint_definition(def) {
                    continue;
                }
                if self.is_definition_used(scope_id, def_id, def) {
                    continue;
                }
                if scope_id == top_level && self.is_exported(scope_id, def) {
                    continue;
                }
                out.push(self.make_diagnostic(scope_id, def));
            }
        }
        out
    }

    fn is_exported(&self, scope_id: ScopeId, def: &Definition) -> bool {
        if self.exports.is_empty() {
            return false;
        }
        let name = self.index.symbols(scope_id).symbol_id(def.symbol()).name();
        self.exports.contains(name)
    }

    /// Workaround for oak not recognising `%<>%` as an assignment. Walk the
    /// AST for `x %<>% f(...)` expressions; if no later use of `x` exists
    /// in the same scope (or via closure capture), emit a synthetic
    /// "unused_object" diagnostic on the LHS identifier.
    fn collect_assignment_pipe_diagnostics(&self, expressions: &[RSyntaxNode]) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for expr in expressions {
            for node in expr.descendants() {
                if node.kind() != RSyntaxKind::R_BINARY_EXPRESSION {
                    continue;
                }
                let Some(bin) = node.clone().cast::<RBinaryExpression>() else {
                    continue;
                };
                let Ok(op) = bin.operator() else { continue };
                if op.text_trimmed() != "%<>%" {
                    continue;
                }
                let Ok(lhs) = bin.left() else { continue };
                if lhs.syntax().kind() != RSyntaxKind::R_IDENTIFIER {
                    continue;
                }
                let Some(ident) = lhs.syntax().clone().cast::<air_r_syntax::RIdentifier>() else {
                    continue;
                };
                let name = ident.name_text();
                let lhs_range = lhs.syntax().text_trimmed_range();
                let expr_end = bin.syntax().text_trimmed_range().end();

                // Find the scope containing this expression.
                let (scope_id, _) = self.index.scope_at(lhs_range.start());

                // Look for any later use of `name` in this scope (after
                // `expr_end`).
                let symbol = self.index.symbols(scope_id).id(&name);
                let later_use = symbol.is_some_and(|sym| {
                    self.index
                        .uses(scope_id)
                        .iter()
                        .any(|(_, u)| u.symbol() == sym && u.range().start() >= expr_end)
                });
                // Also check escape via closures.
                let closure_use = symbol
                    .is_some_and(|sym| self.closure_escaped_symbols.contains(&(scope_id, sym)));
                // Top-level exported names are used by external callers.
                let exported = scope_id == ScopeId::from(0) && self.exports.contains(&name);

                if !later_use && !closure_use && !exported {
                    out.push(Diagnostic::new(
                        ViolationData::new(
                            "unused_object".to_string(),
                            format!("Object `{name}` is defined but never used."),
                            None,
                        ),
                        lhs_range,
                        Fix::empty(),
                    ));
                }
            }
        }
        out
    }

    fn should_lint_definition(&self, def: &Definition) -> bool {
        match def.kind() {
            DefinitionKind::Parameter(_)
            | DefinitionKind::ForVariable(_)
            | DefinitionKind::SuperAssignment(_)
            | DefinitionKind::Import { .. } => return false,
            DefinitionKind::Assignment(node) => {
                // 4.10 RHS is a function literal: never lint.
                if assignment_rhs_is_function_def(node) {
                    return false;
                }
                // 4.1 Replacement-function or subset assignment LHS
                // (e.g. `names(x) <-`, `x[1] <-`, `x$a <-`): skip — the LHS
                // construct reads x and the test contract is that the
                // surrounding `x <- list()` is still considered used.
                if assignment_lhs_is_complex(node) {
                    return false;
                }
            }
        }

        // 4.9 Formula `=` is not assignment.
        if self.in_any_range(def.range(), &self.formula_ranges) {
            return false;
        }

        true
    }

    fn is_definition_used(
        &self,
        scope_id: ScopeId,
        def_id: DefinitionId,
        def: &Definition,
    ) -> bool {
        let symbol_name = self.index.symbols(scope_id).symbol_id(def.symbol()).name();

        // AST-derived synthetic uses (string interp, do.call, ..cols, on.exit)
        if self.synthetic_used_names.contains(symbol_name) {
            return true;
        }

        if self.is_reached_by_use_in_scope(scope_id, def_id, def.symbol()) {
            return true;
        }

        // 4.5 Closure escape + call-site analysis.
        if self.is_used_via_closure(scope_id, def_id, def.symbol()) {
            return true;
        }

        // 4.11 `local({…})` shortcut.
        if self.is_used_inside_local_body(scope_id, def) {
            return true;
        }

        false
    }

    fn is_reached_by_use_in_scope(
        &self,
        scope_id: ScopeId,
        def_id: DefinitionId,
        symbol: SymbolId,
    ) -> bool {
        let use_def = self.index.use_def_map(scope_id);
        let uses = self.index.uses(scope_id);
        for (use_id, u) in uses.iter() {
            if u.symbol() != symbol {
                continue;
            }
            if self.in_any_range(u.range(), &self.nse_ranges) {
                continue;
            }
            let bindings = use_def.bindings_at_use(use_id);
            if bindings.contains_definition(def_id) {
                return true;
            }
        }
        false
    }

    fn is_used_via_closure(
        &self,
        scope_id: ScopeId,
        def_id: DefinitionId,
        symbol: SymbolId,
    ) -> bool {
        if self.closure_escaped_symbols.contains(&(scope_id, symbol)) {
            return true;
        }
        self.closure_used_defs.contains(&(scope_id, def_id))
    }

    /// For each function scope F that's a child of P, classify F's name uses
    /// in P into call sites vs. escapes. If F escapes, mark its free vars
    /// in P as escaped (all defs of those names get a free pass). If F has
    /// only call-site uses, find the latest reaching definition of each
    /// free variable at each call site (in source order) and mark those.
    fn precompute_closure_uses(&mut self, scopes: &[ScopeId]) {
        // Map child Function scope → its bound name in the parent scope (if
        // any) and the parent ScopeId.
        let mut parent_of: HashMap<ScopeId, ScopeId> = HashMap::new();
        for &scope in scopes {
            for child in self.index.child_scopes(scope) {
                parent_of.insert(child, scope);
            }
        }

        for &child in scopes {
            let Some(&parent) = parent_of.get(&child) else {
                continue;
            };
            self.classify_closure(parent, child);
        }
    }

    fn classify_closure(&mut self, parent: ScopeId, child: ScopeId) {
        // Find the binding name of `child` in `parent`. Heuristic: an
        // assignment in `parent` whose syntax range exactly contains
        // `child.range()` and whose RHS is a function definition. If we
        // can't find one, treat the closure as anonymous → escape.
        let child_range = self.index.scope(child).range();

        let mut binding_name: Option<String> = None;
        for (_, def) in self.index.definitions(parent).iter() {
            if let DefinitionKind::Assignment(node) = def.kind()
                && node.text_trimmed_range().contains_range(child_range)
                && assignment_rhs_is_function_def(node)
            {
                let name = self
                    .index
                    .symbols(parent)
                    .symbol_id(def.symbol())
                    .name()
                    .to_string();
                binding_name = Some(name);
                break;
            }
        }

        let Some(name) = binding_name else {
            // Anonymous closure: any free variable's enclosing-scope defs
            // must be conservatively considered used.
            self.escape_free_vars(parent, child);
            return;
        };

        // Find this name in the parent's symbol table.
        let Some(parent_symbol) = self.index.symbols(parent).id(&name) else {
            self.escape_free_vars(parent, child);
            return;
        };

        // Collect call-site offsets and detect escapes by walking parent
        // uses of `parent_symbol`.
        let mut call_offsets: Vec<TextSize> = Vec::new();
        let mut escaped = false;
        for (_, u) in self.index.uses(parent).iter() {
            if u.symbol() != parent_symbol {
                continue;
            }
            if self.use_is_callee(u.range()) {
                call_offsets.push(u.range().end());
            } else {
                escaped = true;
                break;
            }
        }

        if escaped || call_offsets.is_empty() {
            // If no calls, the closure is bound but never invoked here.
            // Conservative: treat as escaped (could be returned implicitly,
            // exported, etc.). This matches the test contract for
            // `f <- function() { x <- 1; f2 <- function() x; f2 }` where
            // f2 is returned and we want to keep `x <- 1` alive.
            self.escape_free_vars(parent, child);
            return;
        }

        // For each free variable of `child`: find reaching defs in `parent`
        // at each call offset, and mark them used.
        let free_vars = self.free_variables(child, parent);
        for parent_sym in free_vars {
            let parent_defs: Vec<(DefinitionId, TextSize)> = self
                .index
                .definitions(parent)
                .iter()
                .filter(|(_, d)| d.symbol() == parent_sym)
                .map(|(id, d)| (id, d.range().end()))
                .collect();
            for &offset in &call_offsets {
                if let Some((def_id, _)) = parent_defs
                    .iter()
                    .filter(|(_, def_end)| *def_end <= offset)
                    .max_by_key(|(_, def_end)| *def_end)
                {
                    self.closure_used_defs.insert((parent, *def_id));
                }
            }
        }
    }

    fn escape_free_vars(&mut self, parent: ScopeId, child: ScopeId) {
        for parent_sym in self.free_variables(child, parent) {
            self.closure_escaped_symbols.insert((parent, parent_sym));
        }
    }

    /// Names that the child scope (or any nested descendant of it)
    /// references but doesn't bind locally — i.e. free variables that
    /// resolve to the parent scope. Returns the corresponding parent
    /// `SymbolId`s.
    fn free_variables(&self, child: ScopeId, parent: ScopeId) -> Vec<SymbolId> {
        let mut out = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for descendant in self.descendants(child) {
            for (_, u) in self.index.uses(descendant).iter() {
                let name = self
                    .index
                    .symbols(descendant)
                    .symbol_id(u.symbol())
                    .name()
                    .to_string();
                if !seen.insert(name.clone()) {
                    continue;
                }
                if let Some(parent_sym) = self.index.symbols(parent).id(&name) {
                    out.push(parent_sym);
                }
            }
        }
        out
    }

    /// Is the given identifier-use range the callee of an `R_CALL`? We
    /// approximate by walking the AST starting from any descendant whose
    /// range matches; cheaper than tracking node IDs.
    fn use_is_callee(&self, range: TextRange) -> bool {
        // We can't access the syntax root from `Ctx`. Instead, rely on the
        // separately-tracked callee ranges populated during AST walk.
        self.callee_ranges
            .iter()
            .any(|r| r.contains_range(range) || *r == range)
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

    fn descendants(&self, scope_id: ScopeId) -> Vec<ScopeId> {
        let mut out = vec![scope_id];
        let mut stack = vec![scope_id];
        while let Some(s) = stack.pop() {
            for child in self.index.child_scopes(s) {
                out.push(child);
                stack.push(child);
            }
        }
        out
    }

    fn in_any_range(&self, target: TextRange, ranges: &[TextRange]) -> bool {
        ranges.iter().any(|r| r.contains_range(target))
    }

    fn make_diagnostic(&self, scope_id: ScopeId, def: &Definition) -> Diagnostic {
        let name = self
            .index
            .symbols(scope_id)
            .symbol_id(def.symbol())
            .name()
            .to_string();
        let range = lhs_range_for_definition(def).unwrap_or_else(|| def.range());
        Diagnostic::new(
            ViolationData::new(
                "unused_object".to_string(),
                format!("Object `{name}` is defined but never used."),
                None,
            ),
            range,
            Fix::empty(),
        )
    }
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

fn assignment_rhs_is_function_def(node: &RSyntaxNode) -> bool {
    // The RHS of a `<-` / `=` / `->` binary expression. We just look for
    // any `R_FUNCTION_DEFINITION` direct child — `RBinaryExpression`'s left
    // and right slots are themselves `R_FUNCTION_DEFINITION` if applicable.
    for child in node.children() {
        if child.kind() == RSyntaxKind::R_FUNCTION_DEFINITION {
            return true;
        }
    }
    false
}

fn assignment_lhs_is_complex(node: &RSyntaxNode) -> bool {
    // For binary assignment, peek at the LHS and treat anything other than
    // a bare identifier as "complex" (e.g. `names(x)`, `x[1]`, `x$a`).
    // For `->`/`->>` assignments the LHS is the rightmost side; we treat
    // those identically.
    let Some(bin) = node.clone().cast::<RBinaryExpression>() else {
        return false;
    };
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

fn lhs_range_for_definition(def: &Definition) -> Option<TextRange> {
    let node = match def.kind() {
        DefinitionKind::Assignment(n) | DefinitionKind::SuperAssignment(n) => n,
        DefinitionKind::Parameter(n) | DefinitionKind::ForVariable(n) => {
            return Some(n.text_trimmed_range());
        }
        DefinitionKind::Import { .. } => return None,
    };
    let bin = node.clone().cast::<RBinaryExpression>()?;
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
        // Fall back to the whole assignment range if the LHS isn't a
        // simple identifier.
        None
    }
}

fn call_name(call: &RCall) -> Option<String> {
    let func = call.function().ok()?;
    match func {
        AnyRExpression::RIdentifier(ident) => Some(ident.name_text()),
        AnyRExpression::RNamespaceExpression(ns) => {
            // Match on the right-hand identifier (`pkg::name` ⇒ `name`).
            ns.right()
                .ok()
                .and_then(|r| r.syntax().first_token())
                .map(|t| t.text_trimmed().to_string())
        }
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

/// Return the LHS identifier name of a binary assignment expression
/// (`x <- …`, `x = …`, `… -> x`, `x <<- …`, `… ->> x`). Returns None for
/// any other binary expression or for assignments whose LHS isn't a bare
/// identifier.
fn assignment_lhs_name(node: &RSyntaxNode) -> Option<String> {
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
    if node.kind() != RSyntaxKind::R_STRING_VALUE {
        return None;
    }
    let token = node.first_token()?;
    let text = token.text_trimmed();
    let bytes = text.as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    let first = bytes[0];
    let last = *bytes.last().unwrap();
    if (first == b'"' || first == b'\'') && first == last {
        Some(text[1..text.len() - 1].to_string())
    } else {
        None
    }
}
