use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use air_r_syntax::RSyntaxNode;
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

impl LintChecker for DuplicatedArguments {
    fn check(&self, ast: &RSyntaxNode, file: &str) -> Result<Vec<Diagnostic>> {
        let mut diagnostics: Vec<Diagnostic> = vec![];
        if ast.kind() != RSyntaxKind::R_CALL {
            return Ok(diagnostics);
        }

        let call = RCall::cast(ast.clone());
        let function = call.unwrap().function();

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
                    "in {}, couldn't find function name for duplicated_arguments linter.",
                    file
                ));
            }
        };

        let whitelisted_funs = ["mutate", "summarize", "transmute"];
        if whitelisted_funs.contains(&fun_name.as_str()) {
            return Ok(diagnostics);
        }

        let named_args = ast
            .descendants()
            .find(|x| x.kind() == RSyntaxKind::R_ARGUMENT_LIST)
            .ok_or(anyhow!("Couldn't find argument list"))?
            .children()
            .filter(|x| {
                x.kind() == RSyntaxKind::R_ARGUMENT
                    && x.first_child()
                        .map(|child| child.kind() == RSyntaxKind::R_ARGUMENT_NAME_CLAUSE)
                        .unwrap_or(false)
            })
            .collect::<Vec<_>>();

        if named_args.is_empty() {
            return Ok(diagnostics);
        }

        let arg_names = named_args
            .iter()
            .map(|arg| {
                arg.first_child()
                    .map(|child| {
                        child
                            .first_child()
                            .map(|child2| {
                                child2
                                    .text_trimmed()
                                    .to_string()
                                    .replace(&['\'', '"', '`'][..], "")
                            })
                            .unwrap_or("".to_string())
                    })
                    .unwrap_or("".to_string())
            })
            .collect::<Vec<String>>();

        if has_duplicates(&arg_names) {
            let range = ast.text_trimmed_range();
            diagnostics.push(Diagnostic::new(
                DuplicatedArguments,
                file,
                range,
                Fix::empty(),
            ))
        }
        Ok(diagnostics)
    }
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
