pub(crate) mod negative_index;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "negative_index", None)
    }

    #[test]
    fn test_no_lint_negative_index() {
        for code in [
            "x[-1]",
            "x[-2L]",
            "x[-1.5]",
            "x[-1e1]",
            "x[-0x2L]",
            "x[-1i]",
            "x[-Inf]",
            "x[-NaN]",
            "x[-TRUE]",
            "x[-NULL]",
            "x[-\"text\"]",
            "x[c(-1, -2)]",
            "x[c(-1, 2)]",
            "x[c(-1, c(-2))]",
            // Comparisons and logical operators turn their operands into logical indices.
            "x[idx == -1]",
            "x[idx != -1]",
            "x[idx < -limit]",
            "x[idx <= -limit]",
            "x[idx > -limit]",
            "x[idx >= -limit]",
            "x[!(-idx)]",
            "x[keep & -idx]",
            "x[keep && -idx]",
            "x[keep | -idx]",
            "x[keep || -idx]",
            "x[(idx > -1) & !(idx < -limit)]",
            // Position-generating calls do not pass negative arguments through.
            "x[which(idx < -1)]",
            "x[order(-idx)]",
            "x[base::which(-idx)]",
            "x[base::order(-idx)]",
            "x[1 + order(-idx)]",
            // `between()` produces logical indices, like comparisons.
            "x[between(x, -1, 1)]",
            "x[dplyr::between(x, -lower, upper)]",
            "x[data.table::between(x, -1, 1)]",
            "x[((-((1))))]",
            "x[-1, idx]",
            "x[]",
            "x[, ]",
            "x[idx]",
            "x[1 - idx]",
            "x[+idx]",
            "x[\"-idx\"]",
            "x[`-idx`]",
            "x[ # -idx\n idx]",
            "x[[-idx]]",
            "-x[idx]",
            "x[f(function() -idx)]",
            "x[y[-1]]",
            "x[y[[-idx]]]",
            "x[-1] <- value",
            "# jarl-ignore negative_index: index is known to be nonempty\nx[-idx]",
        ] {
            expect_no_lint(code, "negative_index", None);
        }
    }

    #[test]
    fn test_lint_negative_index() {
        assert_snapshot!(snapshot_lint("x[-idx]"), @"
        warning: negative_index
         --> <test>:1:3
          |
        1 | x[-idx]
          |   ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[-which(x == 99)]"), @"
        warning: negative_index
         --> <test>:1:3
          |
        1 | x[-which(x == 99)]
          |   --------------- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[-base::order(idx)]"), @"
        warning: negative_index
         --> <test>:1:3
          |
        1 | x[-base::order(idx)]
          |   ----------------- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[-(idx > -1)]"), @"
        warning: negative_index
         --> <test>:1:3
          |
        1 | x[-(idx > -1)]
          |   ----------- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[-between(x, -1, 1)]"), @"
        warning: negative_index
         --> <test>:1:3
          |
        1 | x[-between(x, -1, 1)]
          |   ------------------ Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // Unknown functions and other namespaces may pass negative values through.
        assert_snapshot!(snapshot_lint("x[f(-idx)]"), @"
        warning: negative_index
         --> <test>:1:3
          |
        1 | x[f(-idx)]
          |   ------- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[pkg::order(-idx)]"), @"
        warning: negative_index
         --> <test>:1:3
          |
        1 | x[pkg::order(-idx)]
          |   ---------------- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[pkg::between(x, -1, 1)]"), @"
        warning: negative_index
         --> <test>:1:3
          |
        1 | x[pkg::between(x, -1, 1)]
          |   ---------------------- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[-1 * idx]"), @"
        warning: negative_index
         --> <test>:1:3
          |
        1 | x[-1 * idx]
          |   -------- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[c(-1, -idx)]"), @"
        warning: negative_index
         --> <test>:1:3
          |
        1 | x[c(-1, -idx)]
          |   ----------- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[c(-1, idx)]"), @"
        warning: negative_index
         --> <test>:1:3
          |
        1 | x[c(-1, idx)]
          |   ---------- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[-2 * 3]"), @"
        warning: negative_index
         --> <test>:1:3
          |
        1 | x[-2 * 3]
          |   ------ Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[-((idx + 1))]"), @"
        warning: negative_index
         --> <test>:1:3
          |
        1 | x[-((idx + 1))]
          |   ------------ Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[-rows, -cols, drop = FALSE]"), @"
        warning: negative_index
         --> <test>:1:3
          |
        1 | x[-rows, -cols, drop = FALSE]
          |   ----- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        warning: negative_index
         --> <test>:1:10
          |
        1 | x[-rows, -cols, drop = FALSE]
          |          ----- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 2 errors.
        ");
        assert_snapshot!(snapshot_lint("x[y[-idx]]"), @"
        warning: negative_index
         --> <test>:1:5
          |
        1 | x[y[-idx]]
          |     ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[(-idx)[1]]"), @"
        warning: negative_index
         --> <test>:1:3
          |
        1 | x[(-idx)[1]]
          |   --------- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[-idx] <- value"), @"
        warning: negative_index
         --> <test>:1:3
          |
        1 | x[-idx] <- value
          |   ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("x[\n  # Exclude matching elements.\n  -which(x == 99)\n]"), @"
        warning: negative_index
         --> <test>:3:3
          |
        3 |   -which(x == 99)
          |   --------------- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
    }
}
