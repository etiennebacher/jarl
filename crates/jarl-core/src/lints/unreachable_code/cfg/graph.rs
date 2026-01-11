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
}

/// How control flow exits a basic block
#[derive(Debug, Clone)]
pub enum Terminator {
    /// No terminator yet (block under construction)
    None,

    /// Unconditional jump to another block
    Goto,

    /// Return from function (exits the CFG)
    Return,

    /// Throw an error from function (exits the CFG)
    Stop,

    /// Break statement (exits innermost loop)
    Break,

    /// Next statement (continue to next iteration)
    Next,

    /// Conditional branch (if/else)
    Branch,

    /// Loop (for/while/repeat)
    Loop,
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
}

impl ControlFlowGraph {
    pub fn new() -> Self {
        let entry = BasicBlock::new(BlockId(0));
        let exit = BasicBlock::new(BlockId(1));

        Self {
            blocks: vec![entry, exit],
            entry: BlockId(0),
            exit: BlockId(1),
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
