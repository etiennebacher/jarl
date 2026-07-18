use crate::diagnostic::*;
use crate::utils::{
    get_arg_by_name, get_arg_by_name_then_position, get_function_name,
    get_function_namespace_prefix, node_contains_comments,
};
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

    if arguments.iter().count() != 2 {
        return Ok(None);
    }

    let object_argument = unwrap_or_return_none!(get_arg_by_name_then_position(&arguments, "x", 1));
    let length_argument = unwrap_or_return_none!(get_arg_by_name(&arguments, "length.out"));
    let object = unwrap_or_return_none!(object_argument.value());
    let length = unwrap_or_return_none!(length_argument.value());

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
