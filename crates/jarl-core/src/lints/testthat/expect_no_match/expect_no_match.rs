use crate::diagnostic::*;
use crate::utils::{
    get_arg_by_name_then_position, get_function_name, get_function_namespace_prefix,
    get_nested_functions_content, node_contains_comments,
};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

pub struct ExpectNoMatch;

/// ## What it does
///
/// Checks for usage of `expect_false(grepl(...))`.
///
/// ## Why is this bad?
///
/// `expect_no_match()` is more explicit and clearer in intent than wrapping
/// `grepl()` in `expect_false()`. It also provides better error messages when
/// tests fail.
///
/// Note: negated forms like `expect_false(!grepl(...))` are intentionally
/// ignored by this rule and handled by `expect_not`.
///
/// This rule is **disabled by default**. Select it either with the rule name
/// `"expect_no_match"` or with the rule group `"TESTTHAT"`.
///
/// ## Example
///
/// ```r
/// expect_false(grepl("foo", x))
/// expect_false(grepl("bar", x, perl = FALSE, fixed = FALSE))
/// ```
///
/// Use instead:
/// ```r
/// expect_no_match(x, "foo")
/// expect_no_match(x, "bar", perl = FALSE, fixed = FALSE)
/// ```
impl Violation for ExpectNoMatch {
    fn name(&self) -> String {
        "expect_no_match".to_string()
    }

    fn body(&self) -> String {
        "`expect_false(grepl(...))` is not as clear as `expect_no_match(...)`.".to_string()
    }

    fn suggestion(&self) -> Option<String> {
        Some("Use `expect_no_match(...)` instead.".to_string())
    }
}

pub fn expect_no_match(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let range = ast.syntax().text_trimmed_range();
    let function = ast.function()?;
    let function_name = get_function_name(function.clone());
    if function_name != "expect_false" {
        return Ok(None);
    }

    // For pipe cases (`grepl(...) |> expect_false()`), lint but skip fix.
    // Fix seems reasonably complex as x & pattern position are swapped
    if let Some((_inner_content, outer_syntax)) =
        get_nested_functions_content(ast, "expect_false", "grepl")?
        && outer_syntax.kind() == RSyntaxKind::R_BINARY_EXPRESSION
    {
        // Ignore negated pipe (e.g. `!grepl(...) |> expect_false()`) false positive.
        // Negation is intentionally handled by `expect_not`.
        if let Some(parent) = outer_syntax.parent()
            && let Some(unary) = RUnaryExpression::cast(parent)
            && let Ok(operator) = unary.operator()
            && operator.kind() == RSyntaxKind::BANG
        {
            return Ok(None);
        }

        let range = outer_syntax.text_trimmed_range();
        return Ok(Some(Diagnostic::new(ExpectNoMatch, range, Fix::empty())));
    }

    let args = ast.arguments()?.items();

    // Get first argument
    let object = unwrap_or_return_none!(get_arg_by_name_then_position(&args, "object", 1));

    let object_value = unwrap_or_return_none!(object.value());

    let grepl_call = unwrap_or_return_none!(object_value.as_r_call());
    let grepl_function = grepl_call.function()?;
    let grepl_name = get_function_name(grepl_function);
    if grepl_name != "grepl" {
        return Ok(None);
    }

    // All grepl args can be passed to expect_no_match, so keep them all for fix
    let grepl_args = grepl_call.arguments()?.items();
    let pattern_arg =
        unwrap_or_return_none!(get_arg_by_name_then_position(&grepl_args, "pattern", 1));
    let x_arg = unwrap_or_return_none!(get_arg_by_name_then_position(&grepl_args, "x", 2));

    let x_text = unwrap_or_return_none!(x_arg.value())
        .to_trimmed_text()
        .to_string();
    let pattern_text = unwrap_or_return_none!(pattern_arg.value())
        .to_trimmed_text()
        .to_string();

    // Give lint but no fix if expect_false has additional args
    let outer_args_count = args.iter().count();
    if outer_args_count > 1 {
        return Ok(Some(Diagnostic::new(ExpectNoMatch, range, Fix::empty())));
    }

    // Collect remaining args (neither `x` nor `pattern`) to pass to expect_no_match.
    // If any extra grepl args are positional, lint but skip fix to avoid changing
    // semantics when converting to expect_no_match(..., ..., ..., perl =, fixed =).
    let pattern_range = pattern_arg.syntax().text_trimmed_range();
    let x_range = x_arg.syntax().text_trimmed_range();

    let has_unnamed_optional_grepl_arg = grepl_args
        .iter()
        .flatten()
        .filter(|arg| {
            let range = arg.syntax().text_trimmed_range();
            range != pattern_range && range != x_range
        })
        .any(|arg| arg.name_clause().is_none());

    if has_unnamed_optional_grepl_arg {
        return Ok(Some(Diagnostic::new(ExpectNoMatch, range, Fix::empty())));
    }

    let optional_args: Vec<String> = grepl_args
        .iter()
        .flatten()
        .filter(|arg| {
            let range = arg.syntax().text_trimmed_range();
            range != pattern_range && range != x_range
        })
        .map(|arg| arg.syntax().text_trimmed().to_string())
        .collect();

    let inner_content = [x_text, pattern_text]
        .into_iter()
        .chain(optional_args)
        .collect::<Vec<_>>();

    // Preserve namespace prefix if present
    let namespace_prefix = get_function_namespace_prefix(function).unwrap_or_default();

    let diagnostic = Diagnostic::new(
        ExpectNoMatch,
        range,
        Fix {
            content: format!(
                "{}expect_no_match({})",
                namespace_prefix,
                inner_content.join(", ")
            ),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
