use crate::diagnostic::*;
use crate::utils::{get_function_name, get_function_namespace_prefix, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for usage of `rep(x, length.out = n)`.
///
/// ## Why is this bad?
///
/// `rep(x, length.out = n)` calls `rep_len(x, n)` internally. The latter
/// is thus more direct and equally readable.
///
/// This rule is disabled by default.
///
/// This rule has an unsafe automatic fix because `rep_len()` drops most
/// attributes, including names, while `rep()` can preserve them.
///
/// ## Example
///
/// ```r
/// rep(1:3, length.out = 10)
/// ```
///
/// Use instead:
/// ```r
/// rep_len(1:3, 10)
/// ```
///
/// ## References
///
/// See `?rep`
pub fn rep_len(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let RCallFields { function, arguments } = ast.as_fields();
    let function = function?;

    if get_function_name(function.clone()) != "rep" {
        return Ok(None);
    }

    let arguments = arguments?.items();

    let argument_count = arguments.iter().count();
    if !(2..=3).contains(&argument_count) {
        return Ok(None);
    }

    // Match the formals in `rep(x, times, length.out)` order. R resolves exact
    // argument names before assigning unnamed arguments to the remaining slots.
    let mut matched_arguments = [None, None, None];
    let mut unnamed_arguments = Vec::new();
    for argument in arguments.iter() {
        let argument = argument?;
        let Some(name_clause) = argument.name_clause() else {
            unnamed_arguments.push(argument);
            continue;
        };
        let position = match name_clause.name()?.to_trimmed_string().as_str() {
            "x" => 0,
            "times" => 1,
            "length.out" => 2,
            // Skip partial and unknown names rather than risk an incorrect fix.
            _ => return Ok(None),
        };
        if matched_arguments[position].replace(argument).is_some() {
            return Ok(None);
        }
    }

    // Positional arguments fill the first unmatched formal from left to right.
    for argument in unnamed_arguments {
        let slot = unwrap_or_return_none!(matched_arguments.iter_mut().find(|slot| slot.is_none()));
        *slot = Some(argument);
    }

    // Only the `x` and `length.out` arguments are required for a valid replacement.
    let [Some(object_argument), _, Some(length_argument)] = matched_arguments else {
        return Ok(None);
    };
    let object = unwrap_or_return_none!(object_argument.value());
    let length = unwrap_or_return_none!(length_argument.value());
    if length.as_r_na_expression().is_some() {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();
    let namespace_prefix = get_function_namespace_prefix(function).unwrap_or_default();
    let replacement = format!(
        "{namespace_prefix}rep_len({}, {})",
        object.to_trimmed_text(),
        length.to_trimmed_text()
    );
    let linted_text = ast.to_trimmed_text();

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            "rep_len".to_string(),
            format!("`{replacement}` is more explicit than `{linted_text}`."),
            Some(format!("Use `{replacement}` instead.")),
        ),
        range,
        Fix {
            content: replacement,
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    )))
}
