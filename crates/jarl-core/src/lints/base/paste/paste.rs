use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{
    argument_name, get_arg_by_name, get_function_name, get_unnamed_args, node_contains_comments,
};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

/// <!-- docs: start -->
/// Version added: unreleased
///
/// ## What it does
///
/// Checks for calls to `paste()` and `paste0()` that can be replaced by a
/// simpler function:
///
/// 1. `paste(..., sep = "")` can be replaced by `paste0(...)`;
/// 2. `paste(x, collapse = ", ")` can be replaced by `toString(x)`;
/// 3. `paste(rep(x, times), collapse = "")` can be replaced by
///    `strrep(x, times)`.
///
/// The same applies to `paste0()` for the last two cases.
///
/// `paste(..., sep = "")` inside `expression()` is not reported: there,
/// `paste()` is the plotmath function that juxtaposes its arguments (see
/// `?plotmath`).
///
/// ## Why is this bad?
///
/// `paste0()`, `toString()` and `strrep()` are dedicated functions for these
/// cases. They are shorter and make the intent clearer.
///
/// This rule has an automatic fix.
///
/// ## Example
///
/// ```r
/// paste("a", "b", sep = "")
/// paste(x, collapse = ", ")
/// paste0(rep("*", 10), collapse = "")
/// ```
///
/// Use instead:
/// ```r
/// paste0("a", "b")
/// toString(x)
/// strrep("*", 10)
/// ```
///
/// ## References
///
/// See `?paste`, `?toString`, `?strrep` and `?plotmath`
/// <!-- docs: end -->
pub fn paste(
    ast: &RCall,
    fn_name: &str,
    ns_prefix: Option<&str>,
) -> anyhow::Result<Option<Diagnostic>> {
    if fn_name != "paste" && fn_name != "paste0" {
        return Ok(None);
    }
    if let Some(ns) = ns_prefix
        && ns != "base::"
    {
        return Ok(None);
    }

    let arg_list = ast.arguments()?.items();
    let args: Vec<RArgument> = arg_list.iter().filter_map(|arg| arg.ok()).collect();

    // Skip calls with an argument that has no value, e.g. `paste(x, sep = )`.
    if args.iter().any(|arg| arg.value().is_none()) {
        return Ok(None);
    }

    if let Some(diagnostic) = check_collapse(ast, fn_name, &arg_list, &args)? {
        return Ok(Some(diagnostic));
    }

    check_empty_sep(ast, fn_name, ns_prefix, &arg_list, &args)
}

/// Reports `paste(rep(x, times), collapse = "")` and `paste(x, collapse = ", ")`.
///
/// Both need a single input to `paste()`, so the call must have exactly one
/// unnamed argument and `collapse`.
fn check_collapse(
    ast: &RCall,
    fn_name: &str,
    arg_list: &RArgumentList,
    args: &[RArgument],
) -> anyhow::Result<Option<Diagnostic>> {
    if args.len() != 2 || is_pipe_rhs(ast)? {
        return Ok(None);
    }

    let collapse = unwrap_or_return_none!(get_arg_by_name(arg_list, "collapse"));
    let collapse = unwrap_or_return_none!(string_literal(&collapse));

    let unnamed = get_unnamed_args(arg_list);
    if unnamed.len() != 1 {
        return Ok(None);
    }
    let input = unwrap_or_return_none!(unnamed[0].value());

    let range = ast.syntax().text_trimmed_range();
    let has_comments = node_contains_comments(ast.syntax());

    if collapse.is_empty() {
        let rep_call = unwrap_or_return_none!(input.as_r_call());
        let Some((x, times)) = rep_with_string_input(rep_call)? else {
            return Ok(None);
        };
        let fix = match times {
            Some(times) => Fix::new(range, format!("strrep({x}, {times})"), has_comments),
            None => Fix::empty(),
        };
        return Ok(Some(Diagnostic::new(
            ViolationData::new(
                Rule::Paste,
                format!("`{fn_name}(rep(x, times), collapse = \"\")` can be simplified."),
                Some("Use `strrep(x, times)` instead.".to_string()),
            ),
            range,
            fix,
        )));
    }

    if collapse == ", " {
        let input_text = input.syntax().text_trimmed().to_string();
        // `toString()` only takes one input, `...` can contain several.
        if input_text == "..." {
            return Ok(None);
        }
        return Ok(Some(Diagnostic::new(
            ViolationData::new(
                Rule::Paste,
                format!("`{fn_name}(x, collapse = \", \")` can be simplified."),
                Some("Use `toString(x)` instead.".to_string()),
            ),
            range,
            Fix::new(range, format!("toString({input_text})"), has_comments),
        )));
    }

    Ok(None)
}

