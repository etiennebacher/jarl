use super::graph::*;
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

#[allow(clippy::empty_line_after_doc_comments)]
/// This details the process to convert the AST into a DFG. See the docs in
/// graph.rs for more info on the general concepts in the DFG.
///
/// We have the full AST as input and process it recursively node by node using
/// a lot of `process_*()` functions, such as `process_function_def()` or
/// `process_binary()`. In each of these steps, we add vertices in the graph.
/// These vertices contain a unique ID, a kind, a range, control dependencies,
/// and sometimes extra data.
///
/// Each of those functions returns a `DataflowInformation`. This contains a node
/// ID, identifiers that were defined, unknown references, and a few more information.
/// To understand that, it is useful to think about it in terms of subtrees.
///
/// Suppose that we have this code:
///
/// ```r
/// x <- 1
/// x
/// ```
///
/// Before we start processing this, we have an empty environment and an empty
/// graph.
///
/// We process `x <- 1`, which is our first subtree. This adds two
/// vertices in the graph (a Definition vertex and a Value vertex) and one edge
/// (DefinedBy, going from `x` to `1`). This also returns a DataflowInformation
/// that tells that we have a write "x" corresponding to a particular node ID.
/// We update the environment with this information and move on to process the
/// second expression, `x`.
///
/// On its own, processing the second expression adds a Use vertex with an
/// unknown reference because we don't know where `x` comes from (based on
/// this single subtree). It also returns a DataflowInformation saying that we
/// have an unknown reference named "x". After this second subtree is processed,
/// we can resolve unknown references against the environment. `x` is now present
/// there thanks to the first step, so there's no unknown references anymore.
/// Now that we know all references, we can add a "Reads" edge from the Use
/// vertex `x` to the Definition vertex `x`.
///
///
/// ## Environment
///
/// Most of these processing steps populate the same (global) environment.
/// However, we need to use another environment when we process function arguments
/// and function body.
///
/// We don't want references defined in those environments to be available in the
/// global environment, so when we enter the argument and the body nodes, we
/// create a child environment and we exit it when we leave those nodes.
///
///
/// ## Forward references
///
/// There are some cases where references cannot be known after we process all
/// the nodes, such as this case:
///
/// ```r
/// f <- function() x
/// x <- 1
/// f()
/// ```
///
/// In `process_function_def()`, we have an unknown reference `x` (because `x <- 1`
/// hasn't been processed yet) and we leave the environment that we made to
/// process the function body.
///
/// The solution is to do a final pass after the builder is complete to resolve
/// all unknown references against the global environment. At this stage, we'll
/// have our Definition vertex `x`, so the unknown reference `x` in the function
/// body will link to it and we can add a Reads edge.
///
///
/// ## Control dependencies
///
/// The builder (which contains the graph we're building) also contains information
/// on CDs. When we process a new node, the current CDs are attached to all new
/// vertices. The builder gets new CDs when we enter a branch and loses
/// them when it exits this branch. Therefore, all nodes present in this branch
/// get the same CDs.
///
/// Importantly, in an if/else, both branches use the environment that exists
/// *before* entering the if condition and each branch gets its own environment.
/// After all branches are analyzed, we merge both environments. This is important
/// because it guarantees that the environment in a branch doesn't affect the
/// processing of other branches, and it also guarantees that the environment
/// after the if/else gets all the information and doesn't skip info from a
/// specific branch.
///
///
/// ## Exit points
///
/// Exit points contain information about which expression a function may return
/// Usually, it's the last expression, but a function body can return early with
/// a `return()` call. This is useful to add more Returns edges from a FunctionDef
/// vertex to the exit point vertices.
///
/// There are also `break` and `next` in loops. Expressions after an
/// unconditional break, next, or return are dead code and are not processed.

/// Builds a [`DataflowGraph`] from an R syntax tree.
///
/// The builder walks the AST recursively (top-down), threading an
/// [`Environment`] for name resolution and accumulating vertices and edges
/// in the graph.
#[derive(Debug)]
pub struct DfgBuilder {
    graph: DataflowGraph,
    env: Environment,
    /// Stack of active control-dependency guards.
    active_cds: Vec<ControlDependency>,
}

/// The information returned by processing a single AST sub-tree.
///
/// Mirrors flowR's `DataflowInformation`: it carries the references produced
/// by the sub-tree so the caller can wire them up.
#[derive(Debug, Clone)]
pub struct DataflowInformation {
    /// The NodeId of the root vertex created for this sub-tree.
    pub node_id: NodeId,
    /// Identifiers not yet classified as read or write (flowr: `unknownReferences`).
    /// Resolution happens in the expression-list processor.
    pub unknown_refs: Vec<(String, NodeId)>,
    /// Identifiers resolved as reads (flowr: `in`).
    /// Currently always empty — resolution happens in expression lists
    /// and edges are added directly to the graph.
    #[allow(dead_code)]
    pub reads: Vec<(String, NodeId)>,
    /// Identifiers written (flowr: `out`).
    pub writes: Vec<(String, NodeId)>,
    /// Exit points from this sub-tree.
    pub exit_points: Vec<ExitPoint>,
    /// Unresolved references from `on.exit()` bodies. Resolved in a deferred
    /// pass at the end of `process_function_def` because the body might refer
    /// to objects defined after the `on.exit()` call.
    pub on_exit_unresolved: Vec<(String, NodeId)>,
}

impl DfgBuilder {
    fn new() -> Self {
        Self {
            graph: DataflowGraph::new(),
            env: Environment::new_global(),
            active_cds: Vec::new(),
        }
    }

    /// Build a dataflow graph for the entire R file (the root node).
    pub fn build_from_root(root: &RSyntaxNode) -> DataflowGraph {
        let mut builder = Self::new();
        builder.process_node(root);

        // Collect unresolved closure references from all function definitions,
        // then resolve them against the final top-level environment.
        // This handles forward references like:
        // ```
        // f <- function() x
        // x <- 1
        // f()
        // ```
        let unresolved: Vec<(String, NodeId)> = builder
            .graph
            .vertices()
            .filter(|v| v.kind == VertexKind::FunctionDef)
            .filter_map(|v| match &v.data {
                VertexData::FunctionDef { unresolved, .. } => Some(unresolved.clone()),
                _ => None,
            })
            .flatten()
            .collect();

        for (name, ref_id) in &unresolved {
            if let Some(defs) = builder.env.resolve(name) {
                for def in defs {
                    builder
                        .graph
                        .add_edge(*ref_id, def.node_id, EdgeType::Reads);
                }
            }
        }
        builder.graph
    }

    // ------------------------------------------------------------------
    // Dispatch
    // ------------------------------------------------------------------

    /// Process any R syntax node and return sub-tree info.
    fn process_node(&mut self, node: &RSyntaxNode) -> Option<DataflowInformation> {
        match node.kind() {
            // Literals / values
            RSyntaxKind::R_DOUBLE_VALUE
            | RSyntaxKind::R_INTEGER_VALUE
            | RSyntaxKind::R_COMPLEX_VALUE
            | RSyntaxKind::R_STRING_VALUE => Some(self.process_value(node)),

            RSyntaxKind::R_NA_EXPRESSION => Some(self.process_value(node)),

            // Identifiers (variable references or assignment targets)
            RSyntaxKind::R_IDENTIFIER => Some(self.process_identifier(node)),

            // Assignment / binary expressions
            RSyntaxKind::R_BINARY_EXPRESSION => self.process_binary(node),

            // Function definitions
            RSyntaxKind::R_FUNCTION_DEFINITION => Some(self.process_function_def(node)),

            // Function calls
            RSyntaxKind::R_CALL => Some(self.process_call(node)),

            // Control flow: if
            RSyntaxKind::R_IF_STATEMENT => self.process_if(node),

            // Control flow: for
            RSyntaxKind::R_FOR_STATEMENT => self.process_for(node),

            // Control flow: while
            RSyntaxKind::R_WHILE_STATEMENT => self.process_while(node),

            // Control flow: repeat
            RSyntaxKind::R_REPEAT_STATEMENT => self.process_repeat(node),

            // Braced expressions – process children sequentially
            RSyntaxKind::R_BRACED_EXPRESSIONS => self.process_expression_list(node),

            // Parenthesized expression – transparent wrapper
            RSyntaxKind::R_PARENTHESIZED_EXPRESSION => self.process_children_last(node),

            // Unary expression
            RSyntaxKind::R_UNARY_EXPRESSION => self.process_children_last(node),

            // Return / break / next
            RSyntaxKind::R_RETURN_EXPRESSION
            | RSyntaxKind::R_BREAK_EXPRESSION
            | RSyntaxKind::R_NEXT_EXPRESSION => self.process_children_last(node),

            // Expression list (top-level file)
            RSyntaxKind::R_EXPRESSION_LIST => self.process_expression_list(node),

            // Root
            RSyntaxKind::R_ROOT => self.process_children_last(node),

            // Namespace expression  pkg::foo
            RSyntaxKind::R_NAMESPACE_EXPRESSION => self.process_namespace(node),

            // Extract expression  x$y  x@y
            RSyntaxKind::R_EXTRACT_EXPRESSION => self.process_children_last(node),

            // Subset  x[i]  x[[i]]  x[, ..cols]
            RSyntaxKind::R_SUBSET => self.process_subset(node),
            RSyntaxKind::R_SUBSET2 => self.process_children_last(node),

            // Fallback: walk children
            _ => self.process_children_last(node),
        }
    }

    // ------------------------------------------------------------------
    // Leaf: value literal
    // ------------------------------------------------------------------

