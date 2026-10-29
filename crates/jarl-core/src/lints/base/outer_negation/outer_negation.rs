use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{argument_name, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// <!-- docs: start -->
/// Version added: 0.1.0
///
/// ## What it does
///
/// Checks for usage of `all(!x)` or `any(!x)`.
///
/// ## Why is this bad?
///
/// Those two patterns may be hard to read and understand, especially when the
/// expression after `!` is lengthy. Using `!any(x)` instead of `all(!x)` and
/// `!all(x)` instead of `any(!x)` may be more readable.
///
/// In addition, using the `!` operator outside the function call is more
/// efficient since it only has to invert one value instead of all values inside
/// the function call.
///
/// ## Example
///
/// ```r
/// any(!x)
/// all(!x)
/// any(!x, na.rm = TRUE)
/// ```
///
/// Use instead:
/// ```r
/// !all(x)
/// !any(x)
/// !all(x, na.rm = TRUE)
/// ```
/// <!-- docs: end -->
pub fn outer_negation(
    ast: &RCall,
    fn_name: &str,
    ns_prefix: Option<&str>,
) -> anyhow::Result<Option<Diagnostic>> {
    let replacement_function = match fn_name {
        "any" => "all",
        "all" => "any",
        _ => return Ok(None),
    };
    let function = ast.function()?;
    let name_range = match &function {
        AnyRExpression::RIdentifier(_) => function.syntax().text_trimmed_range(),
        AnyRExpression::RNamespaceExpression(namespace) if ns_prefix == Some("base::") => {
            namespace.right()?.syntax().text_trimmed_range()
        }
        _ => return Ok(None),
    };

    // A pipe can supply additional data that is not negated. Do not rewrite
    // its right-hand call, even when that call is wrapped in parentheses.
    let mut node = ast.syntax().clone();
    while let Some(parent) = node.parent() {
        if RParenthesizedExpression::can_cast(parent.kind()) {
            node = parent;
            continue;
        }
        if let Some(binary) = RBinaryExpression::cast(parent)
            && binary.right()?.syntax() == &node
        {
            let operator = binary.operator()?;
            if operator.kind() == RSyntaxKind::PIPE
                || (operator.kind() == RSyntaxKind::SPECIAL
                    && matches!(operator.text_trimmed(), "%>%" | "%!>%" | "%T>%" | "%<>%"))
            {
                return Ok(None);
            }
        }
        break;
    }

    // all() and any() take data in `...`; only an exact `na.rm` name is a
    // control argument. Keep one data argument and at most one na.rm argument.
    let mut data = None;
    let mut has_na_rm = false;
    for arg in ast.arguments()?.items() {
        let arg = arg?;
        let value = unwrap_or_return_none!(arg.value());
        if argument_name(&arg).as_deref() == Some("na.rm") {
            if has_na_rm {
                return Ok(None);
            }
            has_na_rm = true;
        } else if data.replace(value).is_some() {
            return Ok(None);
        }
    }

    let mut value = unwrap_or_return_none!(data);
    while let AnyRExpression::RParenthesizedExpression(paren) = value {
        value = paren.body()?;
    }
    let unary = unwrap_or_return_none!(value.as_r_unary_expression());
    if unary.operator()?.kind() != RSyntaxKind::BANG {
        return Ok(None);
    }
    let operand = unary.argument()?;
    // Leave tidy evaluation's !! and !!! alone, but allow !-x and !+x.
    if let AnyRExpression::RUnaryExpression(inner) = &operand
        && inner.operator()?.kind() == RSyntaxKind::BANG
    {
        return Ok(None);
    }

    // Group the new negation when this call is an operand, so e.g.
    // any(!x) + 2 becomes (!all(x)) + 2, and !any(!x) avoids creating !!all(x).
    let needs_parentheses = match ast.syntax().parent().and_then(AnyRExpression::cast) {
        Some(AnyRExpression::RBinaryExpression(binary)) => !matches!(
            binary.operator()?.kind(),
            RSyntaxKind::ASSIGN
                | RSyntaxKind::SUPER_ASSIGN
                | RSyntaxKind::ASSIGN_RIGHT
                | RSyntaxKind::SUPER_ASSIGN_RIGHT
                | RSyntaxKind::EQUAL
        ),
        Some(
            AnyRExpression::RUnaryExpression(_)
            | AnyRExpression::RExtractExpression(_)
            | AnyRExpression::RSubset(_)
            | AnyRExpression::RSubset2(_)
            | AnyRExpression::RCall(_),
        ) => true,
        _ => false,
    };

    let range = ast.syntax().text_trimmed_range();
    // Edit just the callee and negated expression to retain argument names,
    // order, formatting, and the original namespace qualifier.
    let mut edits = vec![
        Edit::insertion(
            range.start(),
            if needs_parentheses { "(!" } else { "!" }.to_string(),
        ),
        Edit::replacement(name_range, replacement_function.to_string()),
        Edit::replacement(
            unary.syntax().text_trimmed_range(),
            operand.to_trimmed_string(),
        ),
    ];
    if needs_parentheses {
        edits.push(Edit::insertion(range.end(), ")".to_string()));
    }

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::OuterNegation,
            format!("`{fn_name}(!...)` may be hard to read."),
            Some(format!("Use `!{replacement_function}(...)` instead.")),
        ),
        range,
        Fix::from_edits(edits, node_contains_comments(ast.syntax())),
    )))
}
