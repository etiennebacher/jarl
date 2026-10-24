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

        // Leading comments before the block.
        expect_no_lint(
            r#"
# Setup
library(dplyr)
library(purrr)
x <- 1
"#,
            "library_call",
            None,
        );

        // `options()` and `Sys.setenv()` calls before the block.
        expect_no_lint(
            r#"
options(stringsAsFactors = FALSE)
Sys.setenv(LANG = "en")
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
        expect_no_lint(
            r#"
if (foo) library(bar)
library(baz)
"#,
            "library_call",
            None,
        );

        // Should not be reported because not all the if-else is about library()
        expect_no_lint(
            r#"
library(baz)
x <- 1
if (foo) {
    print(x + 1)
    library(bar)
}
"#,
            "library_call",
            None,
        );
        expect_no_lint(
            r#"
library(baz)
x <- 1
if (foo) {
    library(bar)
} else {
    library(bar2)
    print(x + 1)
}
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
        // No `library()` call at the top.
        insta::assert_snapshot!(
            snapshot_lint(
                r#"
x <- 1

library(foo)
"#
            ),
            @"
        warning: library_call
         --> <test>:4:1
          |
        4 | library(foo)
          | ------------ `library()` calls should be grouped at the top of the script.
          |
          = help: Move this call next to the other `library()` calls.
        Found 1 error.
        "
        );

        // Setup code other than `options()`/`Sys.setenv()` before the first
        // `library()` call.
        insta::assert_snapshot!(
            snapshot_lint(
                r#"
options(stringsAsFactors = FALSE)
x <- 1
library(dplyr)
"#
            ),
            @"
        warning: library_call
         --> <test>:4:1
          |
        4 | library(dplyr)
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
library(baz)
x <- 1
if (foo) {
    library(bar)
} else {
    library(bar2)
}
"#
            ),
            @"
        warning: library_call
         --> <test>:4:1
          |
        4 | / if (foo) {
        5 | |     library(bar)
        6 | | } else {
        7 | |     library(bar2)
        8 | | }
          | |_- `library()` calls should be grouped at the top of the script.
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
                    "library(baz)\nx <- 1\nif (foo) {\n    library(bar)\n} else {\n    library(bar2)\n}"
                ],
                "library_call",
            )
        );

        // No `library()` call at the top: calls are moved at the very top of
        // the file, before leading comments.
        insta::assert_snapshot!(
            "fix_output_no_block_at_top",
            get_unsafe_fixed_text(
                vec![
                    "x <- 1\n\nlibrary(foo)\n",
                    "# Header\nx <- 1\nlibrary(foo)\ny <- 2\nlibrary(bar)\n",
                    "options(warn = 1)\nSys.setenv(LANG = \"en\")\nx <- 1\nlibrary(foo)\n",
                ],
                "library_call",
            )
        );
    }

    #[test]
    fn test_library_call_with_comments() {
        insta::assert_snapshot!(
            "fix_with_comments",
            get_unsafe_fixed_text(
                vec![
                    // Leading comment inside the call.
                    "library(dplyr)\nx <- 1\nlibrary(\n  # leading\n  purrr\n)\n",
                    // Inline comment inside the call.
                    "library(dplyr)\nx <- 1\nlibrary(purrr # inline\n)\n",
                    // Trailing comment on the same line.
                    "library(dplyr)\nx <- 1\nlibrary(purrr) # trailing\n",
                    // Comment preceding the call is not moved.
                    "library(dplyr)\nx <- 1\n# preceding\nlibrary(purrr)\n",
                    // Other code on the same line: no fix.
                    "library(dplyr)\nx <- 1\nlibrary(purrr); y <- 2 # trailing\n",
                ],
                "library_call",
            )
        );
    }
}
