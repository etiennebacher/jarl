use air_r_syntax::TextRange;
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt;

// ---------------------------------------------------------------------------
// NodeId – unique identifier for a vertex in the dataflow graph
// ---------------------------------------------------------------------------

/// Unique, sequential identifier assigned to each AST node that participates
/// in the dataflow graph.  Mirrors the concept from flowR where every
/// normalized-AST node receives a deterministic id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u32);

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "n{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Edge types  (bitmask, following flowR)
// ---------------------------------------------------------------------------

/// Relationship between a source and a target vertex.
///
/// Stored as a bitmask so that a single edge record can carry multiple
/// relationship types at once (e.g. `Reads | DefinedBy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum EdgeType {
    /// Source *reads* target (`x` reads the definition of `x`).
    Reads = 1,
    /// Source is *defined by* target (`x <- expr` — `x` is defined by `expr`).
    DefinedBy = 2,
    /// Source *calls* target (a call vertex calls a function definition).
    Calls = 4,
    /// Source *returns* target (a function definition returns a value).
    Returns = 8,
    /// Source (argument) defines target (parameter) at a call site.
    DefinesOnCall = 16,
    /// Source (parameter) is defined by target (argument) at a call site.
    DefinedByOnCall = 32,
    /// Formal argument edge.
    Argument = 64,
    /// Side-effect caused by a call.
    SideEffectOnCall = 128,
    /// Non-standard evaluation context (e.g. inside `quote()`).
    NonStandardEvaluation = 256,
}

/// A packed set of [`EdgeType`] bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeTypeBits(pub u16);

impl EdgeTypeBits {
    pub fn new(t: EdgeType) -> Self {
        Self(t as u16)
    }

    pub fn contains(self, t: EdgeType) -> bool {
        self.0 & (t as u16) != 0
    }

    pub fn insert(&mut self, t: EdgeType) {
        self.0 |= t as u16;
    }
}

impl From<EdgeType> for EdgeTypeBits {
    fn from(t: EdgeType) -> Self {
        Self::new(t)
    }
}

impl std::ops::BitOr<EdgeType> for EdgeTypeBits {
    type Output = Self;
    fn bitor(self, rhs: EdgeType) -> Self {
        Self(self.0 | rhs as u16)
    }
}

