//! Events emitted by the [SemanticEventExtractor] which are then constructed into the Semantic Model
use air_r_syntax::*;
use biome_rowan::TextSize;
use biome_rowan::{syntax::Preorder, AstNode, SyntaxNodeOptionExt, TokenText};
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::mem;
use RSyntaxKind::*;

use crate::ScopeId;

/// Events emitted by the [SemanticEventExtractor].
/// These events are later made into the Semantic Model.
#[derive(Debug, Eq, PartialEq)]
pub enum SemanticEvent {
    /// Tracks where a new symbol declaration is found.
    /// Generated for:
    /// - Variable Assignments (in R, assignment creates variables)
    /// - Function parameters
    DeclarationFound { range: TextRange, scope_id: ScopeId },

    /// Tracks where a symbol is read
    /// Generated for:
    /// - Variable references
    /// - Function calls
    Read {
        range: TextRange,
        declaration_at: TextSize,
        scope_id: ScopeId,
    },

    /// Tracks where a symbol is written
    /// Generated for:
    /// - Assignment operations (<-, =, ->)
    Write {
        range: TextRange,
        declaration_at: TextSize,
        scope_id: ScopeId,
    },

    /// Tracks references that do not have any matching binding
    UnresolvedReference { is_read: bool, range: TextRange },

    /// Tracks where a new scope starts
    /// Generated for:
    /// - Function definitions
    /// - Block expressions ({ ... })
    ScopeStarted {
        range: TextRange,
        parent_scope_id: Option<ScopeId>,
        is_function: bool,
    },

    /// Tracks where a scope ends
    ScopeEnded { range: TextRange },
}

impl SemanticEvent {
    pub fn range(&self) -> TextRange {
        match self {
            Self::DeclarationFound { range, .. }
            | Self::ScopeStarted { range, .. }
            | Self::ScopeEnded { range }
            | Self::Read { range, .. }
            | Self::Write { range, .. }
            | Self::UnresolvedReference { range, .. } => *range,
        }
    }
}

#[derive(Debug)]
pub struct SemanticEventExtractor {
    /// Event queue
    stash: VecDeque<SemanticEvent>,
    /// Stack of scopes
    scopes: Vec<Scope>,
    /// Number of generated scopes
    scope_count: usize,
    /// Current available bindings and their range
    bindings: FxHashMap<BindingName, BindingInfo>,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
enum BindingName {
    Value(TokenText),
}

#[derive(Debug, Clone)]
struct BindingInfo {
    /// range start of the name
    range_start: TextSize,
    /// Kind of the declaration
    declaration_kind: RSyntaxKind,
}

impl BindingInfo {
    fn new(range_start: TextSize, declaration_kind: RSyntaxKind) -> Self {
        Self { range_start, declaration_kind }
    }
}

#[derive(Debug, Clone)]
enum Reference {
    /// Read a value
    /// ```r
    /// x  # reading x
    /// f() # reading f
    /// ```
    Read(TextRange),

    /// Assignment
    /// ```r
    /// x <- 1  # writing to x
    /// ```
    Write(TextRange),
}

impl Reference {
    const fn is_write(&self) -> bool {
        matches!(self, Self::Write { .. })
    }

    const fn range(&self) -> TextRange {
        match self {
            Self::Read(range) | Self::Write(range) => *range,
        }
    }
}

#[derive(Debug)]
struct Scope {
    scope_id: ScopeId,
    /// All bindings declared inside this scope
    bindings: Vec<BindingName>,
    /// References that need to be bound
    references: FxHashMap<BindingName, Vec<Reference>>,
    /// Shadowed bindings to be restored after scope ends
    shadowed: Vec<(BindingName, BindingInfo)>,
}

#[derive(Debug, Default)]
struct ScopeOptions {
    /// Is the scope a function?
    is_function: bool,
}

// Update Default implementation
impl Default for SemanticEventExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticEventExtractor {
    pub fn new() -> Self {
        let mut extractor = Self {
            stash: VecDeque::new(),
            scopes: Vec::new(),
            scope_count: 0,
            bindings: FxHashMap::default(),
        };

        // Create root scope
        extractor.push_scope(
            TextRange::default(), // or get the full file range if available
            ScopeOptions { is_function: false },
        );

        extractor
    }

