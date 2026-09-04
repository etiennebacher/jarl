use air_r_syntax::{AnyRExpression, AnyRValue, RBinaryExpression, RParameter, RSyntaxKind};
use biome_rowan::{AstNode, TextRange};
use oak_core::syntax_ext::{RIdentifierExt, RStringValueExt};

use crate::diagnostic::{Diagnostic, Fix, ViolationData};
use crate::lints::base::object_name::options::ResolvedObjectNameOptions;
use crate::rule_set::Rule;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks the names of variables and arguments used in assignments and function
/// definitions. For `$` and `@` assignments, it checks only the base object
/// name.
///
/// ## Why is this bad?
///
/// Consistent names make code easier to read and maintain.
///
/// ## Example
///
/// ```r
/// badName <- 1
/// f <- function(badArg) badArg
/// badName$member <- 1
/// ```
///
/// Use instead:
///
/// ```r
/// bad_name <- 1
/// f <- function(bad_arg) bad_arg
/// bad_name$member <- 1
/// ```
pub fn object_name(
    syntax: &air_r_syntax::RSyntaxNode,
    options: &ResolvedObjectNameOptions,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for node in syntax.descendants() {
        if let Some(parameter) = RParameter::cast(node.clone())
            && let Ok(air_r_syntax::AnyRParameterName::RIdentifier(identifier)) = parameter.name()
            && let Some(diagnostic) = name_diagnostic(
                &identifier.name_text(),
                identifier.syntax().text_trimmed_range(),
                options,
            )
        {
            diagnostics.push(diagnostic);
        }

        if let Some(binary) = RBinaryExpression::cast(node)
            && let Ok(operator) = binary.operator()
            && let Some(target) = assignment_target(&binary, operator.kind())
            && let Some(diagnostic) = name_diagnostic(&target.0, target.1, options)
        {
            diagnostics.push(diagnostic);
        }
    }

    diagnostics
}

fn assignment_target(
    binary: &RBinaryExpression,
    operator: RSyntaxKind,
) -> Option<(String, TextRange)> {
    let expression = match operator {
        RSyntaxKind::ASSIGN | RSyntaxKind::EQUAL | RSyntaxKind::SUPER_ASSIGN => {
            binary.left().ok()?
        }
        RSyntaxKind::ASSIGN_RIGHT | RSyntaxKind::SUPER_ASSIGN_RIGHT => binary.right().ok()?,
        _ => return None,
    };

    root_name(&expression)
}

fn root_name(expression: &AnyRExpression) -> Option<(String, TextRange)> {
    match expression {
        AnyRExpression::RIdentifier(identifier) => Some((
            identifier.name_text(),
            identifier.syntax().text_trimmed_range(),
        )),
        AnyRExpression::AnyRValue(AnyRValue::RStringValue(value)) => {
            Some((value.string_text()?, value.syntax().text_trimmed_range()))
        }
        AnyRExpression::RExtractExpression(extract) => {
            let operator = extract.operator().ok()?;
            if !matches!(operator.kind(), RSyntaxKind::DOLLAR | RSyntaxKind::AT) {
                return None;
            }
            let left = extract.left().ok()?;
            root_name(&left)
        }
        _ => None,
    }
}

fn name_diagnostic(
    name: &str,
    range: TextRange,
    options: &ResolvedObjectNameOptions,
) -> Option<Diagnostic> {
    if options.is_special_name(name) || options.matches(name) {
        return None;
    }

    Some(Diagnostic::new(
        ViolationData::new(
            Rule::ObjectName,
            format!(
                "Variable and function name style should match {}.",
                options.expected()
            ),
            None,
        ),
        range,
        Fix::empty(),
    ))
}
