pub(crate) mod outdated_suppression;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    fn snapshot_lint(code: &str, rules: &str) -> String {
        format_diagnostics(code, rules, None)
    }

    #[test]
    fn test_no_lint_outdated_suppression() {
        expect_no_lint(
            "
# jarl-ignore any_is_na: <reason>
any(is.na(x))",
            "outdated_suppression,any_is_na",
            None,
        );
        expect_no_lint(
            "
# jarl-ignore any_is_na: <reason>
f <- function(x) {
  any(is.na(x))
}",
            "outdated_suppression,any_is_na",
            None,
        );

        expect_no_lint(
            "
# jarl-ignore-file any_is_na: <reason>
any(is.na(x))
any(is.na(y))",
            "outdated_suppression,any_is_na",
            None,
        );

        expect_no_lint(
            "
# jarl-ignore-start any_is_na: <reason>
any(is.na(x))
# jarl-ignore-end any_is_na
x <- 1",
            "outdated_suppression,any_is_na",
            None,
        );
    }

    #[test]
    fn test_lint_outdated_suppression() {
        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore any_is_na: <reason>
x <- 1", "outdated_suppression,any_is_na"), @r"
        warning: outdated_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore any_is_na: <reason>
          | --------------------------------- This suppression comment is unused, no violation would be reported without it.
          |
          = help: Remove this suppression comment or verify that it's still needed.
        Found 1 error.
        "
        );

        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore any_is_na: <reason>
f <- function(x) {
  1 + 1
}", "outdated_suppression,any_is_na"), @r"
        warning: outdated_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore any_is_na: <reason>
          | --------------------------------- This suppression comment is unused, no violation would be reported without it.
          |
          = help: Remove this suppression comment or verify that it's still needed.
        Found 1 error.
        "
        );

        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore-file any_is_na: <reason>
x <- 1
y <- 2", "outdated_suppression,any_is_na"), @r"
        warning: outdated_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore-file any_is_na: <reason>
          | -------------------------------------- This suppression comment is unused, no violation would be reported without it.
          |
          = help: Remove this suppression comment or verify that it's still needed.
        Found 1 error.
        "
        );

        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore-start any_is_na: <reason>
x <- 1
# jarl-ignore-end any_is_na
y <- 2", "outdated_suppression,any_is_na"), @r"
        warning: outdated_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore-start any_is_na: <reason>
          | --------------------------------------- This suppression comment is unused, no violation would be reported without it.
          |
          = help: Remove this suppression comment or verify that it's still needed.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_outdated_suppression_wrong_rule() {
        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore any_is_na: <reason>
x == NA", "outdated_suppression,any_is_na,equals_na"), @r"
        warning: equals_na
         --> <test>:3:1
          |
        3 | x == NA
          | ------- Comparing to NA with `==`, `!=` or `%in%` is problematic.
          |
          = help: Use `is.na()` instead.
        warning: outdated_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore any_is_na: <reason>
          | --------------------------------- This suppression comment is unused, no violation would be reported without it.
          |
          = help: Remove this suppression comment or verify that it's still needed.
        Found 2 errors.
        "
        );
    }
}
