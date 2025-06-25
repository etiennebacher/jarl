pub(crate) mod length_levels;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_length_levels() {
        use insta::assert_snapshot;
        let expected_message = "Use `nlevels(...)` instead";

        expect_lint("2:length(levels(x))", expected_message, "length_levels");
        expect_lint(
            "2:length(levels(foo(a)))",
            expected_message,
            "length_levels",
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec!["2:length(levels(x))", "2:length(levels(foo(a)))",],
                "length_levels"
            )
        );
    }

    #[test]
    fn test_no_lint_length_levels() {
        expect_no_lint("length(c(levels(x), 'a'))", "length_levels");
    }
}
