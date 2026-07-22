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
/// This rule has an automatic fix for direct calls where `which()` only
/// contains the `grepl()` call, and for pipe chains where the final `which()`
/// has no arguments and the piped value can be unambiguously assigned to
/// `grepl()`'s `pattern` or `x` argument.
///
/// Calls with additional arguments to `which()` are reported but not fixed
/// because those arguments cannot generally be preserved by replacing
/// `which()` with `grep()`. The exception is a literal `arr.ind = TRUE` or
/// `FALSE`: `grepl()` returns a vector without dimensions, so `arr.ind` has no
/// effect.
///
/// ## Example
///
/// ```r
/// x <- c("hello", "there")
/// which(grepl("hell", x))
/// which(grepl("foo", x))
/// ```
///
/// Use instead:
/// ```r
/// grep("hell", x)
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
    let direct_match = if let Some(argument) = get_arg_by_name_then_position(&arguments, "x", 1)
        && let Some(value) = argument.value()
        && let Some(inner_call) = value.as_r_call()
        && get_function_name(inner_call.function()?) == "grepl"
    {
        // Keep the outer argument so it can be excluded when checking whether
        // `which()` has any additional arguments that prevent a safe fix.
        Some((
            inner_call.arguments()?.items().into_syntax().to_string(),
            argument,
        ))
    } else {
        None
    };

    let (inner_content, outer_syntax, input) = if let Some((content, argument)) = direct_match {
        (content, ast.syntax().clone(), Some(argument))
    } else {
        // Handle pipeline input.
        let nested_content = get_nested_functions_content(ast, fn_name, "which", "grepl")?;
        let (mut content, syntax) = unwrap_or_return_none!(nested_content);

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

        (content, syntax, None)
    };

    // A direct call includes the `grepl()` argument in `arguments`, whereas a
    // pipe supplies it implicitly. After excluding that input, both forms can
    // use the same logic for checking additional arguments. The count check
    // below also prevents a fix if any argument failed to parse.
    let extra_arguments = arguments
        .iter()
        .filter_map(Result::ok)
        .filter(|argument| {
            input
                .as_ref()
                .is_none_or(|input| argument.syntax() != input.syntax())
        })
        .collect::<Vec<_>>();
    let can_fix = extra_arguments.len() + usize::from(input.is_some()) == arguments.len()
        && match extra_arguments.as_slice() {
            [] => true,
            [argument] => {
                // Once `x` has been supplied, a sole unnamed argument occupies
                // the `arr.ind` position. Only boolean literals are safe to
                // remove because evaluating a dynamic value may have side effects.
                let is_arr_ind = argument.name_clause().is_none_or(|clause| {
                    clause
                        .name()
                        .is_ok_and(|name| name.to_string().trim() == "arr.ind")
                });

                is_arr_ind
                    && argument.value().is_some_and(|value| {
                        value.as_r_true_expression().is_some()
                            || value.as_r_false_expression().is_some()
                    })
            }
            _ => false,
        };

    let range = outer_syntax.text_trimmed_range();
    let replacement = format!("grep({inner_content})");

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            "which_grepl".to_string(),
            "`which(grepl(pattern, x))` is less efficient than `grep(pattern, x)`.".to_string(),
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
