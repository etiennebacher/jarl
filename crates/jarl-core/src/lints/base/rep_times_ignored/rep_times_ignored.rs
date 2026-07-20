use crate::diagnostic::*;
use crate::utils::{get_function_name, get_function_namespace_prefix, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for `rep()` calls that supply both `times` and `length.out`.
///
/// ## Why is this bad?
///
/// When both arguments are supplied, `length.out` takes priority and `times`
/// is ignored. This likely indicates a mistake in the call.
///
/// This rule is disabled by default and has an unsafe fix because
/// `length.out` can evaluate to `NA` or another invalid value, in which case
/// `times` is still used.
///
/// ## Example
///
/// ```r
/// rep(1:3, times = 2, length.out = 10)
/// ```
///
/// Use instead:
/// ```r
/// rep(1:3, length.out = 10)
/// ```
///
/// ## References
///
/// See `?rep`
pub fn rep_times_ignored(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let RCallFields { function, arguments } = ast.as_fields();
    let function = function?;

    if get_function_name(function.clone()) != "rep" {
        return Ok(None);
    }

    let namespace_prefix = get_function_namespace_prefix(function);
    if namespace_prefix
        .as_deref()
        .is_some_and(|namespace| namespace != "base::")
    {
        return Ok(None);
    }

    let arguments = arguments?.items();

    if !(3..=4).contains(&arguments.iter().count()) {
        return Ok(None);
    }

    // Match the formals in `rep(x, times, length.out, each)` order. R resolves
    // exact names before assigning unnamed arguments to the remaining slots.
    let mut matched_arguments = [None, None, None, None];
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
            "each" => 3,
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

    let [
        Some(object_argument),
        Some(times_argument),
        Some(length_argument),
        each_argument,
    ] = matched_arguments
    else {
        return Ok(None);
    };
    let object = unwrap_or_return_none!(object_argument.value());
    let _times = unwrap_or_return_none!(times_argument.value());
    let length = unwrap_or_return_none!(length_argument.value());

    // A missing `each` value (`each =`) has the same effect as omitting it.
    let each = each_argument.and_then(|argument| argument.value());

    let range = ast.syntax().text_trimmed_range();
    let namespace_prefix = namespace_prefix.unwrap_or_default();
    let each = each
        .map(|each| format!(", each = {}", each.to_trimmed_text()))
        .unwrap_or_default();
    let replacement = format!(
        "{namespace_prefix}rep({}, length.out = {}{each})",
        object.to_trimmed_text(),
        length.to_trimmed_text()
    );

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            "rep_times_ignored".to_string(),
            "`times` is ignored when `length.out` is supplied.".to_string(),
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
