pub(crate) mod blanket_suppression;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "blanket_suppression", None)
    }

    #[test]
    fn test_lint_blanket_suppression() {
        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore
any_is_na(x)"), @r"
        warning: blanket_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore
          | ------------- This comment isn't used by Jarl because it is missing a rule to ignore.
          |
        Found 1 error.
        ");

        insta::assert_snapshot!(snapshot_lint("
#jarl-ignore
any_is_na(x)"), @r"
        warning: blanket_suppression
         --> <test>:2:1
          |
        2 | #jarl-ignore
          | ------------ This comment isn't used by Jarl because it is missing a rule to ignore.
          |
        Found 1 error.
        ");

        insta::assert_snapshot!(snapshot_lint("
#jarl-ignore: <reason>
any_is_na(x)"), @r"
        warning: blanket_suppression
         --> <test>:2:1
          |
        2 | #jarl-ignore: <reason>
          | ---------------------- This comment isn't used by Jarl because it is missing a rule to ignore.
          |
        Found 1 error.
        ");

        insta::assert_snapshot!(snapshot_lint("
# jarl-ignore : <reason>
any_is_na(x)"), @r"
        warning: blanket_suppression
         --> <test>:2:1
          |
        2 | # jarl-ignore : <reason>
          | ------------------------ This comment isn't used by Jarl because it is missing a rule to ignore.
          |
        Found 1 error.
        ");
    }
}
