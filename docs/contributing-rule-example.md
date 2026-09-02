---
title: Adding a new rule to Jarl
---

This page will explain how to implement a new rule in Jarl.
It is recommended to read the [General information page](contributing.md) first to install the required tools.
Jarl is written in Rust, but this page will *not* explain how to set up or use Rust, this is an entirely different topic.
To get started with Rust, check out the [Rust book](https://doc.rust-lang.org/stable/book/).


## Getting ready

### Find the rule

So far, most (if not all) rules in Jarl come from [the list of rules available in `lintr`](https://lintr.r-lib.org/dev/reference/#individual-linters), so this is the first place to explore.
If you want to add a rule that is not in `lintr`, please [open an issue](https://github.com/etiennebacher/jarl/issues/new/choose) first.

Note that not all `lintr` rules are suitable for Jarl.
In particular, rules that are only about formatting (spaces before parentheses, newlines between arguments, etc.) are **out of scope** for Jarl.

**If you are unsure about whether a rule can or should be implemented, open an issue first.**

### Get familiar with the rule

You may know the most common cases of this rule, but there might exist many corner cases making its implementation difficult.
Take a look at the relevant test file in the [`lintr` test suite](https://github.com/r-lib/lintr/tree/main/tests/testthat) to know more about those corner cases.

Additionally, Jarl uses [tree-sitter](https://tree-sitter.github.io/tree-sitter/) under the hood to parse and navigate the Abstract Syntax Tree (AST) of the code.
Having an idea of what this AST looks like is important when implementing the rule.
I suggest creating a small test R file containing one or two examples of code that violate this rule.
If you have the Air extension installed, you can do the command "Air: View Syntax Tree" to display the AST next to the code.

For example, the code `any(x > 10)` gives this AST:

```
0: R_ROOT@0..12
  0: (empty)
  1: R_EXPRESSION_LIST@0..11
    0: R_CALL@0..11
      0: R_IDENTIFIER@0..3
        0: IDENT@0..3 "any" [] []
      1: R_CALL_ARGUMENTS@3..11
        0: L_PAREN@3..4 "(" [] []
        1: R_ARGUMENT_LIST@4..10
          0: R_ARGUMENT@4..10
            0: (empty)
            1: R_BINARY_EXPRESSION@4..10
              0: R_IDENTIFIER@4..5
                0: IDENT@4..5 "x" [] []
              1: GREATER_THAN@5..7 ">" [Whitespace(" ")] []
              2: R_DOUBLE_VALUE@7..10
                0: R_DOUBLE_LITERAL@7..10 "10" [Whitespace(" ")] []
        2: R_PAREN@10..11 ")" [] []
  2: EOF@11..12 "" [Newline("\n")] []
```

We can see that this is indeed a tree where each element can have "parents" and "children" nodes.
It also shows that the textual representation of the code doesn't matter here: you could write `any(x > 10)` or `any  ( x    >    10)` and the AST would be the same (with the exception of the numbers representing the location of each node in text).
This representation is important to keep in mind when adding a new rule: Jarl only checks the text in very specific cases, for example when we want to get the function name to know if we should apply a rule or not.
Most of the time, the `RSyntaxKind` (`R_CALL`, `R_BINARY_EXPRESSION`, `R_IDENTIFIER`, etc.) is used.


### Get up and running with Rust and Jarl

You should have installed Rust and cloned Jarl.
Do `cargo check` or `cargo test` to know if you are correctly set up.


## Adding a new rule: basic steps

As an example for this entire tutorial, we will analyze [PR #182](https://github.com/etiennebacher/jarl/pull/182/files), which added the rule [`list2df`](rules/list2df.md).
This PR adds a rule to replace calls like `do.call(cbind.data.frame, x)` by `list2DF(x)`.
Importantly, `list2DF()` was added in R 4.0.0.

**Note that the code and the structure described below have slightly evolved since this PR, but the implementation is still similar** (you can see other PRs that implemented new rules [here](https://github.com/etiennebacher/jarl/pulls?q=is%3Apr+label%3Anew-rule+)).

Here's a basic idea of the workflow to add a new rule:

1. add it to the general list of rules
1. add it to the list of rules for the specific kind of node it targets (function calls, if conditions, for loops, etc.)
1. implement the rule
1. add tests
1. document the rule
1. final polishing

From now on, all file paths refer to the subfolder `crates/jarl-core`.

::: {.callout-note}
## Trying your new rule locally

As we progress in the implementation of a new rule, it can be very helpful to have a small R file on which we can run the new rule.
This allows us to see whether the behavior is correct, before implementing proper tests.

This file should contain one or two examples of code that should be reported and code that shouldn't, so that we can quickly detect false positives and false negatives.
We can create `test.R` at the root of the project (this file is already listed in `.gitignore`) and call:

```
cargo run --bin jarl -- check test.R --select <my_rule_name>
```

to run the current implementation on this file.
:::


### Add the new rule to the list of rules

There are three places to modify: `rule_set.rs`, `lints/base/mod.rs` (note that `base` could be another of the `lints` subfolders, depending on the rule), and one file in the `analyze` folder.

`rule_set.rs` contains the list of all rules provided by Jarl, including their metadata: whether they have a fix or not, whether they are enabled by default, the group(s) they belong to, and an optional minimum R version required for the rule to be enabled:

```rust
declare_rules! {
    ...
    List2df => {
        name: "list2df",
        categories: [Perf, Read],
        default: Enabled,
        fix: Safe,
        min_r_version: Some((4, 0, 0)),
    },
    ...
}
```

We also need to add the following line in `lints/base/mod.rs`:

```rust
pub(crate) mod list2df;
```

The file to modify in the `analyze` folder will depend on the rule: here, we look for calls to `do.call()`.
The arguments passed to the function are irrelevant, what matters is that this is a call, so we will modify the file `analyze/call.rs`:

```rust
use crate::lints::list2df::list2df::list2df;

...
if checker.is_rule_enabled(Rule::List2df) {
    checker.report_diagnostic(list2df(r_expr, fn_name)?);
}
...
```

Note that `analyze/call.rs` computes the function name once per call and passes it to every rule as `fn_name`, so rules dispatched from there don't have to extract it themselves.

### Implement the rule

This is the hard part of the process.
It requires knowledge about the AST you want to parse and about the different functions available to us to navigate this AST.
The rule definition must be located in `lints/base/<rule_name>/<rule_name>.rs`, so in this example in `lints/base/list2df/list2df.rs`.

Let's start with a skeleton of this file:

```rust
use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::{Formals, get_arg, get_arg_by_position, node_contains_comments};
use air_r_syntax::*;
use biome_rowan::AstNode;

/// Signature of the function the rule targets, used to match its arguments.
const FORMALS_DO_CALL: Formals = &["what", "args", "quote", "envir"];

pub struct List2Df;

/// Version added: 0.1.2
///
/// ## What it does
///
/// Checks for usage of `do.call(cbind.data.frame, x)`.
///
/// [...]
impl Violation for List2Df {
    fn rule(&self) -> Rule {
        Rule::List2df
    }
    fn body(&self) -> String {
        "`do.call(cbind.data.frame, x)` is inefficient and can be hard to read.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Use `list2DF(x)` instead.".to_string())
    }
}

pub fn list2df(ast: &RCall, fn_name: &str) -> anyhow::Result<Option<Diagnostic>> {

}
```

Let's analyze this by blocks:

* the first lines import required crates and functions, declare the signature of the function we target (more on this below), and define a struct using the rule name (in TitleCase);
* then there is some documentation (truncated here for conciseness). The version number corresponds to the next version, not the current one.
* the `impl` block is where we link the violation to its `Rule` variant (the one declared in `rule_set.rs`) and define the main message (`body`) that will be used in the output of Jarl. Note that there is also a `suggestion()` function which is not always necessary.
* finally, we define the function where we parse the AST.

::: {.callout-note collapse="true"}
## About `impl Violation`

If you explore other rules implementation, you might notice that the `impl Violation` block is sometimes missing.
This is because in some cases, the message and/or the suggestion depend on the AST itself.
For example, for the `assignment` rule, the message will recommend the use of `<-` or `=` depending on the user settings.

In this scenario, the rule, body, and suggestion are defined at the very end, when we build the `Diagnostic`.
:::


Writing this function is the hard part, so let's focus on this.
Usually, a rule implementation contains a lot of early returns, such as "if the function name is not 'xyz' then stop here".
In this example, we want to focus on calls to `do.call()`, so we can stop early if this is not the function name:

```rust
if fn_name != "do.call" {
    return Ok(None);
}
```

The name comes from the caller here, but it is computed by the helper `get_function_name()`, which extracts the function name of an `AnyRExpression`.
Indeed, the called function could be `foo()`, but it could also be `bar::foo()`, or `bar$foo()` if we were working with `R6` for instance.
`get_function_name()` helps us by returning `"foo"` in all those cases.
Rules that are not dispatched from `analyze/call.rs` can call it directly on `ast.function()?`.

We then extract the arguments from the `RCall` object:

```rust
let arguments = ast.arguments()?.items();
```

Past that point, the next step is to check that the arguments correspond to what we want to analyze.
`do.call()` has four arguments: `what`, `args`, `quote`, and `envir`, and we are looking for patterns such as `do.call(cbind.data.frame, x)`, so we want information on the first two.

Reading them by position alone would be wrong, since R lets the user write `do.call(args = x, cbind.data.frame)`.
This is why we hardcoded the signature of `do.call()` as a `Formals` constant at the top of the file:

```rust
const FORMALS_DO_CALL: Formals = &["what", "args", "quote", "envir"];
```

The helper `get_arg()` uses it to apply R's own matching rules: we look for named arguments first, and then use the unnamed ones to fill whatever slots are left.
Combined with the macro `unwrap_or_return_none!`, this gives:

```rust
let what = unwrap_or_return_none!(get_arg(ast, FORMALS_DO_CALL, "what"));
let args = unwrap_or_return_none!(get_arg(ast, FORMALS_DO_CALL, "args"));
```

`do.call(cbind.data.frame, x)`, `do.call(what = cbind.data.frame, args = x)` and `do.call(args = x, cbind.data.frame)` are all matched identically.

`get_arg()` returns an `Option` since the arguments we want to extract maybe do not exist in the code we parsed.
The macro `unwrap_or_return_none!()` makes the code slightly more readable.
It replaces the more verbose `let-some` pattern:

```rust
let Some(what) = get_arg(ast, FORMALS_DO_CALL, "what") else {
    return Ok(None);
};
```

We can now do more early checks:

```rust
// Ensure there isn't more than two arguments, as we probably cannot discard
// `quote` and `envir` if they are specified in `do.call()`.
if get_arg_by_position(&arguments, 3).is_some() {
    return Ok(None);
}

let what_value = unwrap_or_return_none!(what.value());
let txt = what_value.to_trimmed_text();
// `do.call()` accepts quoted function names.
if txt != "cbind.data.frame" && txt != "\"cbind.data.frame\"" && txt != "\'cbind.data.frame\'" {
    return Ok(None);
}
```

The block above ensures that we have a call to `do.call()` with two arguments only and that the argument `what` is one of `cbind.data.frame`, `"cbind.data.frame"`, or `'cbind.data.frame'` (because `do.call()` also accepts quoted function names).

We now reach the last step, which is building the automatic fix.
This requires getting the value of `args` because we want to use it in `list2DF()`:

```rust
let args_value = unwrap_or_return_none!(args.value());
let fix_content = args_value;
```

And finally, we build the diagnostic:

```rust
let range = ast.syntax().text_trimmed_range();
let diagnostic = Diagnostic::new(
    List2Df,
    range,
    Fix::new(
        range,
        format!("list2DF({})", fix_content.to_trimmed_text()),
        node_contains_comments(ast.syntax()),
    ),
);

Ok(Some(diagnostic))
```

All diagnostics contain a `Violation` (we defined the one for `List2Df` just below the documentation), a range indicating where it is located in the code, and `Fix::new()` (or `Fix::empty()` if there is no automatic fix).

Finally, note that `Fix::new()` has an argument `to_skip: node_contains_comments(ast.syntax())`. This tells Jarl not to apply the automatic fix if the node in question contains a comment. Handling comments positions in automatic fixes is quite complicated so, for now, fixes are not applied if the node contains a comment, e.g.:

```r
# This code wouldn't be automatically fixed because we don't know where the
# comment inside should go.
do.call(
    cbind.data.frame,
    # This is a comment to describe `x`.
    x
)
```

At this point, if you have an R file with a couple of examples that should be reported (e.g. `test.R`), you can use `cargo run --bin jarl -- check test.R` (the rule in this example is only valid for R >= 4.0.0, so we also need `--min-r-version 4.1` for instance).

### Add TOML options

It is possible that the rule you want to add benefits from some user customization.
This is possible via rule-specific arguments in `jarl.toml`.
For example, one could list some functions that will not be checked by the rule `duplicated_arguments`:

```toml
[lint]
...

[lint.duplicated_arguments]
# Ignore duplicated arguments in `list()` only.
skipped-functions = ["list"]
```

Not all rules need TOML options.

::: {.callout-note title = "Click to see how to add TOML options for a rule" collapse = true}

Adding options for a rule takes three steps. The example below uses the rule `duplicated_arguments` since `list2df` doesn't have TOML options.

1. Create `src/lints/<group>/<rule_name>/options.rs` and declare `pub(crate) mod options;` in the rule's `mod.rs`. This file contains two types: the TOML options (deserialized as-is from `[lint.<rule_name>]`) and the resolved options (what the rule reads while linting). The resolved type must expose `resolve()`, which takes the TOML options and fills in the defaults:

    ```rust
    /// Default functions that are allowed to have duplicated arguments.
    const DEFAULT_SKIPPED_FUNCTIONS: &[&str] = &["c", "mutate", "summarize", "transmute"];

    #[derive(Clone, Debug, PartialEq, Eq, Default, serde::Deserialize)]
    #[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
    #[serde(deny_unknown_fields, rename_all = "kebab-case")]
    pub struct DuplicatedArgumentsOptions {
        pub skipped_functions: Option<Vec<String>>,
        pub extend_skipped_functions: Option<Vec<String>>,
    }

    #[derive(Clone, Debug)]
    pub struct ResolvedDuplicatedArgumentsOptions {
        pub skipped_functions: HashSet<String>,
    }

    impl ResolvedDuplicatedArgumentsOptions {
        pub fn resolve(options: Option<&DuplicatedArgumentsOptions>) -> anyhow::Result<Self> {
            [...]
        }
    }
    ```

    If the option is a list of functions that can be either replaced or extended by the user (the `<field>` / `extend-<field>` pattern), use the helper `resolve_with_extend()` from `src/rule_options.rs` instead of writing that logic again.

1. Add the TOML field to `LinterTomlOptions` in `src/toml.rs`. The field must be named after the rule, and its documentation ends up in `artifacts/jarl.schema.json`, which editors use to describe the option:

    ```rust
    /// # Options for the `duplicated_arguments` rule
    ///
    /// Use `skipped-functions` to fully replace the default list of functions
    /// that are allowed to have duplicated arguments. Use
    /// `extend-skipped-functions` to add to the default list.
    /// Specifying both is an error.
    #[serde(rename = "duplicated_arguments")]
    pub duplicated_arguments: Option<DuplicatedArgumentsOptions>,
    ```

1. Add one line to `declare_rule_options!` in `src/rule_options.rs`, naming the rule's folder and its resolved type:

    ```rust
    declare_rule_options! {
        [...]
        base::duplicated_arguments => ResolvedDuplicatedArgumentsOptions,
        [...]
    }
    ```

    This generates the field on `ResolvedRuleOptions`, its resolution from the TOML file, and the default value. Forgetting step 2 is a compile error.

The rule can then read its options from the checker, e.g. `checker.rule_options.duplicated_arguments.skipped_functions`.

Finally:

* run `just gen-schema` to update `artifacts/jarl.schema.json`;
* document the option for users in `docs/reference/config-file.md` (this page is written by hand, it is not generated from the Rust code);
* add integration tests in `crates/jarl/tests/integration/toml_rule_args.rs`, covering invalid values, unknown fields in the rule table, and the option actually changing what is reported.

:::

### Add tests

Tests for each rule are stored in `lints/base/<rule_name>/mod.rs`.
It is important to test cases where we expect the rule that we just defined to be violated, *and* to test cases where we don't expect this violation.
Looking at tests for `list2df`, there are four blocks:

1. first, there is some setup that is common to all tests:

    ```rust
    mod tests {
        use crate::utils_test::*;
        use insta::assert_snapshot;

        fn snapshot_lint(code: &str) -> String {
            format_diagnostics(code, "list2df", Some("4.0"))
        }
        [...]
    ```

    (Replace `"list2df"` and `Some("4.0")` as appropriate.)

1. second, we check cases where we don't expect rule violations:

    ```rust
    #[test]
    fn test_no_lint_list2df() {
        expect_no_lint("cbind.data.frame(x, x)", "list2df", Some("4.0"));
        [...]

        // Ignored if R version unknown or below 4.0.0
        expect_no_lint("do.call(cbind.data.frame, x)", "list2df", Some("3.5"));
        [...]

        // Don't know how to handle additional comments
        expect_no_lint(
            "do.call(cbind.data.frame, x, quote = TRUE)",
            "list2df",
            Some("4.0"),
        );

        // Ensure that wrong calls are not reported
        expect_no_lint("do.call(cbind.data.frame)", "list2df", Some("4.0"));
        [...]
    }
    ```

1. third, we check cases where we expect rule violations.

    - we use inline snapshot tests to ensure that the diagnostic is correctly reported. Note that when you write the test, you can leave the expected output as `@r""`. Running `cargo insta test` and `cargo insta accept` will replace the expected output as appropriate.

    ```rust
    #[test]
    fn test_lint_list2df() {
        assert_snapshot!(
            snapshot_lint("do.call(cbind.data.frame, x)"),
            @r"
        warning: list2df
         --> <test>:1:1
          |
        1 | do.call(cbind.data.frame, x)
          | ---------------------------- `do.call(cbind.data.frame, x)` is inefficient and can be hard to read.
          |
          = help: Use `list2DF(x)` instead.
        Found 1 error.
        "
        );

        [...]
    }
    ```

    - then, we use snapshots and the function `get_fixed_text()` to ensure that automatic fixes are correct:

    ```rust
    #[test]
    fn test_lint_list2df() {
        [...]

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "do.call(cbind.data.frame, x)",
                    [...]
                ],
                "list2df",
                Some("4.0")
            )
        );
    }
    ```

1. finally, if the rule has an automatic fix, we check that having a comment in the middle of the code in question does *not* modify this code. Handling comments in automatic fixes is difficult and is left as an objective for the future.

    ```rust
    #[test]
    fn test_list2df_with_comments_no_fix() {
        use insta::assert_snapshot;
        // Should detect lint but skip fix when comments are present to avoid destroying them
        expect_lint(
            "do.call(\n # a comment\ncbind.data.frame, x)",
            "Use `list2DF(x)` instead",
            "list2df",
            Some("4.0"),
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\ndo.call(cbind.data.frame, x)",
                    "do.call(\n # a comment\ncbind.data.frame, x)",
                    "do.call(cbind.data.frame, x) # trailing comment",
                ],
                "list2df",
                Some("4.0")
            )
        );
    }
    ```

Since we have snapshot tests, we first need to run `cargo insta test` to generate the snapshots and then `cargo insta review` to review and validate them.
After that, run `cargo test` to ensure that all tests pass.


### All the rest

The rule is implemented, all tests pass, perfect!
We now need to document this change:

* update `docs/changelog.md`
* add or update the rule page in `docs/rules/<rule_name>.md`

If you have installed `just` as [recommended](contributing.md#tools), you can now run `just document` to update the website.

Finally, run `just lint` to ensure that `clippy` (the Rust linter) doesn't report any issue and that the code is properly formatted.
You can also run `just lint-fix` to apply `clippy`'s automatic fixes if there are any.


## Proposing your changes

Once all of this is done, it is time to open a PR!

*Note: if you need some guidance, early feedback, or simply want to store your changes in a branch, you can also open an incomplete PR.*

### PR title

Jarl follows [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/#summary), meaning that your PR must start with "feat:", "fix:", or another appropriate name (see the linked documentation).
Use backticks to format rule names as code.
In this case, the PR is titled "feat: Add `list2df_linter`".

### PR automated comments

Once you have opened a PR, you will receive three automated comments after a few minutes:

- code coverage: this checks that all the lines you added are covered by some tests. Try to ensure this is at 100%.
- ecosystem checks: every time there is a change in `jarl-core`, Jarl is run on several R packages and the results are compared to the Jarl version on the main branch. There will be a comment indicating if your changes have revealed new violations or removed some violations in those repositories. Here, we added a rule so we expect either no changes or more violations. New violations will be printed with a link to the lines of code that trigger them, so check a few to ensure those are not false positives.
- benchmark: this is usually irrelevant when adding a new rule, it is simply to ensure there is no catastrophic performance degradation.

Congrats, and thanks for your contribution!