    fn process_value(&mut self, node: &RSyntaxNode) -> DataflowInformation {
        let id = self.graph.fresh_id();
        let text = node_text(node);
        self.graph.add_vertex(DfVertex {
            id,
            kind: VertexKind::Value,
            range: node.text_trimmed_range(),
            name: text,
            cds: self.active_cds.clone(),
            data: VertexData::None,
        });
        DataflowInformation {
            node_id: id,
            unknown_refs: vec![],
            reads: vec![],
            writes: vec![],
            exit_points: vec![ExitPoint {
                node_id: id,
                type_: ExitPointType::Default,
                cds: self.active_cds.clone(),
            }],
            on_exit_unresolved: vec![],
        }
    }

    // ------------------------------------------------------------------
    // Leaf: identifier (use)
    // ------------------------------------------------------------------

    fn process_identifier(&mut self, node: &RSyntaxNode) -> DataflowInformation {
        let name = identifier_name(node);
        let id = self.graph.fresh_id();

        // Create a Use vertex.  Name resolution is deferred — the
        // expression-list processor will add Reads edges later.
        self.graph.add_vertex(DfVertex {
            id,
            kind: VertexKind::Use,
            range: node.text_trimmed_range(),
            name: name.clone(),
            cds: self.active_cds.clone(),
            data: VertexData::None,
        });

        DataflowInformation {
            node_id: id,
            unknown_refs: vec![(name, id)],
            reads: vec![],
            writes: vec![],
            exit_points: vec![ExitPoint {
                node_id: id,
                type_: ExitPointType::Default,
                cds: self.active_cds.clone(),
            }],
            on_exit_unresolved: vec![],
        }
    }

    // ------------------------------------------------------------------
    // Binary expression (assignment or arithmetic)
    // ------------------------------------------------------------------

    fn process_binary(&mut self, node: &RSyntaxNode) -> Option<DataflowInformation> {
        let bin = RBinaryExpression::cast_ref(node)?;
        let fields = bin.as_fields();
        let op = fields.operator.ok()?;
        let op_kind = op.kind();

        // Left-assign: `x <- expr` or `x = expr`
        // `=` inside a formula (`~`) is not an assignment (e.g. `a ~ (b = 1)`).
        if op_kind == RSyntaxKind::ASSIGN
            || (op_kind == RSyntaxKind::EQUAL && !is_inside_formula(node))
        {
            return self.process_left_assignment(node, false);
        }
        // Right-assign: `expr -> x`
        if op_kind == RSyntaxKind::ASSIGN_RIGHT {
            return self.process_right_assignment(node, false);
        }
        // Super-assign: `x <<- expr`
        if op_kind == RSyntaxKind::SUPER_ASSIGN {
            return self.process_left_assignment(node, true);
        }
        // Super right-assign: `expr ->> x`
        if op_kind == RSyntaxKind::SUPER_ASSIGN_RIGHT {
            return self.process_right_assignment(node, true);
        }
        // Pipe: `x |> f()`
        if op_kind == RSyntaxKind::PIPE {
            return self.process_pipe(node);
        }
        // Handle `%<>%` (has a read and a write edge)
        if op_kind == RSyntaxKind::SPECIAL && op.text_trimmed() == "%<>%" {
            return self.process_pipe_assignment(node);
        }

        // Arithmetic / logical / comparison — treat as a call to the operator
        let id = self.graph.fresh_id();
        let op_text = op.text_trimmed().to_string();
        let mut unknown_refs = Vec::new();
        let mut exit_points = Vec::new();
        let mut on_exit_unresolved = Vec::new();

        // Short-circuit: for && / ||, RHS gets a control dependency
        let is_short_circuit = op_text == "&&" || op_text == "||";

        let left = fields.left.ok().and_then(|l| self.process_node(l.syntax()));

        if is_short_circuit && let Some(li) = &left {
            // && → RHS executes when LHS is true
            // || → RHS executes when LHS is false
            let when = op_text == "&&";
            self.active_cds.push(ControlDependency {
                id: li.node_id,
                when: Some(when),
                by_iteration: false,
            });
        }

        let right = fields
            .right
            .ok()
            .and_then(|r| self.process_node(r.syntax()));

        if is_short_circuit && left.is_some() {
            self.active_cds.pop();
        }

        self.graph.add_vertex(DfVertex {
            id,
            kind: VertexKind::FunctionCall,
            range: node.text_trimmed_range(),
            name: op_text,
            cds: self.active_cds.clone(),
            data: VertexData::None,
        });

        if let Some(left_info) = &left {
            self.graph.add_edge(id, left_info.node_id, EdgeType::Reads);
            unknown_refs.extend(left_info.unknown_refs.iter().cloned());
            exit_points.extend(left_info.exit_points.iter().cloned());
            on_exit_unresolved.extend(left_info.on_exit_unresolved.iter().cloned());
        }
        if let Some(right_info) = &right {
            self.graph.add_edge(id, right_info.node_id, EdgeType::Reads);
            unknown_refs.extend(right_info.unknown_refs.iter().cloned());
            exit_points.extend(right_info.exit_points.iter().cloned());
            on_exit_unresolved.extend(right_info.on_exit_unresolved.iter().cloned());
        }

        Some(DataflowInformation {
            node_id: id,
            unknown_refs,
            reads: vec![],
            writes: vec![],
            exit_points,
            on_exit_unresolved,
        })
    }

    /// `x <- expr` or `x = expr` or `x <<- expr`
    fn process_left_assignment(
        &mut self,
        node: &RSyntaxNode,
        is_super: bool,
    ) -> Option<DataflowInformation> {
        let bin = RBinaryExpression::cast_ref(node)?;
        let fields = bin.as_fields();
        let lhs = fields.left.ok()?;
        let rhs = fields.right.ok()?;

        // Process the RHS first (the value being assigned).
        let rhs_info = self.process_node(rhs.syntax());

        let lhs_name = node_text(lhs.syntax());
        let is_complex = !is_simple_ident_node(&lhs_name);

        // For complex LHS like `attr(x, nms[1]) <- val`, process the
        // LHS sub-expressions to capture all reads (e.g. `nms`, `x`),
        // find the root variable, and create a replacement function call.
        let (lhs_unknown, def_name, replacement_call_name) = if is_complex {
            let lhs_info = self.process_node(lhs.syntax());
            let unknowns = lhs_info
                .as_ref()
                .map(|info| info.unknown_refs.clone())
                .unwrap_or_default();
            // Walk the LHS to find the root variable name.
            let root = find_root_variable(lhs.syntax());
            // Determine the replacement function name (e.g., "names<-", "[<-").
            let repl_name = find_replacement_name(lhs.syntax(), is_super);
            (unknowns, root.unwrap_or(lhs_name.clone()), repl_name)
        } else {
            (vec![], lhs_name.clone(), None)
        };

        // Create a Definition vertex for the target variable.
        let def_id = self.graph.fresh_id();

        self.graph.add_vertex(DfVertex {
            id: def_id,
            kind: VertexKind::Definition,
            range: lhs.syntax().text_trimmed_range(),
            name: def_name.clone(),
            cds: self.active_cds.clone(),
            data: VertexData::None,
        });

        // Wire: definition ← rhs value
        let mut unknown_refs = Vec::new();
        let mut exit_points = Vec::new();
        let mut on_exit_unresolved = Vec::new();
        if let Some(rhs_info) = &rhs_info {
            self.graph
                .add_edge(def_id, rhs_info.node_id, EdgeType::DefinedBy);
            unknown_refs.extend(rhs_info.unknown_refs.iter().cloned());
            exit_points.extend(rhs_info.exit_points.iter().cloned());
            on_exit_unresolved.extend(rhs_info.on_exit_unresolved.iter().cloned());
        }
        unknown_refs.extend(lhs_unknown);

        // Create a FunctionCall vertex for the replacement function or
        // the plain assignment operator.
        let call_name = replacement_call_name
            .unwrap_or_else(|| if is_super { "<<-" } else { "<-" }.to_string());
        let assign_id = self.graph.fresh_id();
        self.graph.add_vertex(DfVertex {
            id: assign_id,
            kind: VertexKind::FunctionCall,
            range: node.text_trimmed_range(),
            name: call_name,
            cds: self.active_cds.clone(),
            data: VertexData::None,
        });
        self.graph.add_edge(assign_id, def_id, EdgeType::Returns);
        if let Some(rhs_info) = &rhs_info {
            self.graph
                .add_edge(assign_id, rhs_info.node_id, EdgeType::Reads);
        }

        // Register the new binding in the environment.
        let id_def = IdentifierDef { node_id: def_id, cds: self.active_cds.clone() };
        if is_super {
            self.env.define_super(&def_name, id_def);
            self.graph.mark_super_assign(def_id);
        } else {
            self.env.define(&def_name, id_def);
        }

        Some(DataflowInformation {
            node_id: assign_id,
            unknown_refs,
            reads: vec![],
            writes: vec![(def_name, def_id)],
            exit_points,
            on_exit_unresolved,
        })
    }

