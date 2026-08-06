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
    }

    #[test]
    fn test_lintr_cases_list_comparison() {
        for mapper in ["lapply", "map", "Map", ".mapply"] {
            for comparator in ["==", "!=", ">=", "<=", ">", "<"] {
                let code = format!("{mapper}(x, sum) {comparator} 10");
                assert_eq!(
                    check_code(&code, "list_comparison", None).len(),
                    1,
                    "expected a lint for `{code}`"
                );
            }
        }
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
    }

    #[test]
    fn test_vectorizes_list_comparison() {
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
