use air_r_parser::RParserOptions;
use air_r_syntax::{
    AnyRExpression, RArgumentList, RBinaryExpressionFields, RBracedExpressionsFields,
    RCallArgumentsFields, RCallFields, RExpressionList, RFunctionDefinitionFields,
    RIfStatementFields, RParenthesizedExpressionFields, RSubset, RSubsetFields, RSyntaxKind,
    RSyntaxNode, RWhileStatementFields,
};

// use crate::analyze::rcall;
use crate::config::Config;
use crate::lints::any_duplicated::any_duplicated::AnyDuplicated;
use crate::lints::any_is_na::any_is_na::AnyIsNa;
use crate::lints::class_equals::class_equals::ClassEquals;
use crate::lints::duplicated_arguments::duplicated_arguments::DuplicatedArguments;
use crate::lints::empty_assignment::empty_assignment::EmptyAssignment;
use crate::lints::equal_assignment::equal_assignment::EqualAssignment;
use crate::lints::equals_na::equals_na::EqualsNa;
// use crate::lints::expect_length::expect_length::ExpectLength;
use crate::lints::length_levels::length_levels::LengthLevels;
use crate::lints::length_test::length_test::LengthTest;
use crate::lints::lengths::lengths::Lengths;
use crate::lints::redundant_equals::redundant_equals::RedundantEquals;
use crate::lints::true_false_symbol::true_false_symbol::TrueFalseSymbol;
use crate::lints::which_grepl::which_grepl::WhichGrepl;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
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
    let diagnostics: Vec<Diagnostic> = expressions_vec
        .iter()
        .map(|expression| {
            check_ast(expression, file.to_str().unwrap(), &config, &mut checker).unwrap()
        })
        .flatten()
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
) -> anyhow::Result<Vec<Diagnostic>> {
    let mut diagnostics: Vec<Diagnostic> = vec![];

    match expression {
        air_r_syntax::AnyRExpression::RIdentifier(x) => {
            diagnostics.push(TrueFalseSymbol.check(&x.clone().into())?);
        }

        air_r_syntax::AnyRExpression::RCall(children) => {
            // rcall::rcall(children, checker);

            // println!("Checker: {:?}", checker);

            let any_r_exp: &AnyRExpression = &children.clone().into();
            diagnostics.push(AnyDuplicated.check(any_r_exp)?);
            diagnostics.push(AnyIsNa.check(any_r_exp)?);
            diagnostics.push(DuplicatedArguments.check(any_r_exp)?);
            diagnostics.push(LengthLevels.check(any_r_exp)?);
            diagnostics.push(LengthTest.check(any_r_exp)?);
            diagnostics.push(Lengths.check(any_r_exp)?);
            diagnostics.push(WhichGrepl.check(any_r_exp)?);

            let RCallFields { arguments, .. } = children.as_fields();
            let RCallArgumentsFields { items, .. } = arguments?.as_fields();
            let arg_exprs: Vec<AnyRExpression> = items
                .into_iter()
                .map(|x| x.unwrap().as_fields().value.unwrap())
                .collect();

            for expr in arg_exprs {
                diagnostics.extend(check_ast(&expr, file, config, checker)?);
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
            let any_r_exp: &AnyRExpression = &children.clone().into();
            diagnostics.push(ClassEquals.check(any_r_exp)?);
            diagnostics.push(EmptyAssignment.check(any_r_exp)?);
            diagnostics.push(EqualAssignment.check(any_r_exp)?);
            diagnostics.push(EqualsNa.check(any_r_exp)?);
            diagnostics.push(RedundantEquals.check(any_r_exp)?);
            let RBinaryExpressionFields { left, right, .. } = children.as_fields();
            diagnostics.extend(check_ast(&left?, file, config, checker)?);
            diagnostics.extend(check_ast(&right?, file, config, checker)?);
        }
        air_r_syntax::AnyRExpression::RParenthesizedExpression(children) => {
            let RParenthesizedExpressionFields { body, .. } = children.as_fields();
            diagnostics.extend(check_ast(&body?, file, config, checker)?);
        }
        air_r_syntax::AnyRExpression::RBracedExpressions(children) => {
            let RBracedExpressionsFields { expressions, .. } = children.as_fields();
            let expressions_vec: Vec<_> = expressions.into_iter().collect();

            for expr in expressions_vec {
                diagnostics.extend(check_ast(&expr, file, config, checker)?);
            }
        }
        air_r_syntax::AnyRExpression::RFunctionDefinition(children) => {
            let RFunctionDefinitionFields { body, .. } = children.as_fields();
            diagnostics.extend(check_ast(&body?, file, config, checker)?);
        }
        // | air_r_syntax::AnyRExpression::RExtractExpression(children)
        // | air_r_syntax::AnyRExpression::RNamespaceExpression(children)
        // | air_r_syntax::AnyRExpression::RNaExpression(children)
        // | air_r_syntax::AnyRExpression::RForStatement(children)
        air_r_syntax::AnyRExpression::RWhileStatement(children) => {
            let RWhileStatementFields { condition, .. } = children.as_fields();
            diagnostics.extend(check_ast(&condition?, file, config, checker)?);
        }
        air_r_syntax::AnyRExpression::RIfStatement(children) => {
            let RIfStatementFields { condition, consequence, .. } = children.as_fields();
            diagnostics.extend(check_ast(&condition?, file, config, checker)?);
            diagnostics.extend(check_ast(&consequence?, file, config, checker)?);
        }
        air_r_syntax::AnyRExpression::RSubset(children) => {
            let RSubsetFields { arguments, .. } = children.as_fields();
            let arguments = arguments?.items();
            let expressions_vec: Vec<_> = arguments.into_iter().collect();

            for expr in expressions_vec {
                if let Some(expr) = expr?.value() {
                    diagnostics.extend(check_ast(&expr, file, config, checker)?);
                }
            }
        }
        _ => {
            // println!("Not implemented");
        }
    }

    Ok(diagnostics)
}
