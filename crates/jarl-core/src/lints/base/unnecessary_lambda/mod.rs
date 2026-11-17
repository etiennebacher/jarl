pub(crate) mod options;
pub(crate) mod unnecessary_lambda;

#[cfg(test)]
mod tests {
    use crate::lints::base::unnecessary_lambda::options::ResolvedUnnecessaryLambdaOptions;
    use crate::rule_options::ResolvedRuleOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "unnecessary_lambda", None)
    }

    fn settings_allow_comparison() -> Settings {
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions {
                    unnecessary_lambda: ResolvedUnnecessaryLambdaOptions { allow_comparison: true },
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    fn expect_lint(code: &str) {
        assert_eq!(
            check_code(code, "unnecessary_lambda", None).len(),
            1,
            "Expected one lint for: {code}"
        );
    }

    #[test]
    fn test_no_lint_unnecessary_lambda() {
        let rule = "unnecessary_lambda";
        expect_no_lint("lapply(DF, sum)", rule, None);
        expect_no_lint("apply(M, 1, sum, na.rm = TRUE)", rule, None);
        // Not an apply function
        expect_no_lint("foo(x, function(xi) sum(xi))", rule, None);
        expect_no_lint("Reduce(function(x, y) sum(x, y), l)", rule, None);

        // The first argument is not the formal
        expect_no_lint("sapply(x, function(xi) foo(1, xi))", rule, None);
        expect_no_lint("sapply(x, function(xi) return(foo(1, xi)))", rule, None);

        // The formal is used several times
        expect_no_lint("dendrapply(x, function(xi) foo(xi, xi))", rule, None);
        expect_no_lint(
            "parLapply(cl, x, function(xi) foo(xi, 2, bar(baz(xi))))",
            rule,
            None,
        );

        // Multiple expressions
        expect_no_lint("lapply(x, function(xi) { print(xi); xi^2 })", rule, None);
        expect_no_lint(
            "lapply(x, function(xi) {\n  print(xi); xi^2\n})",
            rule,
            None,
        );
        expect_no_lint(
            "lapply(x, function(xi) {\n  print(xi)\n  xi^2\n})",
            rule,
            None,
        );

        // Infix operators
        expect_no_lint("lapply(x, function(xi) xi %in% tbl)", rule, None);
        expect_no_lint("lapply(x, function(xi) tbl %in% xi)", rule, None);

        // Nested calls
        expect_no_lint("lapply(x, function(xi) foo(bar(xi)))", rule, None);
        expect_no_lint("lapply(x, function(xi) return(foo(bar(xi))))", rule, None);
        // Calls in other arguments are evaluated for each element
        expect_no_lint("lapply(x, function(xi) foo(xi, runif(1)))", rule, None);

        // Extractions
        expect_no_lint("lapply(l, function(x) rle(x)$values)", rule, None);
        expect_no_lint("lapply(l, function(x) rle(x)['values'])", rule, None);
        expect_no_lint("lapply(l, function(x) rle(x)[['values']])", rule, None);
        expect_no_lint("lapply(l, function(x) rle(x)@values)", rule, None);
        expect_no_lint("lapply(l, function(x) return(foo(x)$bar))", rule, None);
        expect_no_lint(
            "lapply(l, function(x) return(rle(x)['values']))",
            rule,
            None,
        );
        expect_no_lint(
            "lapply(l, function(x) return(rle(x)[['values']]))",
            rule,
            None,
        );
        expect_no_lint("lapply(l, function(x) return(rle(x)@values))", rule, None);

        // Other operators
        expect_no_lint("lapply(l, function(x) foo(x) - 1)", rule, None);
        expect_no_lint("lapply(l, function(x) foo(x) * 2)", rule, None);
        expect_no_lint("lapply(l, function(x) foo(x) ^ 3)", rule, None);
        expect_no_lint("lapply(l, function(x) foo(x) %% 4)", rule, None);

        // Other lambdas
        expect_no_lint(
            "{
  lapply(x, function(e) sprintf('%o', e))
  lapply(y, function(e) paste(strlpad(e, '0', width)))
}",
            rule,
            None,
        );

        // Call only on RHS of an operator
        expect_no_lint("lapply(l, function(x) 'a' %in% names(x))", rule, None);
        expect_no_lint("lapply(l, function(x = 1) 'a' %in% names(x))", rule, None);

        // No formals
        expect_no_lint("lapply(l, function() foo())", rule, None);
    }

    #[test]
    fn test_no_lint_unnecessary_lambda_comparison() {
        let rule = "unnecessary_lambda";
        // lapply() returns a list
        expect_no_lint("lapply(x, function(xi) foo(xi) == 2)", rule, None);
        // Not a constant
        expect_no_lint("sapply(x, function(xi) foo(xi) == y)", rule, None);
        // Nested call
        expect_no_lint("sapply(x, function(xi) sum(abs(xi)) == 0)", rule, None);

        // Negations and unary operators
        expect_no_lint("sapply(x, function(xi) !all.equal(xi, y))", rule, None);
        expect_no_lint("sapply(x, \\(xi) !all.equal(xi, y))", rule, None);
        expect_no_lint("purrr::map(x, ~!foo(.x))", rule, None);
        expect_no_lint("sapply(x, function(xi) -foo(xi))", rule, None);
        expect_no_lint("sapply(x, function(xi) +foo(xi))", rule, None);
        expect_no_lint("sapply(x, function(xi) ~foo(xi))", rule, None);
        expect_no_lint("sapply(x, function(xi) (!foo(xi)))", rule, None);
        expect_no_lint("sapply(x, function(xi) (-foo(xi)))", rule, None);
        expect_no_lint("sapply(x, function(xi) { -foo(xi) })", rule, None);

        // Option
        let settings = settings_allow_comparison();
        for code in [
            "sapply(x, function(xi) foo(xi) == 2)",
            "sapply(x, function(xi) foo(xi) == 'a')",
            "sapply(x, function(xi) foo(xi) == 1 + 2i)",
            "sapply(x, \\(xi) foo(xi) >= 2)",
            "vapply(x, function(xi) foo(xi) == 2, logical(1L))",
        ] {
            expect_no_lint_with_settings(code, rule, None, settings.clone());
        }
    }

    #[test]
    fn test_no_lint_unnecessary_lambda_multiple_formals() {
        let rule = "unnecessary_lambda";
        // Swapped positional arguments
        expect_no_lint("mapply(function(xi, yi) foo(yi, xi), x, y)", rule, None);
        // Discarded arguments
        expect_no_lint("mapply(function(xi, yi) foo(xi), x, y)", rule, None);
        expect_no_lint("outer(x, y, function(xi, yi) foo(xi))", rule, None);
        expect_no_lint("purrr::map2(x, y, function(xi, yi) foo(xi))", rule, None);
        // `y` is not supplied by the outer call
        expect_no_lint("lapply(x, function(xi, y) foo(xi, y))", rule, None);
        // `foo()` may not have a formal named `y`
        expect_no_lint("lapply(x, function(xi, y) foo(xi, y), y = 1)", rule, None);
        // `y` may not be the second formal of `foo()`
        expect_no_lint("lapply(x, function(xi, y) foo(xi, y = y), y)", rule, None);
        expect_no_lint(
            "lapply(x, function(xi, y, z) foo(xi, y + z), y, z)",
            rule,
            None,
        );
        expect_no_lint("lapply(x, function(xi, y) foo(xi, bar = y), y)", rule, None);
        expect_no_lint(
            "lapply(x, function(xi, y) foo(xi, bar = y), y = 1)",
            rule,
            None,
        );
        expect_no_lint("lapply(x, function(xi, y) foo(xi, y, y), y)", rule, None);
        expect_no_lint("lapply(x, function(xi, y = 1) foo(xi, y))", rule, None);
    }

    #[test]
    fn test_no_lint_unnecessary_lambda_keyword_args() {
        let rule = "unnecessary_lambda";
        expect_no_lint("lapply(x, function(xi) data.frame(nm = xi))", rule, None);
        expect_no_lint(
            "lapply(x, function(xi) return(data.frame(nm = xi)))",
            rule,
            None,
        );
        expect_no_lint(
            "lapply(x, function(xi) data.frame(nm = # comment\nxi))",
            rule,
            None,
        );
    }

    #[test]
    fn test_no_lint_unnecessary_lambda_purrr() {
        let rule = "unnecessary_lambda";
        expect_no_lint("purrr::map(x, ~.x)", rule, None);
        expect_no_lint("purrr::map_df(x, ~lm(y, .x))", rule, None);
        expect_no_lint("map_dbl(x, ~foo(bar = .x))", rule, None);
        expect_no_lint("map_dbl(x, ~foo(.x, .x))", rule, None);
        expect_no_lint("map_dbl(x, ~foo(.x, .))", rule, None);
        expect_no_lint("map_dbl(x, ~foo(bar = # comment\n.x))", rule, None);
        // Formulas are only lambdas in purrr
        expect_no_lint("lapply(x, ~foo(.x))", rule, None);
    }

    #[test]
    fn test_lint_unnecessary_lambda() {
        for code in [
            "lapply(DF, function(x) sum(x))",
            "lapply(DF, function(x) return(sum(x)))",
            "lapply(DF, \\(x) sum(x))",
            "rapply(l, function(x) is.data.frame(x))",
            "eapply(env, function(x) sum(x, na.rm = TRUE))",
            "eapply(env, function(x) return(sum(x, na.rm = TRUE)))",
            "sapply(x, function(xi) (foo(xi)))",
            "sapply(x, function(xi) { foo(xi) })",
            "sapply(x, function(xi) pkg::foo(xi))",
            "x |> lapply(function(xi, y) foo(xi, y), y)",
            // Multiple formals
            "lapply(x, function(xi, y) foo(xi, y), y)",
            "lapply(x, \\(xi, y) foo(xi, y = y), y = 1)",
            "lapply(x, \\(xi, ...) foo(xi, ...), y = 1)",
            "mapply(function(xi, yi) foo(xi, yi, na.rm = TRUE), x, y)",
            // purrr
            "purrr::map(x, ~foo(.x))",
            "purrr::map_int(x, ~foo(.x, y))",
            "purrr::map_vec(x, ~foo(.x, y))",
            "map(x, ~foo(.))",
            "purrr::map(x, function(xi) foo(xi))",
            // Braces
            "lapply(x, function(xi) { print(xi) })",
            "lapply(x, function(xi) {\n  print(xi)\n})",
            "lapply(x, function(xi) { return(print(xi)) })",
            "lapply(x, function(xi) {\n  return(print(xi))\n})",
            "lapply(y, function(yi) {\n  print(yi) # comment\n})",
            // Comparisons
            "sapply(x, function(xi) foo(xi) == 2)",
            "sapply(x, function(xi) foo(xi) == 'a')",
            "sapply(x, function(xi) foo(xi) == 1 + 2i)",
            "sapply(x, function(xi) 2 < foo(xi))",
            "sapply(x, \\(xi) foo(xi) != TRUE)",
        ] {
            expect_lint(code);
        }
        expect_no_lint(
            "lapply(x, function(xi) {\n  print(xi)\n  xi\n})",
            "unnecessary_lambda",
            None,
        );
        expect_no_lint(
            "lapply(x, function(xi) { foo(xi)$bar })",
            "unnecessary_lambda",
            None,
        );
        expect_no_lint(
            "lapply(x, function(xi) { foo(xi) - 1 })",
            "unnecessary_lambda",
            None,
        );
    }

    #[test]
    fn test_lint_unnecessary_lambda_messages() {
        assert_snapshot!(
            snapshot_lint("lapply(DF, function(x) sum(x))"),
            @"
        warning: unnecessary_lambda
         --> <test>:1:12
          |
        1 | lapply(DF, function(x) sum(x))
          |            ------------------ Unnecessary anonymous function around `sum()`.
          |
          = help: Pass `sum` directly to `lapply()`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("eapply(env, function(x) sum(x, na.rm = TRUE))"),
            @"
        warning: unnecessary_lambda
         --> <test>:1:13
          |
        1 | eapply(env, function(x) sum(x, na.rm = TRUE))
          |             -------------------------------- Unnecessary anonymous function around `sum()`.
          |
          = help: Pass `sum` directly to `eapply()`, followed by its other arguments.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("purrr::map_int(x, ~foo(.x, y))"),
            @"
        warning: unnecessary_lambda
         --> <test>:1:19
          |
        1 | purrr::map_int(x, ~foo(.x, y))
          |                   ----------- Unnecessary anonymous function around `foo()`.
          |
          = help: Pass `foo` directly to `map_int()`, followed by its other arguments.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("sapply(x, function(xi) foo(xi) < 1 + 2i)"),
            @"
        warning: unnecessary_lambda
         --> <test>:1:11
          |
        1 | sapply(x, function(xi) foo(xi) < 1 + 2i)
          |           ----------------------------- Comparing to a constant inside `sapply()` doesn't benefit from vectorization.
          |
          = help: Compare after calling `sapply()`, e.g. `sapply(x, foo) < 1 + 2i`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("vapply(x, function(xi) foo(xi) == 2, logical(1L))"),
            @"
        warning: unnecessary_lambda
         --> <test>:1:11
          |
        1 | vapply(x, function(xi) foo(xi) == 2, logical(1L))
          |           ------------------------- Comparing to a constant inside `vapply()` doesn't benefit from vectorization.
          |
          = help: Compare after calling `vapply()`, e.g. `vapply(x, foo, FUN.VALUE = <intermediate>) == 2`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("{\n  sapply(x, function(xi) sd(xi))\n  lapply(y, function(yi) {\n    sum(yi)\n  })\n}"),
            @"
        warning: unnecessary_lambda
         --> <test>:2:13
          |
        2 |   sapply(x, function(xi) sd(xi))
          |             ------------------- Unnecessary anonymous function around `sd()`.
          |
          = help: Pass `sd` directly to `sapply()`.
        warning: unnecessary_lambda
         --> <test>:3:13
          |
        3 |     lapply(y, function(yi) {
          |  _____________-
        4 | |     sum(yi)
        5 | |   })
          | |___- Unnecessary anonymous function around `sum()`.
          |
          = help: Pass `sum` directly to `lapply()`.
        Found 2 errors.
        "
        );
    }

    #[test]
    fn test_unnecessary_lambda_fix() {
        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text(
                vec![
                    "lapply(DF, function(x) sum(x))",
                    "lapply(DF, \\(x) return(sum(x)))",
                    "lapply(x, function(xi) {\n  print(xi)\n})",
                    "lapply(x, function(xi, y) foo(xi, y), y)",
                    "lapply(x, \\(xi, ...) foo(xi, ...), y = 1)",
                    "purrr::map(x, ~ pkg::foo(.x))",
                    // No fix: extra arguments
                    "eapply(env, function(x) sum(x, na.rm = TRUE))",
                    // No fix: comparison
                    "sapply(x, function(xi) foo(xi) == 2)",
                    // No fix: comments
                    "lapply(y, function(yi) {\n  print(yi) # comment\n})",
                ],
                "unnecessary_lambda"
            )
        );
        // Unsafe fix only
        assert_snapshot!(
            get_fixed_text(vec!["lapply(DF, function(x) sum(x))"], "unnecessary_lambda", None),
            @"
        OLD:
        ====
        lapply(DF, function(x) sum(x))
        NEW:
        ====
        lapply(DF, function(x) sum(x))
        "
        );
    }
}
