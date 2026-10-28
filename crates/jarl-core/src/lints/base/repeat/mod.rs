pub(crate) mod repeat;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "repeat", None)
    }

    #[test]
    fn test_no_lint_repeat() {
        for code in [
            "repeat {}",
            "while (FALSE) {}",
            "while (((FALSE))) {}",
            "while (i < 5) {}",
            "while (j < 5) TRUE",
            "while ((TRUE && j < 5)) {}",
            "while (T) {}",
            "while (1) {}",
            "while (identity(TRUE)) {}",
        ] {
            expect_no_lint(code, "repeat", None);
        }
    }

    #[test]
    fn test_lint_repeat() {
        assert_snapshot!(snapshot_lint("while (TRUE) {}"), @"
        warning: repeat
         --> <test>:1:1
          |
        1 | while (TRUE) {}
          | ------------ `while (TRUE)` is less clear than `repeat` for infinite loops.
          |
          = help: Use `repeat {}` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("while (((TRUE))) break"), @"
        warning: repeat
         --> <test>:1:1
          |
        1 | while (((TRUE))) break
          | ---------------- `while (TRUE)` is less clear than `repeat` for infinite loops.
          |
          = help: Use `repeat {}` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint(
            "for (i in 1:10) {
    while (TRUE) {
        if (i == 5) break
    }
}"
        ), @"
        warning: repeat
         --> <test>:2:5
          |
        2 |     while (TRUE) {
          |     ------------ `while (TRUE)` is less clear than `repeat` for infinite loops.
          |
          = help: Use `repeat {}` instead.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint(
            "while (
    # Keep this condition comment.
    (TRUE)
) break"
        ), @"
        warning: repeat
         --> <test>:1:1
          |
        1 | / while (
        2 | |     # Keep this condition comment.
        3 | |     (TRUE)
        4 | | ) break
          | |_- `while (TRUE)` is less clear than `repeat` for infinite loops.
          |
          = help: Use `repeat {}` instead.
        Found 1 error.
        ");
    }

    #[test]
    fn test_fix_repeat() {
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "while (TRUE) {}",
                    "while(TRUE)break",
                    "while ((TRUE)) next",
                    "while (((TRUE))) {
    # Keep this body comment.
    break
}",
                    "for (i in 1:10) {
    while (TRUE) {
        if (i == 5) break
    }
}",
                    "while (TRUE) if (done) break",
                    "# Before the loop.
while (TRUE) # Between the header and body.
    break # After the loop.",
                    "while (TRUE)
{
    break
}",
                    "while (((TRUE))) {
    while (TRUE) break
    break
}",
                    // A comment in the replaced header must be preserved.
                    "while ( # After the opening parenthesis.
    TRUE
) break",
                    "while (TRUE # Before the closing parenthesis.
) break",
                    "while ((
    # Inside extra parentheses.
    TRUE
)) break",
                ],
                "repeat",
                None
            )
        );
    }
}
