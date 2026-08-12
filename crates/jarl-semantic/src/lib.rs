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
    AnyRArgumentName, AnyRExpression, RArgument, RBinaryExpression, RCall, RForStatement,
    RStringValue, RSyntaxKind, RSyntaxNode,
};
use biome_rowan::{AstNode, AstSeparatedList, SyntaxNodeCast, TextRange, TextSize};
use oak_core::syntax_ext::{AnyRSelectorExt, RIdentifierExt, RStringValueExt};
use oak_semantic::DefinitionId;
use oak_semantic::effects::CallContext;
use oak_semantic::semantic_index::{
    Definition, DefinitionKind, ScopeId, SemanticCallKind, SemanticIndex,
};

/// Run-wide memo of semantic indices built for `source()` targets, keyed by
/// absolutized path.
///
/// Both sides of `source()` handling build the target's index — the resolver
/// for its exported names, [`SemanticInfo`] for its free uses — and several
/// files commonly source the same helper, so without sharing the same target
/// gets re-parsed and re-indexed once per consumer. Cloning shares the
/// underlying map; every resolver and `SemanticInfo` in one run should hold a
/// clone of the same cache.
///
/// Purely a performance layer: cycle handling still runs per source-chain
/// (the resolver's visited set), so a lookup never changes which names
/// resolve. One caveat: an index first built under a cycle-truncated chain
/// view is memoized as-is, so with cyclic `source()` graphs a later consumer
/// can see that truncated view where a fresh build would not — marginal, and
/// pinned by the cycle tests.
///
/// Fix mode rewrites files on disk between iterations, so callers must use a
/// fresh cache per iteration there (mirroring the `use_cached_index`
/// invalidation of per-file indices).
#[derive(Clone, Default)]
pub struct SourceIndexCache {
    inner: std::sync::Arc<
        std::sync::Mutex<
            std::collections::HashMap<std::path::PathBuf, std::sync::Arc<SemanticIndex>>,
        >,
    >,
}

impl std::fmt::Debug for SourceIndexCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceIndexCache")
            .field("entries", &self.inner.lock().unwrap().len())
            .finish()
    }
}

impl SourceIndexCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// The cached index for `key` (absolutized), if a build already completed.
    /// The lock is released before returning, so callers can build and insert
    /// without deadlocking; two threads racing on the same miss just build it
    /// twice (deterministically, so last-write-wins is fine).
    pub fn get(&self, key: &std::path::Path) -> Option<std::sync::Arc<SemanticIndex>> {
        self.inner.lock().unwrap().get(key).cloned()
    }

    pub fn insert(&self, key: std::path::PathBuf, index: std::sync::Arc<SemanticIndex>) {
        self.inner.lock().unwrap().insert(key, index);
    }
}

/// The semantic index of a `source()` target: the memoized one, or a fresh
/// parse and build memoized on the way out. `key` is the absolutized cache
/// key, `path` the target as spelled at the call site (what gets read), and
/// `resolver` supplies the imports resolver the target's own `source()` calls
/// are followed with.
///
/// Both sides of `source()` handling come through here, so a target is parsed
/// and indexed once. Whether a target should be followed *at all* is not
/// decided here: that stays with the per-chain visited set in
/// [`JarlImportsResolver::resolve_source`], and this is only reached for
/// targets it already let through.
fn source_target_index(
    cache: &SourceIndexCache,
    key: &std::path::Path,
    path: &std::path::Path,
    resolver: impl FnOnce(&std::path::Path) -> JarlImportsResolver,
) -> Option<std::sync::Arc<SemanticIndex>> {
    if let Some(index) = cache.get(key) {
        return Some(index);
    }
    let contents = std::fs::read_to_string(path).ok()?;
    let parsed = air_r_parser::parse(&contents, RParserOptions::default());
    if parsed.has_error() {
        return None;
    }
    let index = std::sync::Arc::new(oak_semantic::build_index(&parsed.tree(), resolver(path)));
    cache.insert(key.to_path_buf(), std::sync::Arc::clone(&index));
    Some(index)
}

