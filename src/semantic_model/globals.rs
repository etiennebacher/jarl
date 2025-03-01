use super::*;
use crate::semantic_model::binding::SemanticModelReferenceType;
use air_r_syntax::TextSize;

#[derive(Debug)]
pub struct SemanticModelGlobalBindingData {
    pub references: Vec<SemanticModelGlobalReferenceData>,
}

#[derive(Debug)]
pub struct SemanticModelGlobalReferenceData {
    pub range_start: TextSize,
    pub ty: SemanticModelReferenceType,
}
