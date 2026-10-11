pub(crate) mod function_argument;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "function_argument", None)
    }

    #[test]
    fn test_no_lint_function_argument() {
        for code in [
            "function() {}",
            "function(x, y) {}",
            "function(x, y = 1, z = NULL) {}",
            "function(x = 1, y = 2) {}",
            "function(x = 1, ...) {}",
            "function(..., y) {}",
            "function(x, ..., y) {}",
            "function(x, y, ..., z = 1) {}",
            "function(x = 1, ..., y = NULL) {}",
            r"\(x, y = 1) {}",
            "function(x = 1) { function(a, b) {} }",
            "fun(x = 1, y)",
        ] {
            expect_no_lint(code, "function_argument", None);
        }
    }

    #[test]
    fn test_lint_function_argument() {
        assert_snapshot!(
            snapshot_lint("function(x, y = 1, z, w = 2) {}"),
            @"
        warning: function_argument
         --> <test>:1:20
          |
        1 | function(x, y = 1, z, w = 2) {}
          |                    - Arguments without defaults should come before arguments with defaults.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("function(x = NULL, y, z) {}"),
            @"
        warning: function_argument
         --> <test>:1:20
          |
        1 | function(x = NULL, y, z) {}
          |                    - Arguments without defaults should come before arguments with defaults.
          |
        warning: function_argument
         --> <test>:1:23
          |
        1 | function(x = NULL, y, z) {}
          |                       - Arguments without defaults should come before arguments with defaults.
          |
        Found 2 errors.
        "
        );

        assert_snapshot!(
            snapshot_lint("function(x = 1, ..., y, z = 2) {}"),
            @"
        warning: function_argument
         --> <test>:1:22
          |
        1 | function(x = 1, ..., y, z = 2) {}
          |                      - Arguments without defaults should come before arguments with defaults.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint(r"\(x = 1, y) {}"),
            @r"
        warning: function_argument
         --> <test>:1:10
          |
        1 | \(x = 1, y) {}
          |          - Arguments without defaults should come before arguments with defaults.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("function(\n  x # comment\n  = calculate(),\n  y\n) {}"),
            @"
        warning: function_argument
         --> <test>:4:3
          |
        4 |   y
          |   - Arguments without defaults should come before arguments with defaults.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("function(x = 1, `not syntactic`, 数据) {}"),
            @"
        warning: function_argument
         --> <test>:1:17
          |
        1 | function(x = 1, `not syntactic`, 数据) {}
          |                 --------------- Arguments without defaults should come before arguments with defaults.
          |
        warning: function_argument
         --> <test>:1:34
          |
        1 | function(x = 1, `not syntactic`, 数据) {}
          |                                  ---- Arguments without defaults should come before arguments with defaults.
          |
        Found 2 errors.
        "
        );

        assert_snapshot!(
            snapshot_lint("function(x = 1, y) { function(a = 1, b) {} }"),
            @"
        warning: function_argument
         --> <test>:1:17
          |
        1 | function(x = 1, y) { function(a = 1, b) {} }
          |                 - Arguments without defaults should come before arguments with defaults.
          |
        warning: function_argument
         --> <test>:1:38
          |
        1 | function(x = 1, y) { function(a = 1, b) {} }
          |                                      - Arguments without defaults should come before arguments with defaults.
          |
        Found 2 errors.
        "
        );

        assert_snapshot!(
            snapshot_lint("function(x = function(a = 1, b) {}, y = 2) {}"),
            @"
        warning: function_argument
         --> <test>:1:30
          |
        1 | function(x = function(a = 1, b) {}, y = 2) {}
          |                              - Arguments without defaults should come before arguments with defaults.
          |
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("function(x = 1, y) if (missing(y)) 2 else y"),
            @"
        warning: function_argument
         --> <test>:1:17
          |
        1 | function(x = 1, y) if (missing(y)) 2 else y
          |                 - Arguments without defaults should come before arguments with defaults.
          |
        Found 1 error.
        "
        );
    }
}
