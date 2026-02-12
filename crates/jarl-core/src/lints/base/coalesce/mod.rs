pub(crate) mod coalesce;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "coalesce", Some("4.4"))
    }

    #[test]
    fn test_no_lint_coalesce() {
        let version = Some("4.4");

        expect_no_lint("if (is.null(x)) y", "coalesce", version);
        expect_no_lint("if (!is.null(x)) y", "coalesce", version);
        expect_no_lint("if (!is.null(x)) x", "coalesce", version);
        expect_no_lint("if (is.null(x)) x", "coalesce", version);
        expect_no_lint("c(if (!is.null(E)) E)", "coalesce", version);
        expect_no_lint("if (is.null(x)) y else z", "coalesce", version);
        expect_no_lint("if (!is.null(x)) x[1] else y", "coalesce", version);
        expect_no_lint("if (is.null(x[1])) y else x[2]", "coalesce", version);
        expect_no_lint("if (is.null(x)) y else {x ; z}", "coalesce", version);
        expect_no_lint("if (is.null(x)) y else {x \n z}", "coalesce", version);
        expect_no_lint("if (!is.null(x)) {x ; z} else y", "coalesce", version);
        expect_no_lint("if (!is.null(x)) {x \n z} else y", "coalesce", version);
        expect_no_lint("if (is.null(s <- foo())) y else x", "coalesce", version);
        expect_no_lint("if (!is.null(s <- foo())) x else y", "coalesce", version);

        // TODO: should maybe be reported? lintr reports this
        expect_no_lint("if (is.null(s <- foo(x))) y else s", "coalesce", version);

        // `%||%` doesn't exist in this version
        expect_no_lint("if (is.null(x)) y else x", "coalesce", Some("4.3"));
    }

    #[test]
    fn test_lint_coalesce() {
        let version = Some("4.4");

        assert_snapshot!(

            snapshot_lint("if (is.null(x)) y else x"),

            @r"
        warning: coalesce
         --> <test>:1:1
          |
        1 | if (is.null(x)) y else x
          | ------------------------ `if (is.null(x)) y else x` can be simplified.
          |
          = help: Use `x %||% y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (is.null(x)) { y } else x"),
            @r"
        warning: coalesce
         --> <test>:1:1
          |
        1 | if (is.null(x)) { y } else x
          | ---------------------------- `if (is.null(x)) y else x` can be simplified.
          |
          = help: Use `x %||% y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (is.null(x)) y else { x }"),
            @r"
        warning: coalesce
         --> <test>:1:1
          |
        1 | if (is.null(x)) y else { x }
          | ---------------------------- `if (is.null(x)) y else x` can be simplified.
          |
          = help: Use `x %||% y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (is.null(x)) { y } else { x }"),
            @r"
        warning: coalesce
         --> <test>:1:1
          |
        1 | if (is.null(x)) { y } else { x }
          | -------------------------------- `if (is.null(x)) y else x` can be simplified.
          |
          = help: Use `x %||% y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (is.null(x[1])) y else x[1]"),
            @r"
        warning: coalesce
         --> <test>:1:1
          |
        1 | if (is.null(x[1])) y else x[1]
          | ------------------------------ `if (is.null(x)) y else x` can be simplified.
          |
          = help: Use `x %||% y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (is.null(foo(x))) y else foo(x)"),
            @r"
        warning: coalesce
         --> <test>:1:1
          |
        1 | if (is.null(foo(x))) y else foo(x)
          | ---------------------------------- `if (is.null(x)) y else x` can be simplified.
          |
          = help: Use `x %||% y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (!is.null(x)) x else y"),
            @r"
        warning: coalesce
         --> <test>:1:1
          |
        1 | if (!is.null(x)) x else y
          | ------------------------- `if (!is.null(x)) x else y` can be simplified.
          |
          = help: Use `x %||% y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (!is.null(x)) { x } else y"),
            @r"
        warning: coalesce
         --> <test>:1:1
          |
        1 | if (!is.null(x)) { x } else y
          | ----------------------------- `if (!is.null(x)) x else y` can be simplified.
          |
          = help: Use `x %||% y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (!is.null(x)) x else { y }"),
            @r"
        warning: coalesce
         --> <test>:1:1
          |
        1 | if (!is.null(x)) x else { y }
          | ----------------------------- `if (!is.null(x)) x else y` can be simplified.
          |
          = help: Use `x %||% y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (!is.null(x)) { x } else { y }"),
            @r"
        warning: coalesce
         --> <test>:1:1
          |
        1 | if (!is.null(x)) { x } else { y }
          | --------------------------------- `if (!is.null(x)) x else y` can be simplified.
          |
          = help: Use `x %||% y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (!is.null(x[1])) x[1] else y"),
            @r"
        warning: coalesce
         --> <test>:1:1
          |
        1 | if (!is.null(x[1])) x[1] else y
          | ------------------------------- `if (!is.null(x)) x else y` can be simplified.
          |
          = help: Use `x %||% y` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (!is.null(foo(x))) foo(x) else y"),
            @r"
        warning: coalesce
         --> <test>:1:1
          |
        1 | if (!is.null(foo(x))) foo(x) else y
          | ----------------------------------- `if (!is.null(x)) x else y` can be simplified.
          |
          = help: Use `x %||% y` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "if (is.null(x)) y else x",
                    "if (is.null(x)) { y } else x",
                    "if (is.null(x)) y else { x }",
                    "if (is.null(x)) { y } else { x }",
                    "if (is.null(x[1])) y else x[1]",
                    "if (is.null(foo(x))) y else foo(x)",
                    "if (!is.null(x)) x else y",
                    "if (!is.null(x)) { x } else y",
                    "if (!is.null(x)) x else { y }",
                    "if (!is.null(x)) { x } else { y }",
                    "if (!is.null(x[1])) x[1] else y",
                    "if (!is.null(foo(x))) foo(x) else y",
                    "if (is.null(x)) {\n  y <- 1\n  y\n} else x",
                    "if (is.null(x)) {\n  y <- 1\n  y\n} else {\n  x\n}",
                ],
                "coalesce",
                version
            )
        );
    }

    #[test]
    fn test_coalesce_with_comments_no_fix() {
        // Should detect lint but skip fix when comments are present to avoid destroying them
        assert_snapshot!(
            "no_fix_with_comments",
            get_fixed_text(
                vec![
                    "# leading comment\nif (is.null(x)) {\n  y\n} else x",
                    "if (is.null(x)) {\n  # hello there\n  y\n} else x",
                    "if (is.null(x)) {\n  y\n} else x # trailing comment",
                ],
                "coalesce",
                Some("4.5")
            )
        );
    }
}
