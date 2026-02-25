pub(crate) mod unused_function_argument;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "unused_function_argument", None)
    }

    #[test]
    fn test_no_lint_unused_function_argument() {
        // Argument used directly
        expect_no_lint("function(x) x + 1", "unused_function_argument", None);
        // Dots used
        expect_no_lint("function(...) list(...)", "unused_function_argument", None);
        // Used via assignment
        expect_no_lint(
            "function(x) { y <- x; y }",
            "unused_function_argument",
            None,
        );
        // Both used
        expect_no_lint("function(x, y) x + y", "unused_function_argument", None);
        // Default value, still used
        expect_no_lint("function(x = 1) x", "unused_function_argument", None);
        // Nested function: x used in inner function
        expect_no_lint(
            "function(x) { f <- function() x; f() }",
            "unused_function_argument",
            None,
        );
        // Dots with other used args
        expect_no_lint(
            "function(x, ...) { x + 1 }",
            "unused_function_argument",
            None,
        );
        // Lambda syntax
        expect_no_lint("\\(x) x + 1", "unused_function_argument", None);
        // Used in if condition
        expect_no_lint(
            "function(x) { if (x) 1 else 2 }",
            "unused_function_argument",
            None,
        );
        // Used in function call
        expect_no_lint("function(x) print(x)", "unused_function_argument", None);
    }

    #[test]
    fn test_no_lint_use_method() {
        // S3 generics: UseMethod() dispatches all arguments
        expect_no_lint(
            "function(x, method = \"loess\", ...) { UseMethod(\"f\") }",
            "unused_function_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_next_method() {
        // S3 methods: NextMethod() forwards all arguments
        expect_no_lint(
            "function(data, i, ...) { out <- NextMethod(); out }",
            "unused_function_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_dynamic_arg_access() {
        // match.call() captures all arguments
        expect_no_lint(
            "function(x, y) { cl <- match.call(); cl }",
            "unused_function_argument",
            None,
        );
        // environment() captures all bindings
        expect_no_lint(
            "function(x, y) { .args <- as.list(environment()); do.call(f, .args) }",
            "unused_function_argument",
            None,
        );
        // sys.call()
        expect_no_lint(
            "function(x, y) { cl <- sys.call(); cl }",
            "unused_function_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_on_load() {
        // .onLoad hook: required signature, args often unused
        expect_no_lint(
            ".onLoad <- function(libname, pkgname) { session_r_version <- base::getRversion() }",
            "unused_function_argument",
            None,
        );
        // .onAttach hook
        expect_no_lint(
            ".onAttach <- function(libname, pkgname) { 1 }",
            "unused_function_argument",
            None,
        );
        // With = instead of <-
        expect_no_lint(
            ".onLoad = function(libname, pkgname) { 1 }",
            "unused_function_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_trycatch_handler() {
        // tryCatch error handler
        expect_no_lint(
            "tryCatch(x, error = function(e) 'DEV')",
            "unused_function_argument",
            None,
        );
        // tryCatch warning handler
        expect_no_lint(
            "tryCatch(x, warning = function(w) 'DEV')",
            "unused_function_argument",
            None,
        );
        // withCallingHandlers
        expect_no_lint(
            "withCallingHandlers(x, message = function(m) 'DEV')",
            "unused_function_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_glue_interpolation() {
        // Argument used in glue string
        expect_no_lint(
            "function(x) { glue(\"{x}\") }",
            "unused_function_argument",
            None,
        );
        // Argument used in cli_abort glue string
        expect_no_lint(
            "function(x) { cli_abort(\"{x}\") }",
            "unused_function_argument",
            None,
        );
        // Multiple glue references
        expect_no_lint(
            "function(x, y) { glue(\"{x} and {y}\") }",
            "unused_function_argument",
            None,
        );
        // Complex glue expression
        expect_no_lint(
            "function(x) { glue(\"{x + 1}\") }",
            "unused_function_argument",
            None,
        );
        // Glue with function call
        expect_no_lint(
            "function(x) { glue(\"{paste0(x, 'suffix')}\") }",
            "unused_function_argument",
            None,
        );
        // Escaped braces should NOT match
        expect_no_lint(
            "function(x) { glue(\"{{not_a_ref}} {x}\") }",
            "unused_function_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_backtick_identifiers() {
        // Argument used via backtick-quoted reference
        expect_no_lint(
            "function(self) { f(`self`) }",
            "unused_function_argument",
            None,
        );
        // Backtick in nested function
        expect_no_lint(
            "f5 <- function(self) { function() { f(.Call(f, `self`)) } }",
            "unused_function_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_keyword_as_parameter() {
        // `return` used as parameter name — parser treats bare `return` as keyword
        expect_no_lint(
            "function(return) { return }",
            "unused_function_argument",
            None,
        );
    }

    #[test]
    fn test_no_lint_dollar_access() {
        // Argument used via $ extraction (e.g. Shiny's input$var)
        expect_no_lint(
            "function(input) { input$var }",
            "unused_function_argument",
            None,
        );
    }

    #[test]
    fn test_lint_unused_function_argument() {
        assert_snapshot!(
            snapshot_lint("function(x) 1"),
            @r#"
        warning: unused_function_argument
         --> <test>:1:10
          |
        1 | function(x) 1
          |          - Argument "x" is not used in the function body.
          |
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("function(x, y) x + 1"),
            @r#"
        warning: unused_function_argument
         --> <test>:1:13
          |
        1 | function(x, y) x + 1
          |             - Argument "y" is not used in the function body.
          |
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("function(x, y, z) x + z"),
            @r#"
        warning: unused_function_argument
         --> <test>:1:13
          |
        1 | function(x, y, z) x + z
          |             - Argument "y" is not used in the function body.
          |
        Found 1 error.
        "#
        );
    }

    #[test]
    fn test_lint_unused_function_argument_lambda() {
        assert_snapshot!(
            snapshot_lint("\\(x, y) x + 1"),
            @r#"
        warning: unused_function_argument
         --> <test>:1:6
          |
        1 | \(x, y) x + 1
          |      - Argument "y" is not used in the function body.
          |
        Found 1 error.
        "#
        );
    }

    #[test]
    fn test_lint_unused_function_argument_multiple() {
        assert_snapshot!(
            snapshot_lint("function(x, y, z) 1"),
            @r#"
        warning: unused_function_argument
         --> <test>:1:10
          |
        1 | function(x, y, z) 1
          |          - Argument "x" is not used in the function body.
          |
        warning: unused_function_argument
         --> <test>:1:13
          |
        1 | function(x, y, z) 1
          |             - Argument "y" is not used in the function body.
          |
        warning: unused_function_argument
         --> <test>:1:16
          |
        1 | function(x, y, z) 1
          |                - Argument "z" is not used in the function body.
          |
        Found 3 errors.
        "#
        );
    }
}