/// Reports `paste(..., sep = "")`.
fn check_empty_sep(
    ast: &RCall,
    fn_name: &str,
    ns_prefix: Option<&str>,
    arg_list: &RArgumentList,
    args: &[RArgument],
) -> anyhow::Result<Option<Diagnostic>> {
    if fn_name != "paste" {
        return Ok(None);
    }

    let sep = unwrap_or_return_none!(get_arg_by_name(arg_list, "sep"));
    let sep_value = unwrap_or_return_none!(string_literal(&sep));
    if !sep_value.is_empty() || is_inside_expression(ast) {
        return Ok(None);
    }

    let other_args = args
        .iter()
        .filter(|arg| arg.syntax() != sep.syntax())
        .map(|arg| arg.syntax().text_trimmed().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let prefix = ns_prefix.unwrap_or("");

    let range = ast.syntax().text_trimmed_range();
    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::Paste,
            "`paste(..., sep = \"\")` can be simplified.".to_string(),
            Some("Use `paste0(...)` instead.".to_string()),
        ),
        range,
        Fix::new(
            range,
            format!("{prefix}paste0({other_args})"),
            node_contains_comments(ast.syntax()),
        ),
    )))
}

/// If `call` is `rep()` whose first argument is a string literal, returns the
/// text of that argument and, when the call is exactly `rep(x, times)`, the
/// text of `times`.
fn rep_with_string_input(call: &RCall) -> anyhow::Result<Option<(String, Option<String>)>> {
    let function = call.function()?;
    if get_function_name(function.clone()) != "rep" {
        return Ok(None);
    }
    if let Some(ns) = function.as_r_namespace_expression()
        && ns.left()?.syntax().text_trimmed() != "base"
    {
        return Ok(None);
    }

    let rep_args: Vec<RArgument> = call
        .arguments()?
        .items()
        .iter()
        .filter_map(|arg| arg.ok())
        .collect();

    let first = unwrap_or_return_none!(rep_args.first());
    if first.name_clause().is_some() || string_literal(first).is_none() {
        return Ok(None);
    }
    let x = unwrap_or_return_none!(first.value())
        .syntax()
        .text_trimmed()
        .to_string();

    let times = match rep_args.as_slice() {
        [_, second] if is_unnamed_or_named(second, "times") => second
            .value()
            .map(|value| value.syntax().text_trimmed().to_string()),
        _ => None,
    };

    Ok(Some((x, times)))
}

fn is_unnamed_or_named(arg: &RArgument, name: &str) -> bool {
    arg.name_clause().is_none() || argument_name(arg).as_deref() == Some(name)
}

/// The content of the argument's value if it is a string literal.
fn string_literal(arg: &RArgument) -> Option<String> {
    let value = arg.value()?;
    let string = value.as_any_r_value()?.as_r_string_value()?.clone();
    string
        .inner_string_text()
        .ok()
        .map(|text| text.text().to_string())
}

/// Whether `ast` is the right-hand side of a pipe, which supplies an extra
/// input to the call.
fn is_pipe_rhs(ast: &RCall) -> anyhow::Result<bool> {
    let Some(binary) = ast.syntax().parent().and_then(RBinaryExpression::cast) else {
        return Ok(false);
    };
    Ok(binary.right()?.syntax() == ast.syntax()
        && matches!(
            binary.operator()?.text_trimmed(),
            "|>" | "%>%" | "%!>%" | "%T>%" | "%<>%"
        ))
}

/// Whether `ast` is inside an `expression()` call, where `paste()` is the
/// plotmath juxtaposition function (see `?plotmath`).
fn is_inside_expression(ast: &RCall) -> bool {
    ast.syntax()
        .ancestors()
        .skip(1)
        .filter_map(RCall::cast)
        .any(|call| {
            call.function()
                .is_ok_and(|function| get_function_name(function) == "expression")
        })
}