/// Per-file semantic info derived from oak's [`SemanticIndex`] plus AST
/// passes over the syntax tree. Computed once per file; consumed by lints.
pub struct SemanticInfo<'a> {
    index: &'a SemanticIndex,
    /// Root syntax node of the analyzed file. Needed to resolve
    /// `AstPtr` references stored in [`DefinitionKind`] back to nodes.
    root: RSyntaxNode,
    /// Shared memo of `source()` target indices for this run.
    source_cache: SourceIndexCache,
    /// Assignments sitting inside a short-circuit operand (`cond || (x <- 2)`),
    /// stored as `(name, assignment range)` and resolved in
    /// [`Self::precompute_short_circuit_defs`].
    short_circuit_defs: Vec<(String, TextRange)>,
    /// Assignments of one name spread over sibling arguments of a call
    /// (`ifelse(cond, w <- 1, w <- 2)`), stored as `(name, argument range)`
    /// and resolved in [`Self::precompute_branching_defs`]. These are
    /// alternatives rather than redefinitions, so none of them is dead.
    branching_defs: Vec<(String, TextRange)>,
    /// Ranges re-entered by a loop's back edge, used in
    /// [`Self::precompute_loop_back_edges`]: the whole statement for
    /// `while`/`repeat`, only the body for `for`.
    loop_ranges: Vec<TextRange>,
    /// Position-aware reads collected by the AST pass — reads oak's index
    /// doesn't record: string interpolation (`glue("{x}")`, cli markup,
    /// custom delimiters), lookups by name (`do.call("f", …)`, data.table's
    /// `..cols`), custom infix operators, and the names a `source()`d file
    /// reads out of the caller's environment. Stored as `(name, read range)`
    /// pairs and resolved in [`Self::precompute_positional_uses`]: a read
    /// resolves to the definition it actually sees, so a *later* same-scope
    /// reassignment of the name is not kept alive by it.
    positional_uses: Vec<(String, TextRange)>,
    /// Identifier `Use` ranges that should be ignored because they sit inside
    /// a quoting call oak's effects registry doesn't cover (`Quote(…)`,
    /// `expression(…)`, `alist(…)`). `quote()`, `bquote()` and `substitute()`
    /// are modeled by oak itself (their quoted arguments produce no uses or
    /// definitions in the index), so they don't need ranges here.
    nse_ranges: Vec<TextRange>,
    /// Packages this file can reach: attached with `library()`/`require()`,
    /// listed in DESCRIPTION when linting package code, or named in a
    /// `pkg::fn()` call. Package-specific idioms (`{x}` interpolation,
    /// data.table's `..name`) are only honoured when their package is here,
    /// so a file that never mentions cli doesn't get cli's markup rules.
    available_packages: HashSet<String>,
    /// Ranges of formula RHSes (`~ rhs`).
    formula_ranges: Vec<TextRange>,
    /// Whether any package providing `{...}` interpolation is in reach.
    /// Computed once so a file that uses none of them skips the ancestor walk
    /// [`interpolation_flavor`] does for every string literal.
    has_any_interpolation_package: bool,
    /// Definitions reached by some non-NSE use anywhere in the file. Computed
    /// from oak's `reaching_definitions`, which resolves both local uses and
    /// free-variable uses in nested closures (via enclosing snapshots).
    reaching_used: HashSet<(ScopeId, DefinitionId)>,
}

impl<'a> SemanticInfo<'a> {
    /// Build the info table. Runs the AST pass (collecting synthetic uses,
    /// interpolation reads, NSE ranges, formula ranges) and then the
    /// reaching-use precomputation over oak's use-def maps.
    pub fn build(
        root: &RSyntaxNode,
        expressions: &[RSyntaxNode],
        index: &'a SemanticIndex,
        source_cache: &SourceIndexCache,
        loaded_packages: &[String],
    ) -> Self {
        // `pkg::fn()` reaches a package without attaching it, so namespaced
        // accesses count alongside what the caller resolved from
        // `library()` calls and DESCRIPTION.
        let mut available_packages: HashSet<String> = loaded_packages.iter().cloned().collect();
        available_packages.extend(
            index
                .namespace_accesses()
                .iter()
                .map(|access| access.package().to_string()),
        );
        let has_any_interpolation_package = INTERPOLATION_PACKAGES
            .iter()
            .any(|package| available_packages.contains(*package));
        let mut this = Self {
            index,
            root: root.clone(),
            source_cache: source_cache.clone(),
            short_circuit_defs: Vec::new(),
            branching_defs: Vec::new(),
            loop_ranges: Vec::new(),
            positional_uses: Vec::new(),
            nse_ranges: Vec::new(),
            available_packages,
            formula_ranges: Vec::new(),
            has_any_interpolation_package,
            reaching_used: HashSet::new(),
        };
        this.collect_ast_passes(expressions);
        this.collect_sourced_file_uses();
        let scopes: Vec<ScopeId> = this.scope_ids().collect();
        this.precompute_reaching_uses(&scopes);
        // Before `precompute_positional_uses`, which consumes
        // `positional_uses`: the back-edge pass reads that list too, since an
        // interpolation inside a loop is read again on the next iteration.
        this.precompute_loop_back_edges(&scopes);
        this.precompute_positional_uses();
        this.precompute_branching_defs();
        this.precompute_short_circuit_defs();
        this
    }

    pub fn root(&self) -> &RSyntaxNode {
        &self.root
    }

