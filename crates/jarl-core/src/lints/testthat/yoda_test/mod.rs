pub(crate) mod yoda_test;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "yoda_test", None)
    }

    #[test]
    fn test_no_lint_yoda_test() {
        for code in [
            "expect_equal(foo(x), 2)",
            "expect_identical(x, 'a')",
            "expect_setequal(x, 1L)",
            "expect_equal(x, y)",
            "expect_equal(x$\"key\", 2)",
            "expect_equal(x@\"key\", 2)",
            "expect_equal(c(1, 2), x)",
            "expect_equal(1:3, x)",
            "expect_equal(1 * 2, x)",
            "expect_equal(1 + 2 + 3, x)",
            "expect_equal(1 + x, x)",
            "expect_equal(-x, x)",
            "expect_equal(!TRUE, x)",
            "expect_equal(-'a', x)",
            "expect_equal(NULL, x)",
            "expect_equal(T, x)",
            "expect_gt(2, x)",
            "other::expect_equal(2, x)",
            "x$expect_equal(2, x)",
            // Named arguments are matched before positional arguments.
            "expect_equal(expected = 2, object = x)",
            "expect_equal(expected = 2, x)",
            "expect_equal(`object` = x, 2)",
            "expect_equal('expected' = 2, x)",
            // Pipe input supplies the actual result.
            "x |> expect_equal(expected = 2, object = _)",
            "x %>% (expect_equal(2, .))",
            // Missing, forwarded, or incomplete arguments.
            "expect_equal(1)",
            "expect_equal(, x)",
            "expect_equal(object = 1, expected =)",
            "expect_equal(object = 1, object = x)",
            "expect_equal(..., 1, x)",
            "expect_equal(1, x",
        ] {
            expect_no_lint(code, "yoda_test", None);
        }
        for pipe in ["|>", "%>%", "%!>%", "%T>%", "%<>%"] {
            expect_no_lint(&format!("x {pipe} expect_equal(1, 1)"), "yoda_test", None);
        }
    }

    #[test]
    fn test_lint_yoda_test() {
        // One representative of each literal form; the functions share detection.
        for literal in [
            "1",
            "1L",
            "1i",
            "'a'",
            "r\"(a\\b)\"",
            "TRUE",
            "FALSE",
            "NA",
            "NA_integer_",
            "Inf",
            "NaN",
            "-1",
            "((1))",
            "1 + 1",
            "2 + 1i",
        ] {
            let code = format!("expect_equal({literal}, foo(x))");
            let diagnostics = check_code(&code, "yoda_test", None);
            assert_eq!(diagnostics.len(), 1, "{code}");
            assert_eq!(
                diagnostics[0].message.suggestion.as_deref(),
                Some(format!("Use `expect_equal(foo(x), {literal})` instead.").as_str()),
                "{code}"
            );
        }

        assert_snapshot!(
            "diagnostics",
            snapshot_lint(
                &[
                    "expect_equal(42, calculate_total(items))",
                    "testthat::expect_identical(\"ready\", get_status(job))",
                    "expect_setequal(3L, unique(values))",
                    "expect_equal(1, 1)",
                    "expect_identical(1, 2)",
                    "expect_setequal('a', TRUE)",
                    "expect_equal(expected = total, object = 42)",
                    "expect_equal(7.5, calculate_mean(values), tolerance = 0.1)",
                    "expect_equal(2, # expected\nx)",
                    "test_that(\"你好\", {\n  expect_equal(\n    \"你好\",\n    foo(x)\n  )\n})",
                    // Independent calls inside a pipe must still be checked.
                    "x |> foo(expect_equal(2, y))",
                ]
                .join("\n")
            )
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "expect_equal(42, calculate_total(items))",
                    "testthat::expect_identical('ready', get_status(job))",
                    "testthat:::expect_setequal(1L, x)",
                    "expect_equal(-1, x)",
                    "expect_equal((2 + 1i), x)",
                    "expect_equal(r\"(a\\b)\", x)",
                    "expect_equal(\n  \"你好\",\n  foo(x)\n)",
                    "# leading comment\nexpect_equal(  2 ,  foo(x)  ) # trailing comment",
                    "x %$% expect_equal(2, value)",
                    "x %>% { expect_equal(2, .) }",
                    "expect_equal(2, x |> foo())",
                    "expect_equal(2, expect_identical('a', x))",
                ],
                "yoda_test",
                None
            )
        );

        assert_snapshot!(
            "no_fix_output",
            get_fixed_text(
                vec![
                    // Two literals, named arguments, and extra arguments have no fix.
                    "expect_equal(1, 1)",
                    "expect_identical(1, 2)",
                    "expect_equal(expected = total, object = 42)",
                    "expect_equal(42, expected = total)",
                    "expect_equal(7.5, calculate_mean(values), tolerance = 0.1)",
                    "expect_equal(2, # expected\nx)",
                    "expect_equal(2, foo( # actual\nx))",
                ],
                "yoda_test",
                None
            )
        );
    }
}
