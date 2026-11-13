pub(crate) mod negative_index;
mod nonempty_guard;

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
            // Literals include nested parentheses and literal-only c() calls.
            "x[((-((1))))]",
            "x[-2L]",
            "x[-1i]",
            "x[-Inf]",
            "x[-NaN]",
            "x[-TRUE]",
            "x[-NULL]",
            "x[-\"text\"]",
            "x[c(-1, c(-2, 3))]",
            // Logical indices cannot pass negative values through.
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
            // Known calls consume negative arguments without returning negative indices.
            "x[which(-idx)]",
            "x[base::which(-idx)]",
            "x[1 + order(-idx)]",
            "x[base::order(-idx)]",
            "x[between(x, -1, 1)]",
            "x[dplyr::between(x, -lower, upper)]",
            "x[data.table::between(x, -1, 1)]",
            // Only unary minus contributing to a [] index is relevant.
            "x[, ]",
            "x[1 - idx]",
            "x[+idx]",
            "x[\"-idx\"]",
            "x[`-idx`]",
            "x[ # -idx\n idx]",
            "-x[idx]",
            "x[[-idx]]",
            "x[y[[-idx]]]",
            "x[f(function() -idx)]",
            "# jarl-ignore negative_index: index is known to be nonempty\nx[-idx]",
        ] {
            expect_no_lint(code, "negative_index", None);
        }
    }

    #[test]
    fn test_no_lint_negative_index_conditions() {
        for code in [
            // Length checks, scalar comparisons, and the selected branch establish nonemptiness.
            "if (length(idx) > 0) x[-idx]",
            "if (base::length(x = idx) >= 1L) x[-idx]",
            "if (0 < length(idx)) x[-idx]",
            "if (length(idx) != 0) x[-idx]",
            "if (!((length(idx) == 0))) x[-((idx))]",
            "if (length(idx) == 0) NULL else x[-idx]",
            "if (n <= 1) NULL else x[-n]",
            "if (length(idx) > -1) NULL else x[-idx]",
            "if (length(idx) > +0) x[-idx]",
            // One conjunct suffices, but both alternatives must exclude an empty index.
            "if (length(idx) > 0 && verbose) x[-idx]",
            "if (verbose && length(idx) > 0) x[-idx]",
            "if (length(idx) == 0 || verbose) NULL else x[-idx]",
            "if (length(idx) > 0 || length(idx) >= 1) x[-idx]",
            // Direct length checks and enclosing branches also protect nested uses.
            "if (length(idx)) x[-idx]",
            "if (!length(idx)) NULL else x[-idx]",
            "if (n > 1) {
  if (verbose) print(group_vars[-n])
  grouped_df(out, group_vars[-n])
}",
            // A while condition is checked again on each iteration.
            "while (length(idx)) x[-idx]",
            "while (length(idx) > 0) { x[-idx]; idx <- f() }",
            "while (length(idx)) { for (i in 1:3) x[-idx]; idx <- f() }",
        ] {
            expect_no_lint(code, "negative_index", None);
        }
    }

    #[test]
    fn test_no_lint_negative_index_numeric_assignment() {
        for code in [
            // Direct assignments accept numeric syntax without evaluating expressions.
            "idx <- 1; x[-idx]",
            "idx = -2L; x[-idx]",
            "1.5 -> idx; x[-idx]",
            "idx <- (+(-((0)))); x[-idx]",
            "idx <- 0x2L; x[-idx]",
            "`my idx` <- 1; x[-`my idx`]",
            // The assignment must execute before the use, including on each loop iteration.
            "if (flag) { idx <- 1; x[-idx] }",
            "idx <- 1; if (flag) x[-idx]",
            "idx <- f(); idx <- 1; x[-idx]",
            "while (running) { idx <- 1; x[-idx]; idx <- integer() }",
        ] {
            expect_no_lint(code, "negative_index", None);
        }
    }

    #[test]
    fn test_no_lint_negative_index_early_exit() {
        for code in [
            // All paths through the empty branch must exit.
            "function(x, idx) { if (length(idx) == 0) return(x); x[-idx] }",
            "if (!length(idx)) base::stop('empty'); x[-idx]",
            "if (length(idx)) print(idx) else stop('empty'); x[-idx]",
            "if (!length(idx)) stop('empty') else print(idx); x[-idx]",
            "for (i in 1:3) { if (!length(idx)) next; x[-idx]; idx <- f() }",
            "repeat { if (!length(idx)) break; x[-idx]; idx <- f() }",
            "function(x, idx) { if (!length(idx)) { if (verbose) return(x) else stop('empty') }; x[-idx] }",
            // Writes on an exiting path do not affect the surviving path.
            "function(x, idx) { if (!length(idx)) { idx <- f(); return(x) }; x[-idx] }",
            // A fresh check restores protection after reassignment.
            "if (!length(idx)) stop('empty'); idx <- f(); if (!length(idx)) stop('empty'); x[-idx]",
        ] {
            expect_no_lint(code, "negative_index", None);
        }
    }

    #[test]
    fn test_no_lint_negative_index_assertions() {
        for code in [
            // A true any() establishes nonemptiness with one data argument.
            "if (any(idx > 0)) x[-idx]",
            "if (base::any((0 < idx), na.rm = TRUE)) x[-idx]",
            "if (any(na.rm = FALSE, idx > 0) && verbose) x[-idx]",
            // Assertions must reject an empty index; named conditions are also supported.
            "stopifnot(length(idx) > 0); x[-idx]",
            "base::stopifnot(0 < base::length(idx)); x[-idx]",
            "stopifnot(verbose, nonempty = length(idx) != 0); x[-idx]",
            "stopifnot(!(length(idx) == 0)); x[-idx]",
            "stopifnot(any(idx > 0)); x[-idx]",
            "stopifnot(idx > 0 && TRUE); x[-idx]",
        ] {
            expect_no_lint(code, "negative_index", None);
        }
    }

    #[test]
    fn test_no_lint_negative_index_preserved_guards() {
        for code in [
            // Unrelated writes and deferred function bodies do not invalidate a guard.
            "if (length(idx) > 0) { other <- NULL; x[-idx] }",
            "if (length(idx) > 0) { f <- function() idx <- NULL; x[-idx] }",
            // Writes after a read do not affect it until a subsequent loop iteration.
            "if (length(idx) > 0) { x[-idx]; idx <- NULL }",
            "idx <- 1; idx <- x[-idx]",
            // Preceding guards apply inside nested statements in the same function.
            "function(x, idx) { if (!length(idx)) return(x); if (verbose) print(x[-idx]) }",
            "repeat { stopifnot(length(idx) > 0); x[-idx]; idx <- f() }",
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
    }

    #[test]
    fn test_lint_negative_index_conditions() {
        // The selected branch must exclude empty indices for this variable.
        assert_snapshot!(snapshot_lint("if (length(idx) >= 0) x[-idx]"), @"
        warning: negative_index
         --> <test>:1:25
          |
        1 | if (length(idx) >= 0) x[-idx]
          |                         ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("if (length(idx) > 0 || verbose) x[-idx]"), @"
        warning: negative_index
         --> <test>:1:35
          |
        1 | if (length(idx) > 0 || verbose) x[-idx]
          |                                   ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("if (length(idx) > 0 && verbose) NULL else x[-idx]"), @"
        warning: negative_index
         --> <test>:1:45
          |
        1 | if (length(idx) > 0 && verbose) NULL else x[-idx]
          |                                             ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("if (length(other) > 0) x[-idx]"), @"
        warning: negative_index
         --> <test>:1:26
          |
        1 | if (length(other) > 0) x[-idx]
          |                          ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("if (other::length(idx) > 0) x[-idx]"), @"
        warning: negative_index
         --> <test>:1:31
          |
        1 | if (other::length(idx) > 0) x[-idx]
          |                               ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("if (n > 1) x[-n * idx]"), @"
        warning: negative_index
         --> <test>:1:14
          |
        1 | if (n > 1) x[-n * idx]
          |              -------- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // A false length check and uses in the condition itself are not protected.
        assert_snapshot!(snapshot_lint("if (length(idx)) NULL else x[-idx]"), @"
        warning: negative_index
         --> <test>:1:30
          |
        1 | if (length(idx)) NULL else x[-idx]
          |                              ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("while (length(idx) > length(x[-idx])) {}"), @"
        warning: negative_index
         --> <test>:1:31
          |
        1 | while (length(idx) > length(x[-idx])) {}
          |                               ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
    }

    #[test]
    fn test_lint_negative_index_numeric_assignment() {
        // A conditional assignment need not execute.
        assert_snapshot!(snapshot_lint("if (flag) idx <- 1; x[-idx]"), @"
        warning: negative_index
         --> <test>:1:23
          |
        1 | if (flag) idx <- 1; x[-idx]
          |                       ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // Only direct local assignments of numeric literals establish nonemptiness.
        assert_snapshot!(snapshot_lint("idx[1] <- 1; x[-idx]"), @"
        warning: negative_index
         --> <test>:1:16
          |
        1 | idx[1] <- 1; x[-idx]
          |                ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("idx <<- 1; x[-idx]"), @"
        warning: negative_index
         --> <test>:1:14
          |
        1 | idx <<- 1; x[-idx]
          |              ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("1 ->> idx; x[-idx]"), @"
        warning: negative_index
         --> <test>:1:14
          |
        1 | 1 ->> idx; x[-idx]
          |              ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("idx <- length(x); x[-idx]"), @"
        warning: negative_index
         --> <test>:1:21
          |
        1 | idx <- length(x); x[-idx]
          |                     ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("idx <- 1 + 1; x[-idx]"), @"
        warning: negative_index
         --> <test>:1:17
          |
        1 | idx <- 1 + 1; x[-idx]
          |                 ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
    }

    #[test]
    fn test_lint_negative_index_invalidated_guards() {
        // Reassignment invalidates earlier evidence, including from an enclosing block.
        assert_snapshot!(snapshot_lint("idx <- 1; idx <- get_indices(); x[-idx]"), @"
        warning: negative_index
         --> <test>:1:35
          |
        1 | idx <- 1; idx <- get_indices(); x[-idx]
          |                                   ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("idx <- 1; if (flag) { idx <- NULL; x[-idx] }"), @"
        warning: negative_index
         --> <test>:1:38
          |
        1 | idx <- 1; if (flag) { idx <- NULL; x[-idx] }
          |                                      ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // Rightward and replacement assignments also modify the index.
        assert_snapshot!(snapshot_lint(
            "if (length(idx) > 0) {
  NULL -> idx
  x[-idx]
}"
        ), @"
        warning: negative_index
         --> <test>:3:5
          |
        3 |   x[-idx]
          |     ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint(
            "if (length(idx) > 0) {
  length(idx) <- 0
  x[-idx]
}"
        ), @"
        warning: negative_index
         --> <test>:3:5
          |
        3 |   x[-idx]
          |     ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint(
            "if (length(idx) > 0) {
  idx[1] <- NULL
  x[-idx]
}"
        ), @"
        warning: negative_index
         --> <test>:3:5
          |
        3 |   x[-idx]
          |     ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // Writes while evaluating a condition invalidate its evidence.
        assert_snapshot!(snapshot_lint("if (length(idx) > 0 && { idx <- f(); TRUE }) x[-idx]"), @"
        warning: negative_index
         --> <test>:1:48
          |
        1 | if (length(idx) > 0 && { idx <- f(); TRUE }) x[-idx]
          |                                                ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // A loop variable or a later iteration can change the index.
        assert_snapshot!(snapshot_lint(
            "if (length(idx) > 0) {
  for (idx in values) x[-idx]
}"
        ), @"
        warning: negative_index
         --> <test>:2:25
          |
        2 |   for (idx in values) x[-idx]
          |                         ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint(
            "if (length(idx) > 0) {
  while (more) {
    x[-idx]
    idx <- f()
  }
}"
        ), @"
        warning: negative_index
         --> <test>:3:7
          |
        3 |     x[-idx]
          |       ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // Preceding assignments must also survive subsequent loop iterations.
        assert_snapshot!(snapshot_lint("idx <- 1; while (running) { x[-idx]; idx <- integer() }"), @"
        warning: negative_index
         --> <test>:1:31
          |
        1 | idx <- 1; while (running) { x[-idx]; idx <- integer() }
          |                               ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // An inner loop can invalidate the outer while condition.
        assert_snapshot!(snapshot_lint(
            "while (length(idx)) {
  for (i in 1:3) {
    x[-idx]
    idx <- f()
  }
}"
        ), @"
        warning: negative_index
         --> <test>:3:7
          |
        3 |     x[-idx]
          |       ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // A deferred function can run after the checked value changes.
        assert_snapshot!(snapshot_lint("if (length(idx) > 0) function() x[-idx]"), @"
        warning: negative_index
         --> <test>:1:35
          |
        1 | if (length(idx) > 0) function() x[-idx]
          |                                   ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
    }

    #[test]
    fn test_lint_negative_index_early_exit() {
        // The exit condition must preserve the index and reject the empty case.
        assert_snapshot!(snapshot_lint(
            "if (!length(idx) || { idx <- f(); FALSE }) stop('empty')
x[-idx]"
        ), @"
        warning: negative_index
         --> <test>:2:3
          |
        2 | x[-idx]
          |   ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("if (length(idx)) stop('nonempty'); x[-idx]"), @"
        warning: negative_index
         --> <test>:1:38
          |
        1 | if (length(idx)) stop('nonempty'); x[-idx]
          |                                      ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // Conditional exits must cover both branches.
        assert_snapshot!(snapshot_lint(
            "function(x, idx) {
  if (!length(idx)) {
    if (verbose) return(x) else warning('empty')
  }
  x[-idx]
}"
        ), @"
        warning: negative_index
         --> <test>:5:5
          |
        5 |   x[-idx]
          |     ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint(
            "function(x, idx) {
  if (!length(idx)) {
    if (verbose) return(x)
  }
  x[-idx]
}"
        ), @"
        warning: negative_index
         --> <test>:5:5
          |
        5 |   x[-idx]
          |     ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // The surviving branch must preserve the index.
        assert_snapshot!(snapshot_lint("if (length(idx)) idx <- f() else stop('empty'); x[-idx]"), @"
        warning: negative_index
         --> <test>:1:51
          |
        1 | if (length(idx)) idx <- f() else stop('empty'); x[-idx]
          |                                                   ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("if (!length(idx)) stop('empty') else idx <- f(); x[-idx]"), @"
        warning: negative_index
         --> <test>:1:52
          |
        1 | if (!length(idx)) stop('empty') else idx <- f(); x[-idx]
          |                                                    ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // Caught errors and exits confined to another loop do not protect following code.
        assert_snapshot!(snapshot_lint("if (!length(idx)) tryCatch(stop('empty'), error = identity); x[-idx]"), @"
        warning: negative_index
         --> <test>:1:64
          |
        1 | if (!length(idx)) tryCatch(stop('empty'), error = identity); x[-idx]
          |                                                                ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("if (!length(idx)) repeat { break }; x[-idx]"), @"
        warning: negative_index
         --> <test>:1:39
          |
        1 | if (!length(idx)) repeat { break }; x[-idx]
          |                                       ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // A check after the use cannot protect it.
        assert_snapshot!(snapshot_lint("x[-idx]; if (!length(idx)) stop('empty')"), @"
        warning: negative_index
         --> <test>:1:3
          |
        1 | x[-idx]; if (!length(idx)) stop('empty')
          |   ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
    }

    #[test]
    fn test_lint_negative_index_assertions() {
        // Extra any() data or all() can succeed even when this index is empty.
        assert_snapshot!(snapshot_lint("if (any(idx > 0, TRUE)) x[-idx]"), @"
        warning: negative_index
         --> <test>:1:27
          |
        1 | if (any(idx > 0, TRUE)) x[-idx]
          |                           ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("if (all(idx > 0)) x[-idx]"), @"
        warning: negative_index
         --> <test>:1:21
          |
        1 | if (all(idx > 0)) x[-idx]
          |                     ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // stopifnot() accepts empty comparisons, including their negation.
        assert_snapshot!(snapshot_lint("stopifnot(idx > 0); x[-idx]"), @"
        warning: negative_index
         --> <test>:1:23
          |
        1 | stopifnot(idx > 0); x[-idx]
          |                       ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("stopifnot(!(idx <= 0)); x[-idx]"), @"
        warning: negative_index
         --> <test>:1:27
          |
        1 | stopifnot(!(idx <= 0)); x[-idx]
          |                           ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("stopifnot(idx > 0 || TRUE); x[-idx]"), @"
        warning: negative_index
         --> <test>:1:31
          |
        1 | stopifnot(idx > 0 || TRUE); x[-idx]
          |                               ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // Special evaluation arguments are not ordinary assertions.
        assert_snapshot!(snapshot_lint("stopifnot(exprs = length(idx) > 0); x[-idx]"), @"
        warning: negative_index
         --> <test>:1:39
          |
        1 | stopifnot(exprs = length(idx) > 0); x[-idx]
          |                                       ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // An assertion must preserve the index and finish before the use.
        assert_snapshot!(snapshot_lint("stopifnot(length(idx) > 0, { idx <- f(); TRUE }); x[-idx]"), @"
        warning: negative_index
         --> <test>:1:53
          |
        1 | stopifnot(length(idx) > 0, { idx <- f(); TRUE }); x[-idx]
          |                                                     ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("stopifnot(length(idx) > 0, length(x[-idx]) > 0)"), @"
        warning: negative_index
         --> <test>:1:37
          |
        1 | stopifnot(length(idx) > 0, length(x[-idx]) > 0)
          |                                     ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
    }
}
