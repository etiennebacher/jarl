use crate::diagnostic::*;
use crate::utils::{
    get_arg_by_name, get_arg_by_name_then_position, get_function_name,
    get_nested_functions_content, node_contains_comments,
};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

/// Version added: 0.0.8
///
/// ## What it does
///
/// Checks for usage of `which(grepl(...))` and replaces it with `grep(...)`.
///
/// ## Why is this bad?
///
/// `which(grepl(...))` is harder to read and is less efficient than `grep()`
/// since it requires two passes on the vector.
///
/// ## Example
///
/// ```r
/// which(grepl("foo", x))
/// ```
///
/// Use instead:
/// ```r
/// grep("foo", x)
/// ```
///
/// ## References
///
/// See `?grep`
pub fn which_grepl(ast: &RCall, fn_name: &str) -> anyhow::Result<Option<Diagnostic>> {
    if fn_name != "which" {
        return Ok(None);
    }

    let arguments = ast.arguments()?.items();

    // Handle `which(grepl(...))`, including a named `x` argument to `which()`.
    let direct_content = if let Some(argument) = get_arg_by_name_then_position(&arguments, "x", 1)
        && let Some(value) = argument.value()
        && let Some(inner_call) = value.as_r_call()
        && get_function_name(inner_call.function()?) == "grepl"
    {
        Some(inner_call.arguments()?.items().into_syntax().to_string())
    } else {
        None
    };

    let (inner_content, outer_syntax, can_fix) = if let Some(content) = direct_content {
        (content, ast.syntax().clone(), arguments.len() == 1)
    } else {
        // Handle pipeline input.
        let (mut content, syntax) = unwrap_or_return_none!(get_nested_functions_content(
            ast, fn_name, "which", "grepl"
        )?);

        // The shared helper returns the piped input for
        // `input |> grepl(...) |> which()`, but not the named `grepl()` args.
        // Append those arguments so the replacement preserves the full call.
        if let Some(outer_pipe) = RBinaryExpression::cast(syntax.clone())
            && let Some(inner_pipe) = outer_pipe.left()?.as_r_binary_expression()
        {
            let inner_right = inner_pipe.right()?;
            let inner_call = unwrap_or_return_none!(inner_right.as_r_call());
            let inner_arguments = inner_call.arguments()?.items();
            let has_pattern = get_arg_by_name(&inner_arguments, "pattern").is_some();
            let has_x = get_arg_by_name(&inner_arguments, "x").is_some();

            // The pipe supplies the first unnamed argument. Requiring exactly
            // one of `pattern` and `x` makes its destination unambiguous.
            if has_pattern == has_x {
                return Ok(None);
            }

            content = format!("{content}, {}", inner_arguments.into_syntax());
        }

        (content, syntax, arguments.is_empty())
    };

    let range = outer_syntax.text_trimmed_range();
    let replacement = format!("grep({inner_content})");

    let linted_text = outer_syntax.text_trimmed().to_string();
    Ok(Some(Diagnostic::new(
        ViolationData::new(
            "which_grepl".to_string(),
            format!("`{linted_text}` is less efficient than `{replacement}`."),
            Some(format!("Use `{replacement}` instead.")),
        ),
        range,
        if can_fix {
            Fix {
                content: replacement.clone(),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(&outer_syntax),
            }
        } else {
            Fix::empty()
        },
    )))
}
