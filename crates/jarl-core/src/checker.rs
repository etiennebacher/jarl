use std::collections::HashSet;

use crate::diagnostic::Diagnostic;
use crate::package_cache::PackageCache;
use crate::rule_options::ResolvedRuleOptions;
use crate::rule_set::{Rule, RuleSet};
use crate::suppression::SuppressionManager;
use std::collections::HashMap;
use std::sync::Arc;

/// Packages that R attaches by default on startup (equivalent to
/// `getOption("defaultPackages")` plus `base`). These are always available
/// without an explicit `library()` call.
pub const DEFAULT_PACKAGES: &[&str] = &[
    "base",
    "datasets",
    "graphics",
    "grDevices",
    "methods",
    "stats",
    "utils",
];

/// Result of resolving a bare function name to its source package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageOrigin {
    /// The function is exported by exactly one loaded package.
    Resolved(String),
    /// The function is exported by multiple loaded packages.
    /// The list is in load order (first loaded first), so the last element
    /// is the one that masks the others in R.
    Ambiguous(Vec<String>),
    /// The function was not found in any loaded package. This can happen when
    /// the package was loaded in another file or is not installed.
    Unknown,
}

impl PackageOrigin {
    /// Check if any of the candidate packages matches the given name.
    /// Returns `true` for `Resolved` if it matches, for `Ambiguous` if
    /// any candidate matches, and `false` for `Unknown`.
    pub fn includes(&self, pkg: &str) -> bool {
        match self {
            PackageOrigin::Resolved(p) => p == pkg,
            PackageOrigin::Ambiguous(candidates) => candidates.iter().any(|c| c == pkg),
            PackageOrigin::Unknown => false,
        }
    }
}

#[derive(Debug)]
// The object that will collect diagnostics in check_expressions(). One per
// analyzed file.
pub struct Checker {
    // The diagnostics to report (possibly empty).
    pub diagnostics: Vec<Diagnostic>,
    // A set of rules to apply. Each rule contains metadata about whether it
    // has a safe fix, unsafe fix, or no fix, and the minimum R version required.
    pub rule_set: RuleSet,
    // The R version that is manually passed by the user in the CLI. Any rule
    // that has a minimum R version higher than this value will be deactivated.
    pub minimum_r_version: Option<(u32, u32, u32)>,
    // Tracks comment-based suppression directives like `# jarl-ignore`
    pub suppression: SuppressionManager,
    // Per-rule options resolved from configuration (Arc to avoid expensive clones)
    pub rule_options: Arc<ResolvedRuleOptions>,
    // Pre-computed S3 method names from the package NAMESPACE file.
    // Used by unused_function_argument to skip S3 methods.
    pub package_s3_methods: HashSet<String>,
    // Packages loaded via `library()` in this file (or from DESCRIPTION
    // Depends/Imports when inside an R package), in load order.
    pub loaded_packages: Vec<String>,
    // Shared package cache for looking up installed package metadata.
    pub package_cache: Option<Arc<PackageCache>>,
    // Direct function→package mappings from `importFrom()` in the package's
    // own NAMESPACE. Takes priority over export-list scanning.
    pub import_from: HashMap<String, String>,
}

impl Checker {
    pub(crate) fn new(
        suppression: SuppressionManager,
        rule_options: Arc<ResolvedRuleOptions>,
    ) -> Self {
        Self {
            diagnostics: vec![],
            rule_set: RuleSet::empty(),
            minimum_r_version: None,
            suppression,
            rule_options,
            package_s3_methods: HashSet::new(),
            loaded_packages: Vec::new(),
            package_cache: None,
            import_from: HashMap::new(),
        }
    }

    // This takes an Option<Diagnostic> because each lint rule reports a
    // Some(Diagnostic) or None.
    pub(crate) fn report_diagnostic(&mut self, diagnostic: Option<Diagnostic>) {
        if let Some(diagnostic) = diagnostic {
            self.diagnostics.push(diagnostic);
        }
    }

    pub(crate) fn is_rule_enabled(&mut self, rule: Rule) -> bool {
        self.rule_set.contains(&rule)
    }

    /// Resolve which package a bare function name comes from.
    ///
    /// Resolution order:
    /// 1. Direct `importFrom()` mappings (from the package's own NAMESPACE)
    /// 2. Export-list scanning across `loaded_packages` via the `PackageCache`
    pub fn resolve_package(&self, fn_name: &str) -> PackageOrigin {
        // Check importFrom() first, these don't leave doubt
        if let Some(pkg) = self.import_from.get(fn_name) {
            return PackageOrigin::Resolved(pkg.clone());
        }

        let Some(cache) = self.package_cache.as_ref() else {
            return PackageOrigin::Unknown;
        };

        let mut candidates: Vec<String> = Vec::new();
        for pkg_name in &self.loaded_packages {
            if let Some(info) = cache.get(pkg_name)
                && info.exports.contains(fn_name)
            {
                candidates.push(pkg_name.clone());
            }
        }

        match candidates.len() {
            0 => PackageOrigin::Unknown,
            1 => PackageOrigin::Resolved(candidates.into_iter().next().unwrap()),
            _ => PackageOrigin::Ambiguous(candidates),
        }
    }

    /// Look up the installed version of a package.
    pub fn package_version(&self, pkg_name: &str) -> Option<(u32, u32, u32)> {
        self.package_cache
            .as_ref()?
            .get(pkg_name)
            .and_then(|info| info.version)
    }
}
