use crate::diagnostic::Diagnostic;
use crate::rule_options::ResolvedRuleOptions;
use crate::rule_set::{Rule, RuleSet};
use crate::suppression::SuppressionManager;

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
    // Per-rule options resolved from configuration
    pub rule_options: ResolvedRuleOptions,
    // Pre-computed duplicate top-level assignments for this file (from
    // cross-file package analysis). Each entry is (name, lhs_range, help)
    // where help points to the first definition.
    pub package_duplicate_assignments: Vec<(String, biome_rowan::TextRange, String)>,
    // Pre-computed unused internal functions for this file (from
    // cross-file package analysis). Each entry is (name, lhs_range, help).
    pub package_unused_internal_functions: Vec<(String, biome_rowan::TextRange, String)>,
}

impl Checker {
    pub(crate) fn new(suppression: SuppressionManager, rule_options: ResolvedRuleOptions) -> Self {
        Self {
            diagnostics: vec![],
            rule_set: RuleSet::empty(),
            minimum_r_version: None,
            suppression,
            rule_options,
            package_duplicate_assignments: vec![],
            package_unused_internal_functions: vec![],
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
}
