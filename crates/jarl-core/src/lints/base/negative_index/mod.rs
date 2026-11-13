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
    fn test_no_lint_negative_index_guarded() {
        for code in [
            "if (length(idx) > 0) x[-idx]",
            "if (base::length(x = idx) >= 1L) x[-idx]",
            "if (0 < length(idx)) x[-idx]",
            "if (length(idx) != 0) x[-idx]",
            "if (!((length(idx) == 0))) x[-((idx))]",
            "if (length(idx) == 0) NULL else x[-idx]",
            "if (length(idx) > 0 && verbose) x[-idx]",
            "if (verbose && length(idx) > 0) x[-idx]",
            "if (length(idx) == 0 || verbose) NULL else x[-idx]",
            "if (length(idx) > 0 || length(idx) >= 1) x[-idx]",
            "if (n > 1) x[-n]",
            "if (n <= 1) NULL else x[-n]",
            "if (length(idx) > -1) NULL else x[-idx]",
            "if (length(idx) > +0) x[-idx]",
            "if (length(idx) > 0) { x[-idx]; idx <- NULL }",
            "if (length(idx) > 0) { f <- function() idx <- NULL; x[-idx] }",
            "if (length(idx) > 0) idx <- x[-idx]",
            "for (i in 1:3) { if (length(idx) > 0) x[-idx]; idx <- f() }",
            "if (length(idx) > 0) { other <- NULL; x[-idx] }",
            "if (length(`my idx`) > 0) x[-`my idx`]",
            "function() { if (length(idx) > 0) x[-idx] }",
            // An outer guard also protects uses inside nested conditions.
            r#"if (n > 1) {
  if (verbose) {
    new_groups <- glue_collapse(
      paste0("'", group_vars[-n], "'"),
      sep = ", "
    )
    summarise_inform("has grouped output by {new_groups}")
  }
  out <- grouped_df(out, group_vars[-n], group_by_drop_default(.data))
}"#,
        ] {
            expect_no_lint(code, "negative_index", None);
        }
    }

    #[test]
    fn test_no_lint_negative_index_while_and_length() {
        for code in [
            "if (length(idx)) x[-idx]",
            "if (!length(idx)) NULL else x[-idx]",
            "if ((base::length(x = (idx))) && verbose) x[-idx]",
            "while (length(idx)) x[-idx]",
            "while (length(idx) > 0) { x[-idx]; idx <- f() }",
            "while (n > 1) { if (verbose) x[-n]; n <- f() }",
            "while (length(idx)) { for (i in 1:3) x[-idx]; idx <- f() }",
        ] {
            expect_no_lint(code, "negative_index", None);
        }
    }

    #[test]
    fn test_no_lint_negative_index_early_exit() {
        for code in [
            "function(x, idx) { if (length(idx) == 0) return(x); x[-idx] }",
            "function(x, idx) { if (!length(idx)) { message('empty'); return(x) }; x[-idx] }",
            "if (length(idx) == 0) stop('empty'); x[-idx]",
            "if (length(idx) == 0) base::stop('empty'); x[-idx]",
            "if (length(idx)) print(idx) else stop('empty'); x[-idx]",
            "if (!length(idx)) stop('empty') else print(idx); x[-idx]",
            "function(x, idx) { if (!length(idx)) return(x); if (verbose) print(x[-idx]) }",
            "for (i in 1:3) { if (!length(idx)) next; x[-idx]; idx <- f() }",
            "repeat { if (!length(idx)) break; x[-idx]; idx <- f() }",
            "function(x, idx) { if (!length(idx)) { if (verbose) return(x) else stop('empty') }; x[-idx] }",
            // Writes on the exiting path do not affect the surviving path.
            "function(x, idx) { if (!length(idx)) { idx <- f(); return(x) }; x[-idx] }",
            // A fresh check restores protection after a write.
            "if (!length(idx)) stop('empty'); idx <- f(); if (!length(idx)) stop('empty'); x[-idx]",
            "function(x, idx) { if (!length(idx)) return(x); idx <- x[-idx] }",
            "function(x, idx) { if (!length(idx)) return(x); f <- function() idx <- NULL; x[-idx] }",
        ] {
            expect_no_lint(code, "negative_index", None);
        }
    }

    #[test]
    fn test_no_lint_negative_index_any_and_stopifnot() {
        for code in [
            "if (any(idx > 0)) x[-idx]",
            "if (base::any((0 < idx), na.rm = TRUE)) x[-idx]",
            "if (any(na.rm = FALSE, idx > 0) && verbose) x[-idx]",
            "if (!any(idx > 0)) NULL else x[-idx]",
            "while (any(idx > 0)) { x[-idx]; idx <- f() }",
            "if (!any(idx > 0)) stop('empty'); x[-idx]",
            "stopifnot(length(idx) > 0); x[-idx]",
            "base::stopifnot(0 < base::length(idx)); x[-idx]",
            "stopifnot(verbose, nonempty = length(idx) != 0); x[-idx]",
            "stopifnot(!(length(idx) == 0)); x[-idx]",
            "stopifnot(any(idx > 0)); x[-idx]",
            "stopifnot(length(idx) > 0 && verbose); x[-idx]",
            "stopifnot(idx > 0 && TRUE); x[-idx]",
            "stopifnot(length(idx) > 0); if (verbose) x[-idx]",
            "repeat { stopifnot(length(idx) > 0); x[-idx]; idx <- f() }",
            "stopifnot(length(idx) > 0); idx <- x[-idx]",
        ] {
            expect_no_lint(code, "negative_index", None);
        }
    }

    #[test]
    fn test_lint_negative_index_any_and_stopifnot() {
        assert_snapshot!(snapshot_lint("if (any(idx > 0)) NULL else x[-idx]"), @"
        warning: negative_index
         --> <test>:1:31
          |
        1 | if (any(idx > 0)) NULL else x[-idx]
          |                               ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
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
        assert_snapshot!(snapshot_lint("if (other::any(idx > 0)) x[-idx]"), @"
        warning: negative_index
         --> <test>:1:28
          |
        1 | if (other::any(idx > 0)) x[-idx]
          |                            ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint(
            "if (any(idx > 0)) {
  idx <- f()
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
        // Assertions accept empty logical vectors; a scalar length check is needed.
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
        assert_snapshot!(snapshot_lint("stopifnot(length(idx) >= 0); x[-idx]"), @"
        warning: negative_index
         --> <test>:1:32
          |
        1 | stopifnot(length(idx) >= 0); x[-idx]
          |                                ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("other::stopifnot(length(idx) > 0); x[-idx]"), @"
        warning: negative_index
         --> <test>:1:38
          |
        1 | other::stopifnot(length(idx) > 0); x[-idx]
          |                                      ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // Assert only facts from ordinary arguments evaluated in this environment.
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
        assert_snapshot!(snapshot_lint(
            "stopifnot(length(idx) > 0)
idx <- f()
x[-idx]"
        ), @"
        warning: negative_index
         --> <test>:3:3
          |
        3 | x[-idx]
          |   ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
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
        assert_snapshot!(snapshot_lint("if (verbose) stopifnot(length(idx) > 0); x[-idx]"), @"
        warning: negative_index
         --> <test>:1:44
          |
        1 | if (verbose) stopifnot(length(idx) > 0); x[-idx]
          |                                            ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("stopifnot(length(idx) > 0); function() x[-idx]"), @"
        warning: negative_index
         --> <test>:1:42
          |
        1 | stopifnot(length(idx) > 0); function() x[-idx]
          |                                          ---- Non-literal negative indices may produce unexpected results when empty.
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

    #[test]
    fn test_lint_negative_index_while_and_length() {
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
        assert_snapshot!(snapshot_lint("if (length(idx) || verbose) x[-idx]"), @"
        warning: negative_index
         --> <test>:1:31
          |
        1 | if (length(idx) || verbose) x[-idx]
          |                               ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint("while (length(idx) >= 0) x[-idx]"), @"
        warning: negative_index
         --> <test>:1:28
          |
        1 | while (length(idx) >= 0) x[-idx]
          |                            ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint(
            "while (length(idx)) {
  idx <- f()
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
        // The condition has not established its result while it is evaluated.
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
        // An inner loop can reuse a changed index before the while condition runs again.
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
    }

    #[test]
    fn test_lint_negative_index_early_exit() {
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
        assert_snapshot!(snapshot_lint("if (!length(idx)) function() stop('empty'); x[-idx]"), @"
        warning: negative_index
         --> <test>:1:47
          |
        1 | if (!length(idx)) function() stop('empty'); x[-idx]
          |                                               ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // The empty path must exit unconditionally.
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
        assert_snapshot!(snapshot_lint("if (!length(idx)) warning('empty'); x[-idx]"), @"
        warning: negative_index
         --> <test>:1:39
          |
        1 | if (!length(idx)) warning('empty'); x[-idx]
          |                                       ---- Non-literal negative indices may produce unexpected results when empty.
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
        assert_snapshot!(snapshot_lint("if (!length(idx)) other::stop('empty'); x[-idx]"), @"
        warning: negative_index
         --> <test>:1:43
          |
        1 | if (!length(idx)) other::stop('empty'); x[-idx]
          |                                           ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // A guard in another branch need not have run.
        assert_snapshot!(snapshot_lint("if (verbose) { if (!length(idx)) stop('empty') }; x[-idx]"), @"
        warning: negative_index
         --> <test>:1:53
          |
        1 | if (verbose) { if (!length(idx)) stop('empty') }; x[-idx]
          |                                                     ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        assert_snapshot!(snapshot_lint(
            "function(x, idx) {
  if (!length(idx)) return(x)
  idx <- f()
  x[-idx]
}"
        ), @"
        warning: negative_index
         --> <test>:4:5
          |
        4 |   x[-idx]
          |     ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
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
        // Exits caught by another call or confined to another loop do not protect this use.
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
        assert_snapshot!(snapshot_lint(
            "function(x, idx) {
  if (!length(idx)) return(x)
  function() x[-idx]
}"
        ), @"
        warning: negative_index
         --> <test>:3:16
          |
        3 |   function() x[-idx]
          |                ---- Non-literal negative indices may produce unexpected results when empty.
          |
          = help: Handle empty indices explicitly or use a logical index.
        Found 1 error.
        ");
        // A guard outside the loop does not recheck the index on later iterations.
        assert_snapshot!(snapshot_lint(
            "if (!length(idx)) stop('empty')
while (more) {
  x[-idx]
  idx <- f()
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
    fn test_lint_negative_index_guards() {
        // These checks do not rule out an empty index in the selected branch.
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
        assert_snapshot!(snapshot_lint("if (length(idx) > 0) NULL else x[-idx]"), @"
        warning: negative_index
         --> <test>:1:34
          |
        1 | if (length(idx) > 0) NULL else x[-idx]
          |                                  ---- Non-literal negative indices may produce unexpected results when empty.
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

        // Reassignments invalidate the fact established by the condition.
        assert_snapshot!(snapshot_lint(
            "if (length(idx) > 0) {
  idx <- f(idx)
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
        // A later iteration may reach the use with a changed index.
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
        // A function can be called after the checked variable changes.
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
