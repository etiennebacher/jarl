use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{Formals, get_arg, get_function_name, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};
use jarl_semantic::strings::get_string_literal_contents;

const FORMALS_NCHAR: Formals = &["x"];

/// Version added: 0.5.0
///
/// ## What it does
///
/// Checks for usage of `x != ""` or `x == ""`, or comparisons of `nchar(x)`
/// with zero, such as `nchar(x) == 0`, instead of `nzchar(x)` or `!nzchar(x)`.
///
/// ## Why is this bad?
/// `x == ""` is less efficient than `!nzchar(x)`
/// when x is a large vector of long strings.
///
/// One crucial difference is in the default handling of `NA_character_`,
/// i.e., missing strings. `nzchar(NA_character_)` is TRUE,
/// while `NA_character_ == ""` is NA.
/// Therefore, for strict compatibility, use `nzchar(x, keepNA = TRUE)`.
/// If the input is known to be complete (no missing entries),
/// this argument can be dropped for conciseness.
///
/// This rule comes with a unsafe fix.
///
/// ## Example
///
/// ```r
/// x <- sample(c("abcdefghijklmn", "", "opqrstuvwyz"), 1e7, TRUE)
/// x[x == ""]
/// ```
///
/// Use instead:
/// ```r
/// x <- sample(c("abcdefghijklmn", "", "opqrstuvwyz"), 1e7, TRUE)
/// x[!nzchar(x)]
/// ```
///
/// ## References
///
/// See `?nzchar`
pub fn nzchar(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    // Check for comparisons of `nchar(x)` with zero, such as `nchar(x) == 0`
    if let Some(diagnostic) = nchar_zero_comparison(ast)? {
        return Ok(Some(diagnostic));
    }

    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let left = left?;
    let operator = operator?;
    let right = right?;
    let operator_kind = operator.kind();

    if operator_kind != RSyntaxKind::EQUAL2 && operator_kind != RSyntaxKind::NOT_EQUAL {
        return Ok(None);
    };

    let left_is_empty_string = left
        .as_any_r_value()
        .and_then(|value| get_string_literal_contents(&value.to_trimmed_string()))
        .is_some_and(|content| content.is_empty());
    let right_is_empty_string = right
        .as_any_r_value()
        .and_then(|value| get_string_literal_contents(&value.to_trimmed_string()))
        .is_some_and(|content| content.is_empty());

    if (left_is_empty_string && right_is_empty_string)
        || (!left_is_empty_string && !right_is_empty_string)
    {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();

    let replacement = if left_is_empty_string {
        right.to_trimmed_string()
    } else {
        left.to_trimmed_string()
    };

    let diagnostic = match operator_kind {
        RSyntaxKind::EQUAL2 => Diagnostic::new(
            ViolationData::new(
                Rule::NzChar,
                "`x == \"\"` is inefficient.".to_string(),
                Some("Use `!nzchar(x)` instead.".to_string()),
            ),
            range,
            Fix::new(
                range,
                format!("!nzchar({replacement})"),
                node_contains_comments(ast.syntax()),
            ),
        ),
        RSyntaxKind::NOT_EQUAL => Diagnostic::new(
            ViolationData::new(
                Rule::NzChar,
                "`x != \"\"` is inefficient.".to_string(),
                Some("Use `nzchar(x)` instead.".to_string()),
            ),
            range,
            Fix::new(
                range,
                format!("nzchar({replacement})"),
                node_contains_comments(ast.syntax()),
            ),
        ),
        _ => unreachable!("This case is an early return"),
    };

    Ok(Some(diagnostic))
}

fn nchar_zero_comparison(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let left = left?;
    let operator = operator?;
    let right = right?;
    let operator_kind = operator.kind();

    let (nchar_expression, zero_on_left) = if is_zero_literal(&right) {
        (Some(&left), false)
    } else if is_zero_literal(&left) {
        (Some(&right), true)
    } else {
        (None, false)
    };

    // Normalize equivalent comparisons so `nchar(...)` is treated as the left operand.
    let normalized_operator = if matches!(
        operator_kind,
        RSyntaxKind::GREATER_THAN
            | RSyntaxKind::LESS_THAN
            | RSyntaxKind::GREATER_THAN_OR_EQUAL_TO
            | RSyntaxKind::LESS_THAN_OR_EQUAL_TO
            | RSyntaxKind::EQUAL2
            | RSyntaxKind::NOT_EQUAL
    ) {
        // like `0 < nchar(x)`
        if zero_on_left {
            Some(flip_comparison_operator(operator_kind))
        } else {
            Some(operator_kind)
        }
    } else {
        None
    };

    if let Some(nchar_expression) = nchar_expression
        && let Some(call) = nchar_expression.as_r_call()
        && let Some(operator) = normalized_operator
        && get_function_name(call.function()?) == "nchar"
    {
        if call.arguments()?.items().len() != 1 {
            return Ok(None);
        }
        if let Some(argument) =
            get_arg(call, FORMALS_NCHAR, "x").and_then(|argument| argument.value())
        {
            let range = ast.syntax().text_trimmed_range();
            let argument = argument.to_trimmed_string();
            let (body, suggestion, replacement) = match operator {
                RSyntaxKind::GREATER_THAN => (
                    "`nchar(x) > 0` is inefficient.",
                    "Use `nzchar(x)` instead.",
                    Some(format!("nzchar({argument})")),
                ),
                RSyntaxKind::NOT_EQUAL => (
                    "`nchar(x) != 0` is inefficient.",
                    "Use `nzchar(x)` instead.",
                    Some(format!("nzchar({argument})")),
                ),
                RSyntaxKind::LESS_THAN_OR_EQUAL_TO => (
                    "`nchar(x) <= 0` is inefficient.",
                    "Use `!nzchar(x)` instead.",
                    Some(format!("!nzchar({argument})")),
                ),
                RSyntaxKind::EQUAL2 => (
                    "`nchar(x) == 0` is inefficient.",
                    "Use `!nzchar(x)` instead.",
                    Some(format!("!nzchar({argument})")),
                ),
                RSyntaxKind::GREATER_THAN_OR_EQUAL_TO => (
                    "`nchar(x) >= 0` is always true.",
                    "Maybe you want `nzchar(x)` instead.",
                    None,
                ),
                RSyntaxKind::LESS_THAN => (
                    "`nchar(x) < 0` is always false.",
                    "Maybe you want `!nzchar(x)` instead.",
                    None,
                ),
                _ => unreachable!("Only comparison operators are normalized above"),
            };
            let fix = match replacement {
                Some(replacement) => {
                    Fix::new(range, replacement, node_contains_comments(ast.syntax()))
                }
                None => Fix::empty(),
            };
            return Ok(Some(Diagnostic::new(
                ViolationData::new(Rule::NzChar, body.to_string(), Some(suggestion.to_string())),
                range,
                fix,
            )));
        }
    }

    Ok(None)
}

fn flip_comparison_operator(operator: RSyntaxKind) -> RSyntaxKind {
    match operator {
        // Swapping operands turns `>` into `<` and vice versa.
        RSyntaxKind::GREATER_THAN => RSyntaxKind::LESS_THAN,
        RSyntaxKind::LESS_THAN => RSyntaxKind::GREATER_THAN,
        // Swapping operands turns `>=` into `<=` and vice versa.
        RSyntaxKind::GREATER_THAN_OR_EQUAL_TO => RSyntaxKind::LESS_THAN_OR_EQUAL_TO,
        RSyntaxKind::LESS_THAN_OR_EQUAL_TO => RSyntaxKind::GREATER_THAN_OR_EQUAL_TO,
        // Equality and inequality are unchanged when operands are swapped.
        operator => operator,
    }
}

fn is_zero_literal(expression: &AnyRExpression) -> bool {
    let Some(value) = expression.as_any_r_value() else {
        return false;
    };

    if let Some(integer) = value.as_r_integer_value() {
        return integer
            .value_token()
            .is_ok_and(|token| matches!(token.text_trimmed(), "0" | "0L"));
    }
    if let Some(double) = value.as_r_double_value() {
        return double
            .value_token()
            .is_ok_and(|token| matches!(token.text_trimmed(), "0" | "0.0" | "0."));
    }
    false
}
