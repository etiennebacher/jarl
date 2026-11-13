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
            snapshot_lint("
df <- 1
df <- if (cond) { data.frame() }
"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:3:1
          |
        2 | df <- 1
          | -- `df` was last assigned here
        3 | df <- if (cond) { data.frame() }
          | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Add a final `else` branch if `NULL` is the intended fallback, or move the assignment into the `if` branches.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("
value <- 1
value <- if (a) { 1 } else if (b) { 2 }
"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:3:1
          |
        2 | value <- 1
          | ----- `value` was last assigned here
        3 | value <- if (a) { 1 } else if (b) { 2 }
          | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Add a final `else` branch if `NULL` is the intended fallback, or move the assignment into the `if` branches.
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
    fn test_assignment_on_if_no_else_overwrites_used_value() {
        assert_snapshot!(
            snapshot_lint("
value <- 1
print(value)
value <- if (a) 1
"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:4:1
          |
        2 | value <- 1
          | ----- `value` was last assigned here
        3 | print(value)
        4 | value <- if (a) 1
          | ^^^^^^^^^^^^^^^^^ This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Add a final `else` branch if `NULL` is the intended fallback, or move the assignment into the `if` branches.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_assignment_on_if_no_else_supports_assignment_forms() {
        assert_snapshot!(
            snapshot_lint("
value <- 1
value = if (a) 1
"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:3:1
          |
        2 | value <- 1
          | ----- `value` was last assigned here
        3 | value = if (a) 1
          | ^^^^^^^^^^^^^^^^ This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Add a final `else` branch if `NULL` is the intended fallback, or move the assignment into the `if` branches.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("
value <- 1
value <<- if (a) 1
"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:3:1
          |
        2 | value <- 1
          | ----- `value` was last assigned here
        3 | value <<- if (a) 1
          | ^^^^^^^^^^^^^^^^^^ This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Add a final `else` branch if `NULL` is the intended fallback, or move the assignment into the `if` branches.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("
value <- 1
(if (a) 1) -> value
"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:3:1
          |
        2 | value <- 1
          | ----- `value` was last assigned here
        3 | (if (a) 1) -> value
          | ^^^^^^^^^^^^^^^^^^^ This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Add a final `else` branch if `NULL` is the intended fallback, or move the assignment into the `if` branches.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("
value <- 1
(if (a) 1) ->> value
"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:3:1
          |
        2 | value <- 1
          | ----- `value` was last assigned here
        3 | (if (a) 1) ->> value
          | ^^^^^^^^^^^^^^^^^^^^ This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Add a final `else` branch if `NULL` is the intended fallback, or move the assignment into the `if` branches.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("
value <- 1
value <- (if (a) 1)
"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:3:1
          |
        2 | value <- 1
          | ----- `value` was last assigned here
        3 | value <- (if (a) 1)
          | ^^^^^^^^^^^^^^^^^^^ This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Add a final `else` branch if `NULL` is the intended fallback, or move the assignment into the `if` branches.
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
            snapshot_lint("
df <- 1
foo <- function() {
  df <- if (cond) { data.frame() }
  df
}
  "),
            @"
        warning: assignment_on_if_no_else
         --> <test>:4:3
          |
        2 | df <- 1
          | -- `df` was last assigned here
        3 | foo <- function() {
        4 |   df <- if (cond) { data.frame() }
          |   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Add a final `else` branch if `NULL` is the intended fallback, or move the assignment into the `if` branches.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("
bar <- function() {
  df <- 1
  foo <- function() {
    df <- if (cond) { data.frame() }
    df
  }
}
  "),
            @"
        warning: assignment_on_if_no_else
         --> <test>:5:5
          |
        3 |   df <- 1
          |   -- `df` was last assigned here
        4 |   foo <- function() {
        5 |     df <- if (cond) { data.frame() }
          |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Add a final `else` branch if `NULL` is the intended fallback, or move the assignment into the `if` branches.
        Found 1 error.
        "
        );

        expect_no_lint(
            "foo <- function() {
  df <- 1
  df
}
df <- if (cond) { data.frame() }",
            "assignment_on_if_no_else",
            None,
        );
    }

    #[test]
    fn test_assignment_on_if_no_else_labels_elide_distant_lines() {
        // One line in between is printed rather than elided.
        assert_snapshot!(
            snapshot_lint("
df <- 1
print(1)
df <- if (cond) 2
"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:4:1
          |
        2 | df <- 1
          | -- `df` was last assigned here
        3 | print(1)
        4 | df <- if (cond) 2
          | ^^^^^^^^^^^^^^^^^ This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Add a final `else` branch if `NULL` is the intended fallback, or move the assignment into the `if` branches.
        Found 1 error.
        "
        );
        // Several lines in between are replaced by a separator.
        assert_snapshot!(
            snapshot_lint("
df <- 1
print(1)
print(2)
print(3)
print(4)
df <- if (cond) 2
"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:7:1
          |
        2 | df <- 1
          | -- `df` was last assigned here
        ...
        7 | df <- if (cond) 2
          | ^^^^^^^^^^^^^^^^^ This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Add a final `else` branch if `NULL` is the intended fallback, or move the assignment into the `if` branches.
        Found 1 error.
        "
        );
        // The label points at the last assignment before the `if` one.
        assert_snapshot!(
            snapshot_lint("
df <- 1
df <- 2
df <- if (cond) 3
"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:4:1
          |
        3 | df <- 2
          | -- `df` was last assigned here
        4 | df <- if (cond) 3
          | ^^^^^^^^^^^^^^^^^ This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Add a final `else` branch if `NULL` is the intended fallback, or move the assignment into the `if` branches.
        Found 1 error.
        "
        );
    }
}