    /// `expr -> x` or `expr ->> x`
    fn process_right_assignment(
        &mut self,
        node: &RSyntaxNode,
        is_super: bool,
    ) -> Option<DataflowInformation> {
        let bin = RBinaryExpression::cast_ref(node)?;
        let fields = bin.as_fields();
        let lhs = fields.left.ok()?; // the value
        let rhs = fields.right.ok()?; // the target

        // Process the value (LHS in syntax, but RHS semantically).
        let val_info = self.process_node(lhs.syntax());

        let target_name = node_text(rhs.syntax());
        let is_complex = !is_simple_ident_node(&target_name);

        // For complex targets, process sub-expressions to capture reads.
        let target_unknown = if is_complex {
            self.process_node(rhs.syntax())
                .map(|info| info.unknown_refs)
                .unwrap_or_default()
        } else {
            vec![]
        };

        // Create a Definition vertex for the target.
        let def_id = self.graph.fresh_id();

        self.graph.add_vertex(DfVertex {
            id: def_id,
            kind: VertexKind::Definition,
            range: rhs.syntax().text_trimmed_range(),
            name: target_name.clone(),
            cds: self.active_cds.clone(),
            data: VertexData::None,
        });

        let mut unknown_refs = Vec::new();
        let mut exit_points = Vec::new();
        let mut on_exit_unresolved = Vec::new();
        if let Some(val_info) = &val_info {
            self.graph
                .add_edge(def_id, val_info.node_id, EdgeType::DefinedBy);
            unknown_refs.extend(val_info.unknown_refs.iter().cloned());
            exit_points.extend(val_info.exit_points.iter().cloned());
            on_exit_unresolved.extend(val_info.on_exit_unresolved.iter().cloned());
        }
        unknown_refs.extend(target_unknown);

        // Create a FunctionCall vertex for the assignment operator itself.
        let op_name = if is_super { "->>" } else { "->" };
        let assign_id = self.graph.fresh_id();
        self.graph.add_vertex(DfVertex {
            id: assign_id,
            kind: VertexKind::FunctionCall,
            range: node.text_trimmed_range(),
            name: op_name.to_string(),
            cds: self.active_cds.clone(),
            data: VertexData::None,
        });
        self.graph.add_edge(assign_id, def_id, EdgeType::Returns);
        if let Some(val_info) = &val_info {
            self.graph
                .add_edge(assign_id, val_info.node_id, EdgeType::Reads);
        }

        let id_def = IdentifierDef { node_id: def_id, cds: self.active_cds.clone() };
        if is_super {
            self.env.define_super(&target_name, id_def);
            self.graph.mark_super_assign(def_id);
        } else {
            self.env.define(&target_name, id_def);
        }

        Some(DataflowInformation {
            node_id: assign_id,
            unknown_refs,
            reads: vec![],
            writes: vec![(target_name, def_id)],
            exit_points,
            on_exit_unresolved,
        })
    }

    /// `x |> f()` — rewritten as `f(x)`
    fn process_pipe(&mut self, node: &RSyntaxNode) -> Option<DataflowInformation> {
        let bin = RBinaryExpression::cast_ref(node)?;
        let fields = bin.as_fields();
        let lhs = fields.left.ok()?;
        let rhs = fields.right.ok()?;

        let lhs_info = self.process_node(lhs.syntax());
        let rhs_info = self.process_node(rhs.syntax());

        // The pipe feeds lhs into rhs (which should be a call).
        // Create a read edge from rhs → lhs.
        if let (Some(lhs_info), Some(rhs_info)) = (&lhs_info, &rhs_info) {
            self.graph
                .add_edge(rhs_info.node_id, lhs_info.node_id, EdgeType::Reads);
        }

        let mut unknown_refs = Vec::new();
        let mut exit_points = Vec::new();
        let mut on_exit_unresolved = Vec::new();
        if let Some(li) = &lhs_info {
            unknown_refs.extend(li.unknown_refs.iter().cloned());
            exit_points.extend(li.exit_points.iter().cloned());
            on_exit_unresolved.extend(li.on_exit_unresolved.iter().cloned());
        }
        if let Some(ri) = &rhs_info {
            unknown_refs.extend(ri.unknown_refs.iter().cloned());
            exit_points.extend(ri.exit_points.iter().cloned());
            on_exit_unresolved.extend(ri.on_exit_unresolved.iter().cloned());
        }

        Some(DataflowInformation {
            node_id: rhs_info.map(|i| i.node_id).unwrap_or(NodeId(0)),
            unknown_refs,
            reads: vec![],
            writes: vec![],
            exit_points,
            on_exit_unresolved,
        })
    }

    /// `x %<>% f()`, rewritten as `x <- f(x)`
    fn process_pipe_assignment(&mut self, node: &RSyntaxNode) -> Option<DataflowInformation> {
        let bin = RBinaryExpression::cast_ref(node)?;
        let fields = bin.as_fields();
        let lhs = fields.left.ok()?;
        let rhs = fields.right.ok()?;

        let def_name = node_text(lhs.syntax());

        // Process both sides. The LHS is read (piped into the RHS call).
        let lhs_info = self.process_node(lhs.syntax());
        let rhs_info = self.process_node(rhs.syntax());

        // Pipe: rhs reads lhs.
        if let (Some(li), Some(ri)) = (&lhs_info, &rhs_info) {
            self.graph.add_edge(ri.node_id, li.node_id, EdgeType::Reads);
        }

        // Resolve the LHS unknown refs NOW, before we register the new
        // definition.  Otherwise the Use(x) on the LHS would resolve to
        // the new Definition(x) instead of the prior one.
        let mut unknown_refs = Vec::new();
        if let Some(li) = &lhs_info {
            for (name, ref_id) in &li.unknown_refs {
                if let Some(defs) = self.env.resolve(name) {
                    for def in defs {
                        self.graph.add_edge(*ref_id, def.node_id, EdgeType::Reads);
                    }
                } else {
                    unknown_refs.push((name.clone(), *ref_id));
                }
            }
        }

        // Definition vertex for the write-back to `x`.
        let def_id = self.graph.fresh_id();
        self.graph.add_vertex(DfVertex {
            id: def_id,
            kind: VertexKind::Definition,
            range: lhs.syntax().text_trimmed_range(),
            name: def_name.clone(),
            cds: self.active_cds.clone(),
            data: VertexData::None,
        });

        // The definition's value comes from the RHS.
        if let Some(ri) = &rhs_info {
            self.graph.add_edge(def_id, ri.node_id, EdgeType::DefinedBy);
        }

        // FunctionCall vertex for the `%<>%` operator itself.
        let assign_id = self.graph.fresh_id();
        self.graph.add_vertex(DfVertex {
            id: assign_id,
            kind: VertexKind::FunctionCall,
            range: node.text_trimmed_range(),
            name: "%<>%".to_string(),
            cds: self.active_cds.clone(),
            data: VertexData::None,
        });
        self.graph.add_edge(assign_id, def_id, EdgeType::Returns);
        if let Some(ri) = &rhs_info {
            self.graph.add_edge(assign_id, ri.node_id, EdgeType::Reads);
        }

        // Register the new binding in the environment.
        let id_def = IdentifierDef { node_id: def_id, cds: self.active_cds.clone() };
        self.env.define(&def_name, id_def);

        // LHS unknown_refs were already resolved above; only collect
        // exit_points/on_exit_unresolved from LHS, plus everything from RHS.
        let mut exit_points = Vec::new();
        let mut on_exit_unresolved = Vec::new();
        if let Some(li) = &lhs_info {
            exit_points.extend(li.exit_points.iter().cloned());
            on_exit_unresolved.extend(li.on_exit_unresolved.iter().cloned());
        }
        if let Some(ri) = &rhs_info {
            unknown_refs.extend(ri.unknown_refs.iter().cloned());
            exit_points.extend(ri.exit_points.iter().cloned());
            on_exit_unresolved.extend(ri.on_exit_unresolved.iter().cloned());
        }

        Some(DataflowInformation {
            node_id: assign_id,
            unknown_refs,
            reads: vec![],
            writes: vec![(def_name, def_id)],
            exit_points,
            on_exit_unresolved,
        })
    }

    // ------------------------------------------------------------------
    // Function definition
    // ------------------------------------------------------------------