    #[inline]
    pub fn enter(&mut self, node: &RSyntaxNode) {
        match node.kind() {
            // Function definitions create new scopes
            R_FUNCTION_DEFINITION => {
                self.push_scope(
                    node.text_trimmed_range(),
                    ScopeOptions { is_function: true },
                );
            }

            R_FOR_STATEMENT => {
                // Create a new scope for the loop
                self.push_scope(
                    node.text_trimmed_range(),
                    ScopeOptions { is_function: false },
                );

                // Find and handle the loop variable (it's the first R_IDENTIFIER)
                if let Some(loop_var) = node.children().find(|n| n.kind() == R_IDENTIFIER) {
                    if let Some(name_token) = loop_var.first_token() {
                        let name = name_token.token_text_trimmed();
                        let range = loop_var.text_trimmed_range();

                        // Create a binding for the loop variable in the loop's scope
                        let info = BindingInfo::new(range.start(), R_IDENTIFIER);
                        self.push_binding(BindingName::Value(name.clone()), info);

                        // We need to declare that this variable is written
                        // if we want to mark a Read later.
                        let range = node.text_trimmed_range();
                        self.push_reference(BindingName::Value(name), Reference::Write(range));
                    }
                }
            }

            // Handle identifiers (both references and assignments)
            R_IDENTIFIER => {
                self.enter_identifier_usage(node);
            }

            // Function parameters create new bindings
            R_PARAMETER => {
                if let Some(name) = node.first_token() {
                    self.handle_parameter_declaration(&name);
                }
            }

            _ => {}
        }
    }

    fn handle_parameter_declaration(&mut self, name_token: &RSyntaxToken) {
        let name = name_token.token_text_trimmed();
        let info = BindingInfo::new(
            name_token.text_trimmed_range().start(),
            RSyntaxKind::R_PARAMETER,
        );
        self.push_binding(BindingName::Value(name), info);
    }

    fn enter_identifier_usage(&mut self, node: &RSyntaxNode) {
        // Check if this identifier is part of an assignment
        let is_assignment = node.parent().map_or(false, |p| {
            if p.kind() == R_BINARY_EXPRESSION {
                let bin_expr = RBinaryExpression::cast(p.clone());
                let RBinaryExpressionFields { left, operator, right } =
                    bin_expr.unwrap().as_fields();

                let operator = operator.unwrap();
                let left = left.unwrap();
                let right = right.unwrap();
                let is_left_assignment = operator.kind() == ASSIGN
                    && node.kind() == R_IDENTIFIER
                    && node.text_trimmed().to_string() == left.text();
                let is_right_assignment = operator.kind() == ASSIGN_RIGHT
                    && node.kind() == R_IDENTIFIER
                    && node.text_trimmed().to_string() == right.text();
                return is_left_assignment || is_right_assignment;
            } else {
                return false;
            }
        });

        if let Some(name_token) = node.first_token() {
            let name = name_token.token_text_trimmed();
            let range = node.text_trimmed_range();

            let info = BindingInfo::new(range.start(), R_IDENTIFIER);
            self.push_binding(BindingName::Value(name.clone()), info);

            if is_assignment {
                self.push_reference(BindingName::Value(name), Reference::Write(range));
            } else {
                self.push_reference(BindingName::Value(name), Reference::Read(range));
            }
        }
    }

    #[inline]
    pub fn leave(&mut self, node: &RSyntaxNode) {
        match node.kind() {
            R_FUNCTION_DEFINITION => {
                self.pop_scope(node.text_trimmed_range());
            }
            _ => {}
        }
    }

