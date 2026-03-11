use crate::diagnostic::Diagnostic;
use crate::package_cache::PackageCache;
use crate::rule_options::ResolvedRuleOptions;
use crate::rule_set::{Rule, RuleSet};
use crate::suppression::SuppressionManager;
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
    // Packages loaded via `library()` in this file, in load order.
    pub loaded_packages: Vec<String>,
    // Shared package cache for looking up installed package metadata.
    pub package_cache: Option<Arc<PackageCache>>,
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
            loaded_packages: Vec::new(),
            package_cache: None,
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

    /// Resolve which package a bare function name comes from, based on
    /// the `library()` calls in this file and the installed package metadata.
    pub fn resolve_package(&self, fn_name: &str) -> PackageOrigin {
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
