pub(crate) mod literal_coercion;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "literal_coercion", None)
    }

    #[test]
    fn test_no_lint_literal_coercion_allowed() {
        // The argument is not a literal.
        expect_no_lint("as.numeric(x$\"_f0\")", "literal_coercion", None);
        expect_no_lint("as.numeric(x@\"_f0\")", "literal_coercion", None);
        // Only the first argument of `as.<type>()` is examined.
        expect_no_lint(
            "as.character(as.Date(x), '%Y%m%d')",
            "literal_coercion",
            None,
        );

        // Empty calls are not flagged because they could lead to perf decrease
        // https://stackoverflow.com/questions/79952588/why-is-as-logical-much-faster-than-logical0
        expect_no_lint("as.integer()", "literal_coercion", None);
        expect_no_lint("as.integer(x = )", "literal_coercion", None);

        // We are agnostic about preferring literals over coerced vectors.
        expect_no_lint("as.integer(c(1, 2, 3))", "literal_coercion", None);
        expect_no_lint("as.character(c(1, 2, 3))", "literal_coercion", None);
        // Not possible to declare raw literals.
        expect_no_lint("as.raw(c(1, 2, 3))", "literal_coercion", None);
        // Not taking a stand on as.complex(0) vs. 0 + 0i.
        expect_no_lint("as.complex(0)", "literal_coercion", None);
        // Ignore complex values
        expect_no_lint("as.integer(1i)", "literal_coercion", None);
        // Scientific notation is left alone.
        expect_no_lint("as.integer(1e6)", "literal_coercion", None);
        // A range is not a scalar literal.
        expect_no_lint("as.numeric(1:3)", "literal_coercion", None);
    }

    #[test]
    fn test_no_lint_literal_coercion_rlang_allowed() {
        expect_no_lint("int(1, 2.0, 3)", "literal_coercion", None);
        expect_no_lint("chr('e', 'ab', 'xyz')", "literal_coercion", None);
        expect_no_lint("lgl(0, 1)", "literal_coercion", None);
        expect_no_lint("lgl(0L, 1)", "literal_coercion", None);
        expect_no_lint("dbl(1.2, 1e5, 3L, 2E4)", "literal_coercion", None);
        // Using a namespace (`rlang::`) doesn't create problems.
        expect_no_lint("rlang::int(1, 2, 3)", "literal_coercion", None);
        // Even if scalar, scientific notation is carved out.
        expect_no_lint("int(1.0e6)", "literal_coercion", None);
    }

    #[test]
    fn test_no_lint_literal_coercion_quoted_keyword_args() {
        expect_no_lint("as.numeric(foo('a' = 1))", "literal_coercion", None);
        expect_no_lint(
            "as.numeric(foo('a' # comment\n= 1))",
            "literal_coercion",
            None,
        );
    }

    #[test]
    fn test_no_lint_literal_coercion_unrelated_namespace() {
        expect_no_lint("mypkg::int(1L)", "literal_coercion", None);
    }

    #[test]
    fn test_lint_literal_coercion_basic() {
        assert_snapshot!(
            snapshot_lint("as.integer(1)"),
            @"
        warning: literal_coercion
         --> <test>:1:1
          |
        1 | as.integer(1)
          | ------------- This coercion can be simplified.
          |
          = help: Use `1L` instead of `as.integer(1)`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("as.character(1)"),
            @r#"
        warning: literal_coercion
         --> <test>:1:1
          |
        1 | as.character(1)
          | --------------- This coercion can be simplified.
          |
          = help: Use `"1"` instead of `as.character(1)`.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("as.logical(\"true\")"),
            @r#"
        warning: literal_coercion
         --> <test>:1:1
          |
        1 | as.logical("true")
          | ------------------ This coercion can be simplified.
          |
          = help: Use `TRUE` instead of `as.logical("true")`.
        Found 1 error.
        "#
        );
    }

    #[test]
    fn test_lint_literal_coercion_na() {
        assert_snapshot!(
            snapshot_lint("as.integer(NA)"),
            @"
        warning: literal_coercion
         --> <test>:1:1
          |
        1 | as.integer(NA)
          | -------------- This coercion can be simplified.
          |
          = help: Use `NA_integer_` instead of `as.integer(NA)`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("as.integer('a')"),
            @"
        warning: literal_coercion
         --> <test>:1:1
          |
        1 | as.integer('a')
          | --------------- This coercion can be simplified.
          |
          = help: Use `NA_integer_` instead of `as.integer('a')`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("as.logical('a')"),
            @"
        warning: literal_coercion
         --> <test>:1:1
          |
        1 | as.logical('a')
          | --------------- This coercion can be simplified.
          |
          = help: Use `NA` instead of `as.logical('a')`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("as.integer(2147483648)"),
            @"
        warning: literal_coercion
         --> <test>:1:1
          |
        1 | as.integer(2147483648)
          | ---------------------- This coercion can be simplified.
          |
          = help: Use `NA_integer_` instead of `as.integer(2147483648)`.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_literal_coercion_rlang() {
        assert_snapshot!(
            snapshot_lint("lgl(1L)"),
            @"
        warning: literal_coercion
         --> <test>:1:1
          |
        1 | lgl(1L)
          | ------- This coercion can be simplified.
          |
          = help: Use `TRUE` instead of `lgl(1L)`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("rlang::lgl(1L)"),
            @"
        warning: literal_coercion
         --> <test>:1:1
          |
        1 | rlang::lgl(1L)
          | -------------- This coercion can be simplified.
          |
          = help: Use `TRUE` instead of `rlang::lgl(1L)`.
        Found 1 error.
        "
        );
        // Scalar `list2()` construction with a trailing empty argument.
        assert_snapshot!(
            snapshot_lint("int(1, )"),
            @"
        warning: literal_coercion
         --> <test>:1:1
          |
        1 | int(1, )
          | -------- This coercion can be simplified.
          |
          = help: Use `1L` instead of `int(1,)`.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_literal_coercion_messages() {
        // Base coercions across the supported target types.
        let snippets = vec![
            "as.logical(1L)",
            "as.logical(1)",
            "as.logical(TRUE)",
            "as.logical(\"true\")",
            "as.logical(\"false\")",
            "as.integer(1)",
            "as.integer('1')",
            "as.integer(1L)",
            "as.integer(TRUE)",
            "as.numeric(1)",
            "as.double(1)",
            "as.double(TRUE)",
            "as.double('1')",
            "as.double('hi')",
            "as.character(1)",
            "as.character(1L)",
            "as.character(\"e\")",
            "as.character(TRUE)",
            "as.character(FALSE)",
            "as.integer(NA)",
            "as.numeric(NA)",
            "as.logical(NA)",
            "as.double(NA)",
            "as.character(NA)",
            // rlang helpers.
            "lgl(1L)",
            "rlang::lgl(1L)",
            "int(1.0)",
            "dbl(1L)",
            "chr(\"e\")",
            "chr(\"E\")",
        ];
        assert_snapshot!(get_diagnostic_messages(snippets));
    }

    #[test]
    fn test_literal_coercion_fix() {
        assert_snapshot!(get_fixed_text(
            vec![
                "as.integer(1)",
                "as.numeric(1)",
                "as.character(1)",
                "as.logical(\"true\")",
                "as.integer(NA)",
                "as.character(NA)",
                "lgl(1L)",
                "rlang::lgl(1L)",
                "int(1, )",
            ],
            "literal_coercion",
            None
        ));
    }

    #[test]
    fn test_literal_coercion_comments_no_fix() {
        // A lint is still reported when comments are present, but the fix is
        // skipped so the comments are not destroyed.
        assert_snapshot!(get_fixed_text(
            vec!["as.integer( # comment\n1 # comment\n)"],
            "literal_coercion",
            None
        ));
    }

    /// Render just the diagnostic message body for each snippet, one per line.
    fn get_diagnostic_messages(snippets: Vec<&str>) -> String {
        let mut out = String::new();
        for snippet in snippets {
            let diagnostics = check_code(snippet, "literal_coercion", None);
            let msg = diagnostics
                .first()
                .map(|d| d.message.body.clone())
                .unwrap_or_else(|| "<no lint>".to_string());
            out.push_str(&format!("{snippet}\n  => {msg}\n"));
        }
        out.trim_end().to_string()
    }
}
