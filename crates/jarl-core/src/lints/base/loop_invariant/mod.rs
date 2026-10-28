pub(crate) mod loop_invariant;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "loop_invariant", None)
    }

    #[test]
    fn test_lint_loop_invariant() {
        assert_snapshot!(
            snapshot_lint("
for (i in seq_along(files)) {
  n <- nrow(data)
  out[[i]] <- process(files[i], n)
}
"),
            @"
        warning: loop_invariant
         --> <test>:3:3
          |
        3 |   n <- nrow(data)
          |   --------------- `n` is computed identically at every iteration of the loop.
          |
          = help: Move this assignment before the loop.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint("
for (i in 1:10) {
  x <- base::paste0(prefix, '_')
  y <- toupper(x)
  out[i] <- paste0(y, i)
}
"),
            @"
        warning: loop_invariant
         --> <test>:3:3
          |
        3 |   x <- base::paste0(prefix, '_')
          |   ------------------------------ `x` is computed identically at every iteration of the loop.
          |
          = help: Move this assignment before the loop.
        warning: loop_invariant
         --> <test>:4:3
          |
        4 |   y <- toupper(x)
          |   --------------- `y` is computed identically at every iteration of the loop.
          |
          = help: Move this assignment before the loop.
        Found 2 errors.
        "
        );
        assert_snapshot!(
            snapshot_lint("for (i in 1:10) n <- length(y)"),
            @"
        warning: loop_invariant
         --> <test>:1:17
          |
        1 | for (i in 1:10) n <- length(y)
          |                 -------------- `n` is computed identically at every iteration of the loop.
          |
          = help: Move this assignment before the loop.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_loop_invariant_nested_loops() {
        // `k` depends on `i` but not on `j`, so it can move out of the inner loop.
        assert_snapshot!(
            snapshot_lint("
for (i in 1:3) {
  for (j in 1:3) {
    k <- length(x[[i]])
    out[i, j] <- k + j
  }
}
"),
            @"
        warning: loop_invariant
         --> <test>:4:5
          |
        4 |     k <- length(x[[i]])
          |     ------------------- `k` is computed identically at every iteration of the loop.
          |
          = help: Move this assignment before the loop.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_loop_invariant_in_function() {
        assert_snapshot!(
            snapshot_lint("
f <- function(data, threshold) {
  for (i in seq_len(10)) {
    idx <- which(data$x > threshold)
    print(idx + i)
  }
}
"),
            @"
        warning: loop_invariant
         --> <test>:4:5
          |
        4 |     idx <- which(data$x > threshold)
          |     -------------------------------- `idx` is computed identically at every iteration of the loop.
          |
          = help: Move this assignment before the loop.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_no_lint_loop_invariant_depends_on_loop() {
        expect_no_lint(
            "for (i in 1:3) { n <- nchar(x[i]) }",
            "loop_invariant",
            None,
        );
        expect_no_lint(
            "for (i in 1:3) { x <- df[i, ]; n <- nrow(x) }",
            "loop_invariant",
            None,
        );
        // `a` is defined below `b`, so `b` reads the previous iteration's `a`.
        expect_no_lint(
            "for (i in 1:3) { b <- a * nrow(df); a <- nrow(df) + i }",
            "loop_invariant",
            None,
        );
    }

    #[test]
    fn test_no_lint_loop_invariant_mutated_in_loop() {
        expect_no_lint(
            "for (i in 1:3) { n <- length(res); res[[i]] <- 1 }",
            "loop_invariant",
            None,
        );
        expect_no_lint(
            "for (i in 1:3) { n <- length(res$a); res$a[i] <- 1 }",
            "loop_invariant",
            None,
        );
        expect_no_lint(
            "for (i in 1:3) { nms <- toupper(names(x)); names(x)[i] <- 'a' }",
            "loop_invariant",
            None,
        );
        expect_no_lint(
            "for (i in 1:3) { obj$add(i); n <- length(obj$items) }",
            "loop_invariant",
            None,
        );
    }

    #[test]
    fn test_no_lint_loop_invariant_untracked_state() {
        expect_no_lint(
            "for (i in 1:3) { n <- length(y); y <<- c(y, i) }",
            "loop_invariant",
            None,
        );
        expect_no_lint(
            "for (i in 1:3) { n <- length(y); assign('y', i) }",
            "loop_invariant",
            None,
        );
        expect_no_lint(
            "for (i in 1:3) { n <- nrow(dt); dt[, a := i] }",
            "loop_invariant",
            None,
        );
        expect_no_lint(
            "for (i in 1:3) { n <- nrow(dt); setnames(dt, 'a', 'b') }",
            "loop_invariant",
            None,
        );
        expect_no_lint(
            "for (i in 1:3) { x <- sum(y); eval(expr) }",
            "loop_invariant",
            None,
        );
    }

    #[test]
    fn test_no_lint_loop_invariant_impure_value() {
        expect_no_lint("for (i in 1:3) { x <- rnorm(n) }", "loop_invariant", None);
        expect_no_lint(
            "for (i in 1:3) { x <- custom_fn(n) }",
            "loop_invariant",
            None,
        );
        expect_no_lint(
            "for (i in 1:3) { x <- length(foo(n)) }",
            "loop_invariant",
            None,
        );
        expect_no_lint(
            "for (i in 1:3) { x <- length(obj$get()) }",
            "loop_invariant",
            None,
        );
        expect_no_lint(
            "for (i in 1:3) { x <- pkg::length(n) }",
            "loop_invariant",
            None,
        );
        expect_no_lint(
            "length <- function(x) 1; for (i in 1:3) { x <- length(n) }",
            "loop_invariant",
            None,
        );
        expect_no_lint(
            "for (i in 1:3) { f <- function() length(y) }",
            "loop_invariant",
            None,
        );
        expect_no_lint("for (i in 1:3) { f <- c(y ~ x) }", "loop_invariant", None);
        expect_no_lint(
            "for (i in 1:3) { x <- y %>% length() }",
            "loop_invariant",
            None,
        );
    }

    #[test]
    fn test_no_lint_loop_invariant_not_hoistable() {
        // Read before the assignment.
        expect_no_lint(
            "for (i in 1:3) { print(n); n <- length(y) }",
            "loop_invariant",
            None,
        );
        // Reads its own previous value.
        expect_no_lint(
            "for (i in 1:3) { n <- n + length(y) }",
            "loop_invariant",
            None,
        );
        // Defined twice in the loop.
        expect_no_lint(
            "for (i in 1:3) { n <- length(y); n <- n + i }",
            "loop_invariant",
            None,
        );
        // Used in the loop sequence, which is evaluated before the body.
        expect_no_lint(
            "for (i in seq_len(n)) { n <- length(y) }",
            "loop_invariant",
            None,
        );
        // Not at the top level of the loop body.
        expect_no_lint(
            "for (i in 1:3) { if (cond) n <- length(y) }",
            "loop_invariant",
            None,
        );
    }

    #[test]
    fn test_no_lint_loop_invariant_cheap_value() {
        expect_no_lint(
            "for (i in 1:3) { x <- c('a', 'b') }",
            "loop_invariant",
            None,
        );
        expect_no_lint("for (i in 1:3) { x <- y }", "loop_invariant", None);
        expect_no_lint("for (i in 1:3) { x <- y + 1 }", "loop_invariant", None);
    }

    #[test]
    fn test_no_lint_loop_invariant_quoted() {
        expect_no_lint(
            "expression(for (i in 1:3) { n <- length(y) })",
            "loop_invariant",
            None,
        );
    }
}
