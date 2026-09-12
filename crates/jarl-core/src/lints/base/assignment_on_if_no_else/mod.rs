pub(crate) mod assignment_on_if_no_else;

#[cfg(test)]
mod tests {
    use crate::rule_set::{Category, Rule};
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "assignment_on_if_no_else", None)
    }

    #[test]
    fn test_assignment_on_if_no_else_is_suspicious_and_enabled_by_default() {
        assert!(Rule::AssignmentOnIfNoElse.is_enabled_by_default());
        assert!(Rule::AssignmentOnIfNoElse.has_category(Category::Susp));
    }

    #[test]
    fn test_lint_assignment_on_if_no_else() {
        assert_snapshot!(
            snapshot_lint("df <- if (cond) { data.frame() }"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:1:1
          |
        1 | df <- if (cond) { data.frame() }
          | -------------------------------- This assignment can overwrite the previous value with `NULL` when no `if` branch is taken.
          |
          = help: Move the assignment into the `if` branches or add a final `else` branch.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("value <- if (a) { 1 } else if (b) { 2 }"),
            @"
        warning: assignment_on_if_no_else
         --> <test>:1:1
          |
        1 | value <- if (a) { 1 } else if (b) { 2 }
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
        for code in [
            "value = if (a) 1",
            "value <<- if (a) 1",
            "(if (a) 1) -> value",
            "(if (a) 1) ->> value",
            "value <- (if (a) 1)",
        ] {
            assert_eq!(
                check_code(code, "assignment_on_if_no_else", None).len(),
                1,
                "Expected a diagnostic for {code}"
            );
        }
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
}
