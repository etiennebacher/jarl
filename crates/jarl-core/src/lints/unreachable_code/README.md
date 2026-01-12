# Unreachable Code Detection

This module detects code that can never be executed in R functions using Control Flow Graph (CFG) analysis.

## Overview

The unreachable code detector identifies two main types of unreachable code:

1. **Code after control flow terminators**: Code following `return`, `break`, or `next` statements
2. **Dead branches**: Code in conditional branches with constant conditions (e.g., `if (TRUE)` or `if (FALSE)`)

## Architecture

The module is organized into three main components:

```
unreachable_code/
├── cfg/                    # Control Flow Graph infrastructure
│   ├── builder.rs         # Constructs CFG from R AST
│   ├── graph.rs           # CFG data structures
│   ├── reachability.rs    # Analyzes reachability
│   └── mod.rs             # Module exports
├── unreachable_code.rs    # Main lint implementation
└── mod.rs                 # Tests
```

## Control Flow Graph (CFG)

### What is a CFG?

A Control Flow Graph is a directed graph representation of all paths that might be traversed through a function during its execution. Each node in the graph is a **basic block** - a sequence of statements that always execute together (single entry, single exit).

### Basic Blocks

A **basic block** contains:
- **Statements**: A list of R syntax nodes executed sequentially
- **Successors**: Blocks that can execute after this one
- **Predecessors**: Blocks that can execute before this one
- **Terminator**: How control flow exits this block

### Edges

An **edge** is a directed connection between two basic blocks that represents a possible path of execution. Edges connect a block's successors list to another block's predecessors list:

- **Edge existence**: When block A has an edge to block B, then B appears in A's successors list AND A appears in B's predecessors list
- **Reachability**: A block is reachable if there exists a path of edges from the entry block to it
- **Unreachable blocks**: Some blocks may have predecessors listed but no actual incoming edges. This occurs for unreachable code after terminators or in dead branches from constant conditions

Example:
```
Block A --edge--> Block B
  (A.successors contains B)
  (B.predecessors contains A)
```

### Terminators

Terminators describe how control flow exits a basic block:

- **`None`**: Block under construction
- **`Goto`**: Unconditional jump to another block
- **`Return`**: Exits the function
- **`Stop`**: Throws an error and exits (e.g., `stop()`, `abort()`, `cli_abort()`)
- **`Break`**: Exits the innermost loop
- **`Next`**: Continues to the next loop iteration
- **`Branch`**: Conditional branch (if/else)
- **`Loop`**: Loop construct (for/while/repeat)

### CFG Structure

Every CFG has two special blocks:
- **Entry block**: Where function execution starts
- **Exit block**: Represents the end of the function

Example CFG for a simple function:

```r
foo <- function(x) {
  if (x > 0) {
    return(1)
  }
  return(0)
}
```

```
┌─────────┐
│  Entry  │
│  (bb0)  │
└────┬────┘
     │
     ▼
┌─────────────────┐
│   if (x > 0)    │
│   Branch        │
│     (bb2)       │
└────┬───────┬────┘
     │       │
   true    false
     │       │
     ▼       ▼
┌────────┐ ┌────────┐
│return 1│ │return 0│
│ (bb3)  │ │ (bb4)  │
└────────┘ └────────┘
     │       │
     └───┬───┘
         ▼
    ┌────────┐
    │  Exit  │
    │ (bb1)  │
    └────────┘
```

## Unreachable Code Detection Strategy

### Step 1: Build the CFG

The `CfgBuilder` traverses the R function's AST and constructs a CFG:

1. **Create entry and exit blocks**
2. **Process each statement** in the function body:
   - Regular statements are added to the current block
   - Control flow statements create new blocks and edges
3. **Handle special cases**:
   - When a `return`/`break`/`next`/`stop()` is encountered, remaining statements in that scope are added to a new unreachable block (with a predecessor pointer but no edge)
   - Constant conditions (`if (TRUE)`, `if (FALSE)`) create blocks for dead branches
   - When processing statements in an unreachable block (no incoming edges), statements are added directly without recursive structure building, ensuring all unreachable code is grouped together

#### Identifying Control Flow Statements

When processing `R_CALL` nodes, the builder identifies control flow statements by examining the function name:

- **Recognized terminators**:
  - `return()` → Return terminator
  - `stop()`, `abort()`, `.Defunct()`, `cli_abort()` → Stop terminator
  - `break`, `next` → Loop control terminators

### Step 2: Find Reachable Blocks (BFS)

**BFS (Breadth-First Search)** is a graph traversal algorithm that explores nodes level by level:

1. Start with the entry block in a queue
2. Mark it as visited
3. While the queue is not empty:
   - Remove a block from the queue
   - For each successor of that block:
     - If not visited, mark as visited and add to queue

After BFS completes, any block not visited is unreachable.

**Why BFS?** BFS is efficient (O(V + E) where V = blocks, E = edges) and guarantees we find all reachable blocks in a single pass.

