pub(crate) mod implicit_assignment;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "implicit_assignment", None)
    }

    #[test]
    fn test_lint_implicit_assignment() {
        assert_snapshot!(
            snapshot_lint("if (x <- 1L) TRUE"),
            @r"
        warning: implicit_assignment
         --> <test>:1:5
          |
        1 | if (x <- 1L) TRUE
          |     ------- Avoid implicit assignments in `if()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (1L -> x) TRUE"),
            @r"
        warning: implicit_assignment
         --> <test>:1:5
          |
        1 | if (1L -> x) TRUE
          |     ------- Avoid implicit assignments in `if()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (x <<- 1L) TRUE"),
            @r"
        warning: implicit_assignment
         --> <test>:1:5
          |
        1 | if (x <<- 1L) TRUE
          |     -------- Avoid implicit assignments in `if()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (1L ->> x) TRUE"),
            @r"
        warning: implicit_assignment
         --> <test>:1:5
          |
        1 | if (1L ->> x) TRUE
          |     -------- Avoid implicit assignments in `if()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (A && (B <- foo())) { }"),
            @r"
        warning: implicit_assignment
         --> <test>:1:11
          |
        1 | if (A && (B <- foo())) { }
          |           ---------- Avoid implicit assignments in `if()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("while (x <- 0L) FALSE"),
            @r"
        warning: implicit_assignment
         --> <test>:1:8
          |
        1 | while (x <- 0L) FALSE
          |        ------- Avoid implicit assignments in `while()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("while (0L -> x) FALSE"),
            @r"
        warning: implicit_assignment
         --> <test>:1:8
          |
        1 | while (0L -> x) FALSE
          |        ------- Avoid implicit assignments in `while()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (x in y <- 1:10) print(x)"),
            @r"
        warning: implicit_assignment
         --> <test>:1:11
          |
        1 | for (x in y <- 1:10) print(x)
          |           --------- Avoid implicit assignments in `for()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (x in 1:10 -> y) print(x)"),
            @r"
        warning: implicit_assignment
         --> <test>:1:11
          |
        1 | for (x in 1:10 -> y) print(x)
          |           --------- Avoid implicit assignments in `for()` statements.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_true(x <- 1 > 2)"),
            @r"
        warning: implicit_assignment
         --> <test>:1:13
          |
        1 | expect_true(x <- 1 > 2)
          |             ---------- Avoid implicit assignments in function calls.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_no_lint_implicit_assignment() {
        expect_no_lint("x <- 1", "implicit_assignment", None);
        expect_no_lint("x <- { 3 + 4 }", "implicit_assignment", None);
        expect_no_lint("y <- if (is.null(x)) z else x", "implicit_assignment", None);
        expect_no_lint("foo({\na <- 1L\n})", "implicit_assignment", None);
        expect_no_lint("if (1 + 1) TRUE", "implicit_assignment", None);
        expect_no_lint("a %>% b()", "implicit_assignment", None);
        expect_no_lint("a |> b()", "implicit_assignment", None);
        expect_no_lint("if (TRUE) {\nx <- 1\n}", "implicit_assignment", None);
        expect_no_lint("while (TRUE) {\nx <- 1\n}", "implicit_assignment", None);
        expect_no_lint("for (i in 1:2) {\nx <- 1\n}", "implicit_assignment", None);
        expect_no_lint("if (TRUE) x <- 1", "implicit_assignment", None);
        expect_no_lint("for (i in 1:2) x <- 1", "implicit_assignment", None);
        expect_no_lint("while (TRUE) x <- 1", "implicit_assignment", None);
        expect_no_lint(
            "f <- function() {
  if (TRUE)
    x <- 1
  else
    x <- 2
}
",
            "implicit_assignment",
            None,
        );

        // Whitelist: https://github.com/etiennebacher/jarl/issues/133
        expect_no_lint("expect_message(x <- 1)", "implicit_assignment", None);
        expect_no_lint("expect_warning(x <- 1)", "implicit_assignment", None);
        expect_no_lint("expect_error(x <- 1)", "implicit_assignment", None);
        expect_no_lint("expect_snapshot(x <- 1)", "implicit_assignment", None);
        expect_no_lint("quote(x <- 1)", "implicit_assignment", None);
        expect_no_lint("suppressMessages(x <- 1)", "implicit_assignment", None);
        expect_no_lint("suppressWarnings(x <- 1)", "implicit_assignment", None);
        expect_no_lint("suppressWarnings({x <- 1})", "implicit_assignment", None);
    }
}
