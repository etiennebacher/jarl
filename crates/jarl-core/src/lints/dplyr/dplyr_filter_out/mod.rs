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

    #[test]
    fn test_no_lint_dplyr_filter_out() {
        // No negation
        expect_no_lint("x |> dplyr::filter(a > 1)", "dplyr_filter_out", None);
        // Already using dplyr_filter_out
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
    }

    #[test]
    fn test_lint_explicit_namespace() {
        assert_snapshot!(
            snapshot_lint("x |> dplyr::filter(!is.na(val))"),
            @r"
        warning: dplyr_filter_out
         --> <test>:1:6
          |
        1 | x |> dplyr::filter(!is.na(val))
          |      -------------------------- Negating conditions in `filter()` can be hard to read.
          |
          = help: Use `filter_out(is.na(val))` instead.
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
          = help: Use `filter_out(is.na(val))` instead.
        warning: dplyr_filter_out
         --> <test>:5:19
          |
        5 |             x %>% filter(!is.na(val))
          |                   ------------------- Negating conditions in `filter()` can be hard to read.
          |
          = help: Use `filter_out(is.na(val))` instead.
        Found 2 errors.
        "
        );

        // Without library(dplyr), the pipe heuristic doesn't apply.
        assert_snapshot!(
            snapshot_lint("
            filter(x, !is.na(val))
            x |> filter(!is.na(val))
            x %>% filter(!is.na(val))
            "),
            @"All checks passed!"
        );
    }

    #[test]
    fn test_lint_multiple_args_one_negated() {
        // TODO: this is wrong
        assert_snapshot!(
            snapshot_lint("x |> dplyr::filter(a > 1, !is.na(b))"),
            @r"
        warning: dplyr_filter_out
         --> <test>:1:6
          |
        1 | x |> dplyr::filter(a > 1, !is.na(b))
          |      ------------------------------- Negating conditions in `filter()` can be hard to read.
          |
          = help: Use `filter_out(is.na(b))` instead.
        Found 1 error.
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
          = help: Use `filter_out(a > 1)` instead.
        Found 1 error.
        "
        );
    }
}