Example:

```r
foo <- function() {
  return(1)
  x <- 2    # Unreachable block created here
}
```

CFG:
```
Entry (bb0) → Block with return (bb2) → Exit (bb1)
                                      ↘ Unreachable block (bb3) [no edge!]
```

BFS visits: `bb0 → bb2 → bb1`
Not visited: `bb3` (unreachable!)

### Step 3: Determine Unreachability Reason

For each unreachable block, we determine **why** it's unreachable by examining its context in priority order:

1. **Check predecessors for direct terminators** (highest priority):
   - If a predecessor has a `Return` terminator → `AfterReturn`
   - If a predecessor has a `Stop` terminator → `AfterStop`
   - If a predecessor has a `Break` terminator → `AfterBreak`
   - If a predecessor has a `Next` terminator → `AfterNext`

2. **Check if predecessor is a branch where all successors terminate** → `AfterBranchTerminating`:
   - For if/else where both branches end with `return`/`stop()`, traverse all branch successors to verify they all terminate
   - Example: `if (x) { return(1) } else { return(2) }` followed by unreachable code

3. **Check for dead branches** → `DeadBranch`:
   - If the block has predecessors in the `predecessors` list but no actual edges (from successors)
   - This occurs with constant conditions: `if (TRUE)` or `if (FALSE)`

4. **Fallback** → `NoPathFromEntry`:
   - Used when no specific reason can be determined

### Step 4: Group Contiguous Unreachable Code

Multiple consecutive unreachable statements with the same reason are combined into a single diagnostic:

```r
foo <- function() {
  return(1)
  x <- 2    # ┐
  y <- 3    # ├─ All grouped into one diagnostic
  z <- 4    # ┘
}
```

The grouping algorithm:
1. Iterate through blocks in order
2. For each unreachable block, check if it can be merged with the current group:
   - Same reason? → Extend the text range
   - Different reason? → Emit current group, start new one
3. When a reachable block is encountered → Emit current group
4. At the end → Emit any remaining group

This provides better user experience (1 diagnostic instead of 3) and improves performance.

## Constant Condition Detection

For dead branch detection, we evaluate if conditions are constants:

```r
if (TRUE)   → TRUE
if (FALSE)  → FALSE
if (x > 0)  → None (not constant)
```

When a constant condition is detected:
- For `if (TRUE)`: Only the `then` branch gets an edge; the `else` branch has no edge but a predecessor pointer
- For `if (FALSE)`: Only the `else` branch gets an edge; the `then` branch has no edge but a predecessor pointer

## Examples

### Example 1: Code After Return

```r
foo <- function() {
  return(1)
  x <- 2
  y <- 3
}
```

**Diagnostic**: Lines 3-4 highlighted as unreachable after return statement.

### Example 2: Dead Branch

```r
bar <- function() {
  if (TRUE) {
    "always"
  } else {
    "never"
  }
}
```

**Diagnostic**: Line 5 highlighted as unreachable due to constant condition.

### Example 3: Code After Terminating Branch

```r
baz <- function(x) {
  if (x > 0) {
    return(1)
  } else {
    return(-1)
  }
  print("never reached")
}
```

**Diagnostic**: Line 7 highlighted as unreachable because all branches of the if/else terminate.

### Example 4: Nested Functions

```r
outer <- function() {
  inner <- function() {
    return(1)
    x <- 2  # Unreachable in inner
  }
  y <- 3    # Reachable in outer
}
```

**Diagnostic**: Only line 4 flagged (each function has its own CFG).

## Performance Characteristics

- **CFG Construction**: O(N) where N = number of statements in function
- **Reachability Analysis**: O(V + E) where V = blocks, E = edges
- **Grouping**: O(B) where B = number of blocks

Overall: **O(N)** - linear in the size of the function.

## Limitations

1. **Constant folding is basic**: Only detects literal `TRUE`/`FALSE`, not expressions like `1 == 1` or aliases like `T`/`F`
2. **While loops with constant conditions**: Not currently detected (e.g., `while (FALSE) { ... }`)
3. **Limited stop-like functions**: Only recognizes `stop()`, `abort()`, and `cli_abort()` as functions that never return

## Future Enhancements

Potential improvements:
- Detect `while (FALSE)` loops
- Recognize more functions that never return (e.g., `quit()`, `q()`, custom error functions)
- More sophisticated constant expression evaluation (e.g., `1 == 1`, `TRUE && FALSE`)
- Detect unreachable code after infinite loops (e.g., `repeat { }` with no break)
- Support `T` and `F` as aliases for `TRUE` and `FALSE` (currently not detected as they can be redefined)

## References

- Control Flow Analysis: https://en.wikipedia.org/wiki/Control-flow_analysis
- Breadth-First Search: https://en.wikipedia.org/wiki/Breadth-first_search
- Basic Blocks: https://en.wikipedia.org/wiki/Basic_block
