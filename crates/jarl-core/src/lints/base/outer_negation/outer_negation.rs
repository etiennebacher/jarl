use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{get_arg, node_contains_comments};
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
/// This rule has an automated fix marked unsafe because `any()` and `all()` coerce the
/// input to logical with a warning while `!` would error, e.g. `any(!"a")` is an error but
/// `!all("a")` returns `NA` with a warning.
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
    while let Some(parent) = node
        .parent()
        .filter(|parent| RParenthesizedExpression::can_cast(parent.kind()))
    {
        node = parent;
    }
    if let Some(binary) = node.parent().and_then(RBinaryExpression::cast)
        && binary.right()?.syntax() == &node
        && matches!(
            binary.operator()?.text_trimmed(),
            "|>" | "%>%" | "%!>%" | "%T>%" | "%<>%"
        )
    {
        return Ok(None);
    }

    // Check if there is an outer negation into the call, so !any(!x) becomes all(x).
    let negation = node
        .parent()
        .and_then(RUnaryExpression::cast)
        .filter(|unary| {
            unary
                .operator()
                .is_ok_and(|operator| operator.kind() == RSyntaxKind::BANG)
        });

    // `...` stops positional matching, so only an exact name binds to na.rm.
    // Duplicate na.rm names make get_arg() return None, leaving both arguments
    // in the data count below so the call is skipped.
    let na_rm = get_arg(ast, &["...", "na.rm"], "na.rm");
    let mut data = None;
    for arg in ast.arguments()?.items() {
        let arg = arg?;
        let value = unwrap_or_return_none!(arg.value());
        if na_rm
            .as_ref()
            .is_some_and(|control| control.syntax() == arg.syntax())
        {
            continue;
        }
        if data.replace(value).is_some() {
            return Ok(None);
        }
    }

    let mut value = unwrap_or_return_none!(data);
    let data_range = value.syntax().text_trimmed_range();
    while let AnyRExpression::RParenthesizedExpression(paren) = value {
        value = paren.body()?;
    }
    let unary = unwrap_or_return_none!(value.as_r_unary_expression());
    if unary.operator()?.kind() != RSyntaxKind::BANG {
        return Ok(None);
    }
    let mut operand = unary.argument()?;
    // Leave tidy evaluation's !! and !!! alone, but allow !-x and !+x.
    if let AnyRExpression::RUnaryExpression(inner) = &operand
        && inner.operator()?.kind() == RSyntaxKind::BANG
    {
        return Ok(None);
    }

    // Strip surrounding parentheses, except those that keep `=` an assignment
    // or prevent `...` from expanding into several arguments.
    while let AnyRExpression::RParenthesizedExpression(paren) = &operand {
        let body = paren.body()?;
        if matches!(body, AnyRExpression::RDots(_))
            || matches!(&body, AnyRExpression::RBinaryExpression(binary)
                if binary.operator()?.kind() == RSyntaxKind::EQUAL)
        {
            break;
        }
        operand = body;
    }

    // Group the new negation when this call is an operand, so e.g.
    // any(!x) + 2 becomes (!all(x)) + 2.
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

    let call_range = ast.syntax().text_trimmed_range();
    let (outer, old_bang, new_bang) = match &negation {
        Some(negation) => (negation.syntax(), "!", ""),
        None => (ast.syntax(), "", "!"),
    };
    let range = outer.text_trimmed_range();
    // Edit just the callee and negated expression to retain argument names,
    // order, formatting, and the original namespace qualifier.
    let mut edits = vec![
        Edit::replacement(name_range, replacement_function.to_string()),
        Edit::replacement(data_range, operand.to_trimmed_string()),
    ];
    if negation.is_some() {
        // Drop the outer `!` and any parentheses around the call.
        edits.push(Edit::deletion(TextRange::new(
            range.start(),
            call_range.start(),
        )));
        edits.push(Edit::deletion(TextRange::new(
            call_range.end(),
            range.end(),
        )));
    } else if needs_parentheses {
        edits.push(Edit::insertion(range.start(), "(!".to_string()));
        edits.push(Edit::insertion(range.end(), ")".to_string()));
    } else {
        edits.push(Edit::insertion(range.start(), "!".to_string()));
    }

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::OuterNegation,
            format!("`{old_bang}{fn_name}(!...)` may be hard to read."),
            Some(format!(
                "Use `{new_bang}{replacement_function}(...)` instead."
            )),
        ),
        range,
        Fix::from_edits(edits, node_contains_comments(outer)),
    )))
}
