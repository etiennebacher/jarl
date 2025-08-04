use crate::message::*;
use air_r_syntax::{RCall, RExpressionList, RSyntaxNode};
use anyhow::Result;

/// Takes an AST node and checks whether it satisfies or violates the
/// implemented rule.
pub trait LintChecker {
    fn check(&self, ast: &RCall, file: &str) -> Result<Vec<Diagnostic>>;
}