    fn process_function_def(&mut self, node: &RSyntaxNode) -> DataflowInformation {
        let func = RFunctionDefinition::cast_ref(node).unwrap();
        let fields = func.as_fields();

        let fdef_id = self.graph.fresh_id();

        // Enter a child scope for the function body.
        let parent_env = self.env.clone();
        self.env = Environment::new_child(self.env.clone());

        // Process parameters.
        let mut param_ids = Vec::new();
        let mut unknown_refs = Vec::new();

        if let Ok(params) = fields.parameters {
            {
                let param_list = params.items();
                for param_result in param_list.iter() {
                    if let Ok(param) = param_result
                        && let Ok(name_node) = param.name()
                        && let Some(ident) = name_node.as_r_identifier()
                        && let Ok(name_token) = ident.name_token()
                    {
                        let pname = name_token.token_text_trimmed().to_string();
                        let pid = self.graph.fresh_id();
                        self.graph.add_vertex(DfVertex {
                            id: pid,
                            kind: VertexKind::Definition,
                            range: ident.syntax().text_trimmed_range(),
                            name: pname.clone(),
                            cds: self.active_cds.clone(),
                            data: VertexData::None,
                        });
                        self.env.define(
                            &pname,
                            IdentifierDef { node_id: pid, cds: self.active_cds.clone() },
                        );
                        param_ids.push(pid);

                        // If the parameter has a default value, process it
                        if let Some(default) = param.default()
                            && let Ok(val) = default.value()
                            && let Some(val_info) = self.process_node(val.syntax())
                        {
                            self.graph
                                .add_edge(pid, val_info.node_id, EdgeType::DefinedBy);

                            for (name, ref_id) in &val_info.unknown_refs {
                                unknown_refs.push((name.clone(), *ref_id));
                            }
                        }
                    }
                }
            }
        }

        // Process the body.
        let mut body_ids = Vec::new();
        let mut exit_point_ids = Vec::new();
        if let Ok(body) = fields.body
            && let Some(info) = self.process_node(body.syntax())
        {
            body_ids.push(info.node_id);
            // The last expression in the body is the implicit return value.
            exit_point_ids.push(info.node_id);
            // Collect Return exit points as additional exit points.
            for ep in &info.exit_points {
                if ep.type_ == ExitPointType::Return {
                    exit_point_ids.push(ep.node_id);
                }
            }
            // Closure tracking: resolve unknown refs against the
            // function's own scope first, then bubble remaining
            // up as closure references to the parent.
            for (name, ref_id) in &info.unknown_refs {
                if let Some(defs) = self.env.resolve(name) {
                    for def in defs {
                        self.graph.add_edge(*ref_id, def.node_id, EdgeType::Reads);
                    }
                } else {
                    unknown_refs.push((name.clone(), *ref_id));
                }
            }

            // Deferred pass: resolve on.exit() refs that were unresolved
            // during the immediate pass (forward references to objects
            // defined later in the body).
            for (name, ref_id) in &info.on_exit_unresolved {
                if let Some(defs) = self.env.resolve(name) {
                    for def in defs {
                        self.graph.add_edge(*ref_id, def.node_id, EdgeType::Reads);
                    }
                } else {
                    unknown_refs.push((name.clone(), *ref_id));
                }
            }
        }

        // Restore parent environment.
        self.env = parent_env;

        // Resolve closure references against the parent environment.
        for (name, ref_id) in &unknown_refs {
            if let Some(defs) = self.env.resolve(name) {
                for def in defs {
                    self.graph.add_edge(*ref_id, def.node_id, EdgeType::Reads);
                }
            }
        }

        self.graph.add_vertex(DfVertex {
            id: fdef_id,
            kind: VertexKind::FunctionDef,
            range: node.text_trimmed_range(),
            name: "<function>".to_string(),
            cds: self.active_cds.clone(),
            data: VertexData::FunctionDef {
                params: param_ids,
                unresolved: unknown_refs.clone(),
                body_nodes: body_ids,
                exit_points: exit_point_ids,
            },
        });

        DataflowInformation {
            node_id: fdef_id,
            // Remaining unresolved closure refs bubble up further
            unknown_refs,
            reads: vec![],
            writes: vec![],
            exit_points: vec![ExitPoint {
                node_id: fdef_id,
                type_: ExitPointType::Default,
                cds: self.active_cds.clone(),
            }],
            on_exit_unresolved: vec![],
        }
    }

    // ------------------------------------------------------------------
    // Function call
    // ------------------------------------------------------------------

    fn process_call(&mut self, node: &RSyntaxNode) -> DataflowInformation {
        let call = RCall::cast_ref(node).unwrap();
        let fields = call.as_fields();

        let call_id = self.graph.fresh_id();
        let func_name = fields
            .function
            .as_ref()
            .map(|f| node_text(f.syntax()))
            .unwrap_or_default();

        // --- local() scoping: push a child environment, process body,
        // then pop so definitions don't leak. ---
        let is_local = func_name == "local";
        if is_local {
            let parent_env = self.env.clone();
            self.env = Environment::new_child(self.env.clone());

            let result = self.process_call_inner(node, call_id, &func_name);

            // Restore parent environment — local defs don't leak.
            self.env = parent_env;
            return result;
        }

        // --- on.exit(): process normally (immediate pass) AND store the
        // first argument for a deferred pass at the end of the enclosing
        // function definition. The immediate pass catches references to
        // variables already defined; the deferred pass catches forward
        // references to variables defined later in the body.
        //
        // We can't just defer this to the end because of cases like this:
        // ```
        // f <- function() {
        //     foo <- TRUE
        //     on.exit(
        //         if (foo) print("bye")
        //     )
        //     <some operation that might error here>
        //     foo <- FALSE
        // }
        // ```
        //
        // If we were only deferring `on.exit()` to the end, we would report
        // the first `foo` as being never used because we would just see that
        // `foo` is defined twice but never explictly used, while `on.exit()`
        // might use it if it thrown because of an error in between.
        if func_name == "on.exit" {
            let mut result = self.process_call_inner(node, call_id, &func_name);
            // Capture unresolved refs from the on.exit body so the deferred
            // pass in process_function_def can resolve forward references.
            result.on_exit_unresolved = result.unknown_refs.clone();
            return result;
        }

        // --- quote() / NSE: mark all vertices inside as NSE. ---
        let is_nse = matches!(
            func_name.as_str(),
            "quote" | "bquote" | "substitute" | "match.arg"
        );
        if is_nse {
            let id_before = self.graph.next_id();
            let result = self.process_call_inner(node, call_id, &func_name);

            // Add NonStandardEvaluation edges from the call to all
            // vertices created inside the NSE argument.
            let id_after = self.graph.next_id();
            for idx in id_before..id_after {
                let inner_id = NodeId(idx);
                if inner_id != call_id {
                    self.graph
                        .add_edge(call_id, inner_id, EdgeType::NonStandardEvaluation);
                }
            }
            return result;
        }

        self.process_call_inner(node, call_id, &func_name)
    }

    /// Inner call processing shared between normal calls, `local()`, and `quote()`.
    fn process_call_inner(
        &mut self,
        node: &RSyntaxNode,
        call_id: NodeId,
        func_name: &str,
    ) -> DataflowInformation {
        let call = RCall::cast_ref(node).unwrap();
        let fields = call.as_fields();

        // Process the function expression (might be a symbol or complex expr).
        let func_info = fields
            .function
            .ok()
            .and_then(|f| self.process_node(f.syntax()));

        // Process arguments.
        let mut arg_data = Vec::new();
        let mut all_unknown = Vec::new();
        let mut all_exit_points = Vec::new();
        let mut on_exit_unresolved = Vec::new();

        if let Ok(args) = fields.arguments {
            let arg_list = args.items();
            for arg in arg_list.iter().flatten() {
                let arg_name = arg
                    .name_clause()
                    .and_then(|nc| nc.name().ok())
                    .map(|n| n.to_string().trim().to_string());

                if let Some(value) = arg.value()
                    && let Some(val_info) = self.process_node(value.syntax())
                {
                    self.graph
                        .add_edge(call_id, val_info.node_id, EdgeType::Argument);
                    arg_data.push(CallArgument { node_id: val_info.node_id, name: arg_name });
                    all_unknown.extend(val_info.unknown_refs.iter().cloned());
                    all_exit_points.extend(val_info.exit_points.iter().cloned());
                    on_exit_unresolved.extend(val_info.on_exit_unresolved.iter().cloned());
                }
            }
        }

        // Reads from the function expression itself.
        if let Some(fi) = &func_info {
            self.graph.add_edge(call_id, fi.node_id, EdgeType::Calls);
            all_unknown.extend(fi.unknown_refs.iter().cloned());
        }

        // Handle built-in side-effect functions.
        let mut writes = Vec::new();
        self.handle_builtin_call(func_name, &arg_data, call_id, &mut writes);

        self.graph.add_vertex(DfVertex {
            id: call_id,
            kind: VertexKind::FunctionCall,
            range: node.text_trimmed_range(),
            name: func_name.to_string(),
            cds: self.active_cds.clone(),
            data: VertexData::Call { args: arg_data },
        });

        // Determine exit point type based on function name.
        let exit_type = match func_name {
            "return" => ExitPointType::Return,
            "break" => ExitPointType::Break,
            "next" => ExitPointType::Next,
            _ => ExitPointType::Default,
        };
        all_exit_points.push(ExitPoint {
            node_id: call_id,
            type_: exit_type,
            cds: self.active_cds.clone(),
        });

        DataflowInformation {
            node_id: call_id,
            unknown_refs: all_unknown,
            reads: vec![],
            writes,
            exit_points: all_exit_points,
            on_exit_unresolved,
        }
    }

