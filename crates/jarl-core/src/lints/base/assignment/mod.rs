pub(crate) mod assignment;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "assignment", None)
    }

    #[test]
    fn test_lint_assignment() {
        assert_snapshot!(
            snapshot_lint("blah=1"),
            @r"
        warning: assignment
         --> <test>:1:1
          |
        1 | blah=1
          | ----- Use `<-` for assignment.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("blah = 1"),
            @r"
        warning: assignment
         --> <test>:1:1
          |
        1 | blah = 1
          | ------ Use `<-` for assignment.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("blah = fun(1)"),
            @r"
        warning: assignment
         --> <test>:1:1
          |
        1 | blah = fun(1)
          | ------ Use `<-` for assignment.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("names(blah) = 'a'"),
            @r"
        warning: assignment
         --> <test>:1:1
          |
        1 | names(blah) = 'a'
          | ------------- Use `<-` for assignment.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x[[1]] = 2"),
            @r"
        warning: assignment
         --> <test>:1:1
          |
        1 | x[[1]] = 2
          | -------- Use `<-` for assignment.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("fun((blah = fun(1)))"),
            @r"
        warning: assignment
         --> <test>:1:6
          |
        1 | fun((blah = fun(1)))
          |      ------ Use `<-` for assignment.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("1 -> fun"),
            @r"
        warning: assignment
         --> <test>:1:3
          |
        1 | 1 -> fun
          |   ------ Use `<-` for assignment.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("1 -> names(fun)"),
            @r"
        warning: assignment
         --> <test>:1:3
          |
        1 | 1 -> names(fun)
          |   ------------- Use `<-` for assignment.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("2 -> x[[1]]"),
            @r"
        warning: assignment
         --> <test>:1:3
          |
        1 | 2 -> x[[1]]
          |   --------- Use `<-` for assignment.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "blah=1",
                    "blah = 1",
                    "blah = fun(1)",
                    "names(blah) = 'a'",
                    "x[[1]] = 2",
                    "fun((blah = fun(1)))",
                    "1 -> fun",
                    "'a' -> names(fun)",
                    "2 -> x[[1]]",
                ],
                "assignment",
                None
            )
        );
    }

    #[test]
    fn test_no_lint_assignment() {
        expect_no_lint("y <- 1", "assignment", None);
        expect_no_lint("fun(y = 1)", "assignment", None);
        expect_no_lint("y == 1", "assignment", None);
    }

    #[test]
    fn test_assignment_diagnostic_ranges() {
        use crate::utils_test::expect_diagnostic_highlight;

        expect_diagnostic_highlight("x = 1", "assignment", "x =");
        expect_diagnostic_highlight("x=1", "assignment", "x=");
        expect_diagnostic_highlight("1 -> x", "assignment", "-> x");
        expect_diagnostic_highlight("foo() |>\n  bar() |>\n  baz() -> x", "assignment", "-> x");
        // TODO: uncomment when https://github.com/etiennebacher/jarl/issues/89 is fixed
        // expect_diagnostic_highlight("1 -> names(\nx)", "assignment", "-> names(\nx)");
    }
}