    /// Walk all scopes (root + descendants), in source order with the file
    /// scope first — the order oak's index guarantees.
    pub fn scope_ids(&self) -> impl Iterator<Item = ScopeId> + '_ {
        self.index.scope_ids()
    }

    // ── High-level queries ────────────────────────────────────────────

    /// True when some use reaches this definition: a read oak's index
    /// records, a position-aware read the AST pass collected, or a loop back
    /// edge. Every one of them resolves through the scope chain, so a
    /// definition is only ever kept alive by a read that could actually see
    /// it.
    pub fn is_definition_used(&self, scope_id: ScopeId, def_id: DefinitionId) -> bool {
        self.reaching_used.contains(&(scope_id, def_id))
    }

    // ── Low-level predicates (compose for new rules) ──────────────────

    pub fn is_in_formula(&self, range: TextRange) -> bool {
        in_any_range(range, &self.formula_ranges)
    }

    /// Whether `package` is reachable from this file, and so whether its
    /// idioms should be recognised here.
    pub fn package_available(&self, package: &str) -> bool {
        self.available_packages.contains(package)
    }

    /// True when `range` sits in a quoted NSE context (`expression(...)`,
    /// `alist(...)`, …) where code is captured rather than evaluated, so
    /// neither an assignment nor a read there touches the live binding. Only
    /// covers the quoting calls oak's effects registry doesn't model —
    /// `quote()`/`bquote()`/`substitute()` arguments never enter the index in
    /// the first place.
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
            RSyntaxKind::R_IDENTIFIER => self.collect_dotdot_identifier(node),
            RSyntaxKind::R_BINARY_EXPRESSION => {
                if let Some(bin) = node.clone().cast::<RBinaryExpression>() {
                    self.visit_binary(&bin);
                }
            }
            // A `while` condition is re-evaluated on every iteration, so it
            // belongs to the back edge; a `for` sequence is evaluated once
            // before the loop starts, so only its body does.
            RSyntaxKind::R_WHILE_STATEMENT | RSyntaxKind::R_REPEAT_STATEMENT => {
                self.loop_ranges.push(node.text_trimmed_range());
            }
            RSyntaxKind::R_FOR_STATEMENT => {
                if let Some(body) = node
                    .clone()
                    .cast::<RForStatement>()
                    .and_then(|stmt| stmt.body().ok())
                {
                    self.loop_ranges.push(body.syntax().text_trimmed_range());
                }
            }
            _ => {}
        }
    }

    /// Only reached for `R_IDENTIFIER`: the lexer reserves `R_DOT_DOT_I` for
    /// `..<digits>` (`..1`), so `..cols` arrives here as a plain identifier.
    fn collect_dotdot_identifier(&mut self, node: &RSyntaxNode) {
        // `dt[, ..cols]` is data.table's "resolve this name in the calling
        // frame" prefix. Anywhere else `..cols` is just an identifier, and
        // reading it says nothing about a binding named `cols`.
        if !self.package_available("data.table") {
            return;
        }
        let Some(token) = node.first_token() else {
            return;
        };
        let text = token.text_trimmed();
        if let Some(stripped) = text.strip_prefix("..")
            && stripped
                .chars()
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '.')
        {
            // The lookup happens where `..cols` is written, so it reads the
            // binding live at that position.
            self.positional_uses
                .push((stripped.to_string(), node.text_trimmed_range()));
        }
    }

    fn collect_string_interpolation(&mut self, node: &RSyntaxNode) {
        // Only strings passed to an interpolating call are code: `{x}` in
        // `message("{x}")` is literal text and reads nothing. And the idiom
        // only applies when the package providing it is in reach, so
        // `glue("{x}")` in a file that never mentions glue stays literal too.
        // That last check is cheapest, so it comes first.
        if !self.has_any_interpolation_package {
            return;
        }
        let Some((flavor, package)) = interpolation_flavor(node) else {
            return;
        };
        if !self.package_available(package) {
            return;
        }
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
                    self.collect_interpolation_reads(segment, read_range);
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
                self.collect_interpolation_reads(segment, read_range);
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
    fn collect_custom_glue_interpolation(&mut self, call_name: &str, args: &[RArgument]) {
        let Some(package) = glue_interpolation_package(call_name) else {
            return;
        };
        if !self.package_available(package) {
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
        for arg in args {
            if arg.name_clause().is_some() {
                continue;
            }
            let Some(value) = arg.value() else {
                continue;
            };
            let value = value.syntax();
            if value.kind() != RSyntaxKind::R_STRING_VALUE {
                continue;
            }
            let Some(content) =
                strings::get_string_literal_contents(&value.text_trimmed().to_string())
            else {
                continue;
            };
            let read_range = value.text_trimmed_range();
            for segment in scan_interpolation_segments(&content, &open, &close) {
                self.collect_interpolation_reads(segment, read_range);
            }
        }
    }

    /// Parse a glue-style `{...}` interpolation as R code and record what it
    /// reads as position-aware reads at `read_range`.
    ///
    /// The snippet is indexed rather than scanned for identifiers, so it gets
    /// the same treatment as any other code: a member name (`x$a`, `pkg::x`)
    /// is not a read, and a name the snippet binds itself
    /// (`{sapply(v, function(a) a)}`) consumes nothing outside it.
    fn collect_interpolation_reads(&mut self, src: &str, read_range: TextRange) {
        let parsed = air_r_parser::parse(src, RParserOptions::default());
        if parsed.has_error() {
            return;
        }
        // A snippet can't `source()` anything or attach a package, so it has
        // nothing to resolve.
        let index = oak_semantic::build_index(&parsed.tree(), oak_semantic::NoopImportsResolver);
        self.positional_uses
            .extend(free_uses(&index).map(|name| (name, read_range)));
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
        // read at the call site. Only user-defined `%...%` bindings can match;
        // R's built-in operators have no local definition to keep alive.
        if op_text.starts_with('%') && op_text.ends_with('%') {
            self.positional_uses
                .push((op_text.to_string(), op.text_trimmed_range()));
        }

        // Short-circuit operators: `cond || (x <- 2)` may skip the assignment
        // entirely, so prior defs of `x` should remain alive. Record the
        // assignment; `precompute_short_circuit_defs` resolves which earlier
        // definitions it keeps alive.
        if op_text == "||" || op_text == "&&" || op_text == "|" || op_text == "&" {
            for descendant in bin.syntax().descendants() {
                if descendant.kind() == RSyntaxKind::R_BINARY_EXPRESSION
                    && let Some(name) = assignment_lhs_name(&descendant)
                {
                    self.short_circuit_defs
                        .push((name, descendant.text_trimmed_range()));
                }
            }
        }
    }

    /// Arguments are promises, so a call can evaluate one of them and not the
    /// others: `ifelse(cond, w <- 1, w <- 2)` and `switch(x, a = w <- 1, b =
    /// w <- 2)` are branches, not a sequence. Oak walks the arguments linearly,
    /// so the later assignment shadows the earlier one and a later read of `w`
    /// only keeps the last branch alive. Workaround: when the same symbol is
    /// assigned in two or more sibling arguments, they are alternatives rather
    /// than redefinitions, so none of them is dead. Recorded per argument
    /// range so the exemption covers exactly those assignments and not every
    /// binding of that name in the file.
    ///
    /// Keyed on the argument structure rather than on a list of callee names,
    /// so user-defined branching wrappers get the same treatment. Repeated
    /// assignments *within* one argument (`f({w <- 1; w <- 2})`) really are
    /// sequential and stay lintable.
    fn collect_branching_argument_assignments(&mut self, args: &[RArgument]) {
        if args.len() < 2 {
            return;
        }

        let mut assigning_args: std::collections::HashMap<String, Vec<TextRange>> =
            std::collections::HashMap::new();
        for arg in args {
            let Some(value) = arg.value() else {
                continue;
            };
            let value = value.syntax();
            let assigned: HashSet<String> = value
                .descendants()
                .filter(|d| d.kind() == RSyntaxKind::R_BINARY_EXPRESSION)
                .filter_map(|d| assignment_lhs_name(&d))
                .collect();
            for name in assigned {
                assigning_args
                    .entry(name)
                    .or_default()
                    .push(value.text_trimmed_range());
            }
        }
        for (name, ranges) in assigning_args {
            // One argument assigning the name is an ordinary definition; it
            // takes a sibling argument assigning it too for them to be
            // alternatives.
            if ranges.len() < 2 {
                continue;
            }
            for range in ranges {
                self.branching_defs.push((name.clone(), range));
            }
        }
    }

    fn visit_call(&mut self, call: &RCall) {
        let args = call_args(call);

        self.collect_branching_argument_assignments(&args);

        let Some(name) = call_name(call) else {
            return;
        };

        self.collect_custom_glue_interpolation(&name, &args);

        match name.as_str() {
            // Quoting calls oak's effects registry doesn't cover. (`quote`,
            // `bquote` and `substitute` are covered: oak drops their quoted
            // arguments from the index entirely, and walks the sub-expressions
            // that escape back to evaluation — `bquote`'s `.()` unquote holes,
            // the symbols `substitute` replaces from its frame — as real uses.)
            //
            // Only the quoted `expr` argument is NSE. Any other argument is
            // evaluated normally, so its identifiers stay real uses.
            "Quote" => {
                let bound = CallContext::default().bind_arguments(call, &["expr"]);
                if let Some(expr) = bound.get("expr") {
                    self.nse_ranges.push(expr.syntax().text_trimmed_range());
                }
            }
            // `expression(...)` and `alist(...)` quote every argument: their
            // values are stored unevaluated, so an assignment like
            // `alist(x <- 1)` is captured code, not a real definition of `x`.
            "expression" | "alist" => {
                for arg in &args {
                    if let Some(value) = arg.value() {
                        self.nse_ranges.push(value.syntax().text_trimmed_range());
                    }
                }
            }
            // A name looked up at the call site, so it reads the binding live
            // there just like an identifier would.
            "do.call" | "match.fun" | "Recall" | "getFunction" => {
                if let Some(value) = args.first().and_then(|arg| arg.value())
                    && let Some(s) = string_literal_value(value.syntax())
                {
                    self.positional_uses
                        .push((s, value.syntax().text_trimmed_range()));
                }
            }
            _ => {}
        }
    }

    /// Record the *free* uses of every file this one `source()`s.
    ///
    /// The call sites come from oak, which already extracted each `source()`
    /// path and resolved it to a URL while building the index. Re-detecting
    /// them here by callee name would skip the gating oak applies: a locally
    /// shadowed `source` (`source <- function(x) invisible(x)`) is not R's
    /// `source()` and reads nothing, and a non-literal `local =` argument
    /// (`source("helper.R", local = new.env())`) runs the target in an
    /// environment this file's bindings never reach.
    ///
    /// `resolved` is `None` when oak could not pin the target down — a
    /// computed path, a missing or unparseable file, a directory-sourcing
    /// idiom this resolver doesn't handle, or a target the `source()` chain
    /// already visited. Skipping those keeps the chain's cycle guard
    /// authoritative instead of reading the file behind its back.
    fn collect_sourced_file_uses(&mut self) {
        let targets: Vec<(std::path::PathBuf, TextRange)> = self
            .index
            .semantic_calls()
            .iter()
            .filter_map(|call| match call.kind() {
                SemanticCallKind::Source { resolved, .. } => {
                    resolved.as_ref().map(|url| (url, call.range()))
                }
                _ => None,
            })
            .filter_map(|(url, range)| Some((url.to_file_path().ok()?, range)))
            .collect();
        for (target, call_range) in &targets {
            self.import_uses_from_sourced_file(target, *call_range);
        }
    }

    /// Build (or reuse) the semantic index of a `source()` target and record
    /// its *free* uses — reads that no definition inside the target reaches —
    /// as reads of this file's bindings at `call_range`. R's `source()` runs
    /// the target in the caller's environment, so a name the sourced script
    /// reads without binding it first consumes a binding in this file — the
    /// one live where the `source()` call sits, which is where the script
    /// runs.
    ///
    /// Going through the index rather than harvesting raw identifiers keeps
    /// non-reads out: a member name (`df$x`, `pkg::x`) is never a use, and a
    /// read that reaches the target's own rebind (`x <- 2; print(x)`) stays
    /// local to the target, so neither keeps a caller binding alive.
    ///
    /// "Free" means *not definitely bound*, not "unbound on every path". A
    /// target that binds a name only conditionally (`if (cond) x <- 2; x`)
    /// still reads the caller's binding when the branch isn't taken, so it
    /// counts as free even though the conditional definition reaches the read.
    fn import_uses_from_sourced_file(&mut self, target: &std::path::Path, call_range: TextRange) {
        // The resolver built this index while indexing the current file, so
        // this normally hits the memo; the build is the fallback for a target
        // that reached us some other way. Its own `source()` chain injects
        // Import definitions, so a read satisfied by a deeper file doesn't
        // count as free here (the cross-file pass credits that file instead).
        let Some(index) = source_target_index(&self.source_cache, target, target, |path| {
            JarlImportsResolver::with_cache(path, self.source_cache.clone())
        }) else {
            return;
        };
        self.positional_uses
            .extend(free_uses(&index).map(|name| (name, call_range)));
    }

    // ── Internal: reach / closure analysis ────────────────────────────

    /// Collect every definition reached by a non-NSE use, in every scope.
    ///
    /// `reaching_definitions` returns both local reaching definitions and, for
    /// a free variable in a nested closure, the enclosing-scope definitions
    /// captured by oak's enclosing snapshots. So a single pass over all uses
    /// covers in-scope reads and closure captures alike. Uses sitting inside an
    /// NSE argument (`expression(x)`, …) are skipped: they don't consume a
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
    /// (`expression(x <- 2)`) is quoted code, not an executed assignment, but
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

    /// Model the loop back edge: at the end of an iteration control jumps
    /// back to the top of the loop, so an assignment inside a loop can be read
    /// by a use sitting *earlier* in that loop — the condition (`while (x) { x
    /// <- f() }`) or an earlier body statement (`for (i in is) { g(x); x <-
    /// h(i) }`). Oak walks the file in textual order and only connects a
    /// definition to later uses, so those reads are missed.
    ///
    /// For every use inside a loop, mark the definitions of the same name that
    /// sit after it but still inside that loop: those are what the next
    /// iteration reads. A definition the loop never reads back is left alone,
    /// so `for (x in 1:3) { y <- x + 1 }` still reports `y`.
    ///
    /// Covers both kinds of read: the ones oak indexes, and the position-aware
    /// ones collected by the AST pass. An interpolation is a read like any
    /// other, so `for (i in 1:3) { print(glue("{x}")); x <- i }` reads `x`
    /// back on the next iteration too.
    fn precompute_loop_back_edges(&mut self, scopes: &[ScopeId]) {
        if self.loop_ranges.is_empty() {
            return;
        }
        let index = self.index;
        // Collect first: marking borrows `self` mutably.
        let mut back_edges: Vec<(String, ScopeId, TextSize, TextRange)> = Vec::new();
        for &scope_id in scopes {
            let symbols = index.symbols(scope_id);
            for (_, u) in index.uses(scope_id).iter() {
                if self.is_in_nse(u.range()) {
                    continue;
                }
                let name = symbols.symbol(u.symbol()).name();
                self.queue_back_edges(&mut back_edges, name, scope_id, u.range());
            }
        }
        // Interpolation reads never enter oak's use list — they're resolved
        // from `positional_uses`, which only ever looks backwards from the
        // read — so the back edge has to be queued for them here.
        for (name, read_range) in &self.positional_uses {
            let (read_scope, _) = index.scope_at(read_range.start());
            self.queue_back_edges(&mut back_edges, name, read_scope, *read_range);
        }
        for (name, scope_id, pos, loop_range) in back_edges {
            self.mark_lookup(&name, scope_id, |def, _| {
                // What the next iteration reads: the definitions following the
                // use but still inside the loop. Those preceding it are what
                // the first iteration reads — they bind the name without the
                // back edge marking them.
                let back_edge =
                    def.range().start() >= pos && loop_range.contains_range(def.range());
                (back_edge || def.range().start() < pos, back_edge)
            });
        }
    }

    /// Queue one back edge per loop enclosing a read of `name` at `range`.
    /// Nested loops: the back edge of every enclosing loop can feed the read,
    /// so all the loops it sits in count.
    fn queue_back_edges(
        &self,
        queue: &mut Vec<(String, ScopeId, TextSize, TextRange)>,
        name: &str,
        scope: ScopeId,
        range: TextRange,
    ) {
        for loop_range in self
            .loop_ranges
            .iter()
            .filter(|loop_range| loop_range.contains_range(range))
        {
            queue.push((name.to_string(), scope, range.start(), *loop_range));
        }
    }

    /// Keep the assignments a call spreads over sibling arguments alive (see
    /// [`Self::collect_branching_argument_assignments`]). Only definitions
    /// sitting inside one of those arguments are exempt, so an unrelated
    /// binding of the same name elsewhere in the file is still reported.
    fn precompute_branching_defs(&mut self) {
        let defs = std::mem::take(&mut self.branching_defs);
        for (name, range) in &defs {
            self.mark_definitions_in_range(name, *range);
        }
    }

    /// Mark every definition of `name` sitting inside `range` as used. The
    /// range decides on its own which scope is meant, so an assignment nested
    /// in a closure inside that range counts too.
    fn mark_definitions_in_range(&mut self, name: &str, range: TextRange) {
        let index = self.index;
        let mut marked: Vec<(ScopeId, DefinitionId)> = Vec::new();
        for scope in index.scope_ids() {
            let Some(symbol_id) = index.symbols(scope).id(name) else {
                continue;
            };
            marked.extend(
                index
                    .definitions(scope)
                    .iter()
                    .filter(|(_, def)| {
                        def.symbol() == symbol_id && range.contains_range(def.range())
                    })
                    .map(|(def_id, _)| (scope, def_id)),
            );
        }
        self.reaching_used.extend(marked);
    }

    /// Model a short-circuit operand not running: a read after `cond || (x <-
    /// 2)` may still see whatever `x` was bound to before it. Oak walks
    /// linearly and resolves that read to the conditional definition only, so
    /// when the conditional definition turns out to be read, mark the earlier
    /// definitions the read may reach instead — exactly the ones a position-
    /// aware read sitting at the assignment would consume.
    ///
    /// A conditional definition nothing ever reads keeps no one alive, so
    /// `if (cond && (y <- 1) > 2)` still reports `y`. Runs last: the read that
    /// justifies it may itself be an interpolation resolved by
    /// [`Self::precompute_positional_uses`].
    fn precompute_short_circuit_defs(&mut self) {
        let defs = std::mem::take(&mut self.short_circuit_defs);
        for (name, range) in &defs {
            if self.short_circuit_def_is_used(name, *range) {
                self.mark_positional_use(name, range.start());
            }
        }
    }

    /// True when the assignment of `name` spanning `range` produces a
    /// definition that some use reaches.
    fn short_circuit_def_is_used(&self, name: &str, range: TextRange) -> bool {
        let index = self.index;
        let (scope, _) = index.scope_at(range.start());
        let Some(symbol_id) = index.symbols(scope).id(name) else {
            return false;
        };
        index.definitions(scope).iter().any(|(def_id, def)| {
            def.symbol() == symbol_id
                && range.contains_range(def.range())
                && self.reaching_used.contains(&(scope, def_id))
        })
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
        let (read_scope, _) = self.index.scope_at(pos);
        self.mark_lookup(name, read_scope, |def, captured| {
            let reaches = captured || def.range().start() < pos;
            (reaches, reaches)
        });
    }

    /// Walk outward from `scope` like an ordinary lookup and mark the
    /// definitions of `name` that a read there consumes.
    ///
    /// `classify` sees every non-NSE definition of `name` in the scope being
    /// examined, plus whether that scope encloses the reading one, and answers
    /// two things: does the definition make this scope the one binding `name`,
    /// and does the read reach it. The two come apart for a loop back edge,
    /// where a definition preceding the read binds the name — it is what the
    /// first iteration reads — without being read back on the next one. The
    /// walk still stops at such a scope, since an outer definition of the same
    /// name is shadowed either way; a scope that merely *mentions* `name`
    /// without binding it doesn't stop it, so it continues to the scope whose
    /// binding the read actually consumes.
    fn mark_lookup(
        &mut self,
        name: &str,
        scope: ScopeId,
        classify: impl Fn(&Definition, bool) -> (bool, bool),
    ) {
        let index = self.index;
        for owner in index.ancestor_scope_ids(scope) {
            let Some(symbol_id) = index.symbols(owner).id(name) else {
                continue;
            };
            let captured = owner != scope;
            let mut binds = false;
            let mut reached: Vec<DefinitionId> = Vec::new();
            for (def_id, def) in index.definitions(owner).iter() {
                if def.symbol() != symbol_id || self.is_in_nse(def.range()) {
                    continue;
                }
                let (def_binds, def_reached) = classify(def, captured);
                binds |= def_binds;
                if def_reached {
                    reached.push(def_id);
                }
            }
            if !binds {
                continue;
            }
            self.reaching_used
                .extend(reached.into_iter().map(|def_id| (owner, def_id)));
            return;
        }
    }
}

// ── Free helpers (also used by rule policy) ──────────────────────────────

/// The names `index` reads without binding them first — the reads that consume
/// a binding from outside the indexed code.
///
/// "Free" means *not definitely bound*, not "unbound on every path": code that
/// binds a name only conditionally (`if (cond) x <- 2; x`) still reads the
/// outer binding when the branch isn't taken, so it counts as free even though
/// the conditional definition reaches the read.
fn free_uses(index: &SemanticIndex) -> impl Iterator<Item = String> {
    let mut names = Vec::new();
    for scope in index.scope_ids() {
        let symbols = index.symbols(scope);
        for (use_id, use_site) in index.uses(scope).iter() {
            if !index.use_is_bound(scope, use_id) {
                names.push(symbols.symbol(use_site.symbol()).name().to_string());
            }
        }
    }
    names.into_iter()
}

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
///
/// Matched by name only, rather than through oak's `bind_arguments`: the glue
/// delimiters this reads sit *after* `...` in glue's signature, and oak's
/// formals stop at the dots because R matches everything past them by name.
fn named_string_arg(args: &[RArgument], name: &str) -> Option<String> {
    let value = args
        .iter()
        .find(|arg| argument_name_is(arg, name))?
        .value()?;
    let node = value.syntax();
    if node.kind() != RSyntaxKind::R_STRING_VALUE {
        return None;
    }
    strings::get_string_literal_contents(&node.text_trimmed().to_string())
}

/// Packages whose functions interpolate `{...}` in their string arguments.
/// Kept in sync with [`glue_interpolation_package`] and
/// [`is_cli_markup_function`], which decide *which* of them applies.
const INTERPOLATION_PACKAGES: &[&str] = &["glue", "stringr", "cli"];

/// The interpolation dialect a string literal is written in.
#[derive(Clone, Copy)]
enum InterpolationFlavor {
    /// glue's plain `{...}` interpolation.
    Glue,
    /// glue interpolation plus cli's inline markup spans (`{.field {x}}`).
    CliMarkup,
}

/// The interpolation dialect applying to `node` and the package providing it,
/// or `None` when no enclosing call interpolates its arguments — a string is
/// only code when it is handed to a glue, stringr or cli function. Walks all
/// ancestors (not just the immediate call) so message strings nested in a
/// `c(...)` bullets vector still count.
fn interpolation_flavor(node: &RSyntaxNode) -> Option<(InterpolationFlavor, &'static str)> {
    node.ancestors()
        .filter_map(|ancestor| ancestor.cast::<RCall>())
        .find_map(|call| {
            let name = call_name(&call)?;
            if is_cli_markup_function(&name) {
                return Some((InterpolationFlavor::CliMarkup, "cli"));
            }
            glue_interpolation_package(&name).map(|package| (InterpolationFlavor::Glue, package))
        })
}

/// The package providing `name` if it glue-interpolates its string arguments,
/// `None` otherwise. Covers glue and stringr, excluding the non-interpolating
/// helpers of both (`as_glue`, `glue_collapse`, `str_c`, …). `str_interp()`
/// uses `${...}` rather than `{...}`, but scanning for `{`/`}` still finds its
/// expressions. Namespaced calls (`glue::glue`) resolve to the bare name via
/// [`call_name`].
fn glue_interpolation_package(name: &str) -> Option<&'static str> {
    match name {
        "glue" | "glue_data" | "glue_col" | "glue_data_col" | "glue_safe" | "glue_data_safe"
        | "glue_sql" | "glue_data_sql" => Some("glue"),
        "str_glue" | "str_glue_data" | "str_interp" => Some("stringr"),
        _ => None,
    }
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

/// The name of the called function for a bare (`f()`) or namespaced
/// (`pkg::f()`) callee, unquoted in both cases. `None` for a computed callee.
///
/// jarl-core has its own `get_function_name` doing the same job; the two can't
/// share one, since jarl-core depends on this crate and not the reverse.
fn call_name(call: &RCall) -> Option<String> {
    match call.function().ok()? {
        AnyRExpression::RIdentifier(ident) => Some(ident.name_text()),
        AnyRExpression::RNamespaceExpression(ns) => ns.right().ok()?.identifier_text(),
        _ => None,
    }
}

/// The call's arguments, in call order. Plain syntax handles: names and values
/// are only materialised by the callers that look at them, which matters
/// because this runs for every call node in the file.
fn call_args(call: &RCall) -> Vec<RArgument> {
    let Ok(arguments) = call.arguments() else {
        return Vec::new();
    };
    arguments.items().iter().flatten().collect()
}

/// Whether `arg` is written as `name = value`.
fn argument_name_is(arg: &RArgument, name: &str) -> bool {
    arg.name_clause()
        .and_then(|clause| clause.name().ok())
        .is_some_and(|arg_name| match arg_name {
            AnyRArgumentName::RIdentifier(ident) => ident.name_text() == name,
            _ => false,
        })
}

fn string_literal_value(node: &RSyntaxNode) -> Option<String> {
    node.clone().cast::<RStringValue>()?.string_text()
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
/// walking their ancestors down to `""` is exactly that chain of anchors. The
/// `source()` argument itself is unconstrained: a `../` in it resolves above
/// the CWD, the same as it would when R runs the script.
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

/// `ImportsResolver` impl that plugs `source("path")` injection and effect
/// resolution into oak's builder.
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
    /// Run-wide memo of already-built target indices. Consulted after the
    /// per-chain cycle check, so it only skips redundant builds and never
    /// changes what a chain resolves.
    cache: SourceIndexCache,
}

impl JarlImportsResolver {
    pub fn new(current_file: impl Into<std::path::PathBuf>) -> Self {
        Self::with_cache(current_file, SourceIndexCache::new())
    }

    /// Resolver sharing `cache` with the rest of the run, so a helper sourced
    /// by many files is parsed and indexed once.
    pub fn with_cache(
        current_file: impl Into<std::path::PathBuf>,
        cache: SourceIndexCache,
    ) -> Self {
        let current_file = current_file.into();
        let mut visited = HashSet::new();
        visited.insert(absolutize_path(&current_file));
        Self {
            current_file,
            visited: std::rc::Rc::new(std::cell::RefCell::new(visited)),
            cache,
        }
    }
}

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
        let sub_index = source_target_index(&self.cache, &target_key, &target, |path| {
            JarlImportsResolver {
                current_file: path.to_path_buf(),
                visited: std::rc::Rc::clone(&self.visited),
                cache: self.cache.clone(),
            }
        })?;
        let names: Vec<String> = sub_index.exports().keys().map(|s| s.to_string()).collect();
        // `library()` in a sourced file attaches to the global search path, so
        // the sourcing file sees the package too. Oak turns these into `Attach`
        // calls at the `source()` position, which puts them in the caller's
        // `attached_packages()` and in the `attached` list effect resolution
        // walks. Only the target's load-time attaches count, and they already
        // include what it forwards from its own `source()` calls.
        let packages: Vec<String> = sub_index
            .attached_packages()
            .into_iter()
            .map(str::to_string)
            .collect();
        Some(oak_semantic::SourceResolution { url, names, packages })
    }
}

