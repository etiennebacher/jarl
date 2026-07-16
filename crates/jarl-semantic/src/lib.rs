//! Per-file semantic info for jarl lint rules.
//!
//! `SemanticInfo` is computed once over a parsed file and exposes the
//! information lint rules need to answer "is this definition used?", without
//! every rule walking oak's `SemanticIndex` themselves.
//!
//! Mirrors ruff's `Binding::is_unused()` style: rules ask
//! `info.is_definition_used(scope, def_id, def)` rather than walking the
//! semantic index themselves.

use std::collections::HashSet;

use air_r_parser::RParserOptions;
use air_r_syntax::{RBinaryExpression, RSyntaxKind, RSyntaxNode};
use biome_rowan::{AstNode, SyntaxNodeCast, TextRange};
use oak_core::syntax_ext::RIdentifierExt;
use oak_semantic::DefinitionId;
use oak_semantic::semantic_index::{Definition, DefinitionKind, ScopeId, SemanticIndex};

/// Per-file semantic info derived from oak's [`SemanticIndex`].
/// Computed once per file; consumed by lints.
pub struct SemanticInfo<'a> {
    index: &'a SemanticIndex,
    /// Root syntax node of the analyzed file. Needed to resolve
    /// `AstPtr` references stored in [`DefinitionKind`] back to nodes.
    root: RSyntaxNode,
    /// Definitions reached by some use anywhere in the file. Computed
    /// from oak's `reaching_definitions`, which resolves both local uses and
    /// free-variable uses in nested closures (via enclosing snapshots).
    reaching_used: HashSet<(ScopeId, DefinitionId)>,
}

impl<'a> SemanticInfo<'a> {
    /// Build the info table: the reaching-use precomputation over oak's
    /// use-def maps. `_expressions` and `_file` are unused for now; they feed
    /// the AST passes (NSE ranges, interpolation reads, sourced-file reads)
    /// as those land.
    pub fn build(
        root: &RSyntaxNode,
        _expressions: &[RSyntaxNode],
        index: &'a SemanticIndex,
        _file: &std::path::Path,
    ) -> Self {
        let mut this = Self {
            index,
            root: root.clone(),
            reaching_used: HashSet::new(),
        };
        let scopes = this.scope_ids();
        this.precompute_reaching_uses(&scopes);
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

    // ── Internal: reach / closure analysis ────────────────────────────

    /// Collect every definition reached by a use, in every scope.
    ///
    /// `reaching_definitions` returns both local reaching definitions and, for
    /// a free variable in a nested closure, the enclosing-scope definitions
    /// captured by oak's enclosing snapshots. So a single pass over all uses
    /// covers in-scope reads and closure captures alike.
    fn precompute_reaching_uses(&mut self, scopes: &[ScopeId]) {
        let index = self.index;
        for &scope_id in scopes {
            for (use_id, _) in index.uses(scope_id).iter() {
                for (def_scope, def_id) in index.reaching_definitions(scope_id, use_id) {
                    self.reaching_used.insert((def_scope, def_id));
                }
            }
        }
    }
}

// ── Free helpers (also used by rule policy) ──────────────────────────────

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
