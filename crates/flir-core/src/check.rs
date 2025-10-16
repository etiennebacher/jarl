use crate::error::ParseError;
use air_fs::relativize_path;
use air_r_parser::RParserOptions;
use air_r_syntax::RForStatementFields;
use air_r_syntax::{
    AnyRExpression, RBinaryExpressionFields, RIfStatementFields, RWhileStatementFields,
};
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::analyze;
use crate::config::Config;
use crate::diagnostic::*;
use crate::fix::*;
use crate::rule_table::RuleTable;
use crate::utils::*;

pub fn check(config: Config) -> Vec<(String, Result<Vec<Diagnostic>, anyhow::Error>)> {
    config
        .paths
        .par_iter()
        .map(|file| {
            let res = check_path(file, config.clone());
            (relativize_path(file), res)
        })
        .collect()
}

pub fn check_path(path: &PathBuf, config: Config) -> Result<Vec<Diagnostic>, anyhow::Error> {
    if config.apply_fixes || config.apply_unsafe_fixes {
        lint_fix(path, config)
    } else {
        lint_only(path, config)
    }
}

pub fn lint_only(path: &PathBuf, config: Config) -> Result<Vec<Diagnostic>, anyhow::Error> {
    let path = relativize_path(path);
    let contents = fs::read_to_string(Path::new(&path))
        .with_context(|| format!("Failed to read file: {path}"))?;

    let checks = get_checks(&contents, &PathBuf::from(&path), config.clone())
        .with_context(|| format!("Failed to get checks for file: {path}"))?;

    Ok(checks)
}

pub fn lint_fix(path: &PathBuf, config: Config) -> Result<Vec<Diagnostic>, anyhow::Error> {
    let path = relativize_path(path);
    let mut has_skipped_fixes = true;
    let mut checks: Vec<Diagnostic>;

    loop {
        let contents = fs::read_to_string(Path::new(&path))
            .with_context(|| format!("Failed to read file: {path}",))?;

        checks = get_checks(&contents, &PathBuf::from(&path), config.clone())
            .with_context(|| format!("Failed to get checks for file: {path}",))?;

        if !has_skipped_fixes {
            break;
        }

        let (new_has_skipped_fixes, fixed_text) = apply_fixes(&checks, &contents);
        has_skipped_fixes = new_has_skipped_fixes;

        fs::write(&path, fixed_text).with_context(|| format!("Failed to write file: {path}",))?;
    }

    Ok(checks)
}

#[derive(Debug)]
// The object that will collect diagnostics in check_expressions(). One per
// analyzed file.
pub struct Checker {
    // The diagnostics to report (possibly empty).
    pub diagnostics: Vec<Diagnostic>,
    // A vector of `Rule`. A `rule` contains the name of the rule to apply,
    // whether it is safe to fix, unsafe to fix, or doesn't have a fix, and
    // the minimum R version from which this rule is available.
    pub rules: RuleTable,
    // The R version that is manually passed by the user in the CLI. Any rule
    // that has a minimum R version higher than this value will be deactivated.
    pub minimum_r_version: Option<(u32, u32, u32)>,
}

impl Checker {
    fn new() -> Self {
        Self {
            diagnostics: vec![],
            rules: RuleTable::empty(),
            minimum_r_version: None,
        }
    }

    // This takes an Option<Diagnostic> because each lint rule reports a
    // Some(Diagnostic) or None.
    pub(crate) fn report_diagnostic(&mut self, diagnostic: Option<Diagnostic>) {
        if let Some(diagnostic) = diagnostic {
            self.diagnostics.push(diagnostic);
        }
    }

    pub(crate) fn is_rule_enabled(&mut self, rule: &str) -> bool {
        self.rules.enabled.iter().any(|r| r.name == rule)
    }
}

