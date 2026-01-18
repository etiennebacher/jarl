use crate::check::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RFunctionDefinition;
use biome_rowan::AstNode;

use crate::lints::unreachable_code::unreachable_code::unreachable_code;

pub fn function_definition(
    func: &RFunctionDefinition,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    let node = func.syntax();

    // Check suppressions once for this node
    let suppressed_rules = checker.get_suppressed_rules(node);

    if checker.is_rule_enabled(Rule::UnreachableCode) && !suppressed_rules.contains(&Rule::UnreachableCode) {
        let diagnostics = unreachable_code(func)?;
        for diagnostic in diagnostics {
            checker.report_diagnostic(Some(diagnostic));
        }
    }

    Ok(())
}
