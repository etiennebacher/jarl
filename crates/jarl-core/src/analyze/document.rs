use air_r_syntax::{RExpressionList, RSyntaxNode};
use biome_rowan::{AstNode, AstNodeList};

use crate::checker::Checker;
use crate::diagnostic::*;
use crate::lints::base::unreachable_code::unreachable_code::unreachable_code_top_level;
use crate::lints::comments::blanket_suppression::blanket_suppression::blanket_suppression;
use crate::lints::comments::invalid_chunk_suppression::invalid_chunk_suppression::invalid_chunk_suppression;
use crate::lints::comments::misnamed_suppression::misnamed_suppression::misnamed_suppression;
use crate::lints::comments::misplaced_file_suppression::misplaced_file_suppression::misplaced_file_suppression;
use crate::lints::comments::misplaced_suppression::misplaced_suppression::misplaced_suppression;
use crate::lints::comments::outdated_suppression::outdated_suppression::outdated_suppression;
use crate::lints::comments::unexplained_suppression::unexplained_suppression::unexplained_suppression;
use crate::lints::comments::unmatched_range_suppression::unmatched_range_suppression::{
    unmatched_range_suppression_end, unmatched_range_suppression_start,
};
use crate::rule_set::Rule;

pub(crate) fn check_document(
    expressions: &RExpressionList,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    // --- Document-level analysis ---

    let expressions: Vec<RSyntaxNode> = expressions.iter().map(|e| e.syntax().clone()).collect();

    // Check for unreachable code at top level
    if checker.is_rule_enabled(Rule::UnreachableCode) {
        for diagnostic in unreachable_code_top_level(&expressions, checker)? {
            checker.report_diagnostic(Some(diagnostic));
        }
    }

    // --- Comment/suppression checks ---

    // Report blanket suppression comments (file-level, done once)
    if checker.is_rule_enabled(Rule::BlanketSuppression) {
        let diagnostics = blanket_suppression(&checker.suppression.blanket_suppressions);
        for diagnostic in diagnostics {
            checker.report_diagnostic(Some(diagnostic));
        }
    }

    // Report chunk suppressions that use the single-line `#|` form
    if checker.is_rule_enabled(Rule::InvalidChunkSuppression) {
        let diagnostics =
            invalid_chunk_suppression(&checker.suppression.invalid_chunk_suppressions);
        for diagnostic in diagnostics {
            checker.report_diagnostic(Some(diagnostic));
        }
    }

    // Report suppressions missing explanations
    if checker.is_rule_enabled(Rule::UnexplainedSuppression) {
        let diagnostics = unexplained_suppression(&checker.suppression.unexplained_suppressions);
        for diagnostic in diagnostics {
            checker.report_diagnostic(Some(diagnostic));
        }
    }

    // Report misplaced file-level suppressions
    if checker.is_rule_enabled(Rule::MisplacedFileSuppression) {
        let diagnostics =
            misplaced_file_suppression(&checker.suppression.misplaced_file_suppressions);
        for diagnostic in diagnostics {
            checker.report_diagnostic(Some(diagnostic));
        }
    }

    // Report misplaced (end-of-line) suppression comments
    if checker.is_rule_enabled(Rule::MisplacedSuppression) {
        let diagnostics = misplaced_suppression(&checker.suppression.misplaced_suppressions);
        for diagnostic in diagnostics {
            checker.report_diagnostic(Some(diagnostic));
        }
    }

    // Report suppressions with invalid rule names
    if checker.is_rule_enabled(Rule::MisnamedSuppression) {
        let diagnostics = misnamed_suppression(&checker.suppression.misnamed_suppressions);
        for diagnostic in diagnostics {
            checker.report_diagnostic(Some(diagnostic));
        }
    }

    // Report unmatched start/end suppression comments
    if checker.is_rule_enabled(Rule::UnmatchedRangeSuppression) {
        let start_diagnostics =
            unmatched_range_suppression_start(&checker.suppression.unmatched_start_suppressions);
        for diagnostic in start_diagnostics {
            checker.report_diagnostic(Some(diagnostic));
        }
        let end_diagnostics =
            unmatched_range_suppression_end(&checker.suppression.unmatched_end_suppressions);
        for diagnostic in end_diagnostics {
            checker.report_diagnostic(Some(diagnostic));
        }
    }

    // Emit package-level duplicate top-level assignment diagnostics.
    // This must come before suppression filtering so that # jarl-ignore
    // and # jarl-ignore-file comments can suppress these diagnostics.
    if checker.is_rule_enabled(Rule::DuplicatedFunctionDefinition) {
        for (name, range, help) in &checker.package_duplicate_assignments.clone() {
            checker.report_diagnostic(Some(Diagnostic::new(
                ViolationData::new(
                    "duplicated_function_definition".to_string(),
                    format!("`{name}` is defined more than once in this package."),
                    Some(help.clone()),
                ),
                *range,
                Fix::empty(),
            )));
        }
    }

    // Filter diagnostics by suppressions. This removes suppressed violations
    // and tracks which suppressions were used (for outdated suppression detection).
    // Must happen BEFORE checking for outdated suppressions.
    checker.diagnostics = checker
        .suppression
        .filter_diagnostics(std::mem::take(&mut checker.diagnostics));

    // Report outdated suppressions (suppressions that didn't suppress anything).
    if checker.is_rule_enabled(Rule::OutdatedSuppression) {
        let unused = checker.suppression.get_unused_suppressions();
        let outdated_diagnostics = outdated_suppression(&unused);
        for diag in outdated_diagnostics {
            checker.report_diagnostic(Some(diag));
        }
    }

    Ok(())
}
