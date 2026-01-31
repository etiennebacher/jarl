pub(crate) mod blanket_suppression;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_blanket_suppression() {
        let lint_msg = "isn't used by Jarl because it suppresses all possible violations";

        expect_lint(
            "
# jarl-ignore
any_is_na(x)",
            lint_msg,
            "blanket_suppression",
            None,
        );
        expect_lint(
            "
#jarl-ignore
any_is_na(x)",
            lint_msg,
            "blanket_suppression",
            None,
        );
        expect_lint(
            "
#jarl-ignore: <reason>
any_is_na(x)",
            lint_msg,
            "blanket_suppression",
            None,
        );

        // With space before colon
        expect_lint(
            "
# jarl-ignore : <reason>
any_is_na(x)",
            lint_msg,
            "blanket_suppression",
            None,
        );
    }
}
