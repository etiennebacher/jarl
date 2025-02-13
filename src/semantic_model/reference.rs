use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReferenceId {
    pub binding_id: BindingId,
    pub index: usize,
}

impl ReferenceId {
    pub fn new(binding_id: BindingId, index: usize) -> Self {
        Self { binding_id, index }
    }
}

#[derive(Debug)]
pub struct SemanticModelUnresolvedReference {
    pub(crate) range: TextRange,
}
