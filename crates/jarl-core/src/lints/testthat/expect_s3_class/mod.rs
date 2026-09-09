pub(crate) mod expect_s3_class;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "expect_s3_class", None)
    }

    #[test]
    fn test_no_lint_expect_s3_class() {
        // We don't handle those args
        expect_no_lint(
            "expect_equal(class(x), 'a', info = 'x should have class k')",
            "expect_s3_class",
            None,
        );
        expect_no_lint(
            "expect_equal(class(x), 'a', label = 'x class')",
            "expect_s3_class",
            None,
        );
        expect_no_lint(
            "expect_equal(class(x), 'a', expected.label = 'target class')",
            "expect_s3_class",
            None,
        );

        // Those do not work in `expect_s3_class()`.
        expect_no_lint("expect_equal(class(x), 'list')", "expect_s3_class", None);
        expect_no_lint("expect_equal(class(x), 'logical')", "expect_s3_class", None);
        expect_no_lint("expect_equal(class(x), 'matrix')", "expect_s3_class", None);

        // Wrong code but no panic
        expect_no_lint("expect_equal(class(x))", "expect_s3_class", None);
        expect_no_lint("expect_equal(class())", "expect_s3_class", None);
        expect_no_lint(
            "expect_equal(object =, expected =)",
            "expect_s3_class",
            None,
        );
        expect_no_lint("expect_true(is.matrix(x))", "expect_s3_class", None);
        expect_no_lint("expect_true(is.nan(x))", "expect_s3_class", None);
        expect_no_lint(
            "expect_true(is.data.frame(x), info = 'context')",
            "expect_s3_class",
            None,
        );
        expect_no_lint("expect_true(is.data.frame())", "expect_s3_class", None);
        expect_no_lint("expect_true(is.data.frame(x, y))", "expect_s3_class", None);
        expect_no_lint("expect_true(is.data.frame(x =))", "expect_s3_class", None);
        expect_no_lint("expect_true(inherits(x))", "expect_s3_class", None);
        expect_no_lint(
            "expect_true(inherits(x, 'matrix'))",
            "expect_s3_class",
            None,
        );
        expect_no_lint(
            "expect_true(inherits(x, 'foo', which = TRUE))",
            "expect_s3_class",
            None,
        );
        expect_no_lint(
            "expect_true(inherits(x, 'foo'), info = 'context')",
            "expect_s3_class",
            None,
        );
    }

    #[test]
    fn test_lint_expect_s3_class() {
        assert_snapshot!(
            snapshot_lint("expect_equal(class(x), 'data.frame')"),
            @"
        warning: expect_s3_class
         --> <test>:1:1
          |
        1 | expect_equal(class(x), 'data.frame')
          | ------------------------------------ `expect_equal(class(x), 'data.frame')` may fail if `x` gets more classes in the future.
          |
          = help: Use `expect_s3_class(x, 'data.frame')` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_equal(class(x), \"data.frame\")"),
            @r#"
        warning: expect_s3_class
         --> <test>:1:1
          |
        1 | expect_equal(class(x), "data.frame")
          | ------------------------------------ `expect_equal(class(x), "data.frame")` may fail if `x` gets more classes in the future.
          |
          = help: Use `expect_s3_class(x, "data.frame")` instead.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("testthat::expect_equal(class(x), 'data.frame')"),
            @"
        warning: expect_s3_class
         --> <test>:1:1
          |
        1 | testthat::expect_equal(class(x), 'data.frame')
          | ---------------------------------------------- `expect_equal(class(x), 'data.frame')` may fail if `x` gets more classes in the future.
          |
          = help: Use `expect_s3_class(x, 'data.frame')` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_equal('data.frame', class(x))"),
            @"
        warning: expect_s3_class
         --> <test>:1:1
          |
        1 | expect_equal('data.frame', class(x))
          | ------------------------------------ `expect_equal('data.frame', class(x))` may fail if `x` gets more classes in the future.
          |
          = help: Use `expect_s3_class(x, 'data.frame')` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_identical(class(foo$bar), \"Date\")"),
            @r#"
        warning: expect_s3_class
         --> <test>:1:1
          |
        1 | expect_identical(class(foo$bar), "Date")
          | ---------------------------------------- `expect_identical(class(foo$bar), "Date")` may fail if `foo$bar` gets more classes in the future.
          |
          = help: Use `expect_s3_class(foo$bar, "Date")` instead.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "expect_equal(class(x), 'data.frame')",
                    "expect_equal(class(x), \"data.frame\")",
                    "testthat::expect_equal(class(x), 'data.frame')",
                    "expect_equal('data.frame', class(x))",
                    "expect_equal(object = class(x), 'data.frame')",
                ],
                "expect_s3_class",
                None,
            )
        );
    }

    #[test]
    fn test_lint_expect_s3_class_dynamic_class() {
        assert_snapshot!(
            snapshot_lint("expect_equal(class(x), classes)"),
            @"
        warning: expect_s3_class
         --> <test>:1:1
          |
        1 | expect_equal(class(x), classes)
          | ------------------------------- `expect_equal(class(x), classes)` may fail if `x` gets more classes in the future.
          |
          = help: Use `expect_s3_class(x, classes)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_true(inherits(x, classes))"),
            @"
        warning: expect_s3_class
         --> <test>:1:1
          |
        1 | expect_true(inherits(x, classes))
          | --------------------------------- `expect_s3_class(x, classes)` is better than `expect_true(inherits(x, classes))`.
          |
          = help: Use `expect_s3_class(x, classes)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            "dynamic_class_no_fix",
            get_fixed_text(
                vec![
                    "expect_equal(class(x), classes)",
                    "expect_equal(classes, class(x))",
                    "expect_true(inherits(x, classes))",
                    "expect_true(inherits(x, c('foo', 'bar')))",
                ],
                "expect_s3_class",
                None,
            )
        );
    }

    #[test]
    fn test_lint_expect_s3_class_predicates() {
        assert_snapshot!(
            snapshot_lint("expect_true(is.data.frame(x))"),
            @r#"
        warning: expect_s3_class
         --> <test>:1:1
          |
        1 | expect_true(is.data.frame(x))
          | ----------------------------- `expect_s3_class(x, "data.frame")` is better than `expect_true(is.data.frame(x))`.
          |
          = help: Use `expect_s3_class(x, "data.frame")` instead.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("testthat::expect_true(utils::is.relistable(foo(x)))"),
            @r#"
        warning: expect_s3_class
         --> <test>:1:1
          |
        1 | testthat::expect_true(utils::is.relistable(foo(x)))
          | --------------------------------------------------- `expect_s3_class(foo(x), "relistable")` is better than `expect_true(utils::is.relistable(foo(x)))`.
          |
          = help: Use `expect_s3_class(foo(x), "relistable")` instead.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("expect_true(is.tskernel(k = x))"),
            @r#"
        warning: expect_s3_class
         --> <test>:1:1
          |
        1 | expect_true(is.tskernel(k = x))
          | ------------------------------- `expect_s3_class(x, "tskernel")` is better than `expect_true(is.tskernel(k = x))`.
          |
          = help: Use `expect_s3_class(x, "tskernel")` instead.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            "predicate_fix_output",
            get_fixed_text(
                vec![
                    "expect_true(is.data.frame(x))",
                    "testthat::expect_true(utils::is.relistable(foo(x)))",
                    "expect_true(is.tskernel(k = x))",
                    "expect_true(is.numeric_version(x))",
                    "expect_true(is.tclObj(x))",
                ],
                "expect_s3_class",
                None,
            )
        );
    }

    #[test]
    fn test_lint_expect_s3_class_inherits() {
        assert_snapshot!(
            snapshot_lint("expect_true(inherits(foo$bar, \"Date\"))"),
            @r#"
        warning: expect_s3_class
         --> <test>:1:1
          |
        1 | expect_true(inherits(foo$bar, "Date"))
          | -------------------------------------- `expect_s3_class(foo$bar, "Date")` is better than `expect_true(inherits(foo$bar, "Date"))`.
          |
          = help: Use `expect_s3_class(foo$bar, "Date")` instead.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("testthat::expect_true(base::inherits(what = 'factor', x = foo(x)))"),
            @"
        warning: expect_s3_class
         --> <test>:1:1
          |
        1 | testthat::expect_true(base::inherits(what = 'factor', x = foo(x)))
          | ------------------------------------------------------------------ `expect_s3_class(foo(x), 'factor')` is better than `expect_true(base::inherits(what = 'factor', x = foo(x)))`.
          |
          = help: Use `expect_s3_class(foo(x), 'factor')` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            "inherits_fix_output",
            get_fixed_text(
                vec![
                    "expect_true(inherits(foo$bar, \"Date\"))",
                    "testthat::expect_true(base::inherits(what = 'factor', x = foo(x)))",
                    "expect_true(inherits(x = foo(x), 'factor'))",
                    "expect_true(inherits(what = 'factor', foo(x)))",
                ],
                "expect_s3_class",
                None,
            )
        );
    }

    #[test]
    fn test_expect_s3_class_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present
        assert_snapshot!(
            snapshot_lint("expect_equal(class(x),\n # a comment \n'data.frame')"),
            @"
        warning: expect_s3_class
         --> <test>:1:1
          |
        1 | / expect_equal(class(x),
        2 | |  # a comment 
        3 | | 'data.frame')
          | |_____________- `expect_equal(class(x), 'data.frame')` may fail if `x` gets more classes in the future.
          |
          = help: Use `expect_s3_class(x, 'data.frame')` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nexpect_equal(class(x), 'data.frame')",
                    "expect_equal(class(x),\n # a comment \n'data.frame')",
                    "expect_equal(class(x), 'data.frame') # trailing comment",
                    "expect_true(is.data.frame(\n # a comment\n x))",
                    "expect_true(inherits(x,\n # a comment\n 'factor'))",
                ],
                "expect_s3_class",
                None
            )
        );
    }
}
