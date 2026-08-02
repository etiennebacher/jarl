pub(crate) mod condition_message;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "condition_message", None)
    }

    #[test]
    fn test_no_lint_condition_message() {
        expect_no_lint("stop('boom')", "condition_message", None);
        expect_no_lint("stop('hello', 'there')", "condition_message", None);
        expect_no_lint("stop('boom', call. = FALSE)", "condition_message", None);
        expect_no_lint(
            "stop('hello', call. = FALSE, 'there')",
            "condition_message",
            None,
        );
        expect_no_lint(
            "stop(paste0('hello', 'there', recycle0 = TRUE))",
            "condition_message",
            None,
        );
        expect_no_lint(
            "stop(paste0('hello', 'there', collapse = ''))",
            "condition_message",
            None,
        );
        // Not covering paste() because we would need to modify the strings themselves,
        // which sounds annoying to do.
        expect_no_lint("stop(paste('hello', 'there'))", "condition_message", None);

        // for warning()
        expect_no_lint("warning('boom', call. = FALSE)", "condition_message", None);
    }

    #[test]
    fn test_lint_condition_message_works_stop() {
        assert_snapshot!(
            snapshot_lint("stop(paste0('hello ', 'there'))"),
            @"
        warning: condition_message
         --> <test>:1:1
          |
        1 | stop(paste0('hello ', 'there'))
          | ------------------------------- `stop(paste0(...))` can be simplified.
          |
          = help: Use `stop(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("stop(paste0('hello ', 'there'), call. = FALSE)"),
            @"
        warning: condition_message
         --> <test>:1:1
          |
        1 | stop(paste0('hello ', 'there'), call. = FALSE)
          | ---------------------------------------------- `stop(paste0(...))` can be simplified.
          |
          = help: Use `stop(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("stop(call. = FALSE, paste0('hello ', 'there'))"),
            @"
        warning: condition_message
         --> <test>:1:1
          |
        1 | stop(call. = FALSE, paste0('hello ', 'there'))
          | ---------------------------------------------- `stop(paste0(...))` can be simplified.
          |
          = help: Use `stop(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("stop(paste0('hello ', 'there'), call. = FALSE, ' again')"),
            @"
        warning: condition_message
         --> <test>:1:1
          |
        1 | stop(paste0('hello ', 'there'), call. = FALSE, ' again')
          | -------------------------------------------------------- `stop(paste0(...))` can be simplified.
          |
          = help: Use `stop(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("stop(paste0('hello ', 'there'), call. = FALSE, domain = foo)"),
            @"
        warning: condition_message
         --> <test>:1:1
          |
        1 | stop(paste0('hello ', 'there'), call. = FALSE, domain = foo)
          | ------------------------------------------------------------ `stop(paste0(...))` can be simplified.
          |
          = help: Use `stop(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("stop(domain = foo, paste0('hello ', 'there'), call. = FALSE)"),
            @"
        warning: condition_message
         --> <test>:1:1
          |
        1 | stop(domain = foo, paste0('hello ', 'there'), call. = FALSE)
          | ------------------------------------------------------------ `stop(paste0(...))` can be simplified.
          |
          = help: Use `stop(...)` instead.
        Found 1 error.
        "
        );
    }
    #[test]
    fn test_lint_condition_message_works_warning() {
        assert_snapshot!(
            snapshot_lint("warning(paste0('hello ', 'there'))"),
            @"
        warning: condition_message
         --> <test>:1:1
          |
        1 | warning(paste0('hello ', 'there'))
          | ---------------------------------- `warning(paste0(...))` can be simplified.
          |
          = help: Use `warning(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("warning(paste0('hello ', 'there'), call. = FALSE)"),
            @"
        warning: condition_message
         --> <test>:1:1
          |
        1 | warning(paste0('hello ', 'there'), call. = FALSE)
          | ------------------------------------------------- `warning(paste0(...))` can be simplified.
          |
          = help: Use `warning(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("warning(call. = FALSE, paste0('hello ', 'there'))"),
            @"
        warning: condition_message
         --> <test>:1:1
          |
        1 | warning(call. = FALSE, paste0('hello ', 'there'))
          | ------------------------------------------------- `warning(paste0(...))` can be simplified.
          |
          = help: Use `warning(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("warning(paste0('hello ', 'there'), call. = FALSE, ' again')"),
            @"
        warning: condition_message
         --> <test>:1:1
          |
        1 | warning(paste0('hello ', 'there'), call. = FALSE, ' again')
          | ----------------------------------------------------------- `warning(paste0(...))` can be simplified.
          |
          = help: Use `warning(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("warning(paste0('hello ', 'there'), call. = FALSE, domain = foo)"),
            @"
        warning: condition_message
         --> <test>:1:1
          |
        1 | warning(paste0('hello ', 'there'), call. = FALSE, domain = foo)
          | --------------------------------------------------------------- `warning(paste0(...))` can be simplified.
          |
          = help: Use `warning(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("warning(domain = foo, paste0('hello ', 'there'), call. = FALSE)"),
            @"
        warning: condition_message
         --> <test>:1:1
          |
        1 | warning(domain = foo, paste0('hello ', 'there'), call. = FALSE)
          | --------------------------------------------------------------- `warning(paste0(...))` can be simplified.
          |
          = help: Use `warning(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("warning(domain = foo, paste0('hello ', 'there'), immediate. = FALSE)"),
            @"
        warning: condition_message
         --> <test>:1:1
          |
        1 | warning(domain = foo, paste0('hello ', 'there'), immediate. = FALSE)
          | -------------------------------------------------------------------- `warning(paste0(...))` can be simplified.
          |
          = help: Use `warning(...)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("warning(domain = foo, paste0('hello ', 'there'), noBreaks. = FALSE)"),
            @"
        warning: condition_message
         --> <test>:1:1
          |
        1 | warning(domain = foo, paste0('hello ', 'there'), noBreaks. = FALSE)
          | ------------------------------------------------------------------- `warning(paste0(...))` can be simplified.
          |
          = help: Use `warning(...)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_fix_condition_message() {
        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text(
                vec![
                    "stop(paste0('hello ', 'there'))",
                    "stop(paste0('hello ', 'there'), call. = FALSE)",
                    "stop(paste0('hello ', 'there'), domain = foo)",
                    "stop(call. = FALSE, paste0('hello ', 'there'), domain = foo)",
                    "warning(paste0('hello ', 'there'))",
                    "warning(paste0('hello ', 'there'), call. = FALSE)",
                    "warning(paste0('hello ', 'there'), domain = foo)",
                    "warning(paste0('hello ', 'there'), immediate. = FALSE)",
                    "warning(paste0('hello ', 'there'), noBreaks. = FALSE)",
                    "warning(call. = FALSE, paste0('hello ', 'there'), domain = foo)",
                ],
                "condition_message",
            )
        );
    }

    #[test]
    fn test_condition_message_with_comments_no_fix() {
        // Lint is detected but the fix is skipped when comments are present in the
        // string to fix.
        assert_snapshot!(
            "no_fix_with_comments",
            get_unsafe_fixed_text(
                vec![
                    "stop( # a comment\n  paste0('hello')\n)",
                    "stop(paste0('hello'), call. = TRUE) # trailing comment",
                ],
                "condition_message",
            )
        );
    }
}
