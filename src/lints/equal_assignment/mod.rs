pub(crate) mod equal_assignment;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_equal_assignment() {
        use insta::assert_snapshot;

        let expected_message = "Use <- for assignment";
        expect_lint("blah=1", expected_message, "equal_assignment");
        expect_lint("blah = 1", expected_message, "equal_assignment");
        expect_lint("blah = fun(1)", expected_message, "equal_assignment");
        expect_lint("fun((blah = fun(1)))", expected_message, "equal_assignment");
        expect_lint("1 -> fun", expected_message, "equal_assignment");

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "blah=1",
                    "blah = 1",
                    "blah = fun(1)",
                    "fun((blah = fun(1)))",
                    "1 -> fun",
                    // TODO
                    // "blah = fun(1) {",
                ],
                "equal_assignment",
            )
        );
    }

    #[test]
    fn test_no_lint_equal_assignment() {
        expect_no_lint("y <- 1", "equal_assignment");
        expect_no_lint("fun(y = 1)", "equal_assignment");
        expect_no_lint("y == 1", "equal_assignment");
    }
}
