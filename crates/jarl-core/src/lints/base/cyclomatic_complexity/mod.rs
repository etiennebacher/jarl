pub(crate) mod cyclomatic_complexity;
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

    #[test]
    fn test_score_of_a_branchless_function_is_one() {
        // A function with no branch has a single path through it.
        expect_no_lint_with_max("foo <- function(x) {\n  x + 1\n}", 1);
    }

    #[test]
    fn test_lint_if_and_else_if() {
        // `if` and `else if` each add a path, a bare `else` doesn't.
        expect_no_lint_with_max("foo <- function(x) if (x) 1 else 2", 2);
        assert_snapshot!(
            snapshot_lint("foo <- function(x) if (x) 1 else if (!x) 2 else 3", 2),
            @r"
        warning: cyclomatic_complexity
         --> <test>:1:8
          |
        1 | foo <- function(x) if (x) 1 else if (!x) 2 else 3
          |        -------- This function has a cyclomatic complexity of 3, above the maximum of 2.
          |
          = help: Split this function into smaller ones.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_loops() {
        expect_no_lint_with_max("foo <- function(x) for (i in x) print(i)", 2);
        expect_no_lint_with_max("foo <- function(x) while (x) print(x)", 2);
        expect_no_lint_with_max("foo <- function(x) repeat break", 2);
        assert_snapshot!(
            snapshot_lint(
                "foo <- function(x) {\n  for (i in x) {\n    while (i) {\n      repeat break\n    }\n  }\n}",
                2
            ),
            @r"
        warning: cyclomatic_complexity
         --> <test>:1:8
          |
        1 | foo <- function(x) {
          |        -------- This function has a cyclomatic complexity of 4, above the maximum of 2.
          |
          = help: Split this function into smaller ones.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_short_circuit_operators() {
        // `&&` and `||` skip their right side, so each one is a branch.
        assert_snapshot!(
            snapshot_lint("foo <- function(x, y) x && y || x", 2),
            @r"
        warning: cyclomatic_complexity
         --> <test>:1:8
          |
        1 | foo <- function(x, y) x && y || x
          |        -------- This function has a cyclomatic complexity of 3, above the maximum of 2.
          |
          = help: Split this function into smaller ones.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_no_lint_vectorized_operators() {
        // `&` and `|` always evaluate both sides, so they add no path.
        expect_no_lint_with_max("foo <- function(x, y) x & y | x", 1);
    }

    #[test]
    fn test_lint_switch_arms() {
        // The first argument is the value being matched, and a single arm
        // leaves only one path.
        expect_no_lint_with_max("foo <- function(x) switch(x, a = 1)", 1);
        assert_snapshot!(
            snapshot_lint("foo <- function(x) switch(x, a = 1, b = 2, 3)", 2),
            @r"
        warning: cyclomatic_complexity
         --> <test>:1:8
          |
        1 | foo <- function(x) switch(x, a = 1, b = 2, 3)
          |        -------- This function has a cyclomatic complexity of 3, above the maximum of 2.
          |
          = help: Split this function into smaller ones.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_error_handlers() {
        // `expr` and `finally` are not handlers: the first is the guarded
        // expression, the second runs on every path.
        expect_no_lint_with_max(
            "foo <- function(x) tryCatch(expr = f(x), finally = cleanup())",
            1,
        );
        assert_snapshot!(
            snapshot_lint(
                "foo <- function(x) {\n  tryCatch(f(x), error = function(e) NULL, warning = function(w) NA)\n}",
                2
            ),
            @r"
        warning: cyclomatic_complexity
         --> <test>:1:8
          |
        1 | foo <- function(x) {
          |        -------- This function has a cyclomatic complexity of 3, above the maximum of 2.
          |
          = help: Split this function into smaller ones.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_try() {
        expect_no_lint_with_max("foo <- function(x) try(f(x))", 2);
        assert_snapshot!(
            snapshot_lint("foo <- function(x) {\n  try(f(x))\n  try(g(x))\n}", 2),
            @r"
        warning: cyclomatic_complexity
         --> <test>:1:8
          |
        1 | foo <- function(x) {
          |        -------- This function has a cyclomatic complexity of 3, above the maximum of 2.
          |
          = help: Split this function into smaller ones.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_no_lint_exits_in_tail_position() {
        // These are how the function normally ends, not a jump out of it.
        expect_no_lint_with_max("foo <- function(x) return(x)", 1);
        expect_no_lint_with_max("foo <- function(x) {\n  x <- x + 1\n  return(x)\n}", 1);
        expect_no_lint_with_max("foo <- function(x) stop('always')", 1);
        // Both branches of a trailing `if` are tail positions, so only the `if`
        // itself counts.
        expect_no_lint_with_max("foo <- function(x) if (x) return(1) else stop('no')", 2);
    }

    #[test]
    fn test_lint_early_exits() {
        // A guard clause jumps out from the middle, which is an extra path on
        // top of the `if` that guards it.
        assert_snapshot!(
            snapshot_lint("foo <- function(x) {\n  if (is.null(x)) return(NULL)\n  x + 1\n}", 2),
            @r"
        warning: cyclomatic_complexity
         --> <test>:1:8
          |
        1 | foo <- function(x) {
          |        -------- This function has a cyclomatic complexity of 3, above the maximum of 2.
          |
          = help: Split this function into smaller ones.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("foo <- function(x) {\n  if (is.null(x)) stop('empty')\n  x + 1\n}", 2),
            @r"
        warning: cyclomatic_complexity
         --> <test>:1:8
          |
        1 | foo <- function(x) {
          |        -------- This function has a cyclomatic complexity of 3, above the maximum of 2.
          |
          = help: Split this function into smaller ones.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_nested_functions_are_scored_on_their_own() {
        // The inner function is reported, the outer one only holds it.
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
    }

    #[test]
    fn test_lint_top_level_code() {
        assert_snapshot!(
            snapshot_lint("x <- 1\nif (x) x <- 2\nfor (i in 1:3) x <- x + i", 2),
            @r"
        warning: cyclomatic_complexity
         --> <test>:1:1
          |
        1 | x <- 1
          | ------ The top-level code of this file has a cyclomatic complexity of 3, above the maximum of 2.
          |
          = help: Move this logic into functions.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_no_lint_function_definitions_at_top_level() {
        // The branches belong to the function, not to the file around it.
        expect_no_lint_with_max(
            "foo <- function(x) {\n  if (x) 1 else if (!x) 2 else 3\n}",
            3,
        );
    }

    #[test]
    fn test_no_lint_with_default_maximum() {
        // A moderately branchy function stays under the default of 15.
        expect_no_lint(
            "foo <- function(x, y) {\n  if (x) {\n    for (i in y) print(i)\n  } else if (y) {\n    while (x) x <- x - 1\n  }\n  if (x && y) 1 else 2\n}",
            "cyclomatic_complexity",
            None,
        );
    }
}
