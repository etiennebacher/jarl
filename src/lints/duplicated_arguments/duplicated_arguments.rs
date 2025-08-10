use crate::message::*;
use air_r_syntax::*;
use anyhow::{Result, anyhow};
use biome_rowan::AstNode;

pub struct DuplicatedArguments;

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
impl Violation for DuplicatedArguments {
    fn name(&self) -> String {
        "duplicated_arguments".to_string()
    }
    fn body(&self) -> String {
        "Avoid duplicate arguments in function calls.".to_string()
    }
}

pub fn duplicated_arguments(ast: &RCall) -> Result<Option<Diagnostic>> {
    let RCallFields { function, arguments } = ast.as_fields();

    let fun_name = match function? {
        AnyRExpression::RNamespaceExpression(x) => x.right()?.text(),
        AnyRExpression::RExtractExpression(x) => x.right()?.text(),
        AnyRExpression::RCall(x) => x.function()?.text(),
        AnyRExpression::RSubset(x) => x.arguments()?.text(),
        AnyRExpression::RSubset2(x) => x.arguments()?.text(),
        AnyRExpression::RIdentifier(x) => x.text(),
        AnyRExpression::AnyRValue(x) => x.text(),
        AnyRExpression::RParenthesizedExpression(x) => x.body()?.text(),
        AnyRExpression::RReturnExpression(x) => x.text(),
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
                Some(name.text().to_string().replace(&['\'', '"', '`'][..], ""))
            } else {
                None
            }
        })
        .collect();

    if arg_names.is_empty() {
        return Ok(None);
    }

    if has_duplicates(&arg_names) {
        let range = ast.clone().into_syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(DuplicatedArguments, range, Fix::empty());
        return Ok(Some(diagnostic));
    }

    Ok(None)
}

fn has_duplicates(v: &[String]) -> bool {
    use std::collections::HashSet;
    let mut seen = HashSet::new();

    for item in v {
        if !seen.insert(item) {
            return true;
        }
    }

    false
}
