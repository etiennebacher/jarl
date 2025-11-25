use crate::diagnostic::*;
use crate::utils::{get_named_args, get_unnamed_args, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};
pub struct SystemFile;

/// ## What it does
///
/// Checks for usage of `system.file(file.path(...))` and replaces it with
/// `system.file(...)`.
///
/// ## Why is this bad?
///
/// In `system.file()`, all unnamed arguments are already passed to `file.path()`
/// under the hood, so `system.file(file.path(...))` is redundant and harder to
/// read.
///
/// ## Example
///
/// ```r
/// system.file(file.path("my_dir", "my_sub_dir"), package = "foo")
/// ```
///
/// Use instead:
/// ```r
/// system.file("my_dir", "my_sub_dir", package = "foo")
/// ```
///
/// ## References
///
/// See `?system.file`
impl Violation for SystemFile {
    fn name(&self) -> String {
        "system_file".to_string()
    }
    fn body(&self) -> String {
        "`system.file(file.path(...))` is redundant.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `system.file(...)` instead.".to_string())
    }
}

pub fn system_file(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let arguments = ast.arguments()?;

    if function.to_trimmed_text() != "system.file" {
        return Ok(None);
    }

    let args = arguments.items();
    let values = get_unnamed_args(&args);

    let file_path: Vec<&RArgument> = values
        .iter()
        .filter(|x| {
            if let Some(val) = x.value()
                && let Some(val) = RCall::cast(val.into())
            {
                let fun = val.function();
                if let Ok(fun) = fun {
                    fun.to_trimmed_text() == "file.path"
                } else {
                    false
                }
            } else {
                false
            }
        })
        .collect::<Vec<&RArgument>>();

    if file_path.len() != 1 {
        return Ok(None);
    }

    // Safety: at this point we know file_path has length 1.
    let Some(file_path_value) = file_path.first().unwrap().value() else {
        return Ok(None);
    };

    let Some(file_path_call) = file_path_value.as_r_call() else {
        return Ok(None);
    };

    let file_path_inner_content = file_path_call.arguments()?.items().iter().filter(|x| {
        if let Ok(x) = x {
            x.value().is_some()
        } else {
            false
        }
    });

    let Some(_) = file_path_inner_content.clone().next() else {
        return Ok(None);
    };

    let file_path_inner_content = file_path_inner_content
        .map(|x| x.unwrap().to_trimmed_string())
        .collect::<Vec<String>>()
        .join(", ");

    let other_args = get_named_args(&args)
        .iter()
        .map(|x| x.to_trimmed_string())
        .collect::<Vec<String>>()
        .join(", ");

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        SystemFile,
        range,
        Fix {
            content: format!("system.file({}, {})", file_path_inner_content, other_args),
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );
    Ok(Some(diagnostic))
}
