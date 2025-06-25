pub(crate) mod length_test;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_length_test() {
        use insta::assert_snapshot;
        let expected_message = "Checking the length of a logical vector";

        expect_lint("length(x != 0)", expected_message, "length_test");
        expect_lint("length(x >= 0)", expected_message, "length_test");
        expect_lint("length(x <= 0)", expected_message, "length_test");
        expect_lint("length(x > 0)", expected_message, "length_test");
        expect_lint("length(x < 0)", expected_message, "length_test");
        expect_lint("length(x < 0)", expected_message, "length_test");
        expect_lint("length(x + y == 2)", expected_message, "length_test");

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "length(x != 0)",
                    "length(x >= 0)",
                    "length(x <= 0)",
                    "length(x > 0)",
                    "length(x < 0)",
                    "length(x < 0)",
                    "length(x + y == 2)"
                ],
                "length_test"
            )
        );
    }

    #[test]
    fn test_no_lint_length_test() {
        expect_no_lint("length(x) > 0", "length_test");
        expect_no_lint("length(DF[key == val, cols])", "length_test");
    }
}
