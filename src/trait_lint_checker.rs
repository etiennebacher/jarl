use crate::message::*;
use air_r_syntax::AnyRExpression;
use anyhow::Result;

/// Takes an AST node and checks whether it satisfies or violates the
/// implemented rule.
pub trait LintChecker {
    fn check(&self, ast: &AnyRExpression) -> Result<Diagnostic>;
}