impl fmt::Display for EdgeTypeBits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: &[(EdgeType, &str)] = &[
            (EdgeType::Reads, "reads"),
            (EdgeType::DefinedBy, "defined-by"),
            (EdgeType::Calls, "calls"),
            (EdgeType::Returns, "returns"),
            (EdgeType::DefinesOnCall, "defines-on-call"),
            (EdgeType::DefinedByOnCall, "defined-by-on-call"),
            (EdgeType::Argument, "argument"),
            (EdgeType::SideEffectOnCall, "side-effect-on-call"),
            (EdgeType::NonStandardEvaluation, "nse"),
        ];
        let mut first = true;
        for &(bit, name) in names {
            if self.contains(bit) {
                if !first {
                    write!(f, "|")?;
                }
                write!(f, "{name}")?;
                first = false;
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Vertices
// ---------------------------------------------------------------------------

/// Discriminant for the kind of dataflow vertex.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexKind {
    /// A constant value (number, string, logical, NULL, NA, …).
    Value,
    /// A *use* of a symbol (reading a variable).
    Use,
    /// A variable definition / assignment target.
    Definition,
    /// A function call (including built-in control-flow treated as calls in R).
    FunctionCall,
    /// A function definition (carries a nested sub-graph for its body).
    FunctionDef,
    /// A parameter in the function definition.
    FunctionParam,
}

/// A vertex in the dataflow graph.
#[derive(Debug, Clone)]
pub struct DfVertex {
    pub id: NodeId,
    pub kind: VertexKind,
    /// The source text range this vertex corresponds to.
    pub range: TextRange,
    /// Human-readable name / lexeme (e.g. variable name, function name,
    /// literal text).
    pub name: String,
    /// Control dependencies: which condition node-ids guard this vertex.
    pub cds: Vec<ControlDependency>,
    /// Extra data depending on `kind`.
    pub data: VertexData,
}

/// Kind-specific payload stored in a vertex.
#[derive(Debug, Clone, Default)]
pub enum VertexData {
    #[default]
    None,
    /// For `FunctionCall`: ordered arguments (each is the NodeId of the arg
    /// expression, plus an optional name).
    Call { args: Vec<CallArgument> },
    /// For `FunctionDef`: the set of parameter NodeIds and the set of body
    /// NodeIds that belong to the nested sub-graph.
    FunctionDef {
        params: Vec<NodeId>,
        unresolved: Vec<(String, NodeId)>,
        body_nodes: Vec<NodeId>,
        exit_points: Vec<NodeId>,
    },
}

/// An argument passed to a function call.
#[derive(Debug, Clone)]
pub struct CallArgument {
    /// The NodeId of the argument expression.
    pub node_id: NodeId,
    /// If the argument is named (`f(x = 1)`), the name.
    pub name: Option<String>,
}

/// A control dependency: the vertex is only executed when the condition
/// identified by `id` evaluates to `when`.
#[derive(Debug, Clone)]
pub struct ControlDependency {
    /// The NodeId of the controlling condition.
    pub id: NodeId,
    /// `Some(true)` = then-branch, `Some(false)` = else-branch,
    /// `None` = unknown / always.
    pub when: Option<bool>,
    /// Whether this CD was created by a loop iteration (as opposed to
    /// a conditional branch).
    pub by_iteration: bool,
}

// ---------------------------------------------------------------------------
// DataflowGraph
// ---------------------------------------------------------------------------

/// A directed graph tracking data dependencies in an R program.
///
/// Vertices represent values, variable uses, definitions, function calls and
/// function definitions.  Edges encode how data flows between them.
#[derive(Clone)]
pub struct DataflowGraph {
    /// All vertices keyed by their [`NodeId`].
    vertices: FxHashMap<NodeId, DfVertex>,
    /// Adjacency list: `edges[source][target] = edge_bits`.
    edges: FxHashMap<NodeId, FxHashMap<NodeId, EdgeTypeBits>>,
    /// Counter for generating fresh [`NodeId`]s.
    next_id: u32,
    /// Definition vertices created by super-assignment (`<<-` / `->>`).
    super_assign_defs: FxHashSet<NodeId>,
}

impl DataflowGraph {
    pub fn new() -> Self {
        Self {
            vertices: FxHashMap::default(),
            edges: FxHashMap::default(),
            next_id: 0,
            super_assign_defs: FxHashSet::default(),
        }
    }

    /// Allocate the next fresh [`NodeId`].
    pub fn fresh_id(&mut self) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Insert a vertex into the graph, returning its id.
    pub fn add_vertex(&mut self, vertex: DfVertex) -> NodeId {
        let id = vertex.id;
        self.vertices.insert(id, vertex);
        id
    }

    /// Add an edge (or merge bits into an existing edge).
    pub fn add_edge(&mut self, from: NodeId, to: NodeId, ty: EdgeType) {
        let entry = self
            .edges
            .entry(from)
            .or_default()
            .entry(to)
            .or_insert(EdgeTypeBits(0));
        entry.insert(ty);
    }

    pub fn vertex(&self, id: NodeId) -> Option<&DfVertex> {
        self.vertices.get(&id)
    }

    pub fn vertices(&self) -> impl Iterator<Item = &DfVertex> {
        self.vertices.values()
    }

    pub fn edges_from(&self, id: NodeId) -> impl Iterator<Item = (NodeId, EdgeTypeBits)> + '_ {
        self.edges
            .get(&id)
            .into_iter()
            .flat_map(|m| m.iter().map(|(&to, &bits)| (to, bits)))
    }

    /// Iterate over all edges pointing *to* a given vertex.
    pub fn edges_to(&self, target: NodeId) -> impl Iterator<Item = (NodeId, EdgeTypeBits)> + '_ {
        self.edges.iter().flat_map(move |(&from, targets)| {
            targets.get(&target).map(|&bits| (from, bits)).into_iter()
        })
    }

    /// Mark a definition as created by super-assignment (`<<-` / `->>`).
    pub fn mark_super_assign(&mut self, id: NodeId) {
        self.super_assign_defs.insert(id);
    }

    /// Check if a definition was created by super-assignment.
    pub fn is_super_assign(&self, id: NodeId) -> bool {
        self.super_assign_defs.contains(&id)
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.values().map(|m| m.len()).sum()
    }
}

impl Default for DataflowGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DataflowGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ids: Vec<_> = self.vertices.keys().copied().collect();
        ids.sort();
        let mut vertices = f.debug_map();
        for id in &ids {
            vertices.entry(id, &self.vertices[id]);
        }
        vertices.finish()?;

        let mut edges_list: Vec<_> = self
            .edges
            .iter()
            .flat_map(|(&from, targets)| targets.iter().map(move |(&to, &bits)| (from, to, bits)))
            .collect();
        edges_list.sort_by_key(|(from, to, _)| (*from, *to));

        write!(f, " edges: [")?;
        for (i, (from, to, bits)) in edges_list.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{from} --{bits}--> {to}")?;
        }
        write!(f, "]")
    }
}