    /// Return the environment argument for `assign()`, `delayedAssign()`, or
    /// `makeActiveBinding()` if one is supplied, meaning the assignment targets
    /// a non-local environment.
    fn env_arg<'a>(func_name: &str, args: &'a [CallArgument]) -> Option<&'a CallArgument> {
        // (env_arg_name, positional_index) for each function.
        let (env_name, env_pos) = match func_name {
            "assign" => ("envir", 2),             // assign(x, value, envir, ...)
            "delayedAssign" => ("assign.env", 3), // delayedAssign(x, value, eval.env, assign.env, ...)
            "makeActiveBinding" => ("env", 2),    // makeActiveBinding(sym, fun, env, ...)
            _ => return None,
        };
        // Named argument present?
        if let Some(arg) = args.iter().find(|a| a.name.as_deref() == Some(env_name)) {
            return Some(arg);
        }
        // Positional argument at the expected index?
        let mut positional_idx = 0;
        for arg in args {
            if arg.name.is_none() {
                if positional_idx == env_pos {
                    return Some(arg);
                }
                positional_idx += 1;
            }
        }
        None
    }

    /// Handle built-in functions that have environment side-effects.
    fn handle_builtin_call(
        &mut self,
        func_name: &str,
        args: &[CallArgument],
        call_id: NodeId,
        writes: &mut Vec<(String, NodeId)>,
    ) {
        match func_name {
            "assign" | "delayedAssign" | "makeActiveBinding" => {
                // When an explicit target environment is supplied the
                // variable is defined there, not the local scope.  Model
                // this as a write-back to the environment variable itself
                // (it is mutated by the call).
                // For instance, in this case, we should report `env`:
                // ```
                // f <- function() {
                //     env <- new.env()
                //     assign("x", 1 + 1, envir = env)
                // }
                // f()
                // ```

                // When an explicit target environment is supplied the
                // variable goes there, not the local scope — just treat
                // as a plain call (all args are reads, no local Definition).
                if Self::env_arg(func_name, args).is_some() {
                    return;
                }
                // assign("x", val) → treat as x <- val
                // First arg should be a string literal with the variable name.
                if let Some(first_arg) = args.first() {
                    // Extract vertex info before mutable borrow.
                    let var_info = self.graph.vertex(first_arg.node_id).and_then(|v| {
                        if v.kind == VertexKind::Value {
                            Some((
                                v.name.trim_matches('"').trim_matches('\'').to_string(),
                                v.range,
                            ))
                        } else {
                            None
                        }
                    });
                    if let Some((var_name, range)) = var_info {
                        let def_id = self.graph.fresh_id();
                        self.graph.add_vertex(DfVertex {
                            id: def_id,
                            kind: VertexKind::Definition,
                            range,
                            name: var_name.clone(),
                            cds: self.active_cds.clone(),
                            data: VertexData::None,
                        });
                        if let Some(val_arg) = args.get(1) {
                            self.graph
                                .add_edge(def_id, val_arg.node_id, EdgeType::DefinedBy);
                        }
                        self.graph.add_edge(def_id, call_id, EdgeType::DefinedBy);
                        self.env.define(
                            &var_name,
                            IdentifierDef { node_id: def_id, cds: self.active_cds.clone() },
                        );
                        writes.push((var_name, def_id));
                    }
                }
            }
            "rm" | "remove" => {
                // rm(x) → resolve reads first, then remove from environment.
                // We must add Reads edges before removing so that the Use
                // vertex counts as a use of the definition.
                let rm_info: Vec<(NodeId, String, Vec<NodeId>)> = args
                    .iter()
                    .filter_map(|arg| {
                        let v = self.graph.vertex(arg.node_id)?;
                        if v.kind == VertexKind::Use {
                            let name = v.name.clone();
                            let def_ids: Vec<NodeId> = self
                                .env
                                .resolve(&name)
                                .map(|defs| defs.iter().map(|d| d.node_id).collect())
                                .unwrap_or_default();
                            Some((arg.node_id, name, def_ids))
                        } else if v.kind == VertexKind::Value {
                            let name = v.name.trim_matches('"').trim_matches('\'').to_string();
                            Some((arg.node_id, name, vec![]))
                        } else {
                            None
                        }
                    })
                    .collect();
                for (use_id, name, def_ids) in &rm_info {
                    for def_id in def_ids {
                        self.graph.add_edge(*use_id, *def_id, EdgeType::Reads);
                    }
                    self.env.remove(name);
                }
            }
            "do.call" => {
                // do.call("f", args) → the string "f" is a read of variable f.
                if let Some(first_arg) = args.first() {
                    let read_info = self.graph.vertex(first_arg.node_id).and_then(|v| {
                        if v.kind == VertexKind::Value {
                            Some(v.name.trim_matches('"').trim_matches('\'').to_string())
                        } else {
                            None
                        }
                    });
                    if let Some(var_name) = read_info
                        && let Some(defs) = self.env.resolve(&var_name)
                    {
                        let def_ids: Vec<NodeId> = defs.iter().map(|d| d.node_id).collect();
                        for def_id in def_ids {
                            self.graph
                                .add_edge(first_arg.node_id, def_id, EdgeType::Reads);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------------
    // Control flow: if / else
    // ------------------------------------------------------------------

    fn process_if(&mut self, node: &RSyntaxNode) -> Option<DataflowInformation> {
        let if_stmt = RIfStatement::cast_ref(node)?;
        let fields = if_stmt.as_fields();

        let if_id = self.graph.fresh_id();

        // Process the condition.
        let cond_info = fields
            .condition
            .ok()
            .and_then(|c| self.process_node(c.syntax()));

        // Create a FunctionCall vertex representing the `if`.
        self.graph.add_vertex(DfVertex {
            id: if_id,
            kind: VertexKind::FunctionCall,
            range: node.text_trimmed_range(),
            name: "if".to_string(),
            cds: self.active_cds.clone(),
            data: VertexData::None,
        });

        if let Some(ci) = &cond_info {
            self.graph.add_edge(if_id, ci.node_id, EdgeType::Reads);
        }

        let mut unknown_refs = cond_info
            .as_ref()
            .map(|c| c.unknown_refs.clone())
            .unwrap_or_default();
        let mut exit_points = Vec::new();
        let mut writes = Vec::new();
        let mut on_exit_unresolved = Vec::new();

        // Save env before branches.
        let env_before = self.env.clone();

        // Process then-branch under a control dependency.
        self.active_cds.push(ControlDependency {
            id: if_id,
            when: Some(true),
            by_iteration: false,
        });
        let then_info = fields
            .consequence
            .ok()
            .and_then(|c| self.process_node(c.syntax()));
        self.active_cds.pop();
        let env_after_then = self.env.clone();

        // Process else-branch (if any).
        self.env = env_before.clone();
        self.active_cds.push(ControlDependency {
            id: if_id,
            when: Some(false),
            by_iteration: false,
        });
        let else_info = fields
            .else_clause
            .and_then(|ec| ec.alternative().ok())
            .and_then(|alt| self.process_node(alt.syntax()));
        self.active_cds.pop();
        let env_after_else = self.env.clone();

        // Merge environments: after an if/else both branches' definitions
        // should be visible (with appropriate CDs).
        self.env = merge_envs(env_after_then, env_after_else);

        if let Some(ti) = &then_info {
            self.graph.add_edge(if_id, ti.node_id, EdgeType::Returns);
            unknown_refs.extend(ti.unknown_refs.iter().cloned());
            exit_points.extend(ti.exit_points.iter().cloned());
            writes.extend(ti.writes.iter().cloned());
            on_exit_unresolved.extend(ti.on_exit_unresolved.iter().cloned());
        }
        if let Some(ei) = &else_info {
            self.graph.add_edge(if_id, ei.node_id, EdgeType::Returns);
            unknown_refs.extend(ei.unknown_refs.iter().cloned());
            exit_points.extend(ei.exit_points.iter().cloned());
            writes.extend(ei.writes.iter().cloned());
            on_exit_unresolved.extend(ei.on_exit_unresolved.iter().cloned());
        }

        Some(DataflowInformation {
            node_id: if_id,
            unknown_refs,
            reads: vec![],
            writes,
            exit_points,
            on_exit_unresolved,
        })
    }

    // ------------------------------------------------------------------
    // Control flow: for
    // ------------------------------------------------------------------

    fn process_for(&mut self, node: &RSyntaxNode) -> Option<DataflowInformation> {
        let for_stmt = RForStatement::cast_ref(node)?;
        let fields = for_stmt.as_fields();

        let for_id = self.graph.fresh_id();

        // Process the sequence/iterator.
        let seq_info = fields
            .sequence
            .ok()
            .and_then(|s| self.process_node(s.syntax()));

        // Create a definition for the loop variable.
        let var_id = self.graph.fresh_id();
        let var_range = fields
            .variable
            .as_ref()
            .ok()
            .map(|v| v.syntax().text_trimmed_range())
            .unwrap_or_default();
        let var_name = fields
            .variable
            .ok()
            .map(|v| node_text(v.syntax()))
            .unwrap_or_default();
        self.graph.add_vertex(DfVertex {
            id: var_id,
            kind: VertexKind::Definition,
            range: var_range,
            name: var_name.clone(),
            cds: self.active_cds.clone(),
            data: VertexData::None,
        });
        if let Some(si) = &seq_info {
            self.graph.add_edge(var_id, si.node_id, EdgeType::DefinedBy);
        }
        self.env.define(
            &var_name,
            IdentifierDef { node_id: var_id, cds: self.active_cds.clone() },
        );

        self.graph.add_vertex(DfVertex {
            id: for_id,
            kind: VertexKind::FunctionCall,
            range: node.text_trimmed_range(),
            name: "for".to_string(),
            cds: self.active_cds.clone(),
            data: VertexData::None,
        });

        let mut unknown_refs = seq_info
            .as_ref()
            .map(|s| s.unknown_refs.clone())
            .unwrap_or_default();

        // Save env before loop body — the loop may not execute, so
        // pre-loop definitions must remain visible afterwards.
        let env_before_loop = self.env.clone();

        // Process body under iteration CD.
        self.active_cds
            .push(ControlDependency { id: for_id, when: None, by_iteration: true });
        let body_info = fields.body.ok().and_then(|b| self.process_node(b.syntax()));
        self.active_cds.pop();

        // Circular loop redefinition linking.
        if let Some(bi) = &body_info {
            link_circular_redefinitions(&mut self.graph, &bi.unknown_refs, &bi.writes);
        }

        // Merge: both pre-loop and post-loop definitions are potential.
        self.env = merge_envs(env_before_loop, self.env.clone());

        let mut exit_points = Vec::new();
        if let Some(bi) = &body_info {
            self.graph.add_edge(for_id, bi.node_id, EdgeType::Returns);
            unknown_refs.extend(bi.unknown_refs.iter().cloned());
            // Filter out Break/Next — they are consumed by the loop.
            exit_points.extend(filter_loop_exit_points(&bi.exit_points));
        }

        Some(DataflowInformation {
            node_id: for_id,
            unknown_refs,
            reads: vec![],
            writes: vec![(var_name, var_id)],
            exit_points,
            on_exit_unresolved: vec![],
        })
    }

    // ------------------------------------------------------------------
    // Control flow: while
    // ------------------------------------------------------------------

    fn process_while(&mut self, node: &RSyntaxNode) -> Option<DataflowInformation> {
        let while_stmt = RWhileStatement::cast_ref(node)?;
        let fields = while_stmt.as_fields();

        let while_id = self.graph.fresh_id();

        let cond_info = fields
            .condition
            .ok()
            .and_then(|c| self.process_node(c.syntax()));

        self.graph.add_vertex(DfVertex {
            id: while_id,
            kind: VertexKind::FunctionCall,
            range: node.text_trimmed_range(),
            name: "while".to_string(),
            cds: self.active_cds.clone(),
            data: VertexData::None,
        });

        let mut unknown_refs = cond_info
            .as_ref()
            .map(|c| c.unknown_refs.clone())
            .unwrap_or_default();

        if let Some(ci) = &cond_info {
            self.graph.add_edge(while_id, ci.node_id, EdgeType::Reads);
        }

        let env_before_loop = self.env.clone();

        self.active_cds
            .push(ControlDependency { id: while_id, when: None, by_iteration: true });
        let body_info = fields.body.ok().and_then(|b| self.process_node(b.syntax()));
        self.active_cds.pop();

        // Circular loop redefinition linking.
        if let Some(bi) = &body_info {
            link_circular_redefinitions(&mut self.graph, &bi.unknown_refs, &bi.writes);
        }

        self.env = merge_envs(env_before_loop, self.env.clone());

        let mut exit_points = Vec::new();
        if let Some(bi) = &body_info {
            self.graph.add_edge(while_id, bi.node_id, EdgeType::Returns);
            unknown_refs.extend(bi.unknown_refs.iter().cloned());
            exit_points.extend(filter_loop_exit_points(&bi.exit_points));
        }

        Some(DataflowInformation {
            node_id: while_id,
            unknown_refs,
            reads: vec![],
            writes: vec![],
            exit_points,
            on_exit_unresolved: vec![],
        })
    }

    // ------------------------------------------------------------------
    // Control flow: repeat
    // ------------------------------------------------------------------

    fn process_repeat(&mut self, node: &RSyntaxNode) -> Option<DataflowInformation> {
        let repeat_stmt = RRepeatStatement::cast_ref(node)?;
        let fields = repeat_stmt.as_fields();

        let repeat_id = self.graph.fresh_id();

        self.graph.add_vertex(DfVertex {
            id: repeat_id,
            kind: VertexKind::FunctionCall,
            range: node.text_trimmed_range(),
            name: "repeat".to_string(),
            cds: self.active_cds.clone(),
            data: VertexData::None,
        });

        let env_before_loop = self.env.clone();

        self.active_cds
            .push(ControlDependency { id: repeat_id, when: None, by_iteration: true });
        let body_info = fields.body.ok().and_then(|b| self.process_node(b.syntax()));
        self.active_cds.pop();

        // Circular loop redefinition linking.
        if let Some(bi) = &body_info {
            link_circular_redefinitions(&mut self.graph, &bi.unknown_refs, &bi.writes);
        }

        self.env = merge_envs(env_before_loop, self.env.clone());

        let mut unknown_refs = Vec::new();
        let mut exit_points = Vec::new();
        if let Some(bi) = &body_info {
            self.graph
                .add_edge(repeat_id, bi.node_id, EdgeType::Returns);
            unknown_refs = bi.unknown_refs.clone();
            exit_points.extend(filter_loop_exit_points(&bi.exit_points));
        }

        Some(DataflowInformation {
            node_id: repeat_id,
            unknown_refs,
            reads: vec![],
            writes: vec![],
            exit_points,
            on_exit_unresolved: vec![],
        })
    }

    // ------------------------------------------------------------------
    // Namespace expression  pkg::foo
    // ------------------------------------------------------------------

    fn process_namespace(&mut self, node: &RSyntaxNode) -> Option<DataflowInformation> {
        let ns = RNamespaceExpression::cast_ref(node)?;
        let fields = ns.as_fields();

        let id = self.graph.fresh_id();
        let ns_name = fields
            .left
            .ok()
            .map(|n| n.to_string().trim().to_string())
            .unwrap_or_default();
        let sym_name = fields
            .right
            .ok()
            .map(|n| node_text(n.syntax()))
            .unwrap_or_default();
        let full_name = format!("{ns_name}::{sym_name}");

        self.graph.add_vertex(DfVertex {
            id,
            kind: VertexKind::Use,
            range: node.text_trimmed_range(),
            name: full_name.clone(),
            cds: self.active_cds.clone(),
            data: VertexData::None,
        });

        Some(DataflowInformation {
            node_id: id,
            unknown_refs: vec![(full_name, id)],
            reads: vec![],
            writes: vec![],
            exit_points: vec![ExitPoint {
                node_id: id,
                type_: ExitPointType::Default,
                cds: self.active_cds.clone(),
            }],
            on_exit_unresolved: vec![],
        })
    }

    // ------------------------------------------------------------------
    // Expression list / block – process children sequentially
    // ------------------------------------------------------------------

    fn process_expression_list(&mut self, node: &RSyntaxNode) -> Option<DataflowInformation> {
        let mut remaining_unknown: Vec<(String, NodeId)> = Vec::new();
        let mut all_writes: Vec<(String, NodeId)> = Vec::new();
        let mut exit_points: Vec<ExitPoint> = Vec::new();
        let mut on_exit_unresolved: Vec<(String, NodeId)> = Vec::new();
        let mut last_info: Option<DataflowInformation> = None;
        let mut had_exit = false;

        for child in node.children() {
            if let Some(info) = self.process_node(&child) {
                // Resolve unknown references against the current environment.
                // This is the core of deferred name resolution (flowr's
                // linkReadNameToWriteIfPossible).
                for (name, ref_id) in &info.unknown_refs {
                    if let Some(defs) = self.env.resolve(name) {
                        for def in defs {
                            self.graph.add_edge(*ref_id, def.node_id, EdgeType::Reads);
                        }
                    } else {
                        // Unresolved — bubble up to caller.
                        remaining_unknown.push((name.clone(), *ref_id));
                    }
                }

                // Track writes.
                all_writes.extend(info.writes.iter().cloned());

                // Propagate on.exit nodes.
                on_exit_unresolved.extend(info.on_exit_unresolved.iter().cloned());

                // Collect non-default exit points.
                collect_non_default_exit_points(&mut exit_points, &info.exit_points);
                if !had_exit {
                    had_exit = info
                        .exit_points
                        .iter()
                        .any(|ep| ep.type_ != ExitPointType::Default);
                }

                // Dead code: stop after unconditional exit.
                if always_exits(&info) {
                    last_info = Some(info);
                    break;
                }

                last_info = Some(info);
            }
        }

        // Default exit point = last expression processed.
        if let Some(ref li) = last_info {
            exit_points.push(ExitPoint {
                node_id: li.node_id,
                type_: ExitPointType::Default,
                cds: self.active_cds.clone(),
            });
        }

        last_info.map(|li| DataflowInformation {
            node_id: li.node_id,
            unknown_refs: remaining_unknown,
            reads: vec![],
            writes: all_writes,
            exit_points,
            on_exit_unresolved,
        })
    }

    fn process_subset(&mut self, node: &RSyntaxNode) -> Option<DataflowInformation> {
        let mut last: Option<DataflowInformation> = None;
        let mut unknown_refs = Vec::new();
        let mut exit_points = Vec::new();
        let mut on_exit_unresolved = Vec::new();
        for child in node.children() {
            if let Some(info) = self.process_node(&child) {
                unknown_refs.extend(info.unknown_refs.iter().cloned());
                exit_points.extend(info.exit_points.iter().cloned());
                on_exit_unresolved.extend(info.on_exit_unresolved.iter().cloned());
                last = Some(info);
            }
        }

        // In data.table, `..var` inside `[` refers to `var` from the parent
        // scope. Strip the `..` prefix so the use resolves to the definition.
        for (name, _id) in &mut unknown_refs {
            if let Some(stripped) = name.strip_prefix("..") {
                *name = stripped.to_string();
            }
        }

        last.map(|l| DataflowInformation {
            node_id: l.node_id,
            unknown_refs,
            reads: vec![],
            writes: l.writes,
            exit_points,
            on_exit_unresolved,
        })
    }

    // ------------------------------------------------------------------
    // Generic: process children, return info of last child
    // ------------------------------------------------------------------

    fn process_children_last(&mut self, node: &RSyntaxNode) -> Option<DataflowInformation> {
        let mut last: Option<DataflowInformation> = None;
        let mut unknown_refs = Vec::new();
        let mut exit_points = Vec::new();
        let mut on_exit_unresolved = Vec::new();
        for child in node.children() {
            if let Some(info) = self.process_node(&child) {
                unknown_refs.extend(info.unknown_refs.iter().cloned());
                exit_points.extend(info.exit_points.iter().cloned());
                on_exit_unresolved.extend(info.on_exit_unresolved.iter().cloned());
                last = Some(info);
            }
        }
        last.map(|l| DataflowInformation {
            node_id: l.node_id,
            unknown_refs,
            reads: vec![],
            writes: l.writes,
            exit_points,
            on_exit_unresolved,
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Merge two environments after a branch.  All definitions from both are
/// kept (since at runtime exactly one branch executes, both are potential
/// definitions at the merge point).
fn merge_envs(a: Environment, b: Environment) -> Environment {
    let mut merged = a;
    for (name, defs) in b.bindings {
        let entry = merged.bindings.entry(name).or_default();
        for def in defs {
            if !entry.iter().any(|d| d.node_id == def.node_id) {
                entry.push(def);
            }
        }
    }
    merged
}

/// Extract a human-readable text from a syntax node.
fn node_text(node: &RSyntaxNode) -> String {
    node.text_trimmed().to_string()
}

/// Walk a complex LHS node to find the root variable name.
/// E.g., for `names(x)`, `x[i]`, `x$y`, `x@y` → returns `"x"`.
/// For `attr(mtcars, "foo")` → returns `"mtcars"`.
fn find_root_variable(node: &RSyntaxNode) -> Option<String> {
    match node.kind() {
        RSyntaxKind::R_IDENTIFIER => Some(identifier_name(node)),
        RSyntaxKind::R_CALL => {
            // For `names(x)` or `attr(mtcars, "foo")`, the root is the
            // first argument of the outermost function call.
            let call = RCall::cast_ref(node)?;
            let args = call.arguments().ok()?;
            let first_arg = args.items().iter().next()?.ok()?;
            let value = first_arg.value()?;
            find_root_variable(value.syntax())
        }
        RSyntaxKind::R_SUBSET | RSyntaxKind::R_SUBSET2 => {
            // For `x[i]` or `x[[i]]`, the root is the object being subscripted.
            node.children()
                .next()
                .and_then(|child| find_root_variable(&child))
        }
        RSyntaxKind::R_EXTRACT_EXPRESSION => {
            // For `x$y` or `x@y`, the root is the LHS of the extract.
            node.children()
                .next()
                .and_then(|child| find_root_variable(&child))
        }
        _ => {
            // Try first child as fallback.
            node.children()
                .next()
                .and_then(|child| find_root_variable(&child))
        }
    }
}

/// Determine the replacement function name for a complex LHS assignment.
/// E.g., `names(x) <- val` → `"names<-"`, `x[i] <- val` → `"[<-"`,
/// `x$y <- val` → `"$<-"`.
fn find_replacement_name(node: &RSyntaxNode, is_super: bool) -> Option<String> {
    let suffix = if is_super { "<<-" } else { "<-" };
    match node.kind() {
        RSyntaxKind::R_CALL => {
            let call = RCall::cast_ref(node)?;
            let func_name = call.function().ok().map(|f| node_text(f.syntax()))?;
            Some(format!("{func_name}{suffix}"))
        }
        RSyntaxKind::R_SUBSET => Some(format!("[{suffix}")),
        RSyntaxKind::R_SUBSET2 => Some(format!("[[{suffix}")),
        RSyntaxKind::R_EXTRACT_EXPRESSION => {
            // Could be $ or @
            let text = node_text(node);
            if text.contains('@') {
                Some(format!("@{suffix}"))
            } else {
                Some(format!("${suffix}"))
            }
        }
        _ => None,
    }
}

/// Check whether `node` is nested inside a formula (`~` binary expression).
fn is_inside_formula(node: &RSyntaxNode) -> bool {
    let mut cursor = node.parent();
    while let Some(parent) = cursor {
        if let Some(bin) = RBinaryExpression::cast_ref(&parent)
            && bin
                .as_fields()
                .operator
                .is_ok_and(|op| op.kind() == RSyntaxKind::TILDE)
        {
            return true;
        }
        cursor = parent.parent();
    }
    false
}

/// Check if a name looks like a bare identifier (no calls, subscripts, etc.).
fn is_simple_ident_node(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('(')
        && !name.contains('[')
        && !name.contains('$')
        && !name.contains('@')
        && !name.contains(' ')
}

/// Filter out loop-level exit points (`Break`, `Next`) — they are consumed
/// by the enclosing loop and should not propagate further.
fn filter_loop_exit_points(exit_points: &[ExitPoint]) -> Vec<ExitPoint> {
    exit_points
        .iter()
        .filter(|ep| !matches!(ep.type_, ExitPointType::Break | ExitPointType::Next))
        .cloned()
        .collect()
}

/// Collect non-default exit points into an accumulator.
fn collect_non_default_exit_points(acc: &mut Vec<ExitPoint>, exit_points: &[ExitPoint]) {
    for ep in exit_points {
        if ep.type_ != ExitPointType::Default {
            acc.push(ep.clone());
        }
    }
}

/// Check if the sub-tree unconditionally exits (has at least one non-default
/// exit point with no guarding control dependencies).
fn always_exits(info: &DataflowInformation) -> bool {
    info.exit_points
        .iter()
        .any(|ep| ep.type_ != ExitPointType::Default && ep.cds.is_empty())
}

/// Link circular redefinitions within a loop body: if a variable is both
/// read (unknown_ref) and written in the same loop body, add a Reads edge
/// from the read to the last write (representing the previous iteration).
fn link_circular_redefinitions(
    graph: &mut DataflowGraph,
    open_reads: &[(String, NodeId)],
    writes: &[(String, NodeId)],
) {
    use rustc_hash::FxHashMap;
    // Find the last write per name.
    let mut last_write: FxHashMap<&str, NodeId> = FxHashMap::default();
    for (name, id) in writes {
        last_write.insert(name.as_str(), *id);
    }
    // For each unresolved read with a matching write, link them.
    for (name, read_id) in open_reads {
        if let Some(&write_id) = last_write.get(name.as_str()) {
            graph.add_edge(*read_id, write_id, EdgeType::Reads);
        }
    }
}

/// Extract the identifier name from an R_IDENTIFIER node.
fn identifier_name(node: &RSyntaxNode) -> String {
    node.first_token()
        .map(|t| t.token_text_trimmed().to_string())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build a [`DataflowGraph`] from the root syntax node of an R file.
pub fn build_dfg(root: &RSyntaxNode) -> DataflowGraph {
    DfgBuilder::build_from_root(root)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use air_r_parser::RParserOptions;

    fn parse_and_build(code: &str) -> DataflowGraph {
        let parsed = air_r_parser::parse(code, RParserOptions::default());
        build_dfg(&parsed.syntax())
    }

    fn has_vertex_named(g: &DataflowGraph, name: &str, kind: VertexKind) -> bool {
        g.vertices().any(|v| v.name == name && v.kind == kind)
    }

    fn has_edge_type(g: &DataflowGraph, from_name: &str, to_name: &str, ty: EdgeType) -> bool {
        let froms: Vec<_> = g.vertices().filter(|v| v.name == from_name).collect();
        let tos: Vec<_> = g.vertices().filter(|v| v.name == to_name).collect();
        for fv in &froms {
            for tv in &tos {
                for (target, bits) in g.edges_from(fv.id) {
                    if target == tv.id && bits.contains(ty) {
                        return true;
                    }
                }
            }
        }
        false
    }

    #[test]
    fn simple_assignment() {
        let g = parse_and_build("x <- 42");
        assert!(has_vertex_named(&g, "x", VertexKind::Definition));
        assert!(has_vertex_named(&g, "42", VertexKind::Value));
        assert!(has_edge_type(&g, "x", "42", EdgeType::DefinedBy));
    }

    #[test]
    fn variable_read() {
        let g = parse_and_build("x <- 1\ny <- x");
        assert!(has_vertex_named(&g, "x", VertexKind::Definition));
        assert!(has_vertex_named(&g, "y", VertexKind::Definition));
        // y is defined by the read of x
        assert!(has_vertex_named(&g, "x", VertexKind::Use));
    }

    #[test]
    fn use_reads_definition() {
        let g = parse_and_build("x <- 1\ny <- x");
        // The Use vertex for `x` (on RHS of second line) should have a Reads
        // edge pointing to the Definition vertex for `x`.
        let def_x: Vec<_> = g
            .vertices()
            .filter(|v| v.name == "x" && v.kind == VertexKind::Definition)
            .collect();
        let use_x: Vec<_> = g
            .vertices()
            .filter(|v| v.name == "x" && v.kind == VertexKind::Use)
            .collect();
        assert!(!def_x.is_empty(), "should have a definition of x");
        assert!(!use_x.is_empty(), "should have a use of x");

        let def_id = def_x[0].id;
        let use_id = use_x[0].id;
        let edges: Vec<_> = g.edges_from(use_id).collect();
        assert!(
            edges
                .iter()
                .any(|(to, bits)| *to == def_id && bits.contains(EdgeType::Reads)),
            "Use(x) should have a Reads edge to Definition(x)"
        );
    }

    #[test]
    fn function_call() {
        let g = parse_and_build("mean(c(1, 2, 3))");
        assert!(has_vertex_named(&g, "mean", VertexKind::FunctionCall));
    }

    #[test]
    fn function_definition_creates_scope() {
        let g = parse_and_build(
            r#"
f <- function(a, b) {
  a + b
}
"#,
        );
        assert!(has_vertex_named(&g, "f", VertexKind::Definition));
        assert!(has_vertex_named(&g, "<function>", VertexKind::FunctionDef));
        // Parameters should be definitions inside the function
        assert!(has_vertex_named(&g, "a", VertexKind::Definition));
        assert!(has_vertex_named(&g, "b", VertexKind::Definition));
    }

    #[test]
    fn right_assignment() {
        let g = parse_and_build("42 -> x");
        assert!(has_vertex_named(&g, "x", VertexKind::Definition));
        assert!(has_vertex_named(&g, "42", VertexKind::Value));
        assert!(has_edge_type(&g, "x", "42", EdgeType::DefinedBy));
    }

    #[test]
    fn if_else_branches() {
        let g = parse_and_build(
            r#"
if (cond) {
  x <- 1
} else {
  x <- 2
}
"#,
        );
        // Both definitions of x should exist, each under a different CD.
        let defs: Vec<_> = g
            .vertices()
            .filter(|v| v.name == "x" && v.kind == VertexKind::Definition)
            .collect();
        assert_eq!(defs.len(), 2, "if/else should produce two definitions of x");
    }

    #[test]
    fn for_loop_variable() {
        let g = parse_and_build(
            r#"
for (i in 1:10) {
  print(i)
}
"#,
        );
        assert!(has_vertex_named(&g, "i", VertexKind::Definition));
        assert!(has_vertex_named(&g, "for", VertexKind::FunctionCall));
    }

    #[test]
    fn display_output() {
        let g = parse_and_build("x <- 1\ny <- x + 2");
        let output = format!("{g}");
        assert!(output.contains("DataflowGraph"));
        assert!(output.contains("vertices"));
    }

    // --- Phase 2: Deferred resolution ---

    #[test]
    fn deferred_resolution_same_as_eager() {
        // Basic: x <- 1; y <- x still produces Use(x) --Reads--> Def(x)
        let g = parse_and_build("x <- 1\ny <- x");
        assert!(
            has_edge_type(&g, "x", "x", EdgeType::Reads),
            "deferred resolution should still link Use(x) to Def(x)"
        );
    }

    // --- Phase 4: Assignment operator FunctionCall vertex ---

    #[test]
    fn assignment_creates_function_call_vertex() {
        let g = parse_and_build("x <- 1");
        assert!(
            has_vertex_named(&g, "<-", VertexKind::FunctionCall),
            "assignment should create a FunctionCall vertex for <-"
        );
        assert!(
            has_edge_type(&g, "<-", "x", EdgeType::Returns),
            "<- should have Returns edge to definition"
        );
        assert!(
            has_edge_type(&g, "<-", "1", EdgeType::Reads),
            "<- should have Reads edge to the value"
        );
    }

    #[test]
    fn right_assignment_creates_function_call_vertex() {
        let g = parse_and_build("1 -> x");
        assert!(
            has_vertex_named(&g, "->", VertexKind::FunctionCall),
            "right assignment should create a FunctionCall vertex for ->"
        );
    }

    // --- Phase 6: Circular loop redefinitions ---

    #[test]
    fn circular_loop_redefinition() {
        let g = parse_and_build(
            r#"
x <- 0
for (i in 1:10) {
  x <- x + 1
}
"#,
        );
        // The Use(x) inside the loop body should have a Reads edge
        // to the Definition(x) inside the loop body (circular link).
        let defs_x: Vec<_> = g
            .vertices()
            .filter(|v| v.name == "x" && v.kind == VertexKind::Definition)
            .collect();
        let uses_x: Vec<_> = g
            .vertices()
            .filter(|v| v.name == "x" && v.kind == VertexKind::Use)
            .collect();
        assert!(defs_x.len() >= 2, "should have at least 2 definitions of x");
        assert!(!uses_x.is_empty(), "should have a use of x");

        // Find the loop-body definition (not the initial x <- 0)
        // and check that some use of x reads from it.
        let loop_def = defs_x.iter().find(|d| {
            // The loop body def has the + operator as its DefinedBy target
            g.edges_from(d.id)
                .any(|(_, bits)| bits.contains(EdgeType::DefinedBy))
                && g.edges_from(d.id).any(|(target, bits)| {
                    bits.contains(EdgeType::DefinedBy)
                        && g.vertex(target).is_some_and(|v| v.name == "+")
                })
        });
        assert!(loop_def.is_some(), "should find loop-body definition of x");

        let loop_def_id = loop_def.unwrap().id;
        let has_circular = uses_x.iter().any(|u| {
            g.edges_from(u.id)
                .any(|(to, bits)| to == loop_def_id && bits.contains(EdgeType::Reads))
        });
        assert!(
            has_circular,
            "Use(x) should have circular Reads edge to loop-body Def(x)"
        );
    }

    // --- Phase 7: Short-circuit operators ---

    #[test]
    fn short_circuit_and_control_dependency() {
        let g = parse_and_build("a <- TRUE\nb <- FALSE\na && b");
        // The Use(b) should have a control dependency from the && operator
        let use_b: Vec<_> = g
            .vertices()
            .filter(|v| v.name == "b" && v.kind == VertexKind::Use)
            .collect();
        assert!(!use_b.is_empty(), "should have Use(b)");
        // At least one use of b should have a control dependency
        let has_cd = use_b
            .iter()
            .any(|v| v.cds.iter().any(|cd| cd.when == Some(true)));
        assert!(has_cd, "Use(b) in `a && b` should have CD with when=true");
    }

    // --- Phase 8: Built-in function handling ---

    #[test]
    fn assign_builtin_creates_definition() {
        let g = parse_and_build(r#"assign("x", 42)"#);
        assert!(
            has_vertex_named(&g, "x", VertexKind::Definition),
            "assign('x', 42) should create a Definition for x"
        );
    }

    // --- Phase 11: Closure tracking ---

    #[test]
    fn closure_captures_outer_variable() {
        let g = parse_and_build("x <- 1\nf <- function() x");
        // The Use(x) inside the function should read the outer Def(x)
        let def_x: Vec<_> = g
            .vertices()
            .filter(|v| v.name == "x" && v.kind == VertexKind::Definition)
            .collect();
        let use_x: Vec<_> = g
            .vertices()
            .filter(|v| v.name == "x" && v.kind == VertexKind::Use)
            .collect();
        assert!(!def_x.is_empty(), "should have Def(x)");
        assert!(!use_x.is_empty(), "should have Use(x) inside function");

        let has_closure_read = use_x.iter().any(|u| {
            g.edges_from(u.id).any(|(to, bits)| {
                bits.contains(EdgeType::Reads) && def_x.iter().any(|d| d.id == to)
            })
        });
        assert!(
            has_closure_read,
            "Use(x) inside function should read outer Def(x)"
        );
    }

    // --- Phase 3: Exit points ---

    #[test]
    fn return_creates_exit_point() {
        let g = parse_and_build("f <- function() { return(1) }");
        // The return() call should exist
        assert!(has_vertex_named(&g, "return", VertexKind::FunctionCall));
    }

    // --- Phase 5: Replacement functions ---

    #[test]
    fn replacement_function_names_x_as_root() {
        // names(x) <- val  should define `x`, not `names(x)`
        let g = parse_and_build("x <- list(a=1)\nnames(x) <- c('b')");
        // There should be a FunctionCall vertex named "names<-"
        assert!(
            has_vertex_named(&g, "names<-", VertexKind::FunctionCall),
            "replacement should create a FunctionCall for names<-"
        );
        // The definition should be for `x` (the root variable)
        let defs_x: Vec<_> = g
            .vertices()
            .filter(|v| v.name == "x" && v.kind == VertexKind::Definition)
            .collect();
        assert!(
            defs_x.len() >= 2,
            "should have at least 2 definitions of x (initial + replacement)"
        );
    }

    #[test]
    fn subset_replacement() {
        let g = parse_and_build("x <- 1:10\nx[1] <- 99");
        assert!(
            has_vertex_named(&g, "[<-", VertexKind::FunctionCall),
            "x[1] <- 99 should create a FunctionCall for [<-"
        );
    }

    #[test]
    fn dollar_replacement() {
        let g = parse_and_build("x <- list()\nx$y <- 1");
        assert!(
            has_vertex_named(&g, "$<-", VertexKind::FunctionCall),
            "x$y <- 1 should create a FunctionCall for $<-"
        );
    }

    // --- Phase 9: local() scoping ---

    #[test]
    fn local_does_not_leak_definitions() {
        let g = parse_and_build("local({ x <- 1 })\nprint(x)");
        // Use(x) after the local() should NOT read the x defined inside local()
        let use_x: Vec<_> = g
            .vertices()
            .filter(|v| v.name == "x" && v.kind == VertexKind::Use)
            .collect();
        // The Use(x) in print(x) should not have any Reads edges
        // (x was defined inside local() and shouldn't leak)
        for u in &use_x {
            let reads_any_def = g.edges_from(u.id).any(|(to, bits)| {
                bits.contains(EdgeType::Reads)
                    && g.vertex(to)
                        .is_some_and(|v| v.name == "x" && v.kind == VertexKind::Definition)
            });
            assert!(
                !reads_any_def,
                "Use(x) after local() should NOT read the x defined inside local()"
            );
        }
    }

    // --- Phase 10: quote() / NSE ---

    #[test]
    fn quote_adds_nse_edges() {
        let g = parse_and_build("x <- 1\nquote(x + 1)");
        // The quote call should have NSE edges to vertices inside it
        let quote_calls: Vec<_> = g
            .vertices()
            .filter(|v| v.name == "quote" && v.kind == VertexKind::FunctionCall)
            .collect();
        assert!(!quote_calls.is_empty(), "should have a quote() call vertex");
        let quote_id = quote_calls[0].id;
        let nse_edges: Vec<_> = g
            .edges_from(quote_id)
            .filter(|(_, bits)| bits.contains(EdgeType::NonStandardEvaluation))
            .collect();
        assert!(
            !nse_edges.is_empty(),
            "quote() should have NonStandardEvaluation edges to its inner vertices"
        );
    }

    // --- Phase 12c: by_iteration ---

    #[test]
    fn loop_cd_has_by_iteration() {
        let g = parse_and_build("for (i in 1:5) { x <- i }");
        // The Def(x) inside the loop should have a CD with by_iteration=true
        let defs_x: Vec<_> = g
            .vertices()
            .filter(|v| v.name == "x" && v.kind == VertexKind::Definition)
            .collect();
        assert!(!defs_x.is_empty());
        let has_iter_cd = defs_x
            .iter()
            .any(|d| d.cds.iter().any(|cd| cd.by_iteration));
        assert!(
            has_iter_cd,
            "Def(x) inside for-loop should have by_iteration=true CD"
        );
    }
}
