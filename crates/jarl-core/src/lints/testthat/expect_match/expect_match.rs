use crate::diagnostic::*;
use crate::utils::{
    get_arg_by_name_then_position, get_function_name, get_function_namespace_prefix,
    get_nested_functions_content, node_contains_comments,
};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

pub struct ExpectMatch;

/// ## What it does
///
/// Checks for usage of `expect_true(grepl(...))`.
///
/// ## Why is this bad?
///
/// `expect_match()` is more explicit and clearer in intent than wrapping
/// `grepl()` in `expect_true()`. It also provides better error messages when
/// tests fail.
///
/// This rule is **disabled by default**. Select it either with the rule name
/// `"expect_match"` or with the rule group `"TESTTHAT"`.
///
/// ## Example
///
/// ```r
/// expect_true(grepl("foo", x))
/// expect_true(base::grepl("bar", x))
/// ```
///
/// Use instead:
/// ```r
/// expect_match(x, "foo")
/// expect_match(x, "bar")
/// ```
impl Violation for ExpectMatch {
    fn name(&self) -> String {
        "expect_match".to_string()
    }

    fn body(&self) -> String {
        "`expect_true(grepl(...))` is not as clear as expect_match(...).".to_string()
    }

    fn suggestion(&self) -> Option<String> {
        Some("Use `expect_match(...)` instead.".to_string())
    }
}

pub fn expect_match(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let range = ast.syntax().text_trimmed_range();
    let function = ast.function()?;
    let function_name = get_function_name(function.clone());
    if function_name != "expect_true" {
        return Ok(None);
    }

    // For pipe cases (`grepl(...) |> expect_true()`), lint but skip fix.
    // Fix seems reasonably complex as x & pattern position are swapped
    if let Some((_inner_content, outer_syntax)) =
        get_nested_functions_content(ast, "expect_true", "grepl")?
        && outer_syntax.kind() == RSyntaxKind::R_BINARY_EXPRESSION
    {
        // Ignore negated pipe (e.g. `!grepl(...) |> expect_true()`) false positive
        // This would be covered by `expect_no_match` or `expect_not`
        if let Some(parent) = outer_syntax.parent()
            && let Some(unary) = RUnaryExpression::cast(parent)
            && let Ok(operator) = unary.operator()
            && operator.kind() == RSyntaxKind::BANG
        {
            return Ok(None);
        }

        let range = outer_syntax.text_trimmed_range();
        return Ok(Some(Diagnostic::new(ExpectMatch, range, Fix::empty())));
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

    // It all grepl args can be passed to expect_match, so keep them all for fix
    let grepl_args = grepl_call.arguments()?.items();
    let pattern_arg =
        unwrap_or_return_none!(get_arg_by_name_then_position(&grepl_args, "pattern", 1));
    let x_arg = unwrap_or_return_none!(get_arg_by_name_then_position(&grepl_args, "x", 2));

    let pattern_value = unwrap_or_return_none!(pattern_arg.value());
    let x_value = unwrap_or_return_none!(x_arg.value());
    let x_text = x_value.to_trimmed_text().to_string();
    let pattern_text = pattern_value.to_trimmed_text().to_string();

    // Give lint but no fix if expect_true has additional args
    let outer_args_count = args.iter().count();
    if outer_args_count > 1 {
        return Ok(Some(Diagnostic::new(ExpectMatch, range, Fix::empty())));
    }

    let pattern_range = pattern_arg.syntax().text_trimmed_range();
    let x_range = x_arg.syntax().text_trimmed_range();
    // Get extra args, skipping x and pattern
    let mut extra_args: Vec<String> = Vec::new();
    for arg in grepl_args.iter() {
        let arg = arg.clone().unwrap();
        let arg_range = arg.syntax().text_trimmed_range();
        if arg_range == pattern_range || arg_range == x_range {
            continue;
        }
        extra_args.push(arg.to_trimmed_text().to_string());
    }

    // Preserve namespace prefix if present
    let namespace_prefix = get_function_namespace_prefix(function).unwrap_or_default();

    let mut grepl_args = vec![x_text, pattern_text];
    grepl_args.extend(extra_args);

    let diagnostic = Diagnostic::new(
        ExpectMatch,
        range,
        Fix {
            content: format!(
                "{}expect_match({})",
                namespace_prefix,
                grepl_args.join(", ")
            ),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
