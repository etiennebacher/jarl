use air_r_parser::RParserOptions;
use air_r_syntax::{
    AnyRExpression, RArgumentList, RBinaryExpressionFields, RBracedExpressionsFields,
    RCallArgumentsFields, RCallFields, RExpressionList, RFunctionDefinitionFields,
    RIfStatementFields, RParenthesizedExpressionFields, RSubset, RSubsetFields, RSyntaxKind,
    RSyntaxNode, RWhileStatementFields,
};

use crate::analyze;
use crate::config::Config;
use crate::message::*;
use crate::utils::*;
use anyhow::Result;
use std::path::Path;

pub fn get_checks(contents: &str, file: &Path, config: Config) -> Result<Vec<Diagnostic>> {
    let parser_options = RParserOptions::default();
    let parsed = air_r_parser::parse(contents, parser_options);

    let syntax = &parsed.syntax();
    let expressions = &parsed.tree().expressions();
    let expressions_vec: Vec<_> = expressions.into_iter().collect();

    let mut checker = Checker::new();
    for expr in expressions_vec {
        check_ast(&expr, file.to_str().unwrap(), &config, &mut checker)?;
    }

    let diagnostics: Vec<Diagnostic> = checker
        .diagnostics
        .into_iter()
        .filter(|x| !x.message.name.is_empty())
        .map(|mut x| {
            x.filename = file.to_path_buf();
            x
        })
        .collect();

    let loc_new_lines = find_new_lines(syntax)?;
    let diagnostics = compute_lints_location(diagnostics, &loc_new_lines);

    Ok(diagnostics)
}

#[derive(Debug)]
pub struct Checker {
    diagnostics: Vec<Diagnostic>,
}

impl Checker {
    fn new() -> Self {
        Self { diagnostics: vec![] }
    }

    pub(crate) fn report_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }
}

pub fn check_ast(
    expression: &air_r_syntax::AnyRExpression,
    file: &str,
    config: &Config,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    match expression {
        air_r_syntax::AnyRExpression::RIdentifier(x) => {
            let _ = analyze::identifier::identifier(x, checker);
        }

        air_r_syntax::AnyRExpression::RCall(children) => {
            let _ = analyze::call::call(children, checker);

            let RCallFields { arguments, .. } = children.as_fields();
            let RCallArgumentsFields { items, .. } = arguments?.as_fields();
            let arg_exprs: Vec<AnyRExpression> = items
                .into_iter()
                .map(|x| x.unwrap().as_fields().value)
                .filter_map(|x| x)
                .collect();

            for expr in arg_exprs {
                check_ast(&expr, file, config, checker)?;
            }
        }
        // | air_r_syntax::AnyRExpression::RSubset(children)
        // | air_r_syntax::AnyRExpression::RSubset2(children)
        // // | air_r_syntax::RParameterList
        // // | air_r_syntax::RParameters
        // // | air_r_syntax::RParameter
        // // | air_r_syntax::RArgument
        // | air_r_syntax::AnyRExpression::RBracedExpressions(children)
        // | air_r_syntax::RRoot
        // | air_r_syntax::AnyRExpression::RRepeatStatement(children)
        // | air_r_syntax::AnyRExpression::RUnaryExpression(children)
        air_r_syntax::AnyRExpression::RBinaryExpression(children) => {
            let _ = analyze::binary_expression::binary_expression(children, checker);
            let RBinaryExpressionFields { left, right, .. } = children.as_fields();
            check_ast(&left?, file, config, checker)?;
            check_ast(&right?, file, config, checker)?;
        }
        air_r_syntax::AnyRExpression::RParenthesizedExpression(children) => {
            let RParenthesizedExpressionFields { body, .. } = children.as_fields();
            check_ast(&body?, file, config, checker)?;
        }
        air_r_syntax::AnyRExpression::RBracedExpressions(children) => {
            let RBracedExpressionsFields { expressions, .. } = children.as_fields();
            let expressions_vec: Vec<_> = expressions.into_iter().collect();

            for expr in expressions_vec {
                check_ast(&expr, file, config, checker)?;
            }
        }
        air_r_syntax::AnyRExpression::RFunctionDefinition(children) => {
            let RFunctionDefinitionFields { body, .. } = children.as_fields();
            check_ast(&body?, file, config, checker)?;
        }
        // | air_r_syntax::AnyRExpression::RExtractExpression(children)
        // | air_r_syntax::AnyRExpression::RNamespaceExpression(children)
        // | air_r_syntax::AnyRExpression::RNaExpression(children)
        // | air_r_syntax::AnyRExpression::RForStatement(children)
        air_r_syntax::AnyRExpression::RWhileStatement(children) => {
            let RWhileStatementFields { condition, .. } = children.as_fields();
            check_ast(&condition?, file, config, checker)?;
        }
        air_r_syntax::AnyRExpression::RIfStatement(children) => {
            let RIfStatementFields { condition, consequence, .. } = children.as_fields();
            check_ast(&condition?, file, config, checker)?;
            check_ast(&consequence?, file, config, checker)?;
        }
        air_r_syntax::AnyRExpression::RSubset(children) => {
            let RSubsetFields { arguments, .. } = children.as_fields();
            let arguments = arguments?.items();
            let expressions_vec: Vec<_> = arguments.into_iter().collect();

            for expr in expressions_vec {
                if let Some(expr) = expr?.value() {
                    check_ast(&expr, file, config, checker)?;
                }
            }
        }
        _ => {
            // println!("Not implemented");
        }
    }

    Ok(())
}
