use air_r_parser::RParserOptions;
use air_r_syntax::{
    AnyRExpression, RArgumentList, RBinaryExpressionFields, RCallArgumentsFields, RCallFields,
    RExpressionList, RIfStatementFields, RParenthesizedExpressionFields, RSyntaxKind, RSyntaxNode,
    RWhileStatementFields,
};

use crate::config::Config;
use crate::lints::any_duplicated::any_duplicated::AnyDuplicated;
use crate::lints::any_is_na::any_is_na::AnyIsNa;
use crate::lints::class_equals::class_equals::ClassEquals;
use crate::lints::duplicated_arguments::duplicated_arguments::DuplicatedArguments;
// use crate::lints::empty_assignment::empty_assignment::EmptyAssignment;
use crate::lints::equal_assignment::equal_assignment::EqualAssignment;
use crate::lints::equals_na::equals_na::EqualsNa;
// use crate::lints::expect_length::expect_length::ExpectLength;
use crate::lints::length_levels::length_levels::LengthLevels;
use crate::lints::length_test::length_test::LengthTest;
use crate::lints::lengths::lengths::Lengths;
use crate::lints::redundant_equals::redundant_equals::RedundantEquals;
// use crate::lints::true_false_symbol::true_false_symbol::TrueFalseSymbol;
use crate::lints::which_grepl::which_grepl::WhichGrepl;
use crate::message::*;
use crate::trait_lint_checker::LintChecker;
use crate::utils::*;
use anyhow::Result;
use std::path::Path;

fn rule_name_to_lint_checker(rule_name: &str) -> Box<dyn LintChecker> {
    match rule_name {
        "any_duplicated" => Box::new(AnyDuplicated),
        "any_is_na" => Box::new(AnyIsNa),
        "class_equals" => Box::new(ClassEquals),
        "duplicated_arguments" => Box::new(DuplicatedArguments),
        // "empty_assignment" => Box::new(EmptyAssignment),
        "equal_assignment" => Box::new(EqualAssignment),
        "equals_na" => Box::new(EqualsNa),
        // "expect_length" => Box::new(ExpectLength),
        "length_levels" => Box::new(LengthLevels),
        "length_test" => Box::new(LengthTest),
        "lengths" => Box::new(Lengths),
        "redundant_equals" => Box::new(RedundantEquals),
        // "true_false_symbol" => Box::new(TrueFalseSymbol),
        "which_grepl" => Box::new(WhichGrepl),
        unknown => unreachable!("unknown rule name: {unknown}"),
    }
}

pub fn get_checks(contents: &str, file: &Path, config: Config) -> Result<Vec<Diagnostic>> {
    let parser_options = RParserOptions::default();
    let parsed = air_r_parser::parse(contents, parser_options);

    let syntax = &parsed.syntax();
    let expressions = &parsed.tree().expressions();
    let expressions_vec: Vec<_> = expressions.into_iter().collect();

    let loc_new_lines = find_new_lines(syntax)?;
    let diagnostics: Vec<Diagnostic> = expressions_vec
        .iter()
        .map(|expression| check_ast(expression, file.to_str().unwrap(), &config).unwrap())
        .flatten()
        .collect();

    let diagnostics = compute_lints_location(diagnostics, &loc_new_lines);

    Ok(diagnostics)
}

pub(crate) struct Checker {
    diagnostics: Vec<Diagnostic>,
}

impl Checker {
    // fn visit_body(&mut self, body: &air_r_syntax::RExpressionList) {
    //     let expressions_vec: Vec<_> = body.into_iter().collect();
    //     for stmt in expressions_vec {
    //         self.visit_stmt(stmt);
    //     }
    // }
}

pub fn check_ast(
    expression: &air_r_syntax::AnyRExpression,
    file: &str,
    config: &Config,
) -> anyhow::Result<Vec<Diagnostic>> {
    let mut diagnostics: Vec<Diagnostic> = vec![];

    match expression {
        // air_r_syntax::RExpressionList
        // air_r_syntax::AnyRExpression::RFunctionDefinition(children) => {
        //     use biome_rowan::AstNode;
        //     let params = children.parameters()?;
        //     diagnostics.extend(check_ast(
        //         &RExpressionList::new_unchecked(params.syntax().clone()),
        //         file,
        //         &config.clone(),
        //     )?);
        // }
        air_r_syntax::AnyRExpression::RCall(children) => {
            let any_r_exp: &AnyRExpression = &children.clone().into();
            diagnostics.extend(AnyDuplicated.check(any_r_exp, file)?);
            diagnostics.extend(AnyIsNa.check(any_r_exp, file)?);
            diagnostics.extend(DuplicatedArguments.check(any_r_exp, file)?);
            diagnostics.extend(LengthLevels.check(any_r_exp, file)?);
            diagnostics.extend(LengthTest.check(any_r_exp, file)?);
            diagnostics.extend(Lengths.check(any_r_exp, file)?);
            diagnostics.extend(WhichGrepl.check(any_r_exp, file)?);

            let RCallFields { arguments, .. } = children.as_fields();
            let RCallArgumentsFields { items, .. } = arguments?.as_fields();
            let arg_exprs: Vec<AnyRExpression> = items
                .into_iter()
                .map(|x| x.unwrap().as_fields().value.unwrap())
                .collect();

            for expr in arg_exprs {
                diagnostics.extend(check_ast(&expr, file, config)?);
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
            diagnostics.extend(ClassEquals.check(any_r_exp, file)?);
            diagnostics.extend(EqualAssignment.check(any_r_exp, file)?);
            diagnostics.extend(EqualsNa.check(any_r_exp, file)?);
            diagnostics.extend(RedundantEquals.check(any_r_exp, file)?);
            let RBinaryExpressionFields { left, right, .. } = children.as_fields();
            diagnostics.extend(check_ast(&left?, file, config)?);
            diagnostics.extend(check_ast(&right?, file, config)?);
        }
        air_r_syntax::AnyRExpression::RParenthesizedExpression(children) => {
            let RParenthesizedExpressionFields { body, .. } = children.as_fields();
            diagnostics.extend(check_ast(&body?, file, config)?);
        }
        // | air_r_syntax::AnyRExpression::RExtractExpression(children)
        // | air_r_syntax::AnyRExpression::RNamespaceExpression(children)
        // | air_r_syntax::AnyRExpression::RNaExpression(children)
        // | air_r_syntax::AnyRExpression::RForStatement(children)
        air_r_syntax::AnyRExpression::RWhileStatement(children) => {
            let RWhileStatementFields { condition, .. } = children.as_fields();
            diagnostics.extend(check_ast(&condition?, file, config)?);
        }
        air_r_syntax::AnyRExpression::RIfStatement(children) => {
            let RIfStatementFields { condition, .. } = children.as_fields();
            diagnostics.extend(check_ast(&condition?, file, config)?);
        }
        _ => {
            // println!("Not implemented");
        }
    }

    Ok(diagnostics)
}
