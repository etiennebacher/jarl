use air_r_parser::RParserOptions;
use air_r_syntax::RForStatementFields;
use air_r_syntax::{
    AnyRExpression, RBinaryExpressionFields, RBracedExpressionsFields, RCallArgumentsFields,
    RCallFields, RFunctionDefinitionFields, RIfStatementFields, RParenthesizedExpressionFields,
    RSubsetFields, RWhileStatementFields,
};

use crate::analyze;
use crate::config::Config;
use crate::message::*;
use crate::utils::*;
use anyhow::Result;
use std::path::Path;

#[derive(Debug)]
pub struct Checker<'a> {
    diagnostics: Vec<Diagnostic>,
    rules: Vec<&'a str>,
}

impl<'a> Checker<'a> {
    fn new() -> Self {
        Self { diagnostics: vec![], rules: vec![] }
    }

    pub(crate) fn report_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub(crate) fn is_enabled(&mut self, rule: &str) -> bool {
        self.rules.contains(&rule)
    }
}

pub fn get_checks(contents: &str, file: &Path, config: Config) -> Result<Vec<Diagnostic>> {
    let parser_options = RParserOptions::default();
    let parsed = air_r_parser::parse(contents, parser_options);

    let syntax = &parsed.syntax();
    let expressions = &parsed.tree().expressions();
    let expressions_vec: Vec<_> = expressions.into_iter().collect();

    let mut checker = Checker::new();
    checker.rules = config.rules_to_apply;
    for expr in expressions_vec {
        check_ast(&expr, &mut checker)?;
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

pub fn check_ast(
    expression: &air_r_syntax::AnyRExpression,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    match expression {
        air_r_syntax::AnyRExpression::RBinaryExpression(children) => {
            analyze::binary_expression::binary_expression(children, checker)?;
            let RBinaryExpressionFields { left, right, .. } = children.as_fields();
            check_ast(&left?, checker)?;
            check_ast(&right?, checker)?;
        }
        air_r_syntax::AnyRExpression::RBracedExpressions(children) => {
            let expressions: Vec<_> = children.expressions().into_iter().collect();

            for expr in expressions {
                check_ast(&expr, checker)?;
            }
        }
        air_r_syntax::AnyRExpression::RCall(children) => {
            analyze::call::call(children, checker)?;

            let arguments: Vec<AnyRExpression> = children
                .arguments()?
                .items()
                .into_iter()
                .filter_map(|x| x.unwrap().as_fields().value)
                .collect();

            for expr in arguments {
                check_ast(&expr, checker)?;
            }
        }
        air_r_syntax::AnyRExpression::RForStatement(children) => {
            let RForStatementFields { body, variable, .. } = children.as_fields();
            analyze::identifier::identifier(&variable?, checker)?;

            check_ast(&body?, checker)?;
        }
        air_r_syntax::AnyRExpression::RFunctionDefinition(children) => {
            let body = children.body();
            check_ast(&body?, checker)?;
        }
        air_r_syntax::AnyRExpression::RIdentifier(x) => {
            analyze::identifier::identifier(x, checker)?;
        }
        air_r_syntax::AnyRExpression::RIfStatement(children) => {
            let RIfStatementFields { condition, consequence, .. } = children.as_fields();
            check_ast(&condition?, checker)?;
            check_ast(&consequence?, checker)?;
        }
        air_r_syntax::AnyRExpression::RParenthesizedExpression(children) => {
            let body = children.body();
            check_ast(&body?, checker)?;
        }
        air_r_syntax::AnyRExpression::RSubset(children) => {
            let arguments: Vec<_> = children.arguments()?.items().into_iter().collect();

            for expr in arguments {
                if let Some(expr) = expr?.value() {
                    check_ast(&expr, checker)?;
                }
            }
        }
        air_r_syntax::AnyRExpression::RWhileStatement(children) => {
            let RWhileStatementFields { condition, body, .. } = children.as_fields();
            check_ast(&condition?, checker)?;
            check_ast(&body?, checker)?;
        }
        // | air_r_syntax::AnyRExpression::RSubset(children)
        // | air_r_syntax::AnyRExpression::RSubset2(children)
        // | air_r_syntax::RParameterList
        // | air_r_syntax::RParameters
        // | air_r_syntax::RParameter
        // | air_r_syntax::RArgument
        // | air_r_syntax::AnyRExpression::RBracedExpressions(children)
        // | air_r_syntax::RRoot
        // | air_r_syntax::AnyRExpression::RRepeatStatement(children)
        // | air_r_syntax::AnyRExpression::RUnaryExpression(children)
        // | air_r_syntax::AnyRExpression::RExtractExpression(children)
        // | air_r_syntax::AnyRExpression::RNamespaceExpression(children)
        // | air_r_syntax::AnyRExpression::RNaExpression(children)
        _ => {
            // println!("Not implemented");
        }
    }

    Ok(())
}
