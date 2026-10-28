pub(crate) mod for_loop_dup_index;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "for_loop_dup_index", None)
    }

    #[test]
    fn test_no_lint_for_loop_dup_index() {
        for code in [
            "for (i in 1:3) {}",
            "for (i in 1:3) {}
for (i in 1:4) {}",
            "for (i in 1:3) {
    for (j in 1:4) {
        for (k in 1:5) {}
    }
}",
            // Define the function outside the loop to avoid nested index reuse.
            "f <- function() {
    for (i in 1:4) {}
}
for (i in 1:3) {
    f()
}",
            "for (`i` in 1:3) {
    for (`j` in 1:4) {}
}",
        ] {
            expect_no_lint(code, "for_loop_dup_index", None);
        }
    }

    #[test]
    fn test_lint_for_loop_dup_index() {
        // Braces and intervening statements do not create a new scope.
        assert_snapshot!(snapshot_lint(
            "for (i in 1:3) {
    print(i)
    {
        for (i in 1:4) {}
    }
}"
        ), @"
        warning: for_loop_dup_index
         --> <test>:4:14
          |
        4 |         for (i in 1:4) {}
          |              -------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        Found 1 error.
        ");

        // Loop bodies do not have to be braced.
        assert_snapshot!(
            snapshot_lint("for (i in 1:3)
    for (i in 1:4)
        i"),
            @"
        warning: for_loop_dup_index
         --> <test>:2:10
          |
        2 |     for (i in 1:4)
          |          -------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        Found 1 error.
        "
        );

        // Look past ancestors with different indices.
        assert_snapshot!(
            snapshot_lint("for (i in 1:3) {
    for (j in 1:4) {
        for (i in 1:5) {}
    }
}"),
            @"
        warning: for_loop_dup_index
         --> <test>:3:14
          |
        3 |         for (i in 1:5) {}
          |              -------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        Found 1 error.
        "
        );

        // Each offending loop gets one diagnostic, even with multiple matches.
        assert_snapshot!(
            snapshot_lint("for (i in 1:3) {
    for (i in 1:4) {
        for (i in 1:5) {}
    }
}"),
            @"
        warning: for_loop_dup_index
         --> <test>:2:10
          |
        2 |     for (i in 1:4) {
          |          -------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        warning: for_loop_dup_index
         --> <test>:3:14
          |
        3 |         for (i in 1:5) {}
          |              -------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        Found 2 errors.
        "
        );

        // Backticks do not change the name, including non-ASCII names.
        assert_snapshot!(
            snapshot_lint("for (`i` in 1:3) {
    for (i in 1:4) {}
}"),
            @"
        warning: for_loop_dup_index
         --> <test>:2:10
          |
        2 |     for (i in 1:4) {}
          |          -------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (数据 in 1:3) {
    for (`数据` in 1:4) {}
}"),
            @"
        warning: for_loop_dup_index
         --> <test>:2:10
          |
        2 |     for (`数据` in 1:4) {}
          |          ------------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        Found 1 error.
        "
        );

        // Reusing an index in a sequence is suspicious even before iteration starts.
        assert_snapshot!(
            snapshot_lint("for (i in {
    for (i in 1:3) {}
    1:2
}) {}"),
            @"
        warning: for_loop_dup_index
         --> <test>:2:10
          |
        2 |     for (i in 1:3) {}
          |          -------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (i in list({
    for (i in 1:3) {}
    1:2
})) {}"),
            @"
        warning: for_loop_dup_index
         --> <test>:2:10
          |
        2 |     for (i in 1:3) {}
          |          -------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        Found 1 error.
        "
        );

        // A loop in a sequence can still overwrite a further enclosing index.
        assert_snapshot!(
            snapshot_lint("for (i in 1:3) {
    for (j in {
        for (i in 1:4) {}
        1:2
    }) {}
}"),
            @"
        warning: for_loop_dup_index
         --> <test>:3:14
          |
        3 |         for (i in 1:4) {}
          |              -------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        Found 1 error.
        "
        );

        // Function definitions do not exempt nested loops from this check.
        // You should define the function outside the loop.
        assert_snapshot!(
            snapshot_lint("for (i in 1:3) {
    f <- function() {
        for (i in 1:4) {}
    }
    f()
}"),
            @"
        warning: for_loop_dup_index
         --> <test>:3:14
          |
        3 |         for (i in 1:4) {}
          |              -------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (i in 1:3) {
    (\\() {
        for (i in 1:4) {}
    })()
}"),
            @"
        warning: for_loop_dup_index
         --> <test>:3:14
          |
        3 |         for (i in 1:4) {}
          |              -------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (i in 1:3) {
    f <- function() {
        for (i in 1:4) {
            for (i in 1:5) {}
        }
    }
}"),
            @"
        warning: for_loop_dup_index
         --> <test>:3:14
          |
        3 |         for (i in 1:4) {
          |              -------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        warning: for_loop_dup_index
         --> <test>:4:18
          |
        4 |             for (i in 1:5) {}
          |                  -------- This index variable is already used in a parent `for` loop.
          |
          = help: Rename this index variable to avoid unexpected results.
        Found 2 errors.
        "
        );
    }
}
