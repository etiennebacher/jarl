pub(crate) mod assignment;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_assignment() {
        use insta::assert_snapshot;

        let expected_message = "Use `<-` for assignment";
        expect_lint("blah=1", expected_message, "assignment", None);
        expect_lint("blah = 1", expected_message, "assignment", None);
        expect_lint("blah = fun(1)", expected_message, "assignment", None);
        expect_lint("names(blah) = 'a'", expected_message, "assignment", None);
        expect_lint("x[[1]] = 2", expected_message, "assignment", None);
        expect_lint("fun((blah = fun(1)))", expected_message, "assignment", None);
        expect_lint("1 -> fun", expected_message, "assignment", None);
        expect_lint("1 -> names(fun)", expected_message, "assignment", None);
        expect_lint("2 -> x[[1]]", expected_message, "assignment", None);

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
