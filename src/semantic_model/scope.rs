use super::*;
use crate::semantic_model::model::{ReferenceId, ScopeId};
use air_r_syntax::{RSyntaxNode, TextRange};
use biome_rowan::TokenText;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

#[derive(Debug)]
pub(crate) struct SemanticModelScopeData {
    // The scope range
    pub(crate) range: TextRange,
    // The parent scope of this scope
    pub(crate) parent: Option<ScopeId>,
    // All children scope of this scope
    pub(crate) children: Vec<ScopeId>,
    // All bindings of this scope (points to SemanticModelData::bindings)
    pub(crate) bindings: Vec<BindingId>,
    // Map pointing to the [bindings] vec of each bindings by its name
    pub(crate) bindings_by_name: FxHashMap<TokenText, BindingId>,
    // All read references of a scope
    pub(crate) read_references: Vec<ReferenceId>,
    // All write references of a scope
    pub(crate) write_references: Vec<ReferenceId>,
    // Identify if this scope is from a closure or not
    pub(crate) is_function: bool,
}

/// Represents a lexical scope in the code
#[derive(Debug)]
pub struct Scope {
    /// The ID of this scope
    pub id: ScopeId,
    /// The parent scope's ID, if any
    pub parent: Option<ScopeId>,
    /// The syntax node that created this scope
    pub node: RSyntaxNode,
    /// The text range this scope covers
    pub range: TextRange,
    /// The bindings defined in this scope
    pub bindings: Vec<String>,
    /// Shadowed bindings in this scope (binding name, previous binding info)
    pub shadowed: VecDeque<(String, String)>,
    /// References to identifiers in this scope
    pub references: FxHashSet<String>,
}

impl Scope {
    pub fn new(id: ScopeId, parent: Option<ScopeId>, node: RSyntaxNode, range: TextRange) -> Self {
        Self {
            id,
            parent,
            node,
            range,
            bindings: Vec::new(),
            shadowed: VecDeque::new(),
            references: FxHashSet::default(),
        }
    }

    /// Check if this scope contains a binding with the given name
    pub fn has_binding(&self, name: &str) -> bool {
        self.bindings.iter().any(|b| b == name)
    }
}
