use crate::diagnostic::*;
use crate::utils::{get_arg_by_name_then_position, get_arg_by_position, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct List2Df;

/// ## What it does
///
/// Checks for usage of `do.call(cbind.data.frame, x)`.
///
/// ## Why is this bad?
///
/// The goal of `do.call(cbind.data.frame, x)` is to concatenate multiple lists
/// elements of the same length into a `data.frame`. Since R 4.0.0, it is
/// possible to do this with `list2DF(x)`, which is more efficient and easier
/// to read than `do.call(cbind.data.frame, x)`.
///
/// This rule comes with a safe fix but is only enabled if the project
/// explicitly uses R >= 4.0.0 (or if the argument `--min-r-version` is passed
/// with a version >= 4.0.0).
///
/// ## Example
///
/// ```r
/// x <- list(a = 1:10, b = 11:20)
/// do.call(cbind.data.frame, x)
/// ```
///
/// Use instead:
/// ```r
/// x <- list(a = 1:10, b = 11:20)
/// list2DF(x)
/// ```
///
/// ## References
///
/// See `?list2DF`
impl Violation for List2Df {
    fn name(&self) -> String {
        "list2df".to_string()
    }
    fn body(&self) -> String {
        "`do.call(cbind.data.frame, x)` is inefficient and can be hard to read.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `list2DF(x)` instead.".to_string())
    }
}

pub fn list2df(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let RCallFields { function, arguments } = ast.as_fields();

    let function = function?;
    let arguments = arguments?.items();

    if function.to_trimmed_text() != "do.call" {
        return Ok(None);
    }

    let what = get_arg_by_name_then_position(&arguments, "what", 1);
    let args = get_arg_by_name_then_position(&arguments, "args", 2);

    // Ensure there's not more than two arguments, don't know how to handle
    // `quote` and `envir` in `do.call()`.
    if get_arg_by_position(&arguments, 3).is_some() {
        return Ok(None);
    }

    let Some(args) = args else {
        return Ok(None);
    };

    let Some(what) = what else {
        return Ok(None);
    };

    let Some(what_value) = what.value() else {
        return Ok(None);
    };

    let txt = what_value.to_trimmed_text();
    // `do.call()` accepts quoted function names.
    if txt != "cbind.data.frame" && txt != "\"cbind.data.frame\"" && txt != "\'cbind.data.frame\'" {
        return Ok(None);
    }

    let Some(args_value) = args.value() else {
        return Ok(None);
    };
    let fix_content = args_value;

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        List2Df,
        range,
        Fix {
            content: format!("list2DF({})", fix_content.to_trimmed_text()),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}
