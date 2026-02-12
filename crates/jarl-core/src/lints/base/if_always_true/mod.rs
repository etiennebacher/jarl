pub(crate) mod if_always_true;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "if_always_true", None)
    }

    #[test]
    fn test_no_lint_if_always_true() {
        expect_no_lint("if (x) { print('hi') }", "if_always_true", None);

        // This is handled by `unreachable_code`
        expect_no_lint(
            "if (TRUE) { print('hi') } else { print('bye') }",
            "if_always_true",
            None,
        );

        // This is handled by `unreachable_code`
        expect_no_lint(
            "if (!FALSE) { print('hi') } else { print('bye') }",
            "if_always_true",
            None,
        );

        expect_no_lint("if (TRUE && x) { print('hi') }", "if_always_true", None);
        expect_no_lint("if (x || FALSE) { print('hi') }", "if_always_true", None);
        expect_no_lint("if (0 || x) { print('hi') }", "if_always_true", None);
        expect_no_lint("if (0) { print('hi') }", "if_always_true", None);
        expect_no_lint("if (-0.0) { print('hi') }", "if_always_true", None);
        expect_no_lint("if (x && FALSE) { print('hi') }", "if_always_true", None);
        //Handled by `unreachable_code`
        expect_no_lint(
            "if (x) { print('hi') } else if (TRUE) { print('bye') } else { print('unreachable') }",
            "if_always_true",
            None,
        );
    }

    #[test]
    fn test_lint_if_always_true() {
        assert_snapshot!(
            snapshot_lint("if (TRUE) print('hi')"),
            @r"
        warning: if_always_true
         --> <test>:1:5
          |
        1 | if (TRUE) print('hi')
          |     ---- `if` condition always evaluates to `TRUE`.
          |
          = help: Modify the `if` condition, or keep only the body.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (TRUE || x) { print('hi') }"),
            @r"
        warning: if_always_true
         --> <test>:1:5
          |
        1 | if (TRUE || x) { print('hi') }
          |     --------- `if` condition always evaluates to `TRUE`.
          |
          = help: Modify the `if` condition, or keep only the body.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (x || TRUE) { print('hi') }"),
            @r"
        warning: if_always_true
         --> <test>:1:5
          |
        1 | if (x || TRUE) { print('hi') }
          |     --------- `if` condition always evaluates to `TRUE`.
          |
          = help: Modify the `if` condition, or keep only the body.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (1) { print('hi') }"),
            @r"
        warning: if_always_true
         --> <test>:1:5
          |
        1 | if (1) { print('hi') }
          |     - `if` condition always evaluates to `TRUE`.
          |
          = help: Modify the `if` condition, or keep only the body.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (-1) { print('hi') }"),
            @r"
        warning: if_always_true
         --> <test>:1:5
          |
        1 | if (-1) { print('hi') }
          |     -- `if` condition always evaluates to `TRUE`.
          |
          = help: Modify the `if` condition, or keep only the body.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (5.5) { print('hi') }"),
            @r"
        warning: if_always_true
         --> <test>:1:5
          |
        1 | if (5.5) { print('hi') }
          |     --- `if` condition always evaluates to `TRUE`.
          |
          = help: Modify the `if` condition, or keep only the body.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (0.1) { print('hi') }"),
            @r"
        warning: if_always_true
         --> <test>:1:5
          |
        1 | if (0.1) { print('hi') }
          |     --- `if` condition always evaluates to `TRUE`.
          |
          = help: Modify the `if` condition, or keep only the body.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (10 || x) { print('hi') }"),
            @r"
        warning: if_always_true
         --> <test>:1:5
          |
        1 | if (10 || x) { print('hi') }
          |     ------- `if` condition always evaluates to `TRUE`.
          |
          = help: Modify the `if` condition, or keep only the body.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (!FALSE) { print('hi') }"),
            @r"
        warning: if_always_true
         --> <test>:1:5
          |
        1 | if (!FALSE) { print('hi') }
          |     ------ `if` condition always evaluates to `TRUE`.
          |
          = help: Modify the `if` condition, or keep only the body.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (Inf) { print('hi') }"),
            @r"
        warning: if_always_true
         --> <test>:1:5
          |
        1 | if (Inf) { print('hi') }
          |     --- `if` condition always evaluates to `TRUE`.
          |
          = help: Modify the `if` condition, or keep only the body.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (-Inf) { print('hi') }"),
            @r"
        warning: if_always_true
         --> <test>:1:5
          |
        1 | if (-Inf) { print('hi') }
          |     ---- `if` condition always evaluates to `TRUE`.
          |
          = help: Modify the `if` condition, or keep only the body.
        Found 1 error.
        "
        );
        //Not handled by `unreachable_code`
        assert_snapshot!(
            snapshot_lint("if (x) { print('hi') } else if (TRUE) { print('bye') }"),
            @r"
        warning: if_always_true
         --> <test>:1:33
          |
        1 | if (x) { print('hi') } else if (TRUE) { print('bye') }
          |                                 ---- `if` condition always evaluates to `TRUE`.
          |
          = help: Modify the `if` condition, or keep only the body.
        Found 1 error.
        "
        );
    }
}
