pub(crate) mod consecutive_signs;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "consecutive_signs", None)
    }

    #[test]
    fn test_no_lint_consecutive_signs() {
        for code in [
            "x + y",
            "x - y",
            "+x",
            "-x",
            "x * -y",
            "x / +y",
            "x + (-y)",
            "x - (+y)",
            "-(-x)",
            "+(+x)",
            "x %++% y",
            "x %--% y",
            "x <- -y",
            "!x",
            "!!x",
            "!!!x",
            "x + (y) + z",
            "-x; +y",
            "-x\n+y",
        ] {
            expect_no_lint(code, "consecutive_signs", None);
        }

        for code in [
            "# ++ -- +- -+",
            "x <- 1 # ++ --",
            r#""++ -- +- -+""#,
            "'++ -- +- -+'",
            r#"r"(x + +y)""#,
            "`++` <- 1; `--` <- 2",
            "`+`(x, `-`(y))",
            r#"x + "+""#,
            "x - '-'",
            "x + # -- ++\n y",
            "# jarl-ignore-file consecutive_signs: intentional DSL\nA--B",
            "# jarl-ignore consecutive_signs: intentional signs\nx + +y",
        ] {
            expect_no_lint(code, "consecutive_signs", None);
        }
    }

    #[test]
    fn test_lint_consecutive_signs() {
        assert_snapshot!(snapshot_lint("x + +y"), @"
        warning: consecutive_signs
         --> <test>:1:3
          |
        1 | x + +y
          |   --- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x + -y"), @"
        warning: consecutive_signs
         --> <test>:1:3
          |
        1 | x + -y
          |   --- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x - +y"), @"
        warning: consecutive_signs
         --> <test>:1:3
          |
        1 | x - +y
          |   --- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x - -y"), @"
        warning: consecutive_signs
         --> <test>:1:3
          |
        1 | x - -y
          |   --- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x++y"), @"
        warning: consecutive_signs
         --> <test>:1:2
          |
        1 | x++y
          |  -- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x--y"), @"
        warning: consecutive_signs
         --> <test>:1:2
          |
        1 | x--y
          |  -- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("++x"), @"
        warning: consecutive_signs
         --> <test>:1:1
          |
        1 | ++x
          | -- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("--x"), @"
        warning: consecutive_signs
         --> <test>:1:1
          |
        1 | --x
          | -- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("+-x"), @"
        warning: consecutive_signs
         --> <test>:1:1
          |
        1 | +-x
          | -- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("-+x"), @"
        warning: consecutive_signs
         --> <test>:1:1
          |
        1 | -+x
          | -- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("1 + + - - + 1"), @"
        warning: consecutive_signs
         --> <test>:1:3
          |
        1 | 1 + + - - + 1
          |   --------- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("+-+-x"), @"
        warning: consecutive_signs
         --> <test>:1:1
          |
        1 | +-+-x
          | ---- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x + -y * z"), @"
        warning: consecutive_signs
         --> <test>:1:3
          |
        1 | x + -y * z
          |   --- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x - -y^2"), @"
        warning: consecutive_signs
         --> <test>:1:3
          |
        1 | x - -y^2
          |   --- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("f(--x, y + +z)"), @"
        warning: consecutive_signs
         --> <test>:1:3
          |
        1 | f(--x, y + +z)
          |   -- Consecutive `+` and `-` operators may be a typo.
          |
        warning: consecutive_signs
         --> <test>:1:10
          |
        1 | f(--x, y + +z)
          |          --- Consecutive `+` and `-` operators may be a typo.
          |
        Found 2 errors.
        ");
        assert_snapshot!(snapshot_lint("--x\n++y"), @"
        warning: consecutive_signs
         --> <test>:1:1
          |
        1 | --x
          | -- Consecutive `+` and `-` operators may be a typo.
          |
        warning: consecutive_signs
         --> <test>:2:1
          |
        2 | ++y
          | -- Consecutive `+` and `-` operators may be a typo.
          |
        Found 2 errors.
        ");
        assert_snapshot!(snapshot_lint("x + (- -y)"), @"
        warning: consecutive_signs
         --> <test>:1:6
          |
        1 | x + (- -y)
          |      --- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("igraph::graph_from_literal(A--B)"), @"
        warning: consecutive_signs
         --> <test>:1:29
          |
        1 | igraph::graph_from_literal(A--B)
          |                             -- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x + # -- ++\n  -y"), @"
        warning: consecutive_signs
         --> <test>:1:3
          |
        1 |   x + # -- ++
          |  ___-
        2 | |   -y
          | |___- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x +\n  +y"), @"
        warning: consecutive_signs
         --> <test>:1:3
          |
        1 |   x +
          |  ___-
        2 | |   +y
          | |___- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint(r#""++ --"; x - -y # ++ --"#), @r#"
        warning: consecutive_signs
         --> <test>:1:12
          |
        1 | "++ --"; x - -y # ++ --
          |            --- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        "#);
        assert_snapshot!(snapshot_lint("# ++ --\nx + +y"), @"
        warning: consecutive_signs
         --> <test>:2:3
          |
        2 | x + +y
          |   --- Consecutive `+` and `-` operators may be a typo.
          |
        Found 1 error.
        ");
    }

    #[test]
    fn test_consecutive_signs_byte_ranges_and_no_fix() {
        assert_snapshot!(get_fixed_text(vec!["x + +y"], "consecutive_signs", None), @"
        OLD:
        ====
        x + +y
        NEW:
        ====
        x + +y
        ");
    }
}
