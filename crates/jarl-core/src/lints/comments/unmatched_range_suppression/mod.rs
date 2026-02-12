pub(crate) mod unmatched_range_suppression;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "unmatched_range_suppression", None)
    }

    #[test]
    fn test_no_lint_unmatched_range_suppression() {
        expect_no_lint(
            "
# jarl-ignore-start any_is_na: <reason>
any(is.na(x))
# jarl-ignore-end any_is_na",
            "unmatched_range_suppression",
            None,
        );

        expect_no_lint(
            "
f <- function() {
  # jarl-ignore-start any_is_na: <reason>
  any(is.na(x))
  # jarl-ignore-end any_is_na
}",
            "unmatched_range_suppression",
            None,
        );

        expect_no_lint(
            "
# jarl-ignore-start any_is_na: <reason>
any(is.na(x))
f <- function() {
  # jarl-ignore-start equals_na: <reason>
  x == NA
  # jarl-ignore-end equals_na
}
# jarl-ignore-end any_is_na",
            "unmatched_range_suppression",
            None,
        );
    }

    #[test]
    fn test_lint_unmatched_start() {
        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore-start any_is_na: <reason>
f <- function() {
  any(is.na(x))
  # jarl-ignore-end any_is_na
}"), @r"
        warning: unmatched_range_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore-start any_is_na: <reason>
          | --------------------------------------- This `jarl-ignore-start` has no matching `jarl-ignore-end` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-end` comment at the same nesting level.
        warning: unmatched_range_suppression
         --> <test>:5:3
          |
        5 |   # jarl-ignore-end any_is_na
          |   --------------------------- This `jarl-ignore-end` has no matching `jarl-ignore-start` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-start` comment at the same nesting level.
        Found 2 errors.
        "
        );

        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore-start any_is_na: <reason>
if (a) {
  any(is.na(x))
  # jarl-ignore-end any_is_na
}"), @r"
        warning: unmatched_range_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore-start any_is_na: <reason>
          | --------------------------------------- This `jarl-ignore-start` has no matching `jarl-ignore-end` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-end` comment at the same nesting level.
        warning: unmatched_range_suppression
         --> <test>:5:3
          |
        5 |   # jarl-ignore-end any_is_na
          |   --------------------------- This `jarl-ignore-end` has no matching `jarl-ignore-start` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-start` comment at the same nesting level.
        Found 2 errors.
        "
        );

        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore-start any_is_na: <reason>
while (a) {
  any(is.na(x))
  # jarl-ignore-end any_is_na
}"), @r"
        warning: unmatched_range_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore-start any_is_na: <reason>
          | --------------------------------------- This `jarl-ignore-start` has no matching `jarl-ignore-end` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-end` comment at the same nesting level.
        warning: unmatched_range_suppression
         --> <test>:5:3
          |
        5 |   # jarl-ignore-end any_is_na
          |   --------------------------- This `jarl-ignore-end` has no matching `jarl-ignore-start` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-start` comment at the same nesting level.
        Found 2 errors.
        "
        );

        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore-start any_is_na: <reason>
for (i in 1:10) {
  any(is.na(x))
  # jarl-ignore-end any_is_na
}"), @r"
        warning: unmatched_range_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore-start any_is_na: <reason>
          | --------------------------------------- This `jarl-ignore-start` has no matching `jarl-ignore-end` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-end` comment at the same nesting level.
        warning: unmatched_range_suppression
         --> <test>:5:3
          |
        5 |   # jarl-ignore-end any_is_na
          |   --------------------------- This `jarl-ignore-end` has no matching `jarl-ignore-start` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-start` comment at the same nesting level.
        Found 2 errors.
        "
        );

        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore-start any_is_na: <reason>
repeat {
  any(is.na(x))
  # jarl-ignore-end any_is_na
}"), @r"
        warning: unmatched_range_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore-start any_is_na: <reason>
          | --------------------------------------- This `jarl-ignore-start` has no matching `jarl-ignore-end` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-end` comment at the same nesting level.
        warning: unmatched_range_suppression
         --> <test>:5:3
          |
        5 |   # jarl-ignore-end any_is_na
          |   --------------------------- This `jarl-ignore-end` has no matching `jarl-ignore-start` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-start` comment at the same nesting level.
        Found 2 errors.
        "
        );

        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore-start any_is_na: <reason>
