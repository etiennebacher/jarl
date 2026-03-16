pub(crate) mod dplyr_group_by_ungroup;

#[cfg(test)]
mod tests {
    use crate::{declare_ns, utils_test::*};
    use insta::assert_snapshot;

    // Needed to get a package cache working without requiring an R runtime.
    declare_ns! {
        "dplyr" => ["ungroup", "filter"],
    }

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics_with_cache(code, "dplyr_group_by_ungroup", None, &NS)
    }

    #[test]
    fn test_no_lint_dplyr_group_by_ungroup() {
        // No ungroup at the end
        expect_no_lint(
            "x |> group_by(grp) |> summarize(a = mean(b))",
            "dplyr_group_by_ungroup",
            None,
        );
        // More than one verb between group_by and ungroup
        expect_no_lint(
            "x |> group_by(grp) |> mutate(a = 1) |> summarize(b = mean(a)) |> ungroup()",
            "dplyr_group_by_ungroup",
            None,
        );
        // group_by() with arguments (not a simple pipe ungroup)
        expect_no_lint(
            "x |> group_by(grp1, grp2, .add = TRUE) |> summarize(a = mean(b)) |> ungroup()",
            "dplyr_group_by_ungroup",
            None,
        );
        // ungroup() with arguments (not a simple pipe ungroup)
        expect_no_lint(
            "x |> group_by(grp1, grp2) |> summarize(a = mean(b)) |> ungroup(grp1)",
            "dplyr_group_by_ungroup",
            None,
        );
        // Verb already has .by
        expect_no_lint(
            "x |> group_by(grp) |> summarize(a = mean(b), .by = grp) |> ungroup()",
            "dplyr_group_by_ungroup",
            None,
        );
        // Slice verb already has by (slice_*() use `by`, not `.by`)
        expect_no_lint(
            "x |> group_by(grp) |> slice_head(n = 1, by = grp) |> ungroup()",
            "dplyr_group_by_ungroup",
            None,
        );
        // Non-dplyr verb
        expect_no_lint(
            "x |> group_by(grp) |> my_custom_fun() |> ungroup()",
            "dplyr_group_by_ungroup",
            None,
        );
        // Standalone ungroup
        expect_no_lint(
            "
            library(dplyr)
x |> ungroup()",
            "dplyr_group_by_ungroup",
            None,
        );
        // Non-dplyr namespace
        expect_no_lint(
            "x |> group_by(grp) |> summarize(a = 1) |> other::ungroup()",
            "dplyr_group_by_ungroup",
            None,
        );
        expect_no_lint(
            "x |> group_by(grp) |> other::summarize(a = 1) |> ungroup()",
            "dplyr_group_by_ungroup",
            None,
        );
    }

    #[test]
    fn test_lint_summarize() {
        assert_snapshot!(
            snapshot_lint("
            library(dplyr)
x |> group_by(grp) |> summarize(a = mean(b)) |> ungroup()
            "),
            @"
        warning: dplyr_group_by_ungroup
         --> <test>:3:6
          |
        3 | x |> group_by(grp) |> summarize(a = mean(b)) |> ungroup()
          |      ---------------------------------------------------- `group_by()` followed by `summarize()` and `ungroup()` can be simplified.
          |
          = help: Use `summarize(..., .by = grp)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_summarise() {
        assert_snapshot!(
            snapshot_lint("
            library(dplyr)
x |> group_by(grp) |> summarise(a = mean(b)) |> ungroup()
            "),
            @"
        warning: dplyr_group_by_ungroup
         --> <test>:3:6
          |
        3 | x |> group_by(grp) |> summarise(a = mean(b)) |> ungroup()
          |      ---------------------------------------------------- `group_by()` followed by `summarise()` and `ungroup()` can be simplified.
          |
          = help: Use `summarise(..., .by = grp)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_mutate() {
        assert_snapshot!(
            snapshot_lint("
            library(dplyr)
x |> group_by(grp) |> mutate(a = mean(b)) |> ungroup()
            "),
            @"
        warning: dplyr_group_by_ungroup
         --> <test>:3:6
          |
        3 | x |> group_by(grp) |> mutate(a = mean(b)) |> ungroup()
          |      ------------------------------------------------- `group_by()` followed by `mutate()` and `ungroup()` can be simplified.
          |
          = help: Use `mutate(..., .by = grp)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_filter() {
        assert_snapshot!(
            snapshot_lint("
            library(dplyr)
x |> group_by(grp) |> filter(a > 1) |> ungroup()"
            ),
            @"
        warning: dplyr_group_by_ungroup
         --> <test>:3:6
          |
        3 | x |> group_by(grp) |> filter(a > 1) |> ungroup()
          |      ------------------------------------------- `group_by()` followed by `filter()` and `ungroup()` can be simplified.
          |
          = help: Use `filter(..., .by = grp)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_multiple_groups() {
        assert_snapshot!(
            snapshot_lint("
            library(dplyr)
x |> group_by(grp1, grp2) |> summarize(a = mean(b)) |> ungroup()
            "),
            @"
        warning: dplyr_group_by_ungroup
         --> <test>:3:6
          |
        3 | x |> group_by(grp1, grp2) |> summarize(a = mean(b)) |> ungroup()
          |      ----------------------------------------------------------- `group_by()` followed by `summarize()` and `ungroup()` can be simplified.
          |
          = help: Use `summarize(..., .by = c(grp1, grp2))` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_namespaced() {
        assert_snapshot!(
            snapshot_lint("
            library(dplyr)
x |> dplyr::group_by(grp) |> dplyr::summarize(a = mean(b)) |> dplyr::ungroup()
            "),
            @"
        warning: dplyr_group_by_ungroup
         --> <test>:3:6
          |
        3 | x |> dplyr::group_by(grp) |> dplyr::summarize(a = mean(b)) |> dplyr::ungroup()
          |      ------------------------------------------------------------------------- `group_by()` followed by `summarize()` and `ungroup()` can be simplified.
          |
          = help: Use `summarize(..., .by = grp)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_magrittr_pipe() {
        assert_snapshot!(
            snapshot_lint("
            library(dplyr)
x %>% group_by(grp) %>% summarize(a = mean(b)) %>% ungroup()
            "),
            @"
        warning: dplyr_group_by_ungroup
         --> <test>:3:7
          |
        3 | x %>% group_by(grp) %>% summarize(a = mean(b)) %>% ungroup()
          |       ------------------------------------------------------ `group_by()` followed by `summarize()` and `ungroup()` can be simplified.
          |
          = help: Use `summarize(..., .by = grp)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_slice_verbs() {
        assert_snapshot!(
            snapshot_lint("
            library(dplyr)
x |> group_by(grp) |> slice(n = 1) |> ungroup()
            "),
            @"
        warning: dplyr_group_by_ungroup
         --> <test>:3:6
          |
        3 | x |> group_by(grp) |> slice(n = 1) |> ungroup()
          |      ------------------------------------------ `group_by()` followed by `slice()` and `ungroup()` can be simplified.
          |
          = help: Use `slice(..., .by = grp)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("
            library(dplyr)
x |> group_by(grp) |> slice_head(n = 1) |> ungroup()
            "),
            @"
        warning: dplyr_group_by_ungroup
         --> <test>:3:6
          |
        3 | x |> group_by(grp) |> slice_head(n = 1) |> ungroup()
          |      ----------------------------------------------- `group_by()` followed by `slice_head()` and `ungroup()` can be simplified.
          |
          = help: Use `slice_head(..., by = grp)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("
            library(dplyr)
x |> group_by(grp) |> slice_tail(n = 1) |> ungroup()
            "),
            @"
        warning: dplyr_group_by_ungroup
         --> <test>:3:6
          |
        3 | x |> group_by(grp) |> slice_tail(n = 1) |> ungroup()
          |      ----------------------------------------------- `group_by()` followed by `slice_tail()` and `ungroup()` can be simplified.
          |
          = help: Use `slice_tail(..., by = grp)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("
            library(dplyr)
x |> group_by(grp) |> slice_min(val) |> ungroup()
            "),
            @"
        warning: dplyr_group_by_ungroup
         --> <test>:3:6
          |
        3 | x |> group_by(grp) |> slice_min(val) |> ungroup()
          |      -------------------------------------------- `group_by()` followed by `slice_min()` and `ungroup()` can be simplified.
          |
          = help: Use `slice_min(..., by = grp)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("
            library(dplyr)
x |> group_by(grp) |> slice_max(val) |> ungroup()
            "),
            @"
        warning: dplyr_group_by_ungroup
         --> <test>:3:6
          |
        3 | x |> group_by(grp) |> slice_max(val) |> ungroup()
          |      -------------------------------------------- `group_by()` followed by `slice_max()` and `ungroup()` can be simplified.
          |
          = help: Use `slice_max(..., by = grp)` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("
            library(dplyr)
x |> group_by(grp) |> slice_sample(n = 5) |> ungroup()
            "),
            @"
        warning: dplyr_group_by_ungroup
         --> <test>:3:6
          |
        3 | x |> group_by(grp) |> slice_sample(n = 5) |> ungroup()
          |      ------------------------------------------------- `group_by()` followed by `slice_sample()` and `ungroup()` can be simplified.
          |
          = help: Use `slice_sample(..., by = grp)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_multiline() {
        assert_snapshot!(
            snapshot_lint(
                "
            library(dplyr)
            x |>
  group_by(grp) |>
  summarize(
    a = mean(b)
  ) |>
  ungroup()"
            ),
            @"
        warning: dplyr_group_by_ungroup
         --> <test>:4:3
          |
        4 | /   group_by(grp) |>
        5 | |   summarize(
        6 | |     a = mean(b)
        7 | |   ) |>
        8 | |   ungroup()
          | |___________- `group_by()` followed by `summarize()` and `ungroup()` can be simplified.
          |
          = help: Use `summarize(..., .by = grp)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_group_by_not_piped() {
        // group_by(data, grp) |> summarize(...) |> ungroup()
        assert_snapshot!(
            snapshot_lint("
            library(dplyr)
group_by(x, grp) |> summarize(a = mean(b)) |> ungroup()
            "),
            @"
        warning: dplyr_group_by_ungroup
         --> <test>:3:1
          |
        3 | group_by(x, grp) |> summarize(a = mean(b)) |> ungroup()
          | ------------------------------------------------------- `group_by()` followed by `summarize()` and `ungroup()` can be simplified.
          |
          = help: Use `summarize(..., .by = grp)` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_fix_single_group() {
        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text_with_cache(
                vec![
                    "library(dplyr); x |> group_by(grp) |> summarize(a = mean(b)) |> ungroup()",
                    "library(dplyr); x |> group_by(grp) |> mutate(a = mean(b)) |> ungroup()",
                    "library(dplyr); x |> group_by(grp) |> filter(a > 1) |> ungroup()",
                    "library(dplyr); x |> group_by(grp) |> slice_head(n = 1) |> ungroup()",
                ],
                "dplyr_group_by_ungroup",
                &NS,
            )
        );
    }

    #[test]
    fn test_fix_multiple_groups() {
        assert_snapshot!(
            "fix_multiple_groups",
            get_unsafe_fixed_text_with_cache(
                vec![
                    "library(dplyr); x |> group_by(grp1, grp2) |> summarize(a = mean(b)) |> ungroup()"
                ],
                "dplyr_group_by_ungroup",
                &NS,
            )
        );
    }

    #[test]
    fn test_fix_magrittr_pipe() {
        assert_snapshot!(
            "fix_magrittr",
            get_unsafe_fixed_text_with_cache(
                vec![
                    "library(dplyr); x %>% group_by(grp) %>% summarize(a = mean(b)) %>% ungroup()"
                ],
                "dplyr_group_by_ungroup",
                &NS,
            )
        );
    }

    #[test]
    fn test_no_fix_group_by_not_piped() {
        assert_snapshot!(
            "no_fix_not_piped",
            get_unsafe_fixed_text_with_cache(
                vec!["library(dplyr); group_by(x, grp) |> summarize(a = mean(b)) |> ungroup()"],
                "dplyr_group_by_ungroup",
                &NS,
            )
        );
    }

    #[test]
    fn test_fix_namespaced() {
        assert_snapshot!(
            "fix_namespaced",
            get_unsafe_fixed_text_with_cache(
                vec![
                    "x |> dplyr::group_by(grp) |> dplyr::summarize(a = mean(b)) |> dplyr::ungroup()",
                ],
                "dplyr_group_by_ungroup",
                &NS,
            )
        );
    }

    #[test]
    fn test_no_fix_with_comments() {
        assert_snapshot!(
            "no_fix_with_comments",
            get_unsafe_fixed_text_with_cache(
                vec![
                    "library(dplyr); x |> group_by(grp) |> # comment\n  summarize(a = mean(b)) |> ungroup()",
                    "library(dplyr); x |>\n  group_by(grp) |>\n  summarize(\n    # comment\n    a = mean(b)\n  ) |>\n  ungroup()",
                ],
                "dplyr_group_by_ungroup",
                &NS,
            )
        );
    }

    #[test]
    fn test_lint_splice() {
        assert_snapshot!(
            snapshot_lint("
            library(dplyr)
x |> group_by(!!!syms(grps)) |> summarize(a = mean(b)) |> ungroup()
            "),
            @"
        warning: dplyr_group_by_ungroup
         --> <test>:3:6
          |
        3 | x |> group_by(!!!syms(grps)) |> summarize(a = mean(b)) |> ungroup()
          |      -------------------------------------------------------------- `group_by()` followed by `summarize()` and `ungroup()` can be simplified.
          |
          = help: Use `summarize(..., .by = c(!!!syms(grps)))` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_fix_splice() {
        assert_snapshot!(
            "fix_splice",
            get_unsafe_fixed_text_with_cache(
                vec![
                    "library(dplyr); x |> group_by(!!!syms(grps)) |> summarize(a = mean(b)) |> ungroup()"
                ],
                "dplyr_group_by_ungroup",
                &NS,
            )
        );
    }
}
