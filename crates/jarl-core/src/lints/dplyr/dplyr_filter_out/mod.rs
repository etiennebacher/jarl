pub(crate) mod dplyr_filter_out;

#[cfg(test)]
mod tests {
    use crate::{declare_ns, utils_test::*};
    use insta::assert_snapshot;

    // Needed to get a package cache working without requiring an R runtime.
    declare_ns! {
        "stats" => ["filter"],
        "dplyr" => ["filter"],
    }

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics_with_cache(code, "dplyr_filter_out", None, &NS)
    }

    fn snapshot_fix(code: &str) -> String {
        get_unsafe_fixed_text_with_cache(vec![code], "dplyr_filter_out", &NS)
    }

    #[test]
    fn test_no_lint() {
        // No negation
        expect_no_lint("x |> dplyr::filter(a > 1)", "dplyr_filter_out", None);
        // Already using filter_out
        expect_no_lint(
            "x |> dplyr::filter_out(is.na(val))",
            "dplyr_filter_out",
            None,
        );
        // Non-dplyr namespace
        expect_no_lint("x |> stats::filter(!cond)", "dplyr_filter_out", None);
        // Named argument with negation (not a filtering condition)
        expect_no_lint(
            "x |> dplyr::filter(a > 1, .preserve = !TRUE)",
            "dplyr_filter_out",
            None,
        );
        // Double bang (tidy eval injection)
        expect_no_lint("x |> dplyr::filter(!!cond)", "dplyr_filter_out", None);
        // Triple bang (tidy eval splice)
        expect_no_lint("x |> dplyr::filter(!!!filters)", "dplyr_filter_out", None);
        // Not all unnamed args are negated
        expect_no_lint(
            "x |> dplyr::filter(a > 1, !is.na(b))",
            "dplyr_filter_out",
            None,
        );
        expect_no_lint(
            "x |> dplyr::filter(!(a > 1), is.na(b))",
            "dplyr_filter_out",
            None,
        );
        // is.na guard for a different variable — don't match
        expect_no_lint(
            "x |> dplyr::filter(a > 1 | is.na(b))",
            "dplyr_filter_out",
            None,
        );
        // unknown named arg, don't report
        expect_no_lint(
            "x |> dplyr::filter(a > 1 | is.na(a), foo = 1)",
            "dplyr_filter_out",
            None,
        );
    }

    // ---- Negation pattern: lint + unsafe fix ----

    #[test]
    fn test_lint_negation_namespaced() {
        assert_snapshot!(
            snapshot_lint("x |> dplyr::filter(!is.na(val))"),
            @r"
        warning: dplyr_filter_out
         --> <test>:1:6
          |
        1 | x |> dplyr::filter(!is.na(val))
          |      -------------------------- Negating conditions in `filter()` can be hard to read.
          |
          = help: You could use `filter_out()` instead (but beware of `NA` handling).
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_with_library() {
        // Can't know if filter() is from stats or dplyr.
        // Use heuristic: if part of a pipe chain, assume it's from dplyr.
        // Non-piped filter(x, ...) is not reported.
        assert_snapshot!(
            snapshot_lint("
            library(dplyr)
            filter(x, !is.na(val))
            x |> filter(!is.na(val))
            x %>% filter(!is.na(val))
            "),
            @r"
        warning: dplyr_filter_out
         --> <test>:4:18
          |
        4 |             x |> filter(!is.na(val))
          |                  ------------------- Negating conditions in `filter()` can be hard to read.
          |
          = help: You could use `filter_out()` instead (but beware of `NA` handling).
        warning: dplyr_filter_out
         --> <test>:5:19
          |
        5 |             x %>% filter(!is.na(val))
          |                   ------------------- Negating conditions in `filter()` can be hard to read.
          |
          = help: You could use `filter_out()` instead (but beware of `NA` handling).
        Found 2 errors.
        "
        );
    }

    #[test]
    fn test_fix_negation() {
        assert_snapshot!(
            snapshot_fix("x |> dplyr::filter(!is.na(val))"),
            @r"
        OLD:
        ====
        x |> dplyr::filter(!is.na(val))
        NEW:
        ====
        x |> dplyr::filter_out(is.na(val))
        "
        );
    }

    // ---- is.na() guard pattern: lint + safe fix ----

    #[test]
    fn test_lint_is_na_guard() {
        assert_snapshot!(
            snapshot_lint("x |> dplyr::filter(a > 1 | is.na(a))"),
            @r"
        warning: dplyr_filter_out
         --> <test>:1:6
          |
        1 | x |> dplyr::filter(a > 1 | is.na(a))
          |      ------------------------------- This `| is.na()` pattern can be replaced by `filter_out()`.
          |
          = help: `filter_out()` keeps `NA` rows automatically, so the guard is unnecessary.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_fix_is_na_guard() {
        assert_snapshot!(
            snapshot_fix("x |> dplyr::filter(a > 1 | is.na(a))"),
            @r"
        OLD:
        ====
        x |> dplyr::filter(a > 1 | is.na(a))
        NEW:
        ====
        x |> dplyr::filter_out(a <= 1)
        "
        );
    }

    #[test]
    fn test_fix_is_na_guard_negated_cond() {
        // filter(!cond | is.na(var)) → filter_out(cond) (strips the !)
        assert_snapshot!(
            snapshot_fix("x |> dplyr::filter(!is_valid | is.na(is_valid))"),
            @r"
        OLD:
        ====
        x |> dplyr::filter(!is_valid | is.na(is_valid))
        NEW:
        ====
        x |> dplyr::filter_out(is_valid)
        "
        );
    }

    #[test]
    fn test_fix_is_na_guard_preserves_namespace() {
        assert_snapshot!(
            snapshot_fix("x |> dplyr::filter(a != 'foo' | is.na(a))"),
            @r#"
        OLD:
        ====
        x |> dplyr::filter(a != 'foo' | is.na(a))
        NEW:
        ====
        x |> dplyr::filter_out(a == 'foo')
        "#
        );
    }

    #[test]
    fn test_fix_is_na_guard_preserves_named_args() {
        assert_snapshot!(
            snapshot_fix("x |> dplyr::filter(a > 1 | is.na(a), .by = grp)"),
            @r"
        OLD:
        ====
        x |> dplyr::filter(a > 1 | is.na(a), .by = grp)
        NEW:
        ====
        x |> dplyr::filter_out(a <= 1, .by = grp)
        "
        );
    }

    #[test]
    fn test_fix_is_na_guard_reversed() {
        // is.na(a) | cond — guard on the left side
        assert_snapshot!(
            snapshot_fix("x |> dplyr::filter(is.na(a) | a > 1)"),
            @r"
        OLD:
        ====
        x |> dplyr::filter(is.na(a) | a > 1)
        NEW:
        ====
        x |> dplyr::filter_out(a <= 1)
        "
        );
    }

    #[test]
    fn test_fix_is_na_guard_identifier_cond() {
        // Condition is a plain identifier — negated with `!`
        assert_snapshot!(
            snapshot_fix("x |> dplyr::filter(is_valid | is.na(is_valid))"),
            @r"
        OLD:
        ====
        x |> dplyr::filter(is_valid | is.na(is_valid))
        NEW:
        ====
        x |> dplyr::filter_out(!is_valid)
        "
        );
    }

    #[test]
    fn test_fix_is_na_guard_call_cond() {
        // Condition is a function call — negated with `!`
        assert_snapshot!(
            snapshot_fix("x |> dplyr::filter(my_fun(a) | is.na(a))"),
            @r"
        OLD:
        ====
        x |> dplyr::filter(my_fun(a) | is.na(a))
        NEW:
        ====
        x |> dplyr::filter_out(!my_fun(a))
        "
        );
    }

    #[test]
    fn test_fix_is_na_guard_multiple_args() {
        // Multiple comma-separated args (AND) → joined with | (OR) by De Morgan
        assert_snapshot!(
            snapshot_fix("x |> dplyr::filter(a > 1 | is.na(a), b < 2 | is.na(b))"),
            @r"
        OLD:
        ====
        x |> dplyr::filter(a > 1 | is.na(a), b < 2 | is.na(b))
        NEW:
        ====
        x |> dplyr::filter_out(a <= 1 | b >= 2)
        "
        );
    }

    #[test]
    fn test_fix_is_na_guard_multiple_args_with_named() {
        assert_snapshot!(
            snapshot_fix("x |> dplyr::filter(a > 1 | is.na(a), b < 2 | is.na(b), .by = grp)"),
            @r"
        OLD:
        ====
        x |> dplyr::filter(a > 1 | is.na(a), b < 2 | is.na(b), .by = grp)
        NEW:
        ====
        x |> dplyr::filter_out(a <= 1 | b >= 2, .by = grp)
        "
        );
    }

    #[test]
    fn test_lint_parenthesized_negation() {
        assert_snapshot!(
            snapshot_lint("x |> dplyr::filter(!(a > 1))"),
            @r"
        warning: dplyr_filter_out
         --> <test>:1:6
          |
        1 | x |> dplyr::filter(!(a > 1))
          |      ----------------------- Negating conditions in `filter()` can be hard to read.
          |
          = help: You could use `filter_out()` instead (but beware of `NA` handling).
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_fix_parenthesized_negation() {
        assert_snapshot!(
            snapshot_fix("x |> dplyr::filter(!(a > 1))"),
            @r"
        OLD:
        ====
        x |> dplyr::filter(!(a > 1))
        NEW:
        ====
        x |> dplyr::filter_out(a <= 1)
        "
        );
    }

    #[test]
    fn test_fix_parenthesized_negation_multi_args() {
        assert_snapshot!(
            snapshot_fix("x |> dplyr::filter(!(a > 1), !foo(a))"),
            @r"
        OLD:
        ====
        x |> dplyr::filter(!(a > 1), !foo(a))
        NEW:
        ====
        x |> dplyr::filter_out(a <= 1 | foo(a))
        "
        );
    }

    #[test]
    fn test_fix_parenthesized_negation_multi_args_with_named_arg() {
        assert_snapshot!(
            snapshot_fix("x |> dplyr::filter(!(a > 1), !foo(a), .by = g)"),
            @r"
        OLD:
        ====
        x |> dplyr::filter(!(a > 1), !foo(a), .by = g)
        NEW:
        ====
        x |> dplyr::filter_out(a <= 1 | foo(a), .by = g)
        "
        );
    }
}
