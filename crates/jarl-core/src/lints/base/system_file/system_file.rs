use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::node_contains_comments;
use air_r_syntax::*;
use biome_rowan::AstNode;
pub struct SystemFile;

/// <!-- docs: start -->
/// Version added: 0.1.2
///
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
/// <!-- docs: end -->
impl Violation for SystemFile {
    fn rule(&self) -> Rule {
        Rule::SystemFile
    }
    fn body(&self) -> String {
        "`system.file(file.path(...))` is redundant.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `system.file(...)` directly.".to_string())
    }
}

pub fn system_file(ast: &RCall, fn_name: &str) -> anyhow::Result<Option<Diagnostic>> {
    if fn_name != "system.file" {
        return Ok(None);
    }

    let function = ast.function()?;
    if let Some(namespace) = function.as_r_namespace_expression()
        && namespace.left()?.to_trimmed_text() != "base"
    {
        return Ok(None);
    }

    let args = ast.arguments()?.items();
    let mut replacement_args = Vec::new();
    let mut has_file_path = false;
    for arg in args {
        let Ok(arg) = arg else {
            return Ok(None);
        };

        // A missing value can be produced by the parser's recovery tree. It
        // must not be silently dropped by an automatic fix.
        let Some(value) = arg.value() else {
            return Ok(None);
        };

        if arg.name_clause().is_none()
            && let Some(file_path_call) = value.as_r_call()
            && matches!(
                file_path_call.function()?.to_trimmed_text().text(),
                "file.path" | "base::file.path"
            )
        {
            let mut inner_args = Vec::new();
            let mut has_named_arg = false;
            for inner_arg in file_path_call.arguments()?.items() {
                let Ok(inner_arg) = inner_arg else {
                    return Ok(None);
                };

                // Named arguments such as `fsep = "-"` belong to `file.path()`
                // and cannot be moved to `system.file()`.
                if inner_arg.name_clause().is_some() {
                    has_named_arg = true;
                    break;
                }

                if inner_arg.value().is_none() {
                    return Ok(None);
                }
                inner_args.push(inner_arg.to_trimmed_string());
            }

            if !has_named_arg && !inner_args.is_empty() {
                has_file_path = true;
                replacement_args.extend(inner_args);
                continue;
            }
        }

        replacement_args.push(arg.to_trimmed_string());
    }

    if !has_file_path {
        return Ok(None);
    }

    let function_text = function.to_trimmed_string();
    let replacement = format!("{}({})", function_text, replacement_args.join(", "));

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        SystemFile,
        range,
        Fix::new(range, replacement, node_contains_comments(ast.syntax())),
    );
    Ok(Some(diagnostic))
}
