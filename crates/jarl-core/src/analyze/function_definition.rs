use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RFunctionDefinition;

use crate::lints::base::unreachable_code::unreachable_code::unreachable_code;

/// Run all function definition-related lints.
/// Suppressions are handled in post-processing via filter_diagnostics.
pub fn function_definition(
    func: &RFunctionDefinition,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::UnreachableCode) {
        let diagnostics = unreachable_code(func)?;
        for diagnostic in diagnostics {
            checker.report_diagnostic(Some(diagnostic));
        }
    }

    Ok(())
}
