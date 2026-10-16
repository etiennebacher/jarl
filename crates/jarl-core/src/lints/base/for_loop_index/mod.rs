pub(crate) mod for_loop_index;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "for_loop_index", None)
    }

    #[test]
    fn test_no_lint_for_loop_index() {
        for code in [
            "for (xi in x) {}",
            "for (col in DF$col) {}",
            "for (col in S4@col) {}",
            "for (col in DT[, col]) {}",
            "for (x in y[[x]]) {}",
            "for (x in foo(x = 1)) {}",
            "for (x in foo(bar(y, baz(2, x = z)))) {}",
            "for (x in foo(`x` = 1, \"x\" = 2)) {}",
            "for (x in x()) {}",
            "for (x in pkg::x()) {}",
            "for (x in x::values) {}",
            "for (x in foo(\"x\")) {}",
            "for (x in foo(function(x) x)) {}",
            "for (x in foo(\\(x) x)) {}",
            "for (x in y) { print(x) }",
            "{ for (i in 1:10) { 42L }; i <- 7L }",
            "for (x in foo(, y, z =)) {}",
        ] {
            expect_no_lint(code, "for_loop_index", None);
        }
    }

    #[test]
    fn test_lint_for_loop_index() {
        assert_snapshot!(
            snapshot_lint("for (x in x) {}"),
            @"
        warning: for_loop_index
         --> <test>:1:6
          |
        1 | for (x in x) {}
          |      ------ Don't re-use any sequence symbols as the index symbol in a for loop.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (x in foo(x)) {}"),
            @"
        warning: for_loop_index
         --> <test>:1:6
          |
        1 | for (x in foo(x)) {}
          |      ----------- Don't re-use any sequence symbols as the index symbol in a for loop.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (x in foo(bar(y, baz(2, x)))) {}"),
            @"
        warning: for_loop_index
         --> <test>:1:6
          |
        1 | for (x in foo(bar(y, baz(2, x)))) {}
          |      --------------------------- Don't re-use any sequence symbols as the index symbol in a for loop.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (x in (x)) {}"),
            @"
        warning: for_loop_index
         --> <test>:1:6
          |
        1 | for (x in (x)) {}
          |      -------- Don't re-use any sequence symbols as the index symbol in a for loop.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (x in foo(-x + 1)) {}"),
            @"
        warning: for_loop_index
         --> <test>:1:6
          |
        1 | for (x in foo(-x + 1)) {}
          |      ---------------- Don't re-use any sequence symbols as the index symbol in a for loop.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (x in x$col) {}"),
            @"
        warning: for_loop_index
         --> <test>:1:6
          |
        1 | for (x in x$col) {}
          |      ---------- Don't re-use any sequence symbols as the index symbol in a for loop.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (x in x@col) {}"),
            @"
        warning: for_loop_index
         --> <test>:1:6
          |
        1 | for (x in x@col) {}
          |      ---------- Don't re-use any sequence symbols as the index symbol in a for loop.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (x in x[1]) {}"),
            @"
        warning: for_loop_index
         --> <test>:1:6
          |
        1 | for (x in x[1]) {}
          |      --------- Don't re-use any sequence symbols as the index symbol in a for loop.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (x in x[[1]]) {}"),
            @"
        warning: for_loop_index
         --> <test>:1:6
          |
        1 | for (x in x[[1]]) {}
          |      ----------- Don't re-use any sequence symbols as the index symbol in a for loop.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (x in foo(y = x)) {}"),
            @"
        warning: for_loop_index
         --> <test>:1:6
          |
        1 | for (x in foo(y = x)) {}
          |      --------------- Don't re-use any sequence symbols as the index symbol in a for loop.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (`x` in x) {}"),
            @"
        warning: for_loop_index
         --> <test>:1:6
          |
        1 | for (`x` in x) {}
          |      -------- Don't re-use any sequence symbols as the index symbol in a for loop.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (x in `x`) {}"),
            @"
        warning: for_loop_index
         --> <test>:1:6
          |
        1 | for (x in `x`) {}
          |      -------- Don't re-use any sequence symbols as the index symbol in a for loop.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (数据 in (数据)) {}"),
            @"
        warning: for_loop_index
         --> <test>:1:6
          |
        1 | for (数据 in (数据)) {}
          |      -------------- Don't re-use any sequence symbols as the index symbol in a for loop.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (x in foo(function(x) x, x)) {}"),
            @"
        warning: for_loop_index
         --> <test>:1:6
          |
        1 | for (x in foo(function(x) x, x)) {}
          |      -------------------------- Don't re-use any sequence symbols as the index symbol in a for loop.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (x in foo(\nx\n)) { TRUE }"),
            @"
        warning: for_loop_index
         --> <test>:1:6
          |
        1 |   for (x in foo(
          |  ______-
        2 | | x
        3 | | )) { TRUE }
          | |_- Don't re-use any sequence symbols as the index symbol in a for loop.
          |
        Found 1 error.
        "
        );
    }
}
