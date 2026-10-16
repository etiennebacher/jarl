pub(crate) mod yoda_condition;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "yoda_condition", None)
    }

    #[test]
    fn test_no_lint_yoda_condition() {
        for code in [
            "x == 1",
            "x != 'a'",
            "x < 2",
            "x <= 2",
            "x > 2",
            "x >= 2",
            "x == y",
            // Two literals have no actual result to reorder.
            "1 == 2",
            "1 < 2",
            "'a' == 'b'",
            "expect_equal(1, 1)",
            "expect_identical(1, 2)",
            "expect_setequal('a', TRUE)",
            "1 + x == y",
            "NULL == x",
            "T == x",
            "1 %in% x",
            "1 + x",
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
            "expect_equal((x) + 1, x)",
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
            // Missing, forwarded, or incomplete arguments.
            "expect_equal(1)",
            "expect_equal(, x)",
            "expect_equal(object = 1, expected =)",
            "expect_equal(object = 1, object = x)",
            "expect_equal(..., 1, x)",
            "expect_equal(1, x",
        ] {
            expect_no_lint(code, "yoda_condition", None);
        }
        for pipe in ["|>", "%>%", "%!>%", "%T>%", "%<>%"] {
            expect_no_lint(
                &format!("x {pipe} expect_equal(1, 1)"),
                "yoda_condition",
                None,
            );
        }
    }

    #[test]
    fn test_lint_yoda_condition() {
        assert_snapshot!(
            snapshot_lint("1 == total"),
            @"
        warning: yoda_condition
         --> <test>:1:1
          |
        1 | 1 == total
          | ---------- Putting the literal first in the comparison is less readable.
          |
          = help: Put the actual result on the left and the literal on the right.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("if (10L < quantity) discount <- 0.1"),
            @"
        warning: yoda_condition
         --> <test>:1:5
          |
        1 | if (10L < quantity) discount <- 0.1
          |     -------------- Putting the literal first in the comparison is less readable.
          |
          = help: Put the actual result on the left and the literal on the right.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_equal(42, calculate_total(items))"),
            @"
        warning: yoda_condition
         --> <test>:1:1
          |
        1 | expect_equal(42, calculate_total(items))
          | ---------------------------------------- Using a literal in the `object` argument is less readable and gives worse error messages.
          |
          = help: Supply the actual result as `object` and the literal as `expected`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("testthat::expect_identical(\"ready\", get_status(job))"),
            @r#"
        warning: yoda_condition
         --> <test>:1:1
          |
        1 | testthat::expect_identical("ready", get_status(job))
          | ---------------------------------------------------- Using a literal in the `object` argument is less readable and gives worse error messages.
          |
          = help: Supply the actual result as `object` and the literal as `expected`.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("expect_setequal(3L, unique(values))"),
            @"
        warning: yoda_condition
         --> <test>:1:1
          |
        1 | expect_setequal(3L, unique(values))
          | ----------------------------------- Using a literal in the `object` argument is less readable and gives worse error messages.
          |
          = help: Supply the actual result as `object` and the literal as `expected`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_equal(expected = total, object = 42)"),
            @"
        warning: yoda_condition
         --> <test>:1:1
          |
        1 | expect_equal(expected = total, object = 42)
          | ------------------------------------------- Using a literal in the `object` argument is less readable and gives worse error messages.
          |
          = help: Supply the actual result as `object` and the literal as `expected`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_equal(7.5, calculate_mean(values), tolerance = 0.1)"),
            @"
        warning: yoda_condition
         --> <test>:1:1
          |
        1 | expect_equal(7.5, calculate_mean(values), tolerance = 0.1)
          | ---------------------------------------------------------- Using a literal in the `object` argument is less readable and gives worse error messages.
          |
          = help: Supply the actual result as `object` and the literal as `expected`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("expect_equal(2, # expected\nx)"),
            @"
        warning: yoda_condition
         --> <test>:1:1
          |
        1 | / expect_equal(2, # expected
        2 | | x)
          | |__- Using a literal in the `object` argument is less readable and gives worse error messages.
          |
          = help: Supply the actual result as `object` and the literal as `expected`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("test_that(\"你好\", {\n  expect_equal(\n    \"你好\",\n    foo(x)\n  )\n})"),
            @r#"
        warning: yoda_condition
         --> <test>:2:3
          |
        2 | /   expect_equal(
        3 | |     "你好",
        4 | |     foo(x)
        5 | |   )
          | |___- Using a literal in the `object` argument is less readable and gives worse error messages.
          |
          = help: Supply the actual result as `object` and the literal as `expected`.
        Found 1 error.
        "#
        );
        assert_snapshot!(
            snapshot_lint("x |> foo(expect_equal(2, y))"),
            @"
        warning: yoda_condition
         --> <test>:1:10
          |
        1 | x |> foo(expect_equal(2, y))
          |          ------------------ Using a literal in the `object` argument is less readable and gives worse error messages.
          |
          = help: Supply the actual result as `object` and the literal as `expected`.
        Found 1 error.
        "
        );
        // An explicit `.` means the pipe does not supply `object`.
        assert_snapshot!(
            snapshot_lint("x %>% expect_equal(2, .)"),
            @"
        warning: yoda_condition
         --> <test>:1:7
          |
        1 | x %>% expect_equal(2, .)
          |       ------------------ Using a literal in the `object` argument is less readable and gives worse error messages.
          |
          = help: Supply the actual result as `object` and the literal as `expected`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("x %>% (expect_equal(2, .))"),
            @"
        warning: yoda_condition
         --> <test>:1:8
          |
        1 | x %>% (expect_equal(2, .))
          |        ------------------ Using a literal in the `object` argument is less readable and gives worse error messages.
          |
          = help: Supply the actual result as `object` and the literal as `expected`.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_yoda_condition_fix() {
        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text(
                vec![
                    "expect_equal(1, foo(x))",
                    "1 == foo(x)",
                    "expect_equal(1L, foo(x))",
                    "1L == foo(x)",
                    "expect_equal(1i, foo(x))",
                    "1i == foo(x)",
                    "expect_equal('a', foo(x))",
                    "'a' == foo(x)",
                    "expect_equal(r\"(a\\b)\", foo(x))",
                    "r\"(a\\b)\" == foo(x)",
                    "expect_equal(TRUE, foo(x))",
                    "TRUE == foo(x)",
                    "expect_equal(FALSE, foo(x))",
                    "FALSE == foo(x)",
                    "expect_equal(NA, foo(x))",
                    "NA == foo(x)",
                    "expect_equal(NA_integer_, foo(x))",
                    "NA_integer_ == foo(x)",
                    "expect_equal(Inf, foo(x))",
                    "Inf == foo(x)",
                    "expect_equal(NaN, foo(x))",
                    "NaN == foo(x)",
                    "expect_equal(-1, foo(x))",
                    "-1 == foo(x)",
                    "expect_equal(((1)), foo(x))",
                    "((1)) == foo(x)",
                    "expect_equal(1 + 1, foo(x))",
                    "1 + 1 == foo(x)",
                    "expect_equal((1) + 1, foo(x))",
                    "(1) + 1 == foo(x)",
                    "expect_equal(2 + 1i, foo(x))",
                    "2 + 1i == foo(x)",
                    "1 == total\n'a' != name\n10L < quantity\n2 <= length(x)\n3 > value\n4 >= value",
                    "if (1 == x) foo()\nwhile (0 < remaining) remaining <- remaining - 1",
                    "dplyr::filter(data, 'ready' == status)",
                    "(1) + 1 == x + 1\n1 == -x\n1 == x * 2\n1 == (x > y)",
                    "1 == !x\n1 == x + !y\n1 == -!x\n1 == if (flag) x else y\n1 == function(x) x",
                    "# leading comment\n'你好'  !=\n  get_name(x) # trailing comment",
                    "expect_equal(TRUE, 1 == total)",
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
                    "expect_equal(2, x) |> identity()",
                    "expect_equal(2, expect_identical('a', x))",
                ],
                "yoda_condition"
            )
        );
    }

    #[test]
    fn test_yoda_condition_no_fix() {
        assert_snapshot!(
            "no_fix_output",
            get_unsafe_fixed_text(
                vec![
                    "1 == # actual\nx",
                    // Named arguments and extra arguments have no fix.
                    "expect_equal(expected = total, object = 42)",
                    "expect_equal(42, expected = total)",
                    "expect_equal(7.5, calculate_mean(values), tolerance = 0.1)",
                    "expect_equal(2, # expected\nx)",
                    "expect_equal(2, foo( # actual\nx))",
                ],
                "yoda_condition"
            )
        );
    }
}
