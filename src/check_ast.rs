use air_r_parser::RParserOptions;
use air_r_syntax::{RSyntaxKind, RSyntaxNode};

use crate::lints::any_duplicated::any_duplicated::AnyDuplicated;
use crate::lints::any_is_na::any_is_na::AnyIsNa;
use crate::lints::class_equals::class_equals::ClassEquals;
use crate::lints::equals_na::equals_na::EqualsNa;
use crate::lints::true_false_symbol::true_false_symbol::TrueFalseSymbol;
use crate::message::*;
use crate::semantic_model;
use crate::trait_lint_checker::LintChecker;
use crate::utils::*;
use crate::SemanticModelOptions;
use anyhow::Result;
use std::path::Path;

fn rule_name_to_lint_checker(rule_name: &str) -> Box<dyn LintChecker> {
    match rule_name {
        "any_duplicated" => Box::new(AnyDuplicated),
        "any_is_na" => Box::new(AnyIsNa),
        "class_equals" => Box::new(ClassEquals),
        "equals_na" => Box::new(EqualsNa),
        "true_false_symbol" => Box::new(TrueFalseSymbol),
        unknown => unreachable!("unknown rule name: {unknown}"),
    }
}

pub fn get_checks(
    contents: &str,
    file: &Path,
    parser_options: RParserOptions,
    rules: Vec<&str>,
) -> Result<Vec<Diagnostic>> {
    let parsed = air_r_parser::parse(contents, parser_options);

    let root = &parsed.tree();
    let semantic = semantic_model(root, SemanticModelOptions::default());
    let mut diagnostics_semantic: Vec<Diagnostic> = vec![];
    // let mut diagnostics_semantic: Vec<Diagnostic> = check_unused_variables(&semantic);

    let syntax = &parsed.syntax();
    let loc_new_lines = find_new_lines(syntax)?;
    let mut diagnostics_lints: Vec<Diagnostic> =
        check_ast(syntax, &loc_new_lines, file.to_str().unwrap(), &rules);

    diagnostics_semantic.append(&mut diagnostics_lints);

    Ok(diagnostics_semantic)
}

pub fn check_ast(
    ast: &RSyntaxNode,
    loc_new_lines: &[usize],
    file: &str,
    rules: &Vec<&str>,
) -> Vec<Diagnostic> {
    let mut diagnostics: Vec<Diagnostic> = vec![];

    let linters: Vec<Box<dyn LintChecker>> = rules
        .iter()
        .map(|rule| rule_name_to_lint_checker(rule))
        .collect();

    for linter in linters {
        diagnostics.extend(linter.check(ast, loc_new_lines, file));
    }

    // if ast.kind() == RSyntaxKind::R_CALL || ast.kind() == RSyntaxKind::R_CALL_ARGUMENTS {
    //     println!("{:?}", ast.kind());
    //     println!("Text: {:?}", ast.text_trimmed());
    //     println!(
    //         "Children: {:?}",
    //         ast.children().map(|x| x.kind()).collect::<Vec<_>>()
    //     );
    // }

    match ast.kind() {
        RSyntaxKind::R_EXPRESSION_LIST
        | RSyntaxKind::R_FUNCTION_DEFINITION
        | RSyntaxKind::R_CALL
        | RSyntaxKind::R_CALL_ARGUMENTS
        | RSyntaxKind::R_SUBSET
        | RSyntaxKind::R_SUBSET2
        | RSyntaxKind::R_PARAMETER_LIST
        | RSyntaxKind::R_PARAMETERS
        | RSyntaxKind::R_PARAMETER
        | RSyntaxKind::R_ARGUMENT_LIST
        | RSyntaxKind::R_ARGUMENT
        | RSyntaxKind::R_BRACED_EXPRESSIONS
        | RSyntaxKind::R_ROOT
        | RSyntaxKind::R_REPEAT_STATEMENT
        | RSyntaxKind::R_UNARY_EXPRESSION
        | RSyntaxKind::R_BINARY_EXPRESSION
        | RSyntaxKind::R_PARENTHESIZED_EXPRESSION
        | RSyntaxKind::R_EXTRACT_EXPRESSION
        | RSyntaxKind::R_NAMESPACE_EXPRESSION
        | RSyntaxKind::R_NA_EXPRESSION
        | RSyntaxKind::R_FOR_STATEMENT
        | RSyntaxKind::R_WHILE_STATEMENT
        | RSyntaxKind::R_IF_STATEMENT => {
            for child in ast.children() {
                diagnostics.extend(check_ast(&child, loc_new_lines, file, rules));
            }
        }
        RSyntaxKind::R_IDENTIFIER => {
            let fc = &ast.first_child();
            let _has_child = fc.is_some();
            let ns = ast.next_sibling();
            let has_sibling = ns.is_some();
            if has_sibling {
                diagnostics.extend(check_ast(&ns.unwrap(), loc_new_lines, file, rules));
            }
        }
        _ => {
            // println!("Unknown kind: {:?}", ast.kind());
            match &ast.first_child() {
                Some(_) => {
                    for child in ast.children() {
                        diagnostics.extend(check_ast(&child, loc_new_lines, file, rules));
                    }
                }
                None => {
                    let ns = ast.next_sibling();
                    let has_sibling = ns.is_some();
                    if has_sibling {
                        diagnostics.extend(check_ast(&ns.unwrap(), loc_new_lines, file, rules));
                    }
                }
            }
        }
    };

    diagnostics
}
