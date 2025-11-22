---
title: Adding a new rule to Jarl
---

This page will explain how to implement a new rule in Jarl.
Jarl is written in Rust, but this page will *not* explain how to set up or use Rust, this is an entirely different topic.
To get started with Rust, check out the [Rust book](https://doc.rust-lang.org/stable/book/).


## Pre-work

1. check the list of rules in `lintr`: https://lintr.r-lib.org/dev/reference/#individual-linters

    - check they are not already implemented in Jarl: https://jarl.etiennebacher.com/rules
    - rules that are only about formatting (spaces before parenthesis, newlines between arguments, etc.) are **out of scope** for Jarl;
    - look for rules that require "pattern detection" only, meaning that they don't need information about the rest of the code (or only very little). For example, `unreachable_code` is **out of scope** for now because we need a way to analyze the rest of the code to determine that some code cannot be reached (e.g. it comes right after a `return()` statement). **If you are unsure about whether a rule can or should be implemented, open an issue first.**

1. get familiar with the rule:

    - you may know the most common cases of this rule, but there might be many corner cases making its implementation difficult. Take a look at the relevant test file in the [`lintr` test suite]() to know more about those corner cases.
    - Jarl uses [tree-sitter]() under the hood to parse and navigate the Abstract Syntax Tree (AST) of the code. Having an idea of what this AST looks like is important when implementing the rule. I suggest you store one or two examples of code that violate this rule. If you have the Air extension installed, you can do the command "Air: View Syntax Tree" to display the AST next to the code.


1. get up and running with Rust and Jarl:

    - have Rust set up
    - clone Jarl and do `cargo check` or `cargo test`. You will quickly know if your setup is ready.

As an example for this entire tutorial, we will analyze [PR #182](https://github.com/etiennebacher/jarl/pull/182), which added the rule [`list2df`](https://jarl.etiennebacher.com/rules/list2df).

## Adding a new rule: basic steps

From now on, all file paths refer to the subfolder `crates/jarl-core`.

Here's a basic idea of the workflow to add a new rule:

1. add it to the general list of rules
1. add it to the list of rules for the specific kind of node it targets (function calls, if conditions, for loops, etc.)
1. implement the rule
1. add tests
1. run cargo clippy
1. add it to the changelog
1. add it to the website docs and update those docs

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
