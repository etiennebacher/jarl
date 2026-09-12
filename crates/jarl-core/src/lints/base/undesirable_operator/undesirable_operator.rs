use crate::diagnostic::*;
use crate::lints::base::undesirable_operator::options::ResolvedUndesirableOperatorOptions;
use crate::rule_set::Rule;
use air_r_syntax::*;
use biome_rowan::{AstNode, TextRange};

pub struct UndesirableOperator {
    pub operator: String,
}

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for use of banned operators.
///
/// ## Why is this bad?
///
/// Some operators should not appear in production code. For example, `:::`
/// accesses a package's internal functions, and `<<-` and `->>` assign outside
/// the current environment.
///
/// ## Configuration
///
/// By default, only `->>`, `:::`, and `<<-` are flagged. You can customize the
/// list in `jarl.toml`:
///
/// To replace the default list entirely:
///
/// ```toml
/// [lint.undesirable_operator]
/// operators = ["$", "@"]
/// ```
///
/// To add to the defaults:
///
/// ```toml
/// [lint.undesirable_operator]
/// extend-operators = ["$", "%in%"]
/// ```
///
/// Specifying both `operators` and `extend-operators` is an error.
///
/// ## Example
///
/// ```r
/// package:::internal_function()  # flagged by default
/// value <<- 1                    # flagged by default
/// ```
impl Violation for UndesirableOperator {
    fn rule(&self) -> Rule {
        Rule::UndesirableOperator
    }

    fn body(&self) -> String {
        format!("`{}` is listed as an undesirable operator.", self.operator)
    }
}

fn undesirable_operator(operator: &str, range: TextRange) -> Option<Diagnostic> {
    Some(Diagnostic::new(
        UndesirableOperator { operator: operator.to_string() },
        range,
        Fix::empty(),
    ))
}

fn check_operator(
    operator: &RSyntaxToken,
    options: &ResolvedUndesirableOperatorOptions,
) -> Option<Diagnostic> {
    let operator_text = operator.text_trimmed();
    if !options.operators.contains(operator_text) {
        return None;
    }

    undesirable_operator(operator_text, operator.text_trimmed_range())
}

pub fn undesirable_operator_binary(
    ast: &RBinaryExpression,
    options: &ResolvedUndesirableOperatorOptions,
) -> anyhow::Result<Option<Diagnostic>> {
    Ok(check_operator(&ast.operator()?, options))
}

pub fn undesirable_operator_namespace(
    ast: &RNamespaceExpression,
    options: &ResolvedUndesirableOperatorOptions,
) -> anyhow::Result<Option<Diagnostic>> {
    Ok(check_operator(&ast.operator()?, options))
}

pub fn undesirable_operator_extract(
    ast: &RExtractExpression,
    options: &ResolvedUndesirableOperatorOptions,
) -> anyhow::Result<Option<Diagnostic>> {
    Ok(check_operator(&ast.operator()?, options))
}

pub fn undesirable_operator_call(
    ast: &RCall,
    options: &ResolvedUndesirableOperatorOptions,
) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let function_text = function.syntax().text_trimmed().to_string();
    let Some(operator_text) = function_text
        .strip_prefix('`')
        .and_then(|text| text.strip_suffix('`'))
    else {
        return Ok(None);
    };

    if !options.operators.contains(operator_text) {
        return Ok(None);
    }

    Ok(undesirable_operator(
        operator_text,
        function.syntax().text_trimmed_range(),
    ))
}
