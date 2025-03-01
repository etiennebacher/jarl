use super::*;
use air_r_syntax::{RIdentifier, RSyntaxNode, TextRange, TextSize};

#[derive(Debug)]
pub struct SemanticModelBindingData {
    pub range: TextRange,
    pub references: Vec<SemanticModelReference>,
    pub export_by_start: smallvec::SmallVec<[TextSize; 1]>,
}

#[derive(Debug)]
pub struct SemanticModelReference {
    pub range_start: TextSize,
    pub ty: SemanticModelReferenceType,
}

#[derive(Debug)]
pub enum SemanticModelReferenceType {
    Read {},
    Write {},
}