any(is.na(x))"), @r"
        warning: unmatched_range_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore-start any_is_na: <reason>
          | --------------------------------------- This `jarl-ignore-start` has no matching `jarl-ignore-end` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-end` comment at the same nesting level.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_unmatched_end() {
        insta::assert_snapshot!(snapshot_lint("
f <- function() {
  # jarl-ignore-start any_is_na: <reason>
  any(is.na(x))
}
# jarl-ignore-end any_is_na"), @r"
        warning: unmatched_range_suppression
         --> <test>:3:3
          |
        3 |   # jarl-ignore-start any_is_na: <reason>
          |   --------------------------------------- This `jarl-ignore-start` has no matching `jarl-ignore-end` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-end` comment at the same nesting level.
        warning: unmatched_range_suppression
         --> <test>:6:1
          |
        6 | # jarl-ignore-end any_is_na
          | --------------------------- This `jarl-ignore-end` has no matching `jarl-ignore-start` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-start` comment at the same nesting level.
        Found 2 errors.
        "
        );

        insta::assert_snapshot!(snapshot_lint("
if (a) {
  # jarl-ignore-start any_is_na: <reason>
  any(is.na(x))
}
# jarl-ignore-end any_is_na"), @r"
        warning: unmatched_range_suppression
         --> <test>:3:3
          |
        3 |   # jarl-ignore-start any_is_na: <reason>
          |   --------------------------------------- This `jarl-ignore-start` has no matching `jarl-ignore-end` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-end` comment at the same nesting level.
        warning: unmatched_range_suppression
         --> <test>:6:1
          |
        6 | # jarl-ignore-end any_is_na
          | --------------------------- This `jarl-ignore-end` has no matching `jarl-ignore-start` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-start` comment at the same nesting level.
        Found 2 errors.
        "
        );

        insta::assert_snapshot!(snapshot_lint("
while (a) {
  # jarl-ignore-start any_is_na: <reason>
  any(is.na(x))
}
# jarl-ignore-end any_is_na"), @r"
        warning: unmatched_range_suppression
         --> <test>:3:3
          |
        3 |   # jarl-ignore-start any_is_na: <reason>
          |   --------------------------------------- This `jarl-ignore-start` has no matching `jarl-ignore-end` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-end` comment at the same nesting level.
        warning: unmatched_range_suppression
         --> <test>:6:1
          |
        6 | # jarl-ignore-end any_is_na
          | --------------------------- This `jarl-ignore-end` has no matching `jarl-ignore-start` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-start` comment at the same nesting level.
        Found 2 errors.
        "
        );

        insta::assert_snapshot!(snapshot_lint("
for (i in 1:10) {
  # jarl-ignore-start any_is_na: <reason>
  any(is.na(x))
}
# jarl-ignore-end any_is_na"), @r"
        warning: unmatched_range_suppression
         --> <test>:3:3
          |
        3 |   # jarl-ignore-start any_is_na: <reason>
          |   --------------------------------------- This `jarl-ignore-start` has no matching `jarl-ignore-end` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-end` comment at the same nesting level.
        warning: unmatched_range_suppression
         --> <test>:6:1
          |
        6 | # jarl-ignore-end any_is_na
          | --------------------------- This `jarl-ignore-end` has no matching `jarl-ignore-start` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-start` comment at the same nesting level.
        Found 2 errors.
        "
        );

        insta::assert_snapshot!(snapshot_lint("
repeat {
  # jarl-ignore-start any_is_na: <reason>
  any(is.na(x))
}
# jarl-ignore-end any_is_na"), @r"
        warning: unmatched_range_suppression
         --> <test>:3:3
          |
        3 |   # jarl-ignore-start any_is_na: <reason>
          |   --------------------------------------- This `jarl-ignore-start` has no matching `jarl-ignore-end` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-end` comment at the same nesting level.
        warning: unmatched_range_suppression
         --> <test>:6:1
          |
        6 | # jarl-ignore-end any_is_na
          | --------------------------- This `jarl-ignore-end` has no matching `jarl-ignore-start` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-start` comment at the same nesting level.
        Found 2 errors.
        "
        );

        insta::assert_snapshot!(snapshot_lint("
any(is.na(x))
# jarl-ignore-end any_is_na"), @r"
        warning: unmatched_range_suppression
         --> <test>:3:1
          |
        3 | # jarl-ignore-end any_is_na
          | --------------------------- This `jarl-ignore-end` has no matching `jarl-ignore-start` at the same nesting level.
          |
          = help: Add a matching `jarl-ignore-start` comment at the same nesting level.
        Found 1 error.
        "
        );
    }
}
