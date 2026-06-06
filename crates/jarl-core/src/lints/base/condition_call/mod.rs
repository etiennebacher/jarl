pub(crate) mod condition_call;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "condition_call", None)
    }

    #[test]
    fn test_no_lint_condition_call() {
        // `call. = FALSE` is the desired state.
        expect_no_lint("stop('boom', call. = FALSE)", "condition_call", None);
        expect_no_lint("stop(call. = FALSE, 'boom')", "condition_call", None);
        // Non-literal `call.` values can't be reasoned about.
        expect_no_lint("stop('boom', call. = x)", "condition_call", None);
        expect_no_lint("stop('boom', call. = some_flag())", "condition_call", None);
        // Other functions are not affected.
        expect_no_lint("warning('boom')", "condition_call", None);
        expect_no_lint("message('boom')", "condition_call", None);
        expect_no_lint("stopifnot(x > 0)", "condition_call", None);
    }

    #[test]
    fn test_lint_condition_call_missing_arg() {
        assert_snapshot!(
            snapshot_lint("stop('boom')"),
            @"
        warning: condition_call
         --> <test>:1:1
          |
        1 | stop('boom')
          | ------------ `stop()` includes the call in the error message by default, which may lead to confusion.
          |
          = help: Add `call. = FALSE` to hide it.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("stop('boom', domain = x)"),
            @"
        warning: condition_call
         --> <test>:1:1
          |
        1 | stop('boom', domain = x)
          | ------------------------ `stop()` includes the call in the error message by default, which may lead to confusion.
          |
          = help: Add `call. = FALSE` to hide it.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_condition_call_true() {
        assert_snapshot!(
            snapshot_lint("stop('boom', call. = TRUE)"),
            @"
        warning: condition_call
         --> <test>:1:1
          |
        1 | stop('boom', call. = TRUE)
          | -------------------------- Including the call in the error message may lead to confusion.
          |
          = help: Use `call. = FALSE` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("stop('boom', call. = TRUE, domain = x)"),
            @"
        warning: condition_call
         --> <test>:1:1
          |
        1 | stop('boom', call. = TRUE, domain = x)
          | -------------------------------------- Including the call in the error message may lead to confusion.
          |
          = help: Use `call. = FALSE` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_fix_condition_call() {
        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text(
                vec![
                    "stop('boom')",
                    "stop('a', 'b')",
                    "stop()",
                    "stop('boom', call. = TRUE, domain = x)",
                    "stop('boom', domain = x, call. = TRUE)",
                    "stop(call. = TRUE)",
                ],
                "condition_call",
            )
        );
    }

    #[test]
    fn test_condition_call_with_comments_no_fix() {
        // Lint is detected but the fix is skipped when comments are present to
        // avoid destroying them.
        assert_snapshot!(
            "no_fix_with_comments",
            get_unsafe_fixed_text(
                vec![
                    "stop( # a comment\n  'boom'\n)",
                    "stop('boom', call. = TRUE) # trailing comment",
                ],
                "condition_call",
            )
        );
    }
}
