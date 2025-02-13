use super::*;
use crate::semantic_model::binding::SemanticModelBindingData;
use crate::semantic_model::globals::SemanticModelGlobalBindingData;
use crate::semantic_model::reference::SemanticModelUnresolvedReference;
use crate::semantic_model::scope::SemanticModelScopeData;
use air_r_syntax::{RRoot, RSyntaxNode, TextRange};
use std::sync::Arc;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct BindingId(u32);

impl BindingId {
    pub(crate) fn new(index: usize) -> Self {
        // SAFETY: We didn't handle files execedding `u32::MAX` bytes.
        // Thus, it isn't possible to execedd `u32::MAX` bindings.
        Self(index as u32)
    }

    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct ReferenceId(BindingId, u32);

impl ReferenceId {
    pub(crate) fn new(binding_id: BindingId, index: usize) -> Self {
        // SAFETY: We didn't handle files execedding `u32::MAX` bytes.
        // Thus, it isn't possible to execedd `u32::MAX` refernces.
        Self(binding_id, index as u32)
    }

    // Points to [SemanticModel]::bindings vec
    pub(crate) fn binding_id(&self) -> BindingId {
        self.0
    }

    pub(crate) fn index(self) -> usize {
        self.1 as usize
    }
}

#[derive(Debug)]
pub struct SemanticModel {
    data: Arc<SemanticModelData>,
}

#[derive(Debug)]
pub(super) struct SemanticModelData {
    pub root: RRoot,
    pub scopes: Vec<SemanticModelScopeData>,
    pub scope_by_range: Lapper<u32, ScopeId>,
    pub binding_node_by_start: FxHashMap<TextSize, RSyntaxNode>,
    pub scope_node_by_range: FxHashMap<TextRange, RSyntaxNode>,
    pub bindings: Vec<SemanticModelBindingData>,
    pub bindings_by_start: FxHashMap<TextSize, BindingId>,
    pub declared_at_by_start: FxHashMap<TextSize, BindingId>,
    pub unresolved_references: Vec<SemanticModelUnresolvedReference>,
    pub globals: Vec<SemanticModelGlobalBindingData>,
}

impl SemanticModel {
    pub(super) fn new(data: SemanticModelData) -> Self {
        Self { data: Arc::new(data) }
    }

    // ... rest of implementation ...
}

// We use `NonZeroU32` to allow niche optimizations.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ScopeId(std::num::NonZeroU32);

// We don't implement `From<usize> for ScopeId` and `From<ScopeId> for usize`
// to ensure that the API consumers don't create `ScopeId`.
impl ScopeId {
    pub(crate) fn new(index: usize) -> Self {
        // SAFETY: We didn't handle files execedding `u32::MAX` bytes.
        // Thus, it isn't possible to execedd `u32::MAX` scopes.
        //
        // Adding 1 ensurtes that the value is never equal to 0.
        // Instead of adding 1, we could XOR the value with `u32::MAX`.
        // This is what the [nonmax](https://docs.rs/nonmax/latest/nonmax/) crate does.
        // However, this doesn't preserve the order.
        // It is why we opted for adding 1.
        Self(unsafe { std::num::NonZeroU32::new_unchecked(index.unchecked_add(1) as u32) })
    }

    pub(crate) fn index(self) -> usize {
        // SAFETY: The internal representation ensures that the value is never equal to 0.
        // Thus, it is safe to substract 1.
        (unsafe { self.0.get().unchecked_sub(1) }) as usize
    }
}
