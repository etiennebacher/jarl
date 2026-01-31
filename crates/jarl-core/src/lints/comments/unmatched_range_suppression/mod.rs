pub(crate) mod unmatched_range_suppression;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_unmatched_range_suppression() {
        // Properly matched start/end at top level
        expect_no_lint(
            "
# jarl-ignore-start any_is_na: <reason>
any(is.na(x))
# jarl-ignore-end any_is_na",
            "unmatched_range_suppression",
            None,
        );

        // Properly matched start/end inside a function
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

        // Multiple nested levels, each properly matched
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
        let lint_msg = "no matching `jarl-ignore-end`";

        // Start at top level, end inside function
        expect_lint(
            "
# jarl-ignore-start any_is_na: <reason>
f <- function() {
  any(is.na(x))
  # jarl-ignore-end any_is_na
}",
            lint_msg,
            "unmatched_range_suppression",
            None,
        );

        // Start at top level, end inside if statement
        expect_lint(
            "
# jarl-ignore-start any_is_na: <reason>
if (a) {
  any(is.na(x))
  # jarl-ignore-end any_is_na
}",
            lint_msg,
            "unmatched_range_suppression",
            None,
        );

        // Start at top level, end inside while statement
        expect_lint(
            "
# jarl-ignore-start any_is_na: <reason>
while (a) {
  any(is.na(x))
  # jarl-ignore-end any_is_na
}",
            lint_msg,
            "unmatched_range_suppression",
            None,
        );

        // Start at top level, end inside for loop
        expect_lint(
            "
# jarl-ignore-start any_is_na: <reason>
for (i in 1:10) {
  any(is.na(x))
  # jarl-ignore-end any_is_na
}",
            lint_msg,
            "unmatched_range_suppression",
            None,
        );

        // Start at top level, end inside repeat statement
        expect_lint(
            "
# jarl-ignore-start any_is_na: <reason>
repeat {
  any(is.na(x))
  # jarl-ignore-end any_is_na
}",
            lint_msg,
            "unmatched_range_suppression",
            None,
        );

        // Start without any end
        expect_lint(
            "
# jarl-ignore-start any_is_na: <reason>
any(is.na(x))",
            lint_msg,
            "unmatched_range_suppression",
            None,
        );
    }

    #[test]
    fn test_lint_unmatched_end() {
        let lint_msg = "no matching `jarl-ignore-start`";

        // Start inside function, end at top level (mismatched nesting)
        expect_lint(
            "
f <- function() {
  # jarl-ignore-start any_is_na: <reason>
  any(is.na(x))
}
# jarl-ignore-end any_is_na",
            lint_msg,
            "unmatched_range_suppression",
            None,
        );

        // Start inside if statement, end at top-level
        expect_lint(
            "
if (a) {
  # jarl-ignore-start any_is_na: <reason>
  any(is.na(x))
}
# jarl-ignore-end any_is_na",
            lint_msg,
            "unmatched_range_suppression",
            None,
        );

        // Start inside while statement, end at top-level
        expect_lint(
            "
while (a) {
  # jarl-ignore-start any_is_na: <reason>
  any(is.na(x))
}
# jarl-ignore-end any_is_na",
            lint_msg,
            "unmatched_range_suppression",
            None,
        );

        // Start inside for loop, end at top-level
        expect_lint(
            "
for (i in 1:10) {
  # jarl-ignore-start any_is_na: <reason>
  any(is.na(x))
}
# jarl-ignore-end any_is_na",
            lint_msg,
            "unmatched_range_suppression",
            None,
        );

        // Start inside repeat statement, end at top-level
        expect_lint(
            "
repeat {
  # jarl-ignore-start any_is_na: <reason>
  any(is.na(x))
}
# jarl-ignore-end any_is_na",
            lint_msg,
            "unmatched_range_suppression",
            None,
        );

        // End without any start
        expect_lint(
            "
any(is.na(x))
# jarl-ignore-end any_is_na",
            lint_msg,
            "unmatched_range_suppression",
            None,
        );
    }
}
