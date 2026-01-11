use air_r_syntax::{RSyntaxNode, TextRange};
use std::fmt;

/// Unique identifier for a basic block in the control flow graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

/// A basic block: a sequence of statements with a single entry and exit point
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    /// Statements in this block (in execution order)
    pub statements: Vec<RSyntaxNode>,
    /// Blocks that can execute after this one
    pub successors: Vec<BlockId>,
    /// Blocks that can execute before this one
    pub predecessors: Vec<BlockId>,
    /// How control flow exits this block
    pub terminator: Terminator,
    /// Text range covering this block (for diagnostics)
    pub range: Option<TextRange>,
}

impl BasicBlock {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            statements: Vec::new(),
            successors: Vec::new(),
            predecessors: Vec::new(),
            terminator: Terminator::None,
            range: None,
        }
    }

    /// Check if this block is empty (no statements and no terminator)
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty() && matches!(self.terminator, Terminator::None)
    }
}

/// How control flow exits a basic block
#[derive(Debug, Clone)]
pub enum Terminator {
    /// No terminator yet (block under construction)
    None,

    /// Unconditional jump to another block
    Goto(BlockId),

    /// Return from function (exits the CFG)
    Return {
        node: RSyntaxNode,
    },

    /// Break statement (exits innermost loop)
    Break {
        node: RSyntaxNode,
        target: BlockId, // Block after the loop
    },

    /// Next statement (continue to next iteration)
    Next {
        node: RSyntaxNode,
        target: BlockId, // Loop condition/header
    },

    /// Conditional branch (if/else)
    Branch {
        condition: RSyntaxNode,
        then_block: BlockId,
        else_block: BlockId,
    },

    /// Loop (for/while/repeat)
    Loop {
        condition: Option<RSyntaxNode>, // None for repeat loops
        body: BlockId,
        after: BlockId, // Block after loop exits
    },
}

impl Terminator {
    /// Get the successor blocks from this terminator
    pub fn successors(&self) -> Vec<BlockId> {
        match self {
            Terminator::None => vec![],
            Terminator::Goto(target) => vec![*target],
            Terminator::Return { .. } => vec![],
            Terminator::Break { target, .. } => vec![*target],
            Terminator::Next { target, .. } => vec![*target],
            Terminator::Branch { then_block, else_block, .. } => {
                vec![*then_block, *else_block]
            }
            Terminator::Loop { body, after, .. } => vec![*body, *after],
        }
    }
}

/// Control Flow Graph for a function
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    /// All basic blocks in the graph
    pub blocks: Vec<BasicBlock>,
    /// Entry block (where execution starts)
    pub entry: BlockId,
    /// Exit block (implicit return point)
    pub exit: BlockId,
    /// The function node this CFG represents
    pub function_node: RSyntaxNode,
}

impl ControlFlowGraph {
    pub fn new(function_node: RSyntaxNode) -> Self {
        let entry = BasicBlock::new(BlockId(0));
        let exit = BasicBlock::new(BlockId(1));

        Self {
            blocks: vec![entry, exit],
            entry: BlockId(0),
            exit: BlockId(1),
            function_node,
        }
    }

    /// Get a block by its ID
    pub fn block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.blocks.get(id.0)
    }

    /// Get a mutable block by its ID
    pub fn block_mut(&mut self, id: BlockId) -> Option<&mut BasicBlock> {
        self.blocks.get_mut(id.0)
    }

    /// Create a new basic block and add it to the graph
    pub fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.blocks.len());
        self.blocks.push(BasicBlock::new(id));
        id
    }

    /// Add an edge from one block to another
    pub fn add_edge(&mut self, from: BlockId, to: BlockId) {
        if let Some(from_block) = self.block_mut(from) {
            if !from_block.successors.contains(&to) {
                from_block.successors.push(to);
            }
        }
        if let Some(to_block) = self.block_mut(to) {
            if !to_block.predecessors.contains(&from) {
                to_block.predecessors.push(from);
            }
        }
    }

    /// Get all blocks that are not the entry or exit
    pub fn regular_blocks(&self) -> impl Iterator<Item = &BasicBlock> {
        self.blocks
            .iter()
            .filter(|b| b.id != self.entry && b.id != self.exit)
    }
}

impl fmt::Display for ControlFlowGraph {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "CFG for function:")?;
        writeln!(f, "  Entry: {}", self.entry)?;
        writeln!(f, "  Exit: {}", self.exit)?;
        writeln!(f, "  Blocks:")?;
        for block in &self.blocks {
            writeln!(f, "    {}: {} statements", block.id, block.statements.len())?;
            if !block.successors.is_empty() {
                write!(f, "      -> ")?;
                for (i, succ) in block.successors.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", succ)?;
                }
                writeln!(f)?;
            }
        }
        Ok(())
    }
}
