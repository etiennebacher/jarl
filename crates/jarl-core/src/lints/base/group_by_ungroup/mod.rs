pub(crate) mod group_by_ungroup;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "group_by_ungroup", None)
    }

    #[test]
    fn test_no_lint_group_by_ungroup() {
        // No ungroup at the end
        expect_no_lint(
            "x |> group_by(grp) |> summarize(a = mean(b))",
            "group_by_ungroup",
            None,
        );
        // More than one verb between group_by and ungroup
        expect_no_lint(
            "x |> group_by(grp) |> mutate(a = 1) |> summarize(b = mean(a)) |> ungroup()",
            "group_by_ungroup",
            None,
        );
        // ungroup() with arguments (not a simple pipe ungroup)
        expect_no_lint(
            "x |> group_by(grp1, grp2) |> summarize(a = mean(b)) |> ungroup(grp1)",
            "group_by_ungroup",
            None,
        );
        // Verb already has .by
        expect_no_lint(
            "x |> group_by(grp) |> summarize(a = mean(b), .by = grp) |> ungroup()",
            "group_by_ungroup",
            None,
        );
        // Non-dplyr verb
        expect_no_lint(
            "x |> group_by(grp) |> my_custom_fun() |> ungroup()",
            "group_by_ungroup",
            None,
        );
        // Standalone ungroup
        expect_no_lint("x |> ungroup()", "group_by_ungroup", None);
        // Non-dplyr namespace
        expect_no_lint(
            "x |> group_by(grp) |> summarize(a = 1) |> other::ungroup()",
            "group_by_ungroup",
            None,
        );
    }

    #[test]
    fn test_lint_summarize() {
        assert_snapshot!(
            snapshot_lint("x |> group_by(grp) |> summarize(a = mean(b)) |> ungroup()"),
            @r"
        warning: group_by_ungroup
         --> <test>:1:6
          |
        1 | x |> group_by(grp) |> summarize(a = mean(b)) |> ungroup()
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
            snapshot_lint("x |> group_by(grp) |> summarise(a = mean(b)) |> ungroup()"),
            @r"
        warning: group_by_ungroup
         --> <test>:1:6
          |
        1 | x |> group_by(grp) |> summarise(a = mean(b)) |> ungroup()
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
            snapshot_lint("x |> group_by(grp) |> mutate(a = mean(b)) |> ungroup()"),
            @r"
        warning: group_by_ungroup
         --> <test>:1:6
          |
        1 | x |> group_by(grp) |> mutate(a = mean(b)) |> ungroup()
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
            snapshot_lint("x |> group_by(grp) |> filter(a > 1) |> ungroup()"),
            @r"
        warning: group_by_ungroup
         --> <test>:1:6
          |
        1 | x |> group_by(grp) |> filter(a > 1) |> ungroup()
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
            snapshot_lint("x |> group_by(grp1, grp2) |> summarize(a = mean(b)) |> ungroup()"),
            @r"
        warning: group_by_ungroup
         --> <test>:1:6
          |
        1 | x |> group_by(grp1, grp2) |> summarize(a = mean(b)) |> ungroup()
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
            snapshot_lint("x |> dplyr::group_by(grp) |> dplyr::summarize(a = mean(b)) |> dplyr::ungroup()"),
            @r"
        warning: group_by_ungroup
         --> <test>:1:6
          |
        1 | x |> dplyr::group_by(grp) |> dplyr::summarize(a = mean(b)) |> dplyr::ungroup()
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
            snapshot_lint("x %>% group_by(grp) %>% summarize(a = mean(b)) %>% ungroup()"),
            @r"
        warning: group_by_ungroup
         --> <test>:1:7
          |
        1 | x %>% group_by(grp) %>% summarize(a = mean(b)) %>% ungroup()
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
            "slice_head",
            snapshot_lint("x |> group_by(grp) |> slice_head(n = 1) |> ungroup()")
        );
        assert_snapshot!(
            "slice_tail",
            snapshot_lint("x |> group_by(grp) |> slice_tail(n = 1) |> ungroup()")
        );
        assert_snapshot!(
            "slice_min",
            snapshot_lint("x |> group_by(grp) |> slice_min(val) |> ungroup()")
        );
        assert_snapshot!(
            "slice_max",
            snapshot_lint("x |> group_by(grp) |> slice_max(val) |> ungroup()")
        );
        assert_snapshot!(
            "slice_sample",
            snapshot_lint("x |> group_by(grp) |> slice_sample(n = 5) |> ungroup()")
        );
    }

    #[test]
    fn test_lint_multiline() {
        assert_snapshot!(
            snapshot_lint(
                "x |>
  group_by(grp) |>
  summarize(
    a = mean(b)
  ) |>
  ungroup()"
            ),
            @r"
        warning: group_by_ungroup
         --> <test>:2:3
          |
        2 | /   group_by(grp) |>
        3 | |   summarize(
        4 | |     a = mean(b)
        5 | |   ) |>
        6 | |   ungroup()
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
            snapshot_lint("group_by(x, grp) |> summarize(a = mean(b)) |> ungroup()"),
            @r"
        warning: group_by_ungroup
         --> <test>:1:1
          |
        1 | group_by(x, grp) |> summarize(a = mean(b)) |> ungroup()
          | ------------------------------------------------------- `group_by()` followed by `summarize()` and `ungroup()` can be simplified.
          |
          = help: Use `summarize(..., .by = grp)` instead.
        Found 1 error.
        "
        );
    }
}
