use super::*;
use air_r_syntax::RSyntaxNode;

pub fn is_function(node: &RSyntaxNode) -> bool {
    matches!(node.kind(), RSyntaxKind::R_FUNCTION_DEFINITION)
}
