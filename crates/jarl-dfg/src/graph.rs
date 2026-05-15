use air_r_syntax::TextRange;
use rustc_hash::FxHashMap;
use std::fmt;

#[allow(clippy::empty_line_after_doc_comments)]
/// This is a simplified explanation of how the DFG is built and can be used
/// in various rules. For a more detailed explanation, see "Statically Analyzing
/// the Dataflow of R Programs" by Sihler & Tichy, 2025
/// https://dl.acm.org/doi/10.1145/3763087
///
///
/// The main idea is to translate the AST into a *directed* graph where:
///
/// * vertices are symbols. They can be of 5 different types:
///   - value (a constant number, `NA`, etc.)
///   - use (object is read)
///   - definition (object that is a target of an assignment)
///   - function call
///   - function definition (almost the same as a definition, but we'll explain
///     the differences later).
///
/// * edges are the connections between two vertices and can also be of various
///   types:
///   - reads (the source reads the target). E.g. in `x <- 1; x`, the source
///     (second `x`, a Use vertex) reads the target (first `x`, a Definition
///     vertex).
///   - defined_by (the source is defined by the target). E.g. in `x <- 1`, the
///     source `x` is defined by the target `1`.
///   - calls (the source calls the target). E.g. in `f <- function() 1; f(x)`,
///     the source call vertex (second `f`) calls the target function definition
///     vertex (first `f`).
///   - returns (the source returns the target). E.g. in `f <- function() { 1 }`,
///     the source function-definition vertex for `f` returns the target value
///     vertex `1`.
///   - defines_on_call (the source argument defines the target parameter at call
///     site). E.g. in `f <- function(x = mean(y))`, the source `mean(y)` defines
///     the target `x` only when `f` is called.
///   - defined_by_on_call (the source parameter is defined by the target argument
///     at call site). E.g. in `f <- function(x = mean(y))`, the source `x` is
///     defined by the target `mean(y)` on call.
///   - argument (the source has the target as an argument). E.g. in `f(x)`, the
///     source call vertex `f` has the target use vertex `x` as an argument.
///   - side_effect_on_call (the source call has a side effect on the target). E.g.
///     in `f <- function(x) { x <<- 1 }`, the source call `f()` has a side effect
///     that modifies the target `x` in the parent environment.
///   - non_standard_evaluation (the source is inside a function that is known to
///     do NSE, e.g. `quote()`).
///
/// This graph also keeps track of the *environment* and of the *control dependencies*.
///
///
/// ## Environment
///
/// The entire graph is part of the global environment, but we can also divide
/// this graph is subgraphs where each subgraph has its own additional environment.
/// For example, with the following code:
///
/// ```r
/// x <- 2
/// add <- function(a, b) {
///   a + b
/// }
/// y <- x + a
/// ```
///
/// the body of the function definition has access to `x`, `a`, and `b`, but the
/// line `y <- x + a` doesn't have access to `a` and `b`. Therefore, when we build
/// the graph, we should keep the information that the vertex `a` in the function
/// body cannot be connected to the vertex `y` because we popped the environment
/// of the function body when exiting it.
///
///
/// ## Control dependencies
///
/// Control dependencies (CDs) are a way to keep the information that a vertex *might*
/// be connected depending on some condition. This is particularly useful to keep
/// the information in `if` conditions, such as:
///
/// ```r
/// x <- 0
/// if (y > 2) {
///   x <- 1
/// } else if (z > 3) {
///   x <- 2
/// } else {
///   x <- 3
/// }
/// ```
///
/// Here, we cannot know statically which condition is going to pass and therefore
/// which `x` will be used. Therefore, we attach a vector of CDs to each of
/// these `x` vertices. A CD contains the node ID of the condition and the value
/// `true` or `false` that tells us what this condition must match. For example,
/// we store `x <- 1` with `[CD(y > 2, true)]`. For `x <- 2`, we rewrite the
/// `else if` branch as a nested `if` so we store `x <- 2` with
/// `[CD(y > 2, false), CD(z > 3, true)]`. Finally, we store `x <- 3` with
/// `[CD(y > 2, false), CD(z > 3, false)]`.
///
/// CDs are also used in for loops and while loops. An object might be redefined
/// in a for loop but we still need to put it in a conditional case since it
/// might happen that the loop goes through zero iteration and therefore the
/// redefine never happens. For example:
///
/// ```r
/// x <- 1
/// for (i in foo(y)) {
///   x <- 5
/// }
/// print(x)
/// ```
/// if `foo(y)` returns an empty vector, `print(x)` will print 1, while if the
/// for loop has some iterations then it will print 5. We cannot guarantee
/// which "Definition" vertex will be used, so we attach a CD with `by_iteration: true`
/// to `x <- 5`.
///
///
/// See the docs in build.rs to know more about how the AST is converted to a
/// graph.

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
///
/// Storage is Vec-based (indexed by [`NodeId`]) rather than HashMap-based,
/// since node IDs are sequential integers starting from 0.
#[derive(Clone)]
pub struct DataflowGraph {
    /// All vertices indexed by their [`NodeId`].
    vertices: Vec<Option<DfVertex>>,
    /// Adjacency list indexed by source [`NodeId`]: `edges[source][target] = edge_bits`.
    edges: Vec<FxHashMap<NodeId, EdgeTypeBits>>,
    /// Reverse adjacency list indexed by target [`NodeId`]: `reverse_edges[target][source] = edge_bits`.
    /// This allows faster access to information on whether a node is a target.
    /// For instance, if we want to know whether a vertex is read (i.e. whether
    /// it is the target of a Reads edge), we can simply do reverse_edges[vertex]
    /// instead of going through all other vertices and see which ones point to
    /// this vertex.
    reverse_edges: Vec<FxHashMap<NodeId, EdgeTypeBits>>,
    /// Counter for generating fresh [`NodeId`]s.
    next_id: u32,
    /// Per-node flag: whether the definition was created by super-assignment (`<<-` / `->>`).
    super_assign_defs: Vec<bool>,
}