impl fmt::Display for DataflowGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "DataflowGraph ({} vertices, {} edges):",
            self.vertex_count(),
            self.edge_count()
        )?;
        let mut ids: Vec<_> = self.vertices.keys().copied().collect();
        ids.sort();
        for id in &ids {
            let v = &self.vertices[id];
            writeln!(f, "  {id} [{:?}] {:?} \"{}\"", v.kind, v.range, v.name)?;
        }
        for id in &ids {
            if let Some(targets) = self.edges.get(id) {
                for (to, bits) in targets {
                    writeln!(f, "  {id} --{bits}--> {to}")?;
                }
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Exit points
// ---------------------------------------------------------------------------

/// Classifies the type of exit point encountered in a sub-tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitPointType {
    /// Implicit exit: the last expression in a block.
    Default,
    /// Explicit `return()` call.
    Return,
    /// Explicit `break` in a loop.
    Break,
    /// Explicit `next` in a loop.
    Next,
}

/// An exit point describes a position that ends the current control-flow
/// structure.  Tracks the node that causes it, its type, and any control
/// dependencies that guard whether it actually fires.
#[derive(Debug, Clone)]
pub struct ExitPoint {
    pub node_id: NodeId,
    pub type_: ExitPointType,
    pub cds: Vec<ControlDependency>,
}

// ---------------------------------------------------------------------------
// Environment – lexical scope chain for name resolution
// ---------------------------------------------------------------------------

/// An identifier in R, optionally qualified with a namespace (`pkg::foo`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub name: String,
    pub namespace: Option<String>,
}

impl Identifier {
    pub fn simple(name: impl Into<String>) -> Self {
        Self { name: name.into(), namespace: None }
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ns) = &self.namespace {
            write!(f, "{ns}::{}", self.name)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

/// A single definition of an identifier: the NodeId where it was defined plus
/// optional control-dependency information.
#[derive(Debug, Clone)]
pub struct IdentifierDef {
    pub node_id: NodeId,
    pub cds: Vec<ControlDependency>,
}

/// A lexical environment (scope).
///
/// Environments form a linked chain via `parent`.  Name resolution walks up
/// the chain until it finds a binding (or reaches the global scope).
#[derive(Debug, Clone)]
pub struct Environment {
    /// Bindings visible in this scope.
    pub bindings: FxHashMap<String, Vec<IdentifierDef>>,
    /// Parent scope (None for the global / top-level scope).
    pub parent: Option<Box<Environment>>,
}

impl Environment {
    pub fn new_global() -> Self {
        Self { bindings: FxHashMap::default(), parent: None }
    }

    pub fn new_child(parent: Environment) -> Self {
        Self {
            bindings: FxHashMap::default(),
            parent: Some(Box::new(parent)),
        }
    }

    /// Define (or shadow) a name in the current scope.
    ///
    /// This **replaces** any existing definitions for the same name in this
    /// scope.  Multiple definitions only arise from branch merging (via
    /// `merge_envs`), never from sequential assignments.
    pub fn define(&mut self, name: &str, def: IdentifierDef) {
        self.bindings.insert(name.to_string(), vec![def]);
    }

    /// Resolve a name by walking up the scope chain.
    /// Returns all definitions visible under this name (the most recent
    /// definition in the innermost scope first).
    pub fn resolve(&self, name: &str) -> Option<&[IdentifierDef]> {
        if let Some(defs) = self.bindings.get(name)
            && !defs.is_empty()
        {
            return Some(defs);
        }
        self.parent.as_ref().and_then(|p| p.resolve(name))
    }

    /// Remove a name from the current scope (used by `rm()`).
    pub fn remove(&mut self, name: &str) {
        self.bindings.remove(name);
    }

    /// Super-assignment (`<<-`): walk up the parent chain to find the scope
    /// that already binds `name` and add the definition there.  If no parent
    /// scope contains the name, define in the outermost (global) scope.
    pub fn define_super(&mut self, name: &str, def: IdentifierDef) {
        if let Some(parent) = &mut self.parent {
            parent.define_super_inner(name, def);
        } else {
            // Already at the global scope — fall back to a normal define.
            self.define(name, def);
        }
    }

    /// Recursive helper: defines in the first ancestor that already has
    /// a binding for `name`, or in the outermost scope if none does.
    fn define_super_inner(&mut self, name: &str, def: IdentifierDef) {
        if self.bindings.contains_key(name) {
            self.bindings.entry(name.to_string()).or_default().push(def);
            return;
        }
        if let Some(parent) = &mut self.parent {
            parent.define_super_inner(name, def);
        } else {
            // Outermost scope — define here.
            self.bindings.entry(name.to_string()).or_default().push(def);
        }
    }
}
