mod binding;
mod builder;
mod closure;
mod globals;
// mod import;
// mod is_constant;
mod model;
mod reference;
mod scope;

pub use self::model::ScopeId;
use crate::SemanticEventExtractor;
use air_r_syntax::{RIdentifier, RLanguage, RRoot, RSyntaxKind, RSyntaxNode, TextRange, TextSize};
use biome_rowan::AstNode;
use rust_lapper::{Interval, Lapper};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeSet;

pub use binding::*;
pub use builder::*;

pub use globals::*;
// pub use import::*;
// pub use is_constant::*;
pub use model::*;
pub use reference::*;
pub use scope::*;

/// Extra options for the [SemanticModel] creation.
#[derive(Default)]
pub struct SemanticModelOptions {
    /// All the allowed globals names
    pub globals: FxHashSet<String>,
}

/// Build the complete [SemanticModel] of a parsed file.
/// For a push based model to build the [SemanticModel], see [SemanticModelBuilder].
pub fn semantic_model(root: &RRoot, options: SemanticModelOptions) -> SemanticModel {
    let mut extractor = SemanticEventExtractor::default();
    let mut builder = SemanticModelBuilder::new(root.clone());

    let SemanticModelOptions { globals } = options;

    for global in globals {
        builder.push_global(global);
    }

    let root = root.syntax();
    for node in root.preorder() {
        match node {
            air_r_syntax::WalkEvent::Enter(node) => {
                builder.push_node(&node);
                extractor.enter(&node);
            }
            air_r_syntax::WalkEvent::Leave(node) => extractor.leave(&node),
        }
    }

    // Pop the global scope at the end to process its references
    extractor.pop_scope(root.text_trimmed_range());

    while let Some(e) = extractor.pop() {
        builder.push_event(e);
    }

    builder.build()
}