/// Absolutize `path` against the process CWD, without touching the
/// filesystem. Gives `source()` targets a canonical key so cycle detection
/// and URL construction agree regardless of how the path was spelled.
///
/// `std::path::absolute` drops `.` but keeps `..`, so `sub/../a.R` and `a.R`
/// would key differently and a self-`source()` spelled with `..` would slip
/// past the cycle guard, minting a fresh key at every level of the chain.
/// Popping `..` lexically makes both spellings collapse to the same key.
///
/// Lexical, not `canonicalize`: the key doubles as the `file://` URL the
/// cross-file pass round-trips back to a lint path, so it has to stay the
/// path as spelled rather than a symlink-resolved one (and it must work for
/// targets that don't exist on disk). The trade-off is that `..` crossing a
/// symlinked directory normalises to a path the OS wouldn't — the file is
/// still read through the un-normalised path, so this only affects keying.
fn absolutize_path(path: &std::path::Path) -> std::path::PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    normalize_parent_dirs(&absolute)
}

/// Resolve `.` and `..` components lexically, without consulting the
/// filesystem. A `..` with nothing to pop is kept: it's either a relative path
/// climbing above its own root, or the filesystem root, where `..` is a no-op
/// R would resolve the same way.
fn normalize_parent_dirs(path: &std::path::Path) -> std::path::PathBuf {
    use std::path::Component;

    let mut normalized = std::path::PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(
                    normalized.components().next_back(),
                    Some(Component::Normal(_))
                ) {
                    normalized.pop();
                } else if !normalized.has_root() {
                    normalized.push(component);
                }
            }
            other => normalized.push(other),
        }
    }
    normalized
}
