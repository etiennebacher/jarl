pub(crate) mod dplyr_filter_out;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "dplyr_filter_out", None)
    }

    #[test]
    fn test_no_lint_dplyr_filter_out() {
        // No negation
        expect_no_lint("x |> dplyr::filter(a > 1)", "dplyr_filter_out", None);
        // Already using dplyr_filter_out
        expect_no_lint("x |> dplyr::dplyr_filter_out(is.na(val))", "dplyr_filter_out", None);
        // Non-dplyr namespace
        expect_no_lint("x |> stats::filter(!cond)", "dplyr_filter_out", None);
        // Bare filter without pipe (could be stats::filter)
        expect_no_lint("filter(x, !cond)", "dplyr_filter_out", None);
        // Named argument with negation (not a filtering condition)
        expect_no_lint("x |> dplyr::filter(.preserve = !TRUE)", "dplyr_filter_out", None);
        // Double bang (tidy eval injection)
        expect_no_lint("x |> dplyr::filter(!!cond)", "dplyr_filter_out", None);
        // Triple bang (tidy eval splice)
        expect_no_lint("x |> dplyr::filter(!!!filters)", "dplyr_filter_out", None);
    }

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
          = help: Use `dplyr_filter_out(is.na(val))` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_negation_piped() {
        assert_snapshot!(
            snapshot_lint("x |> filter(!cond)"),
            @r"
        warning: dplyr_filter_out
         --> <test>:1:6
          |
        1 | x |> filter(!cond)
          |      ------------- Negating conditions in `filter()` can be hard to read.
          |
          = help: Use `dplyr_filter_out(cond)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_magrittr_pipe() {
        assert_snapshot!(
            snapshot_lint("x %>% filter(!cond)"),
            @r"
        warning: dplyr_filter_out
         --> <test>:1:7
          |
        1 | x %>% filter(!cond)
          |       ------------- Negating conditions in `filter()` can be hard to read.
          |
          = help: Use `dplyr_filter_out(cond)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_multiple_args_one_negated() {
        assert_snapshot!(
            snapshot_lint("x |> dplyr::filter(a > 1, !is.na(b))"),
            @r"
        warning: dplyr_filter_out
         --> <test>:1:6
          |
        1 | x |> dplyr::filter(a > 1, !is.na(b))
          |      ------------------------------- Negating conditions in `filter()` can be hard to read.
          |
          = help: Use `dplyr_filter_out(is.na(b))` instead.
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
          = help: Use `dplyr_filter_out(a > 1)` instead.
        Found 1 error.
        "
        );
    }
}