// Takes the R code as a string, parses it, and obtains a (possibly empty)
// vector of `Diagnostic`s.
//
// If there are diagnostics to report, this is also where their range in the
// string is converted to their location (row, column).
pub fn get_checks(contents: &str, file: &Path, config: Config) -> Result<Vec<Diagnostic>> {
    let parser_options = RParserOptions::default();
    let parsed = air_r_parser::parse(contents, parser_options);

    if parsed.has_error() {
        return Err(ParseError { filename: file.to_path_buf() }.into());
    }

    let syntax = &parsed.syntax();
    let expressions = &parsed.tree().expressions();
    let expressions_vec: Vec<_> = expressions.into_iter().collect();

    let mut checker = Checker::new();
    checker.rules = config.rules_to_apply;
    checker.minimum_r_version = config.minimum_r_version;
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
        // TODO: this should be removed once comments in nodes are better
        // handled, #95
        .map(|mut x| {
            if x.fix.to_skip {
                x.fix = Fix::empty();
            }
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
// Some expression types do both (e.g. RBinaryExpression), some only do the
// dispatch to rules (e.g. RIdentifier), some only do the recursive call (e.g.
// RFunctionDefinition).
//
// Not all patterns are covered but they don't necessarily have to be.
// For instance, there are currently no rule for RNaExpression and
// it doesn't have any children expression on which we need to call
// check_expression().
//
// If a rule needs to be applied on RNaExpression in the future, then
// we can add the corresponding match arm at this moment.
pub fn check_expression(
    expression: &air_r_syntax::AnyRExpression,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    match expression {
        AnyRExpression::AnyRValue(children) => {
            analyze::anyvalue::anyvalue(children, checker)?;
        }
        AnyRExpression::RBinaryExpression(children) => {
            analyze::binary_expression::binary_expression(children, checker)?;
            let RBinaryExpressionFields { left, right, .. } = children.as_fields();
            check_expression(&left?, checker)?;
            check_expression(&right?, checker)?;
        }
        AnyRExpression::RBracedExpressions(children) => {
            let expressions: Vec<_> = children.expressions().into_iter().collect();

            for expr in expressions {
                check_expression(&expr, checker)?;
            }
        }
        AnyRExpression::RCall(children) => {
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
        AnyRExpression::RForStatement(children) => {
            analyze::for_loop::for_loop(children, checker)?;
            let RForStatementFields { variable, sequence, body, .. } = children.as_fields();
            analyze::identifier::identifier(&variable?, checker)?;

            check_expression(&sequence?, checker)?;
            check_expression(&body?, checker)?;
        }
        AnyRExpression::RFunctionDefinition(children) => {
            let body = children.body();
            check_expression(&body?, checker)?;
        }
        AnyRExpression::RIdentifier(x) => {
            analyze::identifier::identifier(x, checker)?;
        }
        AnyRExpression::RIfStatement(children) => {
            analyze::if_::if_(children, checker)?;

            let RIfStatementFields { condition, consequence, else_clause, .. } =
                children.as_fields();
            check_expression(&condition?, checker)?;
            check_expression(&consequence?, checker)?;
            if let Some(else_clause) = else_clause {
                let alternative = else_clause.alternative();
                check_expression(&alternative?, checker)?;
            }
        }
        AnyRExpression::RParenthesizedExpression(children) => {
            let body = children.body();
            check_expression(&body?, checker)?;
        }
        AnyRExpression::RRepeatStatement(children) => {
            let body = children.body();
            check_expression(&body?, checker)?;
        }
        AnyRExpression::RSubset(children) => {
            analyze::subset::subset(children, checker)?;
            let arguments: Vec<_> = children.arguments()?.items().into_iter().collect();

            for expr in arguments {
                if let Some(expr) = expr?.value() {
                    check_expression(&expr, checker)?;
                }
            }
        }
        AnyRExpression::RSubset2(children) => {
            let arguments: Vec<_> = children.arguments()?.items().into_iter().collect();

            for expr in arguments {
                if let Some(expr) = expr?.value() {
                    check_expression(&expr, checker)?;
                }
            }
        }
        AnyRExpression::RUnaryExpression(children) => {
            let argument = children.argument();
            check_expression(&argument?, checker)?;
        }
        AnyRExpression::RWhileStatement(children) => {
            analyze::while_::while_(children, checker)?;
            let RWhileStatementFields { condition, body, .. } = children.as_fields();
            check_expression(&condition?, checker)?;
            check_expression(&body?, checker)?;
        }
        _ => {}
    }

    Ok(())
}
