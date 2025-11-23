---
title: Adding a new rule to Jarl
---

This page will explain how to implement a new rule in Jarl.
Jarl is written in Rust, but this page will *not* explain how to set up or use Rust, this is an entirely different topic.
To get started with Rust, check out the [Rust book](https://doc.rust-lang.org/stable/book/).


## Getting ready

### Find the rule

So far, most (if not all) rules in Jarl come from [the list of rules available in `lintr`](https://lintr.r-lib.org/dev/reference/#individual-linters), so this is the first place to explore.
If you want to add a rule that is not in `lintr`, please [open an issue]() first.

Note that not all `lintr` rules are suitable for Jarl.
In particular, rules that are only about formatting (spaces before parenthesis, newlines between arguments, etc.) are **out of scope** for Jarl.
Moreover, you should look for rules that require "pattern detection" only, meaning that they don't need information about the rest of the code (or only very little).
For example, [`unreachable_code`]() is **out of scope** for now because we need a way to analyze the rest of the code, which Jarl doesn't have so far.
**If you are unsure about whether a rule can or should be implemented, open an issue first.**

### Get familiar with the rule

You may know the most common cases of this rule, but there might exist many corner cases making its implementation difficult.
Take a look at the relevant test file in the [`lintr` test suite]() to know more about those corner cases.

Additionally, Jarl uses [tree-sitter]() under the hood to parse and navigate the Abstract Syntax Tree (AST) of the code.
Having an idea of what this AST looks like is important when implementing the rule.
I suggest creating a small test R file containing one or two examples of code that violate this rule.
If you have the Air extension installed, you can do the command "Air: View Syntax Tree" to display the AST next to the code.

### Get up and running with Rust and Jarl:

You should have installed Rust and cloned Jarl.
Do `cargo check` or `cargo test` to know if you are correctly set up.


## Adding a new rule: basic steps

As an example for this entire tutorial, we will analyze [PR #182](https://github.com/etiennebacher/jarl/pull/182/files), which added the rule [`list2df`](https://jarl.etiennebacher.com/rules/list2df).
This PR adds a rule to replace calls like `do.call(cbind.data.frame, x)` by `list2DF(x)`.
Importantly, `list2DF()` was added in R 4.0.0.
I encourage you to check this PR as you advance in this tutorial.

Here's a basic idea of the workflow to add a new rule:

1. add it to the general list of rules
1. add it to the list of rules for the specific kind of node it targets (function calls, if conditions, for loops, etc.)
1. implement the rule
1. add tests
1. run cargo clippy
1. add it to the changelog
1. add it to the website docs and update those docs

From now on, all file paths refer to the subfolder `crates/jarl-core`.

### Add the new rule to the list of rules

There are two places to modify: `lints/mod.rs` and one file in the `analyze` folder.

`lints/mod.rs` contains the list of all rules provided by Jarl.
We can add a rule to the list:

```rust
pub(crate) mod list2df;

...

rule_table.enable("list2df", "PERF,READ", FixStatus::Safe, Some((4, 0, 0)));
```
This contains the rule name, the categories it belongs to (those are described above in the file), whether it has safe fix, unsafe fix, or no fix, and the optional R version from which it is available.

The file to modify in the `analyze` folder will depend on the rule: here, we look for calls to `do.call()`.
The arguments passed to the function are irrelevant, what matters is that this is a call, so we will modify the file `analyze/call.rs`:

```rust
use crate::lints::list2df::list2df::list2df;

...

if checker.is_rule_enabled("list2df") && !checker.should_skip_rule(node, "list2df") {
    checker.report_diagnostic(list2df(r_expr)?);
}
```

### Implement the rule

This is the hard part of the process.
It requires knowledge about the AST you want to parse and about the different functions available to us to navigate this AST.
The rule definition must be located in `lints/<rule_name>/<rule_name>.rs`, so in this example in `lints/list2df/list2df.rs`.

Let's start with a skeleton of this file:

```rust
use crate::diagnostic::*;
use crate::utils::{get_arg_by_name_then_position, get_arg_by_position, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct List2Df;

/// ## What it does
///
/// Checks for usage of `do.call(cbind.data.frame, x)`.
///
/// [...]
impl Violation for List2Df {
    fn name(&self) -> String {
        "list2df".to_string()
    }
    fn body(&self) -> String {
        "`do.call(cbind.data.frame, x)` is inefficient and can be hard to read.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `list2DF(x)` instead.".to_string())
    }
}

pub fn list2df(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {

}
```

Let's analyze this by blocks:

* the first lines import required crates and functions, and define a struct using the rule name (in TitleCase);
* then there is some documentation (truncated here for conciseness);
* the `impl` block is where we define the name and the main message (`body`) that will be used in the output of Jarl. Note that there is also a `suggestion()` function which is not always necessary.
* finally, we define the function where we parse the AST.

Writing this function is the hard part, so let's focus on this.
We start by extracting the important information from the `RCall` object.
In this example, we need both the function name and the arguments:

```rust
let function = ast.function()?;
let arguments = ast.arguments()?;
```

Note that it is sometimes shorter to use




Usually, a rule implementation contains a lot of early returns, such as "if the function name is not 'do.call' then stop here".





## Proposing your changes

Once all of this is done, it is time to open a PR!

*Note: if you need some guidance, early feedback, or simply want to store your changes in a branch, you can also open an incomplete PR.*

### PR title

[Find the link for PR titles like "feat:", "fix:", etc.]

### PR automated comments

Once you have opened a PR, you will receive three automated comments after a few minutes:

- code coverage: this checks that all the lines you added are covered by some tests. Try to ensure this is at 100%.
- ecosystem checks: every time there is a change in `jarl-core`, Jarl is run on several R packages and the results are compared to the Jarl version on the main branch. You will have a comment indicating if your changes have revealed new violations or removed some violations in those repositories. Here, since we added a rule, we expect either no changes or more violations. New violations will be printed with a link to the lines of code that trigger them, so check a few to ensure those are not false positives.
- benchmark: this is usually irrelevant when adding a new rule, it is simply to ensure there's no catastrophic performance degradation.


Congrats, and thanks for your contribution!
