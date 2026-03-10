use crate::diagnostic::Diagnostic;
use crate::package_cache::PackageCache;
use crate::rule_options::ResolvedRuleOptions;
use crate::rule_set::{Rule, RuleSet};
use crate::suppression::SuppressionManager;
use std::sync::Arc;

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
    ///
    /// Walks loaded packages in reverse order (last loaded wins, matching R's
    /// masking behavior). Returns `None` if not resolved.
    pub fn resolve_package(&self, fn_name: &str) -> Option<String> {
        let cache = self.package_cache.as_ref()?;
        for pkg_name in self.loaded_packages.iter().rev() {
            if let Some(info) = cache.get(pkg_name)
                && info.exports.contains(fn_name)
            {
                return Some(pkg_name.clone());
            }
        }
        None
    }

    /// Look up the installed version of a package.
    pub fn package_version(&self, pkg_name: &str) -> Option<(u32, u32, u32)> {
        self.package_cache
            .as_ref()?
            .get(pkg_name)
            .and_then(|info| info.version)
    }
}
