pub(crate) mod library_call;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "library_call", None)
    }

    #[test]
    fn test_no_lint_library_call() {
        // Consecutive block at the very top.
        expect_no_lint(
            r#"
library(dplyr)
library(purrr)
x <- 1
"#,
            "library_call",
            None,
        );

        // Block after a preamble.
        expect_no_lint(
            r#"
options(stringsAsFactors = FALSE)
library(dplyr)
library(purrr)
x <- 1
"#,
            "library_call",
            None,
        );

        // `if` whose branch contains only library calls, followed by another
        // library call: both belong to the block.
        expect_no_lint(
            r#"
if (foo) {
  library(bar)
}
library(baz)
"#,
            "library_call",
            None,
        );

        // `suppressPackageStartupMessages(library(x))` is part of the block.
        expect_no_lint(
            r#"
library(dplyr)
suppressPackageStartupMessages(library(zoo))
x <- 1
"#,
            "library_call",
            None,
        );

        // `library()` inside a function body is never reported.
        expect_no_lint(
            r#"
foo <- function() {
  library(dplyr)
  1
}
"#,
            "library_call",
            None,
        );

        // Order within the block doesn't matter.
        expect_no_lint(
            r#"
library(purrr)
library(dplyr)
x <- 1
"#,
            "library_call",
            None,
        );
    }

    #[test]
    fn test_lint_library_call() {
        insta::assert_snapshot!(
            snapshot_lint(
                r#"
library(dplyr)
x <- 1
library(purrr)
"#
            ),
            @"
        warning: library_call
         --> <test>:4:1
          |
        4 | library(purrr)
          | -------------- `library()` calls should be grouped at the top of the script.
          |
          = help: Move this call next to the other `library()` calls.
        Found 1 error.
        "
        );

        insta::assert_snapshot!(
            snapshot_lint(
                r#"
library(dplyr)
x <- 1
library(purrr)
library(abc)
"#
            ),
            @"
        warning: library_call
         --> <test>:4:1
          |
        4 | library(purrr)
          | -------------- `library()` calls should be grouped at the top of the script.
          |
          = help: Move this call next to the other `library()` calls.
        warning: library_call
         --> <test>:5:1
          |
        5 | library(abc)
          | ------------ `library()` calls should be grouped at the top of the script.
          |
          = help: Move this call next to the other `library()` calls.
        Found 2 errors.
        "
        );
    }

    #[test]
    fn fix_output() {
        insta::assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text(
                vec![
                    "options(stringsAsFactors = FALSE)\nlibrary(dplyr)\nif (interactive()) {\n  library(rlang)\n}\nsuppressPackageStartupMessages(library(zoo))\nx <- 1\nlibrary(purrr)\nlibrary(abc)\n",
                ],
                "library_call",
            )
        );

        // The fix is unsafe: the plain (safe-only) helper shows no change.
        insta::assert_snapshot!(
            "fix_output_unsafe_only",
            get_fixed_text(
                vec!["library(dplyr)\nx <- 1\nlibrary(purrr)\n"],
                "library_call",
                None,
            )
        );
    }

    #[test]
    fn test_library_call_with_comments_no_fix() {
        insta::assert_snapshot!(
            "no_fix_with_comments",
            get_unsafe_fixed_text(
                vec![
                    // Leading comment inside the call.
                    "library(dplyr)\nx <- 1\nlibrary(\n  # leading\n  purrr\n)\n",
                    // Inline comment inside the call.
                    "library(dplyr)\nx <- 1\nlibrary(purrr # inline\n)\n",
                    // Trailing comment on the same line.
                    "library(dplyr)\nx <- 1\nlibrary(purrr) # trailing\n",
                ],
                "library_call",
            )
        );
    }
}
