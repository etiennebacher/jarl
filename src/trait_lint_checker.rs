use crate::message::*;
use air_r_syntax::{AnyRExpression, RCall, RExpressionList, RSyntaxNode};
use anyhow::Result;

/// Takes an AST node and checks whether it satisfies or violates the
/// implemented rule.
pub trait LintChecker {
    fn check(&self, ast: &AnyRExpression, file: &str) -> Result<Vec<Diagnostic>>;
}
