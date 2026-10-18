pub(crate) mod assignment_on_if_no_else;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "assignment_on_if_no_else", None)
    }

    #[test]
    fn test_lint_assignment_on_if_no_else() {
        assert_snapshot!(
            snapshot_lint("df <- 1\ndf <- if (cond) { data.frame() }"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:2:1
          |
        2 | df <- if (cond) { data.frame() }
          | -------------------------------- This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Move the assignment into the `if` branches or add a final `else` branch.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("value <- 1\nvalue <- if (a) { 1 } else if (b) { 2 }"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:2:1
          |
        2 | value <- if (a) { 1 } else if (b) { 2 }
          | --------------------------------------- This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Move the assignment into the `if` branches or add a final `else` branch.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_no_lint_assignment_on_if_no_else() {
        expect_no_lint(
            "value <- if (a) { 1 } else { 2 }",
            "assignment_on_if_no_else",
            None,
        );
        expect_no_lint("value <- if (a) 1", "assignment_on_if_no_else", None);
        expect_no_lint(
            "value <- 1\nprint(value)\nvalue <- if (a) 1",
            "assignment_on_if_no_else",
            None,
        );
        expect_no_lint(
            "if (condition) value <- 1 else value <- if (a) 1",
            "assignment_on_if_no_else",
            None,
        );
        expect_no_lint(
            "value <- if (a) { 1 } else if (b) { 2 } else { 3 }",
            "assignment_on_if_no_else",
            None,
        );
        expect_no_lint("value <- 1", "assignment_on_if_no_else", None);
        expect_no_lint("if (a) value <- 1", "assignment_on_if_no_else", None);
        expect_no_lint("fn(value = if (a) { 1 })", "assignment_on_if_no_else", None);
    }

    #[test]
    fn test_assignment_on_if_no_else_supports_assignment_forms() {
        assert_snapshot!(
            snapshot_lint("value <- 1\nvalue = if (a) 1"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:2:1
          |
        2 | value = if (a) 1
          | ---------------- This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Move the assignment into the `if` branches or add a final `else` branch.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("value <- 1\nvalue <<- if (a) 1"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:2:1
          |
        2 | value <<- if (a) 1
          | ------------------ This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Move the assignment into the `if` branches or add a final `else` branch.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("value <- 1\n(if (a) 1) -> value"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:2:1
          |
        2 | (if (a) 1) -> value
          | ------------------- This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Move the assignment into the `if` branches or add a final `else` branch.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("value <- 1\n(if (a) 1) ->> value"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:2:1
          |
        2 | (if (a) 1) ->> value
          | -------------------- This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Move the assignment into the `if` branches or add a final `else` branch.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("value <- 1\nvalue <- (if (a) 1)"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:2:1
          |
        2 | value <- (if (a) 1)
          | ------------------- This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Move the assignment into the `if` branches or add a final `else` branch.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_assignment_on_if_no_else_does_not_flag_nested_if_values() {
        expect_no_lint(
            "value <- if (a) { if (b) 1 } else { 2 }",
            "assignment_on_if_no_else",
            None,
        );
        expect_no_lint("if (a) { value <- 1 }", "assignment_on_if_no_else", None);
    }

    #[test]
    fn test_assignment_on_if_no_else_across_scopes() {
        assert_snapshot!(
            snapshot_lint("df <- 1\nfoo <- function() {\n  df <- if (cond) { data.frame() }\n  df\n}"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:3:3
          |
        3 |   df <- if (cond) { data.frame() }
          |   -------------------------------- This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Move the assignment into the `if` branches or add a final `else` branch.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("bar <- function() {\n  df <- 1\n  foo <- function() {\n    df <- if (cond) { data.frame() }\n    df\n  }\n}"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:4:5
          |
        4 |     df <- if (cond) { data.frame() }
          |     -------------------------------- This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Move the assignment into the `if` branches or add a final `else` branch.
        Found 1 error.
        "
        );

        expect_no_lint(
            "foo <- function() {\n  df <- 1\n  df\n}\ndf <- if (cond) { data.frame() }",
            "assignment_on_if_no_else",
            None,
        );
    }
}
