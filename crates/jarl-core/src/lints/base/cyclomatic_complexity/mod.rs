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

    /// The score of `code`, read from the message reported with a maximum of 1.
    fn score(code: &str) -> String {
        let output = snapshot_lint(code, 1);
        output
            .split("complexity of ")
            .nth(1)
            .and_then(|rest| rest.split(',').next())
            .unwrap_or("1")
            .to_string()
    }

    // The scores below are the ones of `cyclocomp::cyclocomp()`, unless a test
    // says otherwise.

    #[test]
    fn test_branchless_code_has_a_score_of_one() {
        expect_no_lint_with_max("foo <- function(x) {\n  x + 1\n}", 1);
    }

    #[test]
    fn test_if() {
        assert_snapshot!(score("foo <- function(x) if (x) 1"), @"2");
        // A bare `else` adds nothing, an `else if` is an `if`.
        assert_snapshot!(score("foo <- function(x) if (x) 1 else 2"), @"2");
        assert_snapshot!(score("foo <- function(x) if (x) 1 else if (!x) 2 else 3"), @"3");
    }

    #[test]
    fn test_loops() {
        // A loop adds two paths: entering it and leaving it.
        assert_snapshot!(score("foo <- function(x) for (i in x) 1"), @"3");
        assert_snapshot!(score("foo <- function(x) while (x) 1"), @"3");
        assert_snapshot!(score("foo <- function(x) repeat if (x) break"), @"4");
    }

    #[test]
    fn test_break_and_next() {
        assert_snapshot!(score("foo <- function(x) while (x) { if (x) break }"), @"5");
        assert_snapshot!(score("foo <- function(x) for (i in x) { if (i) next; 1 }"), @"5");
    }

    #[test]
    fn test_return() {
        // `return()` as the last expression is how the function ends.
        expect_no_lint_with_max("foo <- function(x) return(x)", 1);
        assert_snapshot!(score("foo <- function(x) { if (x) return(1); 2 }"), @"3");
        assert_snapshot!(score("foo <- function(x) { if (x) return(1) else return(2) }"), @"2");
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
        assert_snapshot!(score("foo <- function(x, y) x && y"), @"2");
        assert_snapshot!(score("foo <- function(x, y) x && y || x"), @"3");
        // The vectorized operators always evaluate both sides.
        expect_no_lint_with_max("foo <- function(x, y) x & y | x", 1);
    }

    #[test]
    fn test_chained_short_circuit_operators_grow_linearly() {
        // Each operand is one more path (cyclocomp counts 4, 7, 11 and 16).
        assert_snapshot!(score("foo <- function(x) { if (a) stop() }"), @"2");
        assert_snapshot!(score("foo <- function(x) { if (a || b) stop() }"), @"3");
        assert_snapshot!(score("foo <- function(x) { if (a || b || c) stop() }"), @"4");
        assert_snapshot!(score("foo <- function(x) { if (a || b || c || d) stop() }"), @"5");
        assert_snapshot!(score("foo <- function(x) { if (a && b && c && d) stop() }"), @"5");
        assert_snapshot!(score("foo <- function(x) { if ((a || b) || c) stop() }"), @"4");
        assert_snapshot!(score("foo <- function(x) a || b || c || d"), @"4");
    }

    #[test]
    fn test_switch() {
        // cyclocomp doesn't know about `switch()`: it always scores 1.
        expect_no_lint_with_max("foo <- function(x) switch(x)", 1);
        assert_snapshot!(score("foo <- function(x) switch(x, a = 1)"), @"2");
        assert_snapshot!(score("foo <- function(x) switch(x, a = 1, b = 2, 3)"), @"3");
        // Without a default arm, no match returns `NULL`.
        assert_snapshot!(score("foo <- function(x) switch(x, a = 1, b = 2)"), @"3");
        // An empty arm falls through to the next one.
        assert_snapshot!(score("foo <- function(x) switch(x, a = , b = 1, 2)"), @"2");
        assert_snapshot!(score("foo <- function(x) switch(x, a = if (x) 1, b = 2)"), @"4");
    }

    #[test]
    fn test_syntax_converted_by_the_parser() {
        // `x[]` holds an empty argument, `->` swaps its sides, and a pipe
        // calls the function on its left.
        assert_snapshot!(score("foo <- function(x) { x[]; a -> b; x |> f() |> g(); if (x) 1 }"), @"2");
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