impl DataflowGraph {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            edges: Vec::new(),
            reverse_edges: Vec::new(),
            next_id: 0,
            super_assign_defs: Vec::new(),
        }
    }

    /// Allocate the next fresh [`NodeId`].
    pub fn fresh_id(&mut self) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        // Pre-grow all vecs so the slot exists when add_vertex / add_edge
        // is called later.
        let idx = id.0 as usize + 1;
        if self.vertices.len() < idx {
            self.vertices.resize_with(idx, || None);
            self.edges.resize_with(idx, FxHashMap::default);
            self.reverse_edges.resize_with(idx, FxHashMap::default);
            self.super_assign_defs.resize(idx, false);
        }
        id
    }

    /// Insert a vertex into the graph, returning its id.
    pub fn add_vertex(&mut self, vertex: DfVertex) -> NodeId {
        let id = vertex.id;
        let idx = id.0 as usize;
        if idx >= self.vertices.len() {
            let new_len = idx + 1;
            self.vertices.resize_with(new_len, || None);
            self.edges.resize_with(new_len, FxHashMap::default);
            self.reverse_edges.resize_with(new_len, FxHashMap::default);
            self.super_assign_defs.resize(new_len, false);
        }
        self.vertices[idx] = Some(vertex);
        id
    }

    /// Add an edge (or merge bits into an existing edge).
    pub fn add_edge(&mut self, from: NodeId, to: NodeId, ty: EdgeType) {
        let fi = from.0 as usize;
        let ti = to.0 as usize;
        let needed = fi.max(ti) + 1;
        if self.edges.len() < needed {
            self.edges.resize_with(needed, FxHashMap::default);
            self.reverse_edges.resize_with(needed, FxHashMap::default);
        }
        self.edges[fi]
            .entry(to)
            .or_insert(EdgeTypeBits(0))
            .insert(ty);
        self.reverse_edges[ti]
            .entry(from)
            .or_insert(EdgeTypeBits(0))
            .insert(ty);
    }

    pub fn vertex(&self, id: NodeId) -> Option<&DfVertex> {
        self.vertices.get(id.0 as usize).and_then(|v| v.as_ref())
    }

    pub fn vertices(&self) -> impl Iterator<Item = &DfVertex> {
        self.vertices.iter().filter_map(|v| v.as_ref())
    }

    pub fn edges_from(&self, id: NodeId) -> impl Iterator<Item = (NodeId, EdgeTypeBits)> + '_ {
        self.edges
            .get(id.0 as usize)
            .into_iter()
            .flat_map(|m| m.iter().map(|(&to, &bits)| (to, bits)))
    }

    /// Iterate over all edges pointing *to* a given vertex.
    pub fn edges_to(&self, target: NodeId) -> impl Iterator<Item = (NodeId, EdgeTypeBits)> + '_ {
        self.reverse_edges
            .get(target.0 as usize)
            .into_iter()
            .flat_map(|m| m.iter().map(|(&from, &bits)| (from, bits)))
    }

    /// Mark a definition as created by super-assignment (`<<-` / `->>`).
    pub fn mark_super_assign(&mut self, id: NodeId) {
        let idx = id.0 as usize;
        if idx >= self.super_assign_defs.len() {
            self.super_assign_defs.resize(idx + 1, false);
        }
        self.super_assign_defs[idx] = true;
    }

    /// Check if a definition was created by super-assignment.
    pub fn is_super_assign(&self, id: NodeId) -> bool {
        self.super_assign_defs
            .get(id.0 as usize)
            .copied()
            .unwrap_or(false)
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.iter().filter(|v| v.is_some()).count()
    }

    pub fn next_id(&self) -> u32 {
        self.next_id
    }

    pub fn edge_count(&self) -> usize {
        self.edges.iter().map(|m| m.len()).sum()
    }
}

impl Default for DataflowGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DataflowGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut vertices = f.debug_map();
        for (i, slot) in self.vertices.iter().enumerate() {
            if let Some(v) = slot {
                vertices.entry(&NodeId(i as u32), v);
            }
        }
        vertices.finish()?;

        let mut edges_list: Vec<_> = self
            .edges
            .iter()
            .enumerate()
            .flat_map(|(i, targets)| {
                let from = NodeId(i as u32);
                targets.iter().map(move |(&to, &bits)| (from, to, bits))
            })
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
        for (i, slot) in self.vertices.iter().enumerate() {
            if let Some(v) = slot {
                let id = NodeId(i as u32);
                writeln!(f, "  {id} [{:?}] {:?} \"{}\"", v.kind, v.range, v.name)?;
            }
        }
        for (i, targets) in self.edges.iter().enumerate() {
            let id = NodeId(i as u32);
            for (to, bits) in targets {
                writeln!(f, "  {id} --{bits}--> {to}")?;
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
