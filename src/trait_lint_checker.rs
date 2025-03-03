use crate::message::*;
use air_r_syntax::RSyntaxNode;

pub trait LintChecker {
    fn check(&self, ast: &RSyntaxNode, loc_new_lines: &[usize], file: &str) -> Vec<Diagnostic>;
}
