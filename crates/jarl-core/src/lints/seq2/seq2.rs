use crate::{diagnostic::*, utils::node_contains_comments};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

/// ## What it does
///
/// Checks for `seq(length(...))`, `seq(nrow(...))`, `seq(ncol(...))`,
/// `seq(NROW(...))`, `seq(NCOL(...))`. See also [seq](https://jarl.etiennebacher.com/rules/seq).
///
/// ## Why is this bad?
///
/// Those patterns are often used to generate sequences from 1 to a given
/// number. However, when `length(...)` is 0, then this creates a sequence `1,0`
/// which is often overlooked.
///
/// This rule comes with safe automatic fixes using `seq_along()` or `seq_len()`.
///
/// ## Example
///
/// ```r
/// for (i in seq(nrow(data))) {
///   print("hi")
/// }
///
/// for (i in seq(length(data))) {
///   print("hi")
/// }
/// ```
///
/// Use instead:
/// ```r
/// for (i in seq_len(nrow(data))) {
///   print("hi")
/// }
///
/// for (i in seq_along(data)) {
///   print("hi")
/// }
/// ```
pub fn seq2(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let outer_fn_name = function.to_trimmed_string();

    if outer_fn_name != "seq" {
        return Ok(None);
    }

    let items = ast.arguments()?.items();

    // Don't want to report cases like seq(length(x), 2) or seq().
    if items.iter().collect::<Vec<_>>().len() != 1 {
        return Ok(None);
    }

    let Some(unnamed_arg) = items
        .into_iter()
        .find(|x| x.clone().unwrap().name_clause().is_none())
    else {
        return Ok(None);
    };

    let value = unnamed_arg?.value();

    if let Some(inner) = value
        && let Some(inner_call) = inner.as_r_call()
    {
        let RCallFields { function, arguments } = inner_call.as_fields();

        let function = function?;
        let inner_fn_name = function.to_trimmed_string();

        if !["length", "nrow", "ncol", "NROW", "NCOL"].contains(&inner_fn_name.as_str()) {
            return Ok(None);
        }

        let inner_fun_content = arguments?.items().into_syntax().to_string();

        let (suggestion, replacement) = match inner_fn_name.as_str() {
            "length" => ("seq_along(...)", format!("seq_along({inner_fun_content})")),
            "nrow" => (
                "seq_len(nrow(...))",
                format!("seq_len(nrow({inner_fun_content}))"),
            ),
            "ncol" => (
                "seq_len(ncol(...))",
                format!("seq_len(ncol({inner_fun_content}))"),
            ),
            "NROW" => (
                "seq_len(NROW(...))",
                format!("seq_len(NROW({inner_fun_content}))"),
            ),
            "NCOL" => (
                "seq_len(NCOL(...))",
                format!("seq_len(NCOL({inner_fun_content}))"),
            ),
            // We checked the choices of inner_fn_name above.
            _ => unreachable!(),
        };

        let range = ast.syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(
            ViolationData::new(
                "seq2".to_string(),
                format!("`seq({inner_fn_name}(...))` can be wrong if the argument has length 0.")
                    .to_string(),
                Some(format!("Use `{suggestion}` instead.").to_string()),
            ),
            range,
            Fix {
                content: replacement,
                start: range.start().into(),
                end: range.end().into(),
                to_skip: node_contains_comments(ast.syntax()),
            },
        );

        Ok(Some(diagnostic))
    } else {
        Ok(None)
    }
}
