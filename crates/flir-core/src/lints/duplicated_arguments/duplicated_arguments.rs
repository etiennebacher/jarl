use std::collections::{HashMap, HashSet};

use crate::diagnostic::*;
use air_r_syntax::*;
use anyhow::anyhow;
use biome_rowan::AstNode;

/// ## What it does
///
/// Checks for duplicated arguments in function calls.
///
/// ## Why is this bad?
///
/// While some cases of duplicated arguments generate run-time errors (e.g.
/// `mean(x = 1:5, x = 2:3)`), this is not always the case (e.g.
/// `c(a = 1, a = 2)`).
///
/// This linter is used to discourage explicitly providing duplicate names to
/// objects. Duplicate-named objects are hard to work with programmatically and
/// should typically be avoided.
///
/// ## Example
///
/// ```r
/// list(x = 1, x = 2)
/// ```
pub fn duplicated_arguments(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let RCallFields { function, arguments } = ast.as_fields();

    let fun_name = match function? {
        AnyRExpression::RNamespaceExpression(x) => {
            x.right()?.into_syntax().text_trimmed().to_string()
        }
        AnyRExpression::RBracedExpressions(x) => x
            .expressions()
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .join(""),
        AnyRExpression::RExtractExpression(x) => {
            x.right()?.into_syntax().text_trimmed().to_string()
        }
        AnyRExpression::RCall(x) => x.function()?.into_syntax().text_trimmed().to_string(),
        AnyRExpression::RSubset(x) => x.arguments()?.into_syntax().text_trimmed().to_string(),
        AnyRExpression::RSubset2(x) => x.arguments()?.into_syntax().text_trimmed().to_string(),
        AnyRExpression::RIdentifier(x) => x.into_syntax().text_trimmed().to_string(),
        AnyRExpression::AnyRValue(x) => x.into_syntax().text_trimmed().to_string(),
        AnyRExpression::RParenthesizedExpression(x) => {
            x.body()?.into_syntax().text_trimmed().to_string()
        }
        AnyRExpression::RReturnExpression(x) => x.into_syntax().text_trimmed().to_string(),
        _ => {
            return Err(anyhow!(
                "couldn't find function name for duplicated_arguments linter.",
            ));
        }
    };

    let whitelisted_funs = ["c", "mutate", "summarize", "transmute"];
    if whitelisted_funs.contains(&fun_name.as_str()) {
        return Ok(None);
    }

    let arg_names: Vec<String> = arguments?
        .items()
        .into_iter()
        .filter_map(Result::ok) // skip any Err values
        .filter_map(|item| {
            let fields = item.as_fields();
            if let Some(name_clause) = &fields.name_clause
                && let Ok(name) = name_clause.name()
            {
                Some(
                    name.into_syntax()
                        .text_trimmed()
                        .to_string()
                        .replace(&['\'', '"', '`'][..], ""),
                )
            } else {
                None
            }
        })
        .collect();

    if arg_names.is_empty() {
        return Ok(None);
    }

    let duplicated_arg_names = get_duplicates(&arg_names);

    if !duplicated_arg_names.is_empty() {
        let range = ast.syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(
            ViolationData::new(
                "duplicated_arguments".to_string(),
                [
                    "Avoid duplicate arguments in function calls. Duplicated argument(s): ",
                    &duplicated_arg_names
                        .iter()
                        .map(|s| format!("\"{s}\""))
                        .collect::<Vec<String>>()
                        .join(", "),
                    ".",
                ]
                .join("")
                .to_string(),
            ),
            range,
            Fix::empty(),
        );
        return Ok(Some(diagnostic));
    }

    Ok(None)
}

fn get_duplicates(values: &[String]) -> Vec<String> {
    let mut counts = HashMap::new();
    for item in values {
        *counts.entry(item).or_insert(0) += 1;
    }
    let duplicates: HashSet<String> = counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(item, _)| item.clone())
        .collect();

    let duplicates_vec: Vec<String> = duplicates.into_iter().collect();
    duplicates_vec
}
