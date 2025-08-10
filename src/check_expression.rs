use air_r_parser::RParserOptions;
use air_r_syntax::RForStatementFields;
use air_r_syntax::{
    AnyRExpression, RBinaryExpressionFields, RIfStatementFields, RWhileStatementFields,
};

use crate::analyze;
use crate::config::Config;
use crate::message::*;
use crate::utils::*;
use anyhow::Result;
use std::path::Path;

#[derive(Debug)]
// The object that will collect diagnostics in check_expressions().
pub struct Checker<'a> {
    diagnostics: Vec<Diagnostic>,
    rules: Vec<&'a str>,
}

impl<'a> Checker<'a> {
    fn new() -> Self {
        Self { diagnostics: vec![], rules: vec![] }
    }

    pub(crate) fn report_diagnostic(&mut self, diagnostic: Option<Diagnostic>) {
        if let Some(diagnostic) = diagnostic {
            self.diagnostics.push(diagnostic);
        }
    }

    pub(crate) fn is_rule_enabled(&mut self, rule: &str) -> bool {
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
        check_expression(&expr, &mut checker)?;
    }

    let diagnostics: Vec<Diagnostic> = checker
        .diagnostics
        .into_iter()
        .map(|mut x| {
            x.filename = file.to_path_buf();
            x
        })
        .collect();

    let loc_new_lines = find_new_lines(syntax)?;
    let diagnostics = compute_lints_location(diagnostics, &loc_new_lines);

    Ok(diagnostics)
}

// This function does two things:
// - dispatch an expression to its appropriate set of rules, e.g. binary
//   expressions are sent to the rules stored in
//   analyze::binary_expression::binary_expression.
// - apply the function recursively to the expression's children (if any, which
//   is not guaranteed, e.g. for RIdentifier).
//
// Some expression types do both (e.g. RBinaryExpression), some do only the
// dispatch to rules (e.g. RIdentifier), some do only the recursive call (e.g.
// RFunctionDefinition).
pub fn check_expression(
    expression: &air_r_syntax::AnyRExpression,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    match expression {
        air_r_syntax::AnyRExpression::RBinaryExpression(children) => {
            analyze::binary_expression::binary_expression(children, checker)?;
            let RBinaryExpressionFields { left, right, .. } = children.as_fields();
            check_expression(&left?, checker)?;
            check_expression(&right?, checker)?;
        }
        air_r_syntax::AnyRExpression::RBracedExpressions(children) => {
            let expressions: Vec<_> = children.expressions().into_iter().collect();

            for expr in expressions {
                check_expression(&expr, checker)?;
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
                check_expression(&expr, checker)?;
            }
        }
        air_r_syntax::AnyRExpression::RForStatement(children) => {
            let RForStatementFields { body, variable, .. } = children.as_fields();
            analyze::identifier::identifier(&variable?, checker)?;

            check_expression(&body?, checker)?;
        }
        air_r_syntax::AnyRExpression::RFunctionDefinition(children) => {
            let body = children.body();
            check_expression(&body?, checker)?;
        }
        air_r_syntax::AnyRExpression::RIdentifier(x) => {
            analyze::identifier::identifier(x, checker)?;
        }
        air_r_syntax::AnyRExpression::RIfStatement(children) => {
            let RIfStatementFields { condition, consequence, .. } = children.as_fields();
            check_expression(&condition?, checker)?;
            check_expression(&consequence?, checker)?;
        }
        air_r_syntax::AnyRExpression::RParenthesizedExpression(children) => {
            let body = children.body();
            check_expression(&body?, checker)?;
        }
        air_r_syntax::AnyRExpression::RRepeatStatement(children) => {
            let body = children.body();
            check_expression(&body?, checker)?;
        }
        air_r_syntax::AnyRExpression::RSubset(children) => {
            let arguments: Vec<_> = children.arguments()?.items().into_iter().collect();

            for expr in arguments {
                if let Some(expr) = expr?.value() {
                    check_expression(&expr, checker)?;
                }
            }
        }
        air_r_syntax::AnyRExpression::RUnaryExpression(children) => {
            let argument = children.argument();
            check_expression(&argument?, checker)?;
        }
        air_r_syntax::AnyRExpression::RWhileStatement(children) => {
            let RWhileStatementFields { condition, body, .. } = children.as_fields();
            check_expression(&condition?, checker)?;
            check_expression(&body?, checker)?;
        }
        // Not all patterns are covered but they don't necessarily have to be.
        // For instance, there are currently no rule for RNaExpression and
        // it doesn't have any children expression on which we need to call
        // check_expression().
        //
        // If a rule needs to be applied on RNaExpression in the future, then
        // we can add the corresponding match arm at this moment.
        _ => {}
    }

    Ok(())
}
