use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{
    get_arg, get_function_name, get_function_namespace_prefix, node_contains_comments,
};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

/// <!-- docs: start -->
/// Version added: 0.8.0
///
/// ## What it does
///
/// Checks for `names(list(...))`, `length(list(...))`, and `list(...)[[n]]`
/// where `...` is the function's variadic arguments.
///
/// ## Why is this bad?
///
/// `...names()`, `...length()`, and `...elt(n)` access `...` directly without
/// constructing a list. For positive integer literal indices, `..1`, `..2`,
/// and so on are more concise than `...elt(1)`, `...elt(2)`, and so on.
/// Character literal indices and additional list elements are not checked.
///
/// This rule provides an automatic fix and only applies to projects supporting
/// R >= 4.1.0, when `...names()` became available.
/// The replacements do not evaluate unselected arguments, so side effects and
/// errors in those arguments will no longer occur. Element access assumes a
/// single numeric index; character and recursive indices need `list(...)`.
///
/// ## Example
///
/// ```r
/// f <- function(...) {
///   names(list(...))
///   length(list(...))
///   list(...)[[2L]]
///   list(...)[[i + 1]]
/// }
/// ```
///
/// Use instead:
/// ```r
/// f <- function(...) {
///   ...names()
///   ...length()
///   ..2
///   ...elt(i + 1)
/// }
/// ```
/// <!-- docs: end -->
///
/// Read the names of `...` without evaluating the arguments.
pub fn dots_names(
    ast: &RCall,
    fn_name: &str,
    ns_prefix: Option<&str>,
) -> anyhow::Result<Option<Diagnostic>> {
    if fn_name != "names" || ns_prefix.is_some_and(|ns| ns != "base::") {
        return Ok(None);
    }
    dots_list_call(ast, "...names()")
}

/// Count `...` without evaluating the arguments.
pub fn dots_length(
    ast: &RCall,
    fn_name: &str,
    ns_prefix: Option<&str>,
) -> anyhow::Result<Option<Diagnostic>> {
    if fn_name != "length" || ns_prefix.is_some_and(|ns| ns != "base::") {
        return Ok(None);
    }
    dots_list_call(ast, "...length()")
}

/// Select one argument, using `..n` for positive integer literals.
pub fn dots_elt(ast: &RSubset2) -> anyhow::Result<Option<Diagnostic>> {
    if !is_dots_list(ast.function()?)? {
        return Ok(None);
    }
    let args = ast.arguments()?.items();
    if args.len() != 1 {
        return Ok(None);
    }
    let arg = unwrap_or_return_none!(args.iter().next())?;
    if arg.name_clause().is_some() {
        return Ok(None);
    }
    let mut index = unwrap_or_return_none!(arg.value());
    while let AnyRExpression::RParenthesizedExpression(paren) = index {
        index = paren.body()?;
    }

    // `[[` also supports names and recursive indices; `...elt()` does not.
    let supported = match &index {
        AnyRExpression::AnyRValue(value) => matches!(
            value,
            AnyRValue::RIntegerValue(_) | AnyRValue::RDoubleValue(_)
        ),
        AnyRExpression::RNaExpression(_)
        | AnyRExpression::RNullExpression(_)
        | AnyRExpression::RTrueExpression(_)
        | AnyRExpression::RFalseExpression(_)
        | AnyRExpression::RInfExpression(_)
        | AnyRExpression::RNanExpression(_)
        | AnyRExpression::RDots(_) => false,
        // Only accept `list(...)[[+n]]`. `[[-n]]` and `[[!n]]` are not supported.
        AnyRExpression::RUnaryExpression(unary) => unary.operator()?.kind() == RSyntaxKind::PLUS,
        // Skip binary expressions with colon like `list(...)[[1:2]]`
        AnyRExpression::RBinaryExpression(binary) => {
            binary.operator()?.kind() != RSyntaxKind::COLON
        }
        // Skip function calls like `list(...)[[c(1, 2)]]` and `list(...)[[list(1)]]`.
        AnyRExpression::RCall(call) => {
            !matches!(get_function_name(call.function()?).as_str(), "c" | "list")
        }
        _ => true,
    };
    if !supported {
        return Ok(None);
    }

    // Whole-number literals use `..n`; other supported indices use `...elt()`.
    let position = if matches!(
        &index,
        AnyRExpression::AnyRValue(AnyRValue::RIntegerValue(_) | AnyRValue::RDoubleValue(_))
    ) {
        let text = index.to_trimmed_string();
        let digits = text.trim_end_matches('L');
        let Ok(number) = digits.parse::<f64>() else {
            return Ok(None);
        };
        if number < 1.0 {
            return Ok(None);
        }
        (number.fract() == 0.0).then_some(number as u32)
    } else {
        None
    };

    let replacement = position.map_or_else(
        || format!("...elt({})", index.to_trimmed_text()),
        |n| format!("..{n}"),
    );
    Ok(diagnostic(ast.syntax(), replacement))
}

