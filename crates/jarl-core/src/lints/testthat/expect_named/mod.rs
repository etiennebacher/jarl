pub(crate) mod expect_named;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "expect_named", None)
    }

    #[test]
    fn test_no_lint_expect_named() {
        // colnames(), rownames(), and dimnames() tests are not equivalent
        expect_no_lint("expect_equal(colnames(x), 'a')", "expect_named", None);
        expect_no_lint("expect_equal(rownames(x), 'a')", "expect_named", None);
        expect_no_lint("expect_equal(dimnames(x), 'a')", "expect_named", None);

        expect_no_lint("expect_equal(nrow(x), 4L)", "expect_named", None);
        expect_no_lint("testthat::expect_equal(nrow(x), 4L)", "expect_named", None);
        expect_no_lint("expect_equal(colnames(x), names(y))", "expect_named", None);

        // Those are reported in `lintr` and `flir` but I'm actualy not convinced
        // they should.
        //
        // This example:
        //   expect_equal(x, names(iris))
        //
        // doesn't read in the same way as the rewritten one:
        //   expect_named(iris, x)
        //
        // The second one gives the impression that we're testing `iris` when
        // we really want to test `x`.
        expect_no_lint("expect_equal(y, names(x))", "expect_named", None);
        expect_no_lint("expect_equal(y, names(x))", "expect_named", None);
        expect_no_lint("expect_equal(foo(y), names(x))", "expect_named", None);
        expect_no_lint("expect_equal(expected = names(y), x)", "expect_named", None);

        // More readable than expect_named(x, names(y))
        expect_no_lint("expect_equal(names(x), names(y))", "expect_named", None);

        // Not the functions we're looking for
        expect_no_lint("expect_equal(x, 'a')", "expect_named", None);
        expect_no_lint("some_other_function(names(x), 'a')", "expect_named", None);

        // Wrong code but no panic
        expect_no_lint("expect_equal(names(x))", "expect_named", None);
        expect_no_lint("expect_equal(names())", "expect_named", None);
        expect_no_lint("expect_equal(object =, expected =)", "expect_named", None);
    }

    #[test]
    fn test_lint_expect_named() {
        assert_snapshot!(
            snapshot_lint("expect_equal(names(x), 'a')"),
            @r"
        warning: expect_named
         --> <test>:1:1
          |
        1 | expect_equal(names(x), 'a')
          | --------------------------- `expect_named(x, n)` is better than `expect_equal(names(x), n)`.
          |
          = help: Use `expect_named(x, n)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_equal(names(x), c('a', 'b'))"),
            @r"
        warning: expect_named
         --> <test>:1:1
          |
        1 | expect_equal(names(x), c('a', 'b'))
          | ----------------------------------- `expect_named(x, n)` is better than `expect_equal(names(x), n)`.
          |
          = help: Use `expect_named(x, n)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_identical(names(x), 'a')"),
            @r"
        warning: expect_named
         --> <test>:1:1
          |
        1 | expect_identical(names(x), 'a')
          | ------------------------------- `expect_named(x, n)` is better than `expect_identical(names(x), n)`.
          |
          = help: Use `expect_named(x, n)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_equal(names(x), NULL)"),
            @r"
        warning: expect_named
         --> <test>:1:1
          |
        1 | expect_equal(names(x), NULL)
          | ---------------------------- `expect_named(x, n)` is better than `expect_equal(names(x), n)`.
          |
          = help: Use `expect_named(x, n)` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "expect_equal(names(x), 'a')",
                    "testthat::expect_equal(names(x), 'a')",
                    "expect_equal(names(x), c('a', 'b'))",
                    "expect_identical(names(x), 'a')",
                ],
                "expect_named",
                None,
            )
        );
    }

    #[test]
    fn test_expect_named_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present
        assert_snapshot!(
            snapshot_lint("expect_equal(# comment\nnames(x), 'a')"),
            @r"
        warning: expect_named
         --> <test>:1:1
          |
        1 | / expect_equal(# comment
        2 | | names(x), 'a')
          | |______________- `expect_named(x, n)` is better than `expect_equal(names(x), n)`.
          |
          = help: Use `expect_named(x, n)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nexpect_equal(names(x), 'a')",
                    "expect_equal(# comment\nnames(x), 'a')",
                    "expect_equal(names(x), 'a') # trailing comment",
                ],
                "expect_named",
                None
            )
        );
    }
}
