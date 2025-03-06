use crate::location::Location;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::utils::find_row_col;
use air_r_syntax::RSyntaxNode;
use air_r_syntax::*;
use anyhow::{anyhow, Result};
use biome_rowan::AstNode;

pub struct DuplicatedArguments;

impl Violation for DuplicatedArguments {
    fn name(&self) -> String {
        "duplicated_arguments".to_string()
    }
    fn body(&self) -> String {
        "Avoid duplicate arguments in function calls.".to_string()
    }
}

impl LintChecker for DuplicatedArguments {
    fn check(
        &self,
        ast: &RSyntaxNode,
        loc_new_lines: &[usize],
        file: &str,
    ) -> Result<Vec<Diagnostic>> {
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
                ))
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
            let (row, column) = find_row_col(ast, loc_new_lines);
            diagnostics.push(Diagnostic {
                message: DuplicatedArguments.into(),
                filename: file.into(),
                location: Location { row, column },
                fix: Fix::empty(),
            })
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
