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

/// Per-file semantic info derived from oak's [`SemanticIndex`] plus AST
/// passes over the syntax tree. Computed once per file; consumed by lints.
pub struct SemanticInfo<'a> {
    index: &'a SemanticIndex,
    /// Path of the file being analyzed. Used to resolve `source("path")`
    /// arguments against the current file's directory.
    file: &'a std::path::Path,
    /// Names that have a synthetic use from AST passes (string interpolation,
    /// `do.call("f", …)`, `..cols`, `on.exit` bodies, loop assignment LHSes,
    /// short-circuit assignment LHSes). A definition whose symbol name is in
    /// this set is treated as used.
    synthetic_used_names: HashSet<String>,
    /// Identifier `Use` ranges that should be ignored because they sit inside
    /// an NSE call argument (`quote(x)`, `substitute(…)`, `bquote(…)`, …).
    nse_ranges: Vec<TextRange>,
    /// Bodies of `local({...})` calls.
    local_body_ranges: Vec<TextRange>,
    /// Ranges of formula RHSes (`~ rhs`).
    formula_ranges: Vec<TextRange>,
    /// Definitions reachable through call-site analysis of nested closures.
    closure_used_defs: HashSet<(ScopeId, DefinitionId)>,
    /// Symbols whose nested closure escapes (returned/anonymous/passed as
    /// argument); all enclosing-scope defs of that symbol are conservatively
    /// considered used.
    closure_escaped_symbols: HashSet<(ScopeId, SymbolId)>,
    /// Identifier ranges that appear as the callee of an `R_CALL`.
    callee_ranges: Vec<TextRange>,
}

impl<'a> SemanticInfo<'a> {
    /// Build the info table. Runs both the AST pass (collecting synthetic
    /// uses, NSE ranges, formula ranges, local body ranges, callee ranges)
    /// and the closure call-site analysis.
    pub fn build(
        expressions: &[RSyntaxNode],
        index: &'a SemanticIndex,
        file: &'a std::path::Path,
    ) -> Self {
        let mut this = Self {
            index,
            file,
            synthetic_used_names: HashSet::new(),
            nse_ranges: Vec::new(),
            local_body_ranges: Vec::new(),
            formula_ranges: Vec::new(),
            closure_used_defs: HashSet::new(),
            closure_escaped_symbols: HashSet::new(),
            callee_ranges: Vec::new(),
        };
        this.collect_ast_passes(expressions);
        let scopes = this.scope_ids();
        this.precompute_closure_uses(&scopes);
        this
    }

    pub fn index(&self) -> &SemanticIndex {
        self.index
    }

    /// Walk all scopes (root + descendants) in arbitrary order.
    pub fn scope_ids(&self) -> Vec<ScopeId> {
        let mut ids = Vec::new();
        let mut stack = vec![ScopeId::from(0)];
        while let Some(s) = stack.pop() {
            ids.push(s);
            stack.extend(self.index.child_scopes(s));
        }
        ids
    }

    // ── High-level queries ────────────────────────────────────────────

    /// True if any of the supported "is used" conditions hold for this
    /// definition: synthetic AST-derived use, in-scope reaching use,
    /// closure-escape, or `local({…})` body shortcut.
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
        if self.is_reached_by_use_in_scope(scope_id, def_id, def.symbol()) {
            return true;
        }
        if self.is_used_via_closure(scope_id, def_id, def.symbol()) {
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

    pub fn is_in_nse(&self, range: TextRange) -> bool {
        in_any_range(range, &self.nse_ranges)
    }

    pub fn has_synthetic_use(&self, name: &str) -> bool {
        self.synthetic_used_names.contains(name)
    }

    pub fn closure_escaped(&self, scope: ScopeId, symbol: SymbolId) -> bool {
        self.closure_escaped_symbols.contains(&(scope, symbol))
    }

    pub fn is_callee(&self, range: TextRange) -> bool {
        self.callee_ranges
            .iter()
            .any(|r| r.contains_range(range) || *r == range)
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

    // ── Internal: reach / closure analysis ────────────────────────────

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
            if in_any_range(u.range(), &self.nse_ranges) {
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

    fn precompute_closure_uses(&mut self, scopes: &[ScopeId]) {
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
                    .symbol(def.symbol())
                    .name()
                    .to_string();
                binding_name = Some(name);
                break;
            }
        }

        let Some(name) = binding_name else {
            self.escape_free_vars(parent, child);
            return;
        };

        let Some(parent_symbol) = self.index.symbols(parent).id(&name) else {
            self.escape_free_vars(parent, child);
            return;
        };

        let mut call_offsets: Vec<TextSize> = Vec::new();
        let mut escaped = false;
        for (_, u) in self.index.uses(parent).iter() {
            if u.symbol() != parent_symbol {
                continue;
            }
            if self.is_callee(u.range()) {
                call_offsets.push(u.range().end());
            } else {
                escaped = true;
                break;
            }
        }

        if escaped || call_offsets.is_empty() {
            self.escape_free_vars(parent, child);
            return;
        }

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

    fn free_variables(&self, child: ScopeId, parent: ScopeId) -> Vec<SymbolId> {
        let mut out = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for descendant in self.scope_descendants(child) {
            for (_, u) in self.index.uses(descendant).iter() {
                let name = self
                    .index
                    .symbols(descendant)
                    .symbol(u.symbol())
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
            for child in self.index.child_scopes(s) {
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

/// True if the RHS of a binary assignment is a function definition.
pub fn assignment_rhs_is_function_def(node: &RSyntaxNode) -> bool {
    for child in node.children() {
        if child.kind() == RSyntaxKind::R_FUNCTION_DEFINITION {
            return true;
        }
    }
    false
}

/// True if the LHS of a binary assignment is anything other than a bare
/// identifier (e.g. `names(x)`, `x[1]`, `x$a`).
pub fn assignment_lhs_is_complex(node: &RSyntaxNode) -> bool {
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

/// The text range of the bare-identifier LHS of an assignment, if any.
pub fn lhs_range_for_definition(def: &Definition) -> Option<TextRange> {
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
        None
    }
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

/// Resolve a `source("path")` argument against the currently-analyzed file.
/// Absolute paths are taken as-is; relative paths are joined to the
/// directory containing the analyzed file. When the analyzed file has no
/// parent directory (e.g. just a bare filename), the relative path is
/// returned as-is — `std::fs` will resolve it against the process CWD.
fn resolve_sourced_path(current_file: &std::path::Path, path: &str) -> Option<std::path::PathBuf> {
    let candidate = std::path::Path::new(path);
    if candidate.is_absolute() {
        return Some(candidate.to_path_buf());
    }
    match current_file.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => Some(dir.join(candidate)),
        _ => Some(candidate.to_path_buf()),
    }
}