    /// Return any previous extracted [SemanticEvent].
    #[inline]
    pub fn pop(&mut self) -> Option<SemanticEvent> {
        self.stash.pop_front()
    }

    fn push_scope(&mut self, range: TextRange, options: ScopeOptions) {
        let scope_id = ScopeId::new(self.scope_count);
        self.scope_count += 1;
        self.stash.push_back(SemanticEvent::ScopeStarted {
            range,
            parent_scope_id: self.scopes.last().map(|x| x.scope_id),
            is_function: options.is_function,
        });
        self.scopes.push(Scope {
            scope_id,
            bindings: vec![],
            references: FxHashMap::default(),
            shadowed: vec![],
        });
    }

    pub fn pop_scope(&mut self, scope_range: TextRange) {
        let scope = self.scopes.pop().unwrap();
        let scope_id = scope.scope_id;

        // Bind references to declarations
        for (name, references) in scope.references {
            if let Some(&BindingInfo { range_start: declaration_at, .. }) = self.bindings.get(&name)
            {
                // We found the declaration for these references
                for reference in references {
                    let event = match reference {
                        Reference::Read(range) => {
                            SemanticEvent::Read { range, declaration_at, scope_id }
                        }
                        Reference::Write(range) => {
                            SemanticEvent::Write { range, declaration_at, scope_id }
                        }
                    };
                    self.stash.push_back(event);
                }
            } else if let Some(parent) = self.scopes.last_mut() {
                // Promote references to parent scope
                let parent_references = parent.references.entry(name).or_default();
                parent_references.extend(references);
            } else {
                // We're in the global scope and found no declaration
                for reference in references {
                    self.stash.push_back(SemanticEvent::UnresolvedReference {
                        is_read: !reference.is_write(),
                        range: reference.range(),
                    });
                }
            }
        }

        // Remove bindings from this scope
        for binding in scope.bindings {
            self.bindings.remove(&binding);
        }

        // Restore shadowed bindings
        self.bindings.extend(scope.shadowed);

        self.stash
            .push_back(SemanticEvent::ScopeEnded { range: scope_range });
    }

    fn push_binding(&mut self, binding_name: BindingName, binding_info: BindingInfo) {
        // First get the existing binding if any
        let existing_binding = self.bindings.get(&binding_name).cloned();

        // Insert the new binding
        self.bindings.insert(binding_name.clone(), binding_info);

        // Get the current scope and update it
        let scope = self.current_scope_mut();
        if let Some(shadowed) = existing_binding {
            scope.shadowed.push((binding_name.clone(), shadowed));
        }
        scope.bindings.push(binding_name);
    }

    fn push_reference(&mut self, binding_name: BindingName, reference: Reference) {
        self.current_scope_mut()
            .references
            .entry(binding_name)
            .or_default()
            .push(reference);
    }

    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().unwrap()
    }
}

/// Iterator for extracting [SemanticEvent] from [RSyntaxNode].
struct SemanticEventIterator {
    iter: Preorder<RLanguage>,
    extractor: SemanticEventExtractor,
}

impl Iterator for SemanticEventIterator {
    type Item = SemanticEvent;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(e) = self.extractor.pop() {
                break Some(e);
            } else {
                use biome_rowan::WalkEvent;
                match self.iter.next() {
                    Some(WalkEvent::Enter(node)) => {
                        self.extractor.enter(&node);
                    }
                    Some(WalkEvent::Leave(node)) => {
                        self.extractor.leave(&node);
                    }
                    None => {
                        if let Some(e) = self.extractor.pop() {
                            break Some(e);
                        } else {
                            break None;
                        }
                    }
                }
            }
        }
    }
}

/// Creates an iterator that extracts [SemanticEvent] from an [RSyntaxNode].
pub fn semantic_events(root: RSyntaxNode) -> impl IntoIterator<Item = SemanticEvent> {
    SemanticEventIterator {
        iter: root.preorder(),
        extractor: SemanticEventExtractor::default(),
    }
}