fn dots_list_call(ast: &RCall, replacement: &str) -> anyhow::Result<Option<Diagnostic>> {
    // Only accept `names(list(...))` or `names(x = list(...))` (likewise for `length`).
    if ast.arguments()?.items().len() != 1 {
        return Ok(None);
    }
    let arg = unwrap_or_return_none!(get_arg(ast, &["x"], "x"));
    let value = unwrap_or_return_none!(arg.value());
    if !is_dots_list(value)? {
        return Ok(None);
    }
    Ok(diagnostic(ast.syntax(), replacement.to_string()))
}

/// Only a list containing the forwarded arguments, with no extra or named items.
fn is_dots_list(mut expression: AnyRExpression) -> anyhow::Result<bool> {
    while let AnyRExpression::RParenthesizedExpression(paren) = expression {
        expression = paren.body()?;
    }
    let Some(call) = expression.as_r_call() else {
        return Ok(false);
    };
    let function = call.function()?;
    if get_function_name(function.clone()) != "list"
        || get_function_namespace_prefix(function).is_some_and(|ns| ns != "base::")
    {
        return Ok(false);
    }
    // The inner call must contain only an unnamed `...` argument.
    let args = call.arguments()?.items();
    if args.len() != 1 {
        return Ok(false);
    }
    let Some(arg) = args.iter().next() else {
        return Ok(false);
    };
    let arg = arg?;
    Ok(arg.name_clause().is_none() && matches!(arg.value(), Some(AnyRExpression::RDots(_))))
}

fn diagnostic(node: &RSyntaxNode, replacement: String) -> Option<Diagnostic> {
    // Replacement assignments need the list, but their indices are ordinary reads.
    // Skip `names(list(...)) <- nms` and `value -> list(...)[[1]]`,
    // but allow reads such as `x[[length(list(...))]] <- value`.
    for binary in node
        .ancestors()
        .skip(1)
        .take_while(|ancestor| {
            !RSubsetArguments::can_cast(ancestor.kind())
                && !RSubset2Arguments::can_cast(ancestor.kind())
        })
        .filter_map(RBinaryExpression::cast)
    {
        let target = match binary.operator().ok()?.kind() {
            RSyntaxKind::ASSIGN | RSyntaxKind::SUPER_ASSIGN | RSyntaxKind::EQUAL => {
                binary.left().ok()?
            }
            RSyntaxKind::ASSIGN_RIGHT | RSyntaxKind::SUPER_ASSIGN_RIGHT => binary.right().ok()?,
            _ => continue,
        };
        if target.range().contains_range(node.text_trimmed_range()) {
            return None;
        }
    }

    // A pipe can supply another argument that is absent from the call's AST.
    if let Some(parent) = node
        .ancestors()
        .skip(1)
        .find(|ancestor| !RParenthesizedExpression::can_cast(ancestor.kind()))
        && let Some(binary) = RBinaryExpression::cast(parent)
        && binary
            .right()
            .ok()?
            .range()
            .contains_range(node.text_trimmed_range())
    {
        let operator = binary.operator().ok()?;
        if operator.kind() == RSyntaxKind::PIPE
            || matches!(operator.text_trimmed(), "%>%" | "%!>%" | "%T>%" | "%<>%")
        {
            return None;
        }
    }

    let range = node.text_trimmed_range();
    Some(Diagnostic::new(
        ViolationData::new(
            Rule::DotsAccess,
            "Constructing a list to access `...` is unnecessary.".to_string(),
            Some(format!("Use `{replacement}` instead.")),
        ),
        range,
        Fix::new(range, replacement, node_contains_comments(node)),
    ))
}
