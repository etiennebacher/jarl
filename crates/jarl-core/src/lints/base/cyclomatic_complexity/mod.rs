pub(crate) mod cyclomatic_complexity;
pub(crate) mod flowgraph;
pub(crate) mod options;

#[cfg(test)]
mod tests {
    use crate::lints::base::cyclomatic_complexity::options::{
        CyclomaticComplexityOptions, ResolvedCyclomaticComplexityOptions,
    };
    use crate::rule_options::ResolvedRuleOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;
    use insta::assert_snapshot;

    /// Build a `Settings` with a custom `max-complexity`.
    ///
    /// Tests use a low maximum so that the snippets can stay short: what
    /// matters is the score, not the size of the code reaching it.
    fn settings_with_max(max_complexity: usize) -> Settings {
        let options = CyclomaticComplexityOptions { max_complexity: Some(max_complexity) };
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions {
                    cyclomatic_complexity: ResolvedCyclomaticComplexityOptions::resolve(Some(
                        &options,
                    ))
                    .unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    /// Format diagnostics for snapshot testing, with a custom maximum.
    fn snapshot_lint(code: &str, max_complexity: usize) -> String {
        format_diagnostics_with_settings(
            code,
            "cyclomatic_complexity",
            None,
            Some(settings_with_max(max_complexity)),
        )
    }

    /// Assert that `code` has no lint, with a custom maximum.
    fn expect_no_lint_with_max(code: &str, max_complexity: usize) {
        expect_no_lint_with_settings(
            code,
            "cyclomatic_complexity",
            None,
            settings_with_max(max_complexity),
        );
    }

    // The scores below are the ones of `cyclocomp::cyclocomp()`, unless a test
    // says otherwise.

    #[test]
    fn test_branchless_code_has_a_score_of_one() {
        expect_no_lint_with_max("foo <- function(x) {\n  x + 1\n}", 1);
    }

    #[test]
    fn test_if() {
        // A bare `else` adds nothing, an `else if` is an `if`.
        assert_snapshot!(
            snapshot_lint(
                "if_only <- function(x) if (x) 1\nif_else <- function(x) if (x) 1 else 2\nelse_if <- function(x) if (x) 1 else if (!x) 2 else 3",
                1
            ),
            @"
        warning: cyclomatic_complexity
         --> <test>:1:12
          |
        1 | if_only <- function(x) if (x) 1
          |            -------- This function has a cyclomatic complexity of 2, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        warning: cyclomatic_complexity
         --> <test>:2:12
          |
        2 | if_else <- function(x) if (x) 1 else 2
          |            -------- This function has a cyclomatic complexity of 2, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        warning: cyclomatic_complexity
         --> <test>:3:12
          |
        3 | else_if <- function(x) if (x) 1 else if (!x) 2 else 3
          |            -------- This function has a cyclomatic complexity of 3, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        Found 3 errors.
        "
        );
    }

    #[test]
    fn test_loops() {
        // A loop adds two paths: entering it and leaving it.
        assert_snapshot!(
            snapshot_lint(
                "for_loop <- function(x) for (i in x) 1\nwhile_loop <- function(x) while (x) 1\nrepeat_loop <- function(x) repeat if (x) break",
                1
            ),
            @"
        warning: cyclomatic_complexity
         --> <test>:1:13
          |
        1 | for_loop <- function(x) for (i in x) 1
          |             -------- This function has a cyclomatic complexity of 3, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        warning: cyclomatic_complexity
         --> <test>:2:15
          |
        2 | while_loop <- function(x) while (x) 1
          |               -------- This function has a cyclomatic complexity of 3, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        warning: cyclomatic_complexity
         --> <test>:3:16
          |
        3 | repeat_loop <- function(x) repeat if (x) break
          |                -------- This function has a cyclomatic complexity of 4, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        Found 3 errors.
        "
        );
    }

    #[test]
    fn test_break_and_next() {
        assert_snapshot!(
            snapshot_lint(
                "with_break <- function(x) while (x) { if (x) break }\nwith_next <- function(x) for (i in x) { if (i) next; 1 }",
                1
            ),
            @"
        warning: cyclomatic_complexity
         --> <test>:1:15
          |
        1 | with_break <- function(x) while (x) { if (x) break }
          |               -------- This function has a cyclomatic complexity of 5, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        warning: cyclomatic_complexity
         --> <test>:2:14
          |
        2 | with_next <- function(x) for (i in x) { if (i) next; 1 }
          |              -------- This function has a cyclomatic complexity of 5, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        Found 2 errors.
        "
        );
    }

    #[test]
    fn test_return() {
        // `return()` as the last expression is how the function ends.
        expect_no_lint_with_max("foo <- function(x) return(x)", 1);
        assert_snapshot!(
            snapshot_lint(
                "guard_clause <- function(x) { if (x) return(1); 2 }\nboth_branches <- function(x) { if (x) return(1) else return(2) }",
                1
            ),
            @"
        warning: cyclomatic_complexity
         --> <test>:1:17
          |
        1 | guard_clause <- function(x) { if (x) return(1); 2 }
          |                 -------- This function has a cyclomatic complexity of 3, above the maximum of 1.
          |
          = help: Split this function into smaller ones. The largest parts are `if (x) return(1)` (+2, 66%).
        warning: cyclomatic_complexity
         --> <test>:2:18
          |
        2 | both_branches <- function(x) { if (x) return(1) else return(2) }
          |                  -------- This function has a cyclomatic complexity of 2, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        Found 2 errors.
        "
        );
    }

    #[test]
    fn test_stop_and_error_handling_add_no_path() {
        expect_no_lint_with_max("foo <- function(x) { stop('a'); 1 }", 1);
        expect_no_lint_with_max(
            "foo <- function(x) tryCatch(f(x), error = function(e) 1, warning = function(w) 2)",
            1,
        );
        expect_no_lint_with_max("foo <- function(x) try(f(x))", 1);
    }

    #[test]
    fn test_short_circuit_operators() {
        assert_snapshot!(
            snapshot_lint(
                "and <- function(x, y) x && y\nand_or <- function(x, y) x && y || x",
                1
            ),
            @"
        warning: cyclomatic_complexity
         --> <test>:1:8
          |
        1 | and <- function(x, y) x && y
          |        -------- This function has a cyclomatic complexity of 2, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        warning: cyclomatic_complexity
         --> <test>:2:11
          |
        2 | and_or <- function(x, y) x && y || x
          |           -------- This function has a cyclomatic complexity of 3, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        Found 2 errors.
        "
        );
        // The vectorized operators always evaluate both sides.
        expect_no_lint_with_max("foo <- function(x, y) x & y | x", 1);
    }

    #[test]
    fn test_chained_short_circuit_operators_grow_linearly() {
        // Each operand is one more path (cyclocomp counts 4, 7, 11 and 16 for
        // the first four `if` conditions).
        assert_snapshot!(
            snapshot_lint(
                "one <- function(x) { if (a) stop() }
two <- function(x) { if (a || b) stop() }
three <- function(x) { if (a || b || c) stop() }
four <- function(x) { if (a || b || c || d) stop() }
and_four <- function(x) { if (a && b && c && d) stop() }
parentheses <- function(x) { if ((a || b) || c) stop() }
value <- function(x) a || b || c || d",
                1
            ),
            @"
        warning: cyclomatic_complexity
         --> <test>:1:8
          |
        1 | one <- function(x) { if (a) stop() }
          |        -------- This function has a cyclomatic complexity of 2, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        warning: cyclomatic_complexity
         --> <test>:2:8
          |
        2 | two <- function(x) { if (a || b) stop() }
          |        -------- This function has a cyclomatic complexity of 3, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        warning: cyclomatic_complexity
         --> <test>:3:10
          |
        3 | three <- function(x) { if (a || b || c) stop() }
          |          -------- This function has a cyclomatic complexity of 4, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        warning: cyclomatic_complexity
         --> <test>:4:9
          |
        4 | four <- function(x) { if (a || b || c || d) stop() }
          |         -------- This function has a cyclomatic complexity of 5, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        warning: cyclomatic_complexity
         --> <test>:5:13
          |
        5 | and_four <- function(x) { if (a && b && c && d) stop() }
          |             -------- This function has a cyclomatic complexity of 5, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        warning: cyclomatic_complexity
         --> <test>:6:16
          |
        6 | parentheses <- function(x) { if ((a || b) || c) stop() }
          |                -------- This function has a cyclomatic complexity of 4, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        warning: cyclomatic_complexity
         --> <test>:7:10
          |
        7 | value <- function(x) a || b || c || d
          |          -------- This function has a cyclomatic complexity of 4, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        Found 7 errors.
        "
        );
    }

    #[test]
    fn test_switch() {
        // cyclocomp doesn't know about `switch()`: it always scores 1.
        expect_no_lint_with_max("foo <- function(x) switch(x)", 1);
        // Without a default arm, no match returns `NULL`.
        assert_snapshot!(
            snapshot_lint(
                "one_arm <- function(x) switch(x, a = 1)
default <- function(x) switch(x, a = 1, b = 2, 3)
no_default <- function(x) switch(x, a = 1, b = 2)
fall_through <- function(x) switch(x, a = , b = 1, 2)
nested <- function(x) switch(x, a = if (x) 1, b = 2)",
                1
            ),
            @"
        warning: cyclomatic_complexity
         --> <test>:1:12
          |
        1 | one_arm <- function(x) switch(x, a = 1)
          |            -------- This function has a cyclomatic complexity of 2, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        warning: cyclomatic_complexity
         --> <test>:2:12
          |
        2 | default <- function(x) switch(x, a = 1, b = 2, 3)
          |            -------- This function has a cyclomatic complexity of 3, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        warning: cyclomatic_complexity
         --> <test>:3:15
          |
        3 | no_default <- function(x) switch(x, a = 1, b = 2)
          |               -------- This function has a cyclomatic complexity of 3, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        warning: cyclomatic_complexity
         --> <test>:4:17
          |
        4 | fall_through <- function(x) switch(x, a = , b = 1, 2)
          |                 -------- This function has a cyclomatic complexity of 2, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        warning: cyclomatic_complexity
         --> <test>:5:11
          |
        5 | nested <- function(x) switch(x, a = if (x) 1, b = 2)
          |           -------- This function has a cyclomatic complexity of 4, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        Found 5 errors.
        "
        );
    }

    #[test]
    fn test_syntax_converted_by_the_parser() {
        // `x[]` holds an empty argument, `->` swaps its sides, and a pipe
        // calls the function on its left: none of them changes the score.
        assert_snapshot!(
            snapshot_lint(
                "foo <- function(x) { x[]; a -> b; x |> f() |> g(); if (x) 1 }",
                1
            ),
            @"
        warning: cyclomatic_complexity
         --> <test>:1:8
          |
        1 | foo <- function(x) { x[]; a -> b; x |> f() |> g(); if (x) 1 }
          |        -------- This function has a cyclomatic complexity of 2, above the maximum of 1.
          |
          = help: Split this function into smaller ones. The largest parts are `if (x) 1` (+1, 50%).
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_nested_functions_are_scored_on_their_own() {
        // The inner function is reported, the outer one only holds it
        // (cyclocomp #30).
        assert_snapshot!(
            snapshot_lint(
                "foo <- function(x) {\n  inner <- function(y) {\n    if (y) 1 else if (!y) 2 else 3\n  }\n  inner(x)\n}",
                2
            ),
            @r"
        warning: cyclomatic_complexity
         --> <test>:2:12
          |
        2 |   inner <- function(y) {
          |            -------- This function has a cyclomatic complexity of 3, above the maximum of 2.
          |
          = help: Split this function into smaller ones.
        Found 1 error.
        "
        );
        expect_no_lint_with_max(
            "foo <- function(x) lapply(x, function(i) if (i) 1 else 2)",
            2,
        );
    }

    #[test]
    fn test_return_in_a_nested_function_leaves_that_function() {
        expect_no_lint_with_max(
            "foo <- function(x) {\n  inner <- function(y) return(y)\n  inner(x)\n}",
            1,
        );
    }

    #[test]
    fn test_report_the_largest_parts() {
        // 3 of the 5 points. The last `if` is 20%, which is not above the limit.
        assert_snapshot!(
            snapshot_lint("
foo <- function(x) {
  y <- 1
  if (x) {
    for (i in x) print(i)
  }
  if (y) 1
}", 1),
            @"
        warning: cyclomatic_complexity
         --> <test>:2:8
          |
        2 | foo <- function(x) {
          |        -------- This function has a cyclomatic complexity of 5, above the maximum of 1.
          |
          = help: Split this function into smaller ones. The largest parts are `if (x) {` (+3, 60%).
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_no_largest_parts_when_the_score_is_spread() {
        // Each `if` is 1 of the 7 points, nothing stands out.
        assert_snapshot!(
            snapshot_lint("
foo <- function(x) {
  if (a) 1
  if (b) 1
  if (c) 1
  if (d) 1
  if (e) 1
  if (f) 1
}", 1),
            @"
        warning: cyclomatic_complexity
         --> <test>:2:8
          |
        2 | foo <- function(x) {
          |        -------- This function has a cyclomatic complexity of 7, above the maximum of 1.
          |
          = help: Split this function into smaller ones.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_no_lint_when_cyclocomp_errors() {
        // cyclocomp can't compute a score for these, so lintr doesn't report
        // them.
        expect_no_lint_with_max("foo <- function(x) break", 1);
    }

    #[test]
    fn test_no_lint_with_default_maximum() {
        // A moderately branchy function stays under the default of 20.
        expect_no_lint(
            "foo <- function(x, y) {\n  if (x) {\n    for (i in y) print(i)\n  } else if (y) {\n    while (x) x <- x - 1\n  }\n  if (x && y) 1 else 2\n}",
            "cyclomatic_complexity",
            None,
        );
    }
}
