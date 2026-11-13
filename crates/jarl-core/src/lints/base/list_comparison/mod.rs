pub(crate) mod list_comparison;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "list_comparison", None)
    }

    #[test]
    fn test_no_lint_list_comparison() {
        expect_no_lint("sapply(x, sum) > 10", "list_comparison", None);
        expect_no_lint("unlist(lapply(x, sum)) > 10", "list_comparison", None);
        expect_no_lint("lapply(x, sum) + 10", "list_comparison", None);
        expect_no_lint("length(lapply(x, sum)) > 10", "list_comparison", None);
        expect_no_lint("lapply(x, sum)", "list_comparison", None);
        expect_no_lint("x$lapply(sum) > 10", "list_comparison", None);
        expect_no_lint("x |> sapply(sum) > 10", "list_comparison", None);
        expect_no_lint("x |> lapply(sum) |> unlist() > 10", "list_comparison", None);
    }

    #[test]
    fn test_lint_list_comparison() {
        assert_snapshot!(
            snapshot_lint("lapply(x, sum) > 10"),
            @"
        warning: list_comparison
         --> <test>:1:1
          |
        1 | lapply(x, sum) > 10
          | ------------------- `lapply()` returns a list, so R must convert it to a vector before applying `>`.
          |
          = help: Use `vapply()` with an explicit output type instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("10 < purrr::map(x, sum)"),
            @"
        warning: list_comparison
         --> <test>:1:1
          |
        1 | 10 < purrr::map(x, sum)
          | ----------------------- `map()` returns a list, so R must convert it to a vector before applying `<`.
          |
          = help: Use a typed mapper such as `map_chr()` or `map_dbl()` instead.
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint("Map(f, x, y) == 1"),
            @"
        warning: list_comparison
         --> <test>:1:1
          |
        1 | Map(f, x, y) == 1
          | ----------------- `Map()` returns a list, so R must convert it to a vector before applying `==`.
          |
          = help: Use `mapply()` to return a vector directly.
        Found 1 error.
        "
        );

        assert_snapshot!(
            snapshot_lint(".mapply(`+`, list(1:10, 1:10), NULL) != 2"),
            @"
        warning: list_comparison
         --> <test>:1:1
          |
        1 | .mapply(`+`, list(1:10, 1:10), NULL) != 2
          | ----------------------------------------- `.mapply()` returns a list, so R must convert it to a vector before applying `!=`.
          |
          = help: Use `mapply()` to return a vector directly.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_list_comparison_operators() {
        assert_snapshot!(
            snapshot_lint(
                "{
  lapply(x, sum) == 1
  lapply(x, sum) != 1
  lapply(x, sum) >= 1
  lapply(x, sum) <= 1
  lapply(x, sum) > 1
  lapply(x, sum) < 1
}"
            ),
            @"
        warning: list_comparison
         --> <test>:2:3
          |
        2 |   lapply(x, sum) == 1
          |   ------------------- `lapply()` returns a list, so R must convert it to a vector before applying `==`.
          |
          = help: Use `vapply()` with an explicit output type instead.
        warning: list_comparison
         --> <test>:3:3
          |
        3 |   lapply(x, sum) != 1
          |   ------------------- `lapply()` returns a list, so R must convert it to a vector before applying `!=`.
          |
          = help: Use `vapply()` with an explicit output type instead.
        warning: list_comparison
         --> <test>:4:3
          |
        4 |   lapply(x, sum) >= 1
          |   ------------------- `lapply()` returns a list, so R must convert it to a vector before applying `>=`.
          |
          = help: Use `vapply()` with an explicit output type instead.
        warning: list_comparison
         --> <test>:5:3
          |
        5 |   lapply(x, sum) <= 1
          |   ------------------- `lapply()` returns a list, so R must convert it to a vector before applying `<=`.
          |
          = help: Use `vapply()` with an explicit output type instead.
        warning: list_comparison
         --> <test>:6:3
          |
        6 |   lapply(x, sum) > 1
          |   ------------------ `lapply()` returns a list, so R must convert it to a vector before applying `>`.
          |
          = help: Use `vapply()` with an explicit output type instead.
        warning: list_comparison
         --> <test>:7:3
          |
        7 |   lapply(x, sum) < 1
          |   ------------------ `lapply()` returns a list, so R must convert it to a vector before applying `<`.
          |
          = help: Use `vapply()` with an explicit output type instead.
        Found 6 errors.
        "
        );
    }

    #[test]
    fn test_lint_list_comparison_pipe() {
        assert_snapshot!(
            snapshot_lint("x |> lapply(sum) > 10"),
            @"
        warning: list_comparison
         --> <test>:1:1
          |
        1 | x |> lapply(sum) > 10
          | --------------------- `lapply()` returns a list, so R must convert it to a vector before applying `>`.
          |
          = help: Use `vapply()` with an explicit output type instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_list_comparison_multiple_expressions() {
        assert_snapshot!(
            snapshot_lint(
                "{
  sapply(x, sum) > 10
  .mapply(`+`, list(1:10, 1:10), NULL) == 2
  lapply(x, sum) < 5
}"
            ),
            @"
        warning: list_comparison
         --> <test>:3:3
          |
        3 |   .mapply(`+`, list(1:10, 1:10), NULL) == 2
          |   ----------------------------------------- `.mapply()` returns a list, so R must convert it to a vector before applying `==`.
          |
          = help: Use `mapply()` to return a vector directly.
        warning: list_comparison
         --> <test>:4:3
          |
        4 |   lapply(x, sum) < 5
          |   ------------------ `lapply()` returns a list, so R must convert it to a vector before applying `<`.
          |
          = help: Use `vapply()` with an explicit output type instead.
        Found 2 errors.
        "
        );
    }
}
