## How does Ruff work?

This file is for my own understand of Ruff so I can try to mimick it as best as possible.
Ruff is a massive project and it's hard to see how it actually works.
I'm writing this because I learn something better when I have to explain it.


### Getting settings

There are several kinds of settings:

1. which rules should be applied?
1. which files to parse?
1. is caching enabled?
1. should fixes be applied?

There is a struct [`RuleTable`](https://github.com/astral-sh/ruff/blob/main/crates/ruff_linter/src/settings/rule_table.rs) that contains two RuleSets (which are basically lists): one to know if a specific rule is enabled, and one to know if a specific rule has an associated fix.

Caching is mostly handled in `ruff_cache` but there's also an enum [Cache](https://github.com/astral-sh/ruff/blob/170ccd80b439f22c7e08af8ed7506540687aa265/crates/ruff_linter/src/settings/flags.rs#L27).


### Parsing files

TODO

I have a hard time explaining how this works, there are [multiple Structs related to paths and file patterns](https://github.com/astral-sh/ruff/blob/main/crates/ruff_linter/src/settings/types.rs).


### Checking files

There are two important functions for this:

* [`check()`](https://github.com/astral-sh/ruff/blob/c7372d218de365c8298afb37530ee26999ba91b0/crates/ruff/src/commands/check.rs) takes the list of paths, config args, cache, etc. It resolves the paths, does more checks, runs `lint_path()` in parallel, aggregates the output and reports it.
* `lint_path()` is where we get the cached output if the file didn't change, and then run the lint functions. Depending on whether we lint only or fix the file as well, this calls [`lint_only()`](https://github.com/astral-sh/ruff/blob/2362263d5e4db815d18e80e599a09f367e53fb92/crates/ruff_linter/src/linter.rs#L462) or `lint_fix()`. Both of those functions call `check_path()` to generate the diagnostics (and lint_fix() then parses those diagnostics and loops through fixes until stability).

`check_path()` creates a `context` that accumulates all the diagnostics.
Again, it dispatches checks between several functions:

- `check_tokens()`: for cases like  empty comment, invalid character, useless semicolon, etc.
- `check_filepath()`: for cases such as checking module imports, this kind of thing. I think I can ignore it.
- `check_logical_lines()`: whitespace after bracket, etc.
- `check_ast()` => the one I'm interested in.


### Checking the AST

Taken from [this file](https://github.com/astral-sh/ruff/blob/c7372d21/crates/ruff_linter/src/checkers/ast/mod.rs#L1-L13):

> [`Checker`] for AST-based lint rules.
>
> The [`Checker`] is responsible for traversing over the AST, building up the [`SemanticModel`],
> and running any enabled [`Rule`]s at the appropriate place and time.
>
> The [`Checker`] is structured as a single pass over the AST that proceeds in "evaluation" order.
> That is: the [`Checker`] typically iterates over nodes in the order in which they're evaluated
> by the Python interpreter. This includes, e.g., deferring function body traversal until after
> parent scopes have been fully traversed. Individual rules may also perform internal traversals
> of the AST.
>
> The individual [`Visitor`] implementations within the [`Checker`] typically proceed in four
> steps:
>
> 1. Binding: Bind any names introduced by the current node.
> 2. Traversal: Recurse into the children of the current node.
> 3. Clean-up: Perform any necessary clean-up after the current node has been fully traversed.
> 4. Analysis: Run any relevant lint rules on the current node.
>
> The first three steps together compose the semantic analysis phase, while the last step
> represents the lint-rule analysis phase. In the future, these steps may be separated into
> distinct passes over the AST.

Since I focus on lint-rule analysis for now and ignore the semantic analysis. Only point 4 is interesting for us.

There is one giant Checker struct, with some fields that aren't necessary here, either because they are related to the formatter, to the semantic analysis, or to Python itself:

```rust
pub(crate) struct Checker<'a> {
    /// The [`Parsed`] output for the source code.
    parsed: &'a Parsed<ModModule>,
    /// An internal cache for parsed string annotations
    parsed_annotations_cache: ParsedAnnotationsCache<'a>,
    /// The [`Parsed`] output for the type annotation the checker is currently in.
    parsed_type_annotation: Option<&'a ParsedAnnotation>,
    /// The [`Path`] to the file under analysis.
    path: &'a Path,
    /// The [`Path`] to the package containing the current file.
    package: Option<PackageRoot<'a>>,
    /// The [`flags::Noqa`] for the current analysis (i.e., whether to respect suppression
    /// comments).
    noqa: flags::Noqa,
    /// The [`NoqaMapping`] for the current analysis (i.e., the mapping from line number to
    /// suppression commented line number).
    noqa_line_for: &'a NoqaMapping,
    /// The [`LinterSettings`] for the current analysis, including the enabled rules.
    pub(crate) settings: &'a LinterSettings,
    /// The [`Locator`] for the current file, which enables extraction of source code from byte
    /// offsets.
    locator: &'a Locator<'a>,
    /// The [`Stylist`] for the current file, which detects the current line ending, quote, and
    /// indentation style.
    stylist: &'a Stylist<'a>,
    /// The [`Indexer`] for the current file, which contains the offsets of all comments and more.
    indexer: &'a Indexer,
    /// The [`Importer`] for the current file, which enables importing of other modules.
    importer: Importer<'a>,
    /// A set of deferred nodes to be visited after the current traversal (e.g., function bodies).
    visit: deferred::Visit<'a>,
    /// A set of deferred nodes to be analyzed after the AST traversal (e.g., `for` loops).
    analyze: deferred::Analyze,
    /// The cumulative set of diagnostics computed across all lint rules.
    diagnostics: RefCell<Vec<Diagnostic>>,
    /// The end offset of the last visited statement.
    last_stmt_end: TextSize,
    /// The target [`PythonVersion`] for version-dependent checks.
    target_version: PythonVersion,


    /////////// Etienne: not needed
    pub(crate) module: Module<'a>,
    pub(crate) source_type: PySourceType,
    cell_offsets: Option<&'a CellOffsets>,
    notebook_index: Option<&'a NotebookIndex>,
    semantic: SemanticModel<'a>,
    flake8_bugbear_seen: RefCell<FxHashSet<TextRange>>,
    docstring_state: DocstringState,
    semantic_checker: SemanticSyntaxChecker,
    semantic_errors: RefCell<Vec<SemanticSyntaxError>>,
    ///////////

}
```
The interesting part, where the checks actually happen, is the `visit_*()` functions at the end of this file, e.g. `visit_stmt()`.
A big part of those functions is dedicated to steps 1-3 mentioned above in which I'm not interested.
The lint analysis is done by a single line: `analyze::statement(stmt, self)`.
This calls the functions in the file [`analyze/statement.rs`](https://github.com/astral-sh/ruff/blob/c7372d218de365c8298afb37530ee26999ba91b0/crates/ruff_linter/src/checkers/ast/analyze/statement.rs).

This file contains a huge `match` call with every type of statement (for loop, if condition, function definition, assign, expr, etc.).
In each of those types of statement, they check whether a relevant rule is enabled or not, and apply it if it is.

This was for `visit_stmt()`, but there are other visiting functions, such as `visit_expr()`.
This follows the same org as before: the large majority of the function is dedicated to the semantic analysis, and then a few lines are dedicated to linting rules.
There's `analyze::expression(expr, self)` which does the same as `analyze::statement(expr, self)` but this time the `match` call is made on the type of expression (tuple, list, dict, f-string, etc.).

There are a bunch of `visit_*()` functions: `visit_parameter()`, `visit_parameters()`, `visit_body()`, `visit_f_string_element()`, etc.

Most of the rest of the file is dedicated to deferred visit, which concerns semantic analysis.

The final function of the file is [`check_ast()`](https://github.com/astral-sh/ruff/blob/c7372d218de365c8298afb37530ee26999ba91b0/crates/ruff_linter/src/checkers/ast/mod.rs#L2865), which takes a path, builds a Checker, and applies the checks.
A bunch of those are related to semantic analysis, but those I'm probably interested in is `visit_body()`.
This function calls `analyze::suite(body, self)` but I'm not interested in this (I'm not sure why the two checks in this function are here).
The important part is the for loop:

```rust
fn visit_body(&mut self, body: &'a [Stmt]) {
    // Step 4: Analysis
    analyze::suite(body, self);

    // Step 2: Traversal
    for stmt in body {
        self.visit_stmt(stmt);
    }
}
```

PROBLEM: `visit_body()` is not enough. I need to go through the deferred analysis as well because not everything there is related to semantic analysis.
For instance, we have:

```
check_ast()
-> visit_deferred()
-> visit_deferred_functions()
-> visit_parameters()
```
and `visit_parameters()` has a part dedicated to semantic analysis but also a call to `analyze::parameters(parameters, self)`.
I guess `analyze/parameters.rs` is where we could detect duplicated arg names for instance.


SO I GUESS the big question is: should I keep the deferred part?
They do this because the semantic analysis absolutely needs to evaluate things in the right order, meaning all the top level objects before going in the body of functions for instance.
Since I don't do semantic analysis but only pattern based detection, could I bypass that?


### Reporting


Ruff has a struct [Message](https://github.com/astral-sh/ruff/blob/c7372d218de365c8298afb37530ee26999ba91b0/crates/ruff_linter/src/message/mod.rs):


```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    Diagnostic(DiagnosticMessage),
    SyntaxError(SyntaxErrorMessage),
}
```

This contains two types of messages: Diagnostic and SyntaxError.

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticMessage {
    pub kind: DiagnosticKind,
    pub range: TextRange,
    pub fix: Option<Fix>,
    pub parent: Option<TextSize>,
    pub file: SourceFile,
    pub noqa_offset: TextSize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxErrorMessage {
    pub message: String,
    pub range: TextRange,
    pub file: SourceFile,
}
```

`Message` has a lot of methods, for instance to return the message itself, the suggested fix, whether the violation has a fix or not, etc.

### Fixing files

Important structs:

* [`FixResult`](https://github.com/astral-sh/ruff/blob/c7372d218de365c8298afb37530ee26999ba91b0/crates/ruff_linter/src/fix/mod.rs) contains the output, after applying the fixes:

```rust
pub(crate) struct FixResult {
    /// The resulting source code, after applying all fixes.
    pub(crate) code: String,
    /// The number of fixes applied for each [`Rule`].
    pub(crate) fixes: FixTable,
    /// Source map for the fixed source code.
    pub(crate) source_map: SourceMap,
}
```


Two main functions:

* `fix_file()` takes a list of `Message` that contain everything needed (original code, location, replacement, fix applicability, etc.), and only keeps those that have the correct fix applicability (for instance allowing or disallowing unsafe fixes). It runs `apply_fixes()` on those.

* `apply_fixes()` is where the replacement actually happens. It goes through every pair of rule-fix but first they [sort them](https://github.com/astral-sh/ruff/blob/c7372d218de365c8298afb37530ee26999ba91b0/crates/ruff_linter/src/fix/mod.rs#L73) (see below).
There are also the concepts of "edits" and "isolation". Importantly, fixes that concern locations that already received another fix are skipped.
So basically they start from the top of the file and apply fixes in order of appearance (I suppose this is what the "sort" mentioned above is about).
When they apply a fix, they modify the last position modified and the next fix to be applied cannot be applied before this last_pos.

Concepts that I need to understand:

* Locator: a struct that takes the content of a file as well as the index of line start positions. This index is `OnceCell<LineIndex>`. `OnceCell` kinda allows to cache results so that we access LineIndex only once. LineIndex has many helpers to quickly convert an offset to a (line, col) position. It can also return the entire line that matches a certain offset (let's say we have a linebreak at offset 10 and we ask for the entire line at offset 15, then it returns bytes 10-end of line).
* what is the difference between "fix" and "edit"? An Edit is a single action: replace, insert, or delete a specific content at a specific TextRange. A Fix contains all the edits necessary to fix the lint, so it may contain one or more Edits.
* isolation

How do they [sort fixes](https://github.com/astral-sh/ruff/blob/c7372d218de365c8298afb37530ee26999ba91b0/crates/ruff_linter/src/fix/mod.rs#L133)? They compare 2 rules and their fixes but actually the 2 rules are only useful in some specific cases (so unclear whether I'll actually need that).
Then they only compare the start index of each fix.
