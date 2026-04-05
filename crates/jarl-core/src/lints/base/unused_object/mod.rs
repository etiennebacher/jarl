pub(crate) mod unused_object;
#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "unused_object", None)
    }

    #[test]
    fn test_no_lint_used_variable() {
        expect_no_lint("x <- 1\nprint(x)", "unused_object", None);
    }

    #[test]
    fn test_no_lint_variable_in_expression() {
        expect_no_lint("x <- 1\ny <- x + 1\nprint(y)", "unused_object", None);
    }

    #[test]
    fn test_no_lint_function_definition() {
        expect_no_lint("f <- function() 1", "unused_object", None);
    }

    #[test]
    fn test_no_lint_function_parameter() {
        expect_no_lint("f <- function(x) 1", "unused_object", None);
    }

    #[test]
    fn test_no_lint_used_in_closure() {
        expect_no_lint(
            "x <- 1\nf <- function() {\n  y <- x + 1\n  y\n}",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_loop_variable() {
        expect_no_lint("for (i in 1:10) print(i)", "unused_object", None);
    }

    #[test]
    fn test_no_lint_if_else_usage() {
        expect_no_lint(
            "x <- 1\nif (TRUE) print(x) else print(x)",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_super_assignment() {
        expect_no_lint("f <- function() { x <<- 1 }", "unused_object", None);
    }

    #[test]
    fn test_no_lint_replacement_function() {
        expect_no_lint(
            "x <- list()\nnames(x) <- 'a'\nprint(x)",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_subset_replacement() {
        expect_no_lint("x <- 1:3\nx[1] <- 10\nprint(x)", "unused_object", None);
    }

    #[test]
    fn test_no_lint_dollar_replacement() {
        expect_no_lint("x <- list()\nx$a <- 1\nprint(x)", "unused_object", None);
    }

    #[test]
    fn test_no_lint_string_interpolation() {
        expect_no_lint("x <- 1\nmessage(\"value is {x}\")", "unused_object", None);
    }

    #[test]
    fn test_no_lint_string_interpolation_expression() {
        expect_no_lint(
            "n <- 10\nmessage(\"{n} items found\")",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_returned_by_function() {
        expect_no_lint("f <- function() {\n  x <- 1\n  x\n}", "unused_object", None);
    }

    #[test]
    fn test_no_lint_used_as_argument() {
        expect_no_lint("x <- 1\nmean(x)", "unused_object", None);
    }

    #[test]
    fn test_no_lint_used_as_named_argument() {
        expect_no_lint("x <- 1\nfoo(value = x)", "unused_object", None);
    }

    #[test]
    fn test_no_lint_self_read_suppression() {
        expect_no_lint("x <- 1\nx <- x + 1\nprint(x)", "unused_object", None);
    }

    #[test]
    fn test_no_lint_pipe() {
        expect_no_lint("x <- 1\nx |> print()", "unused_object", None);
    }

    #[test]
    fn test_no_lint_used_in_condition() {
        expect_no_lint("x <- TRUE\nif (x) print('yes')", "unused_object", None);
    }

    #[test]
    fn test_no_lint_used_in_while() {
        expect_no_lint("x <- TRUE\nwhile (x) { x <- FALSE }", "unused_object", None);
    }

    #[test]
    fn test_no_lint_right_assignment_used() {
        expect_no_lint("1 -> x\nprint(x)", "unused_object", None);
    }

    #[test]
    fn test_no_lint_equals_assignment_used() {
        expect_no_lint("x = 1\nprint(x)", "unused_object", None);
    }

    #[test]
    fn test_no_lint_multiple_all_used() {
        expect_no_lint(
            "x <- 1\ny <- 2\nz <- x + y\nprint(z)",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_used_in_nested_call() {
        expect_no_lint("x <- 1\nprint(mean(x))", "unused_object", None);
    }

    #[test]
    fn test_no_lint_local_scope() {
        expect_no_lint("local({\n  x <- 1\n  print(x)\n})", "unused_object", None);
    }

    #[test]
    fn test_no_lint_with_unresolved_refs_in_function_def_resolved_later() {
        expect_no_lint("f <- function() x\nx <- 1", "unused_object", None);
    }

    #[test]
    fn test_with_on_exit() {
        // no lint when on.exit() refers to objects defined after it's called
        expect_no_lint(
            "
        f <- function() {
            on.exit(print(a))
            a <- 1
            'hi'
        }
        ",
            "unused_object",
            None,
        );

        // See comment in `process_call()`
        expect_no_lint(
            "
        f <- function() {
            foo <- TRUE
            on.exit(
                if (foo) print('bye')
            )
            # <some operation that might error here>
            foo <- FALSE
        }
        ",
            "unused_object",
            None,
        );
        // report when on.exit() doesn't use objects
        assert_snapshot!(
            snapshot_lint("
f <- function() {
    foo <- TRUE
    on.exit(print('bye'))
    foo <- FALSE
}
        "),
            @"
        warning: unused_object
         --> <test>:3:5
          |
        3 |     foo <- TRUE
          |     --- Object `foo` is defined but never used.
          |
        warning: unused_object
         --> <test>:5:5
          |
        5 |     foo <- FALSE
          |     --- Object `foo` is defined but never used.
          |
        Found 2 errors.
        "
        );
    }

    // ---------------------------------------------------------------
    // Lint cases
    // ---------------------------------------------------------------

    #[test]
    fn test_lint_simple_unused() {
        assert_snapshot!(
            snapshot_lint("x <- 1\nprint(y)"),
            @r"
        warning: unused_object
         --> <test>:1:1
          |
        1 | x <- 1
          | - Object `x` is defined but never used.
          |
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint(".x <- 1\nprint(y)"),
            @"
        warning: unused_object
         --> <test>:1:1
          |
        1 | .x <- 1
          | -- Object `.x` is defined but never used.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_unused_after_reassignment() {
        assert_snapshot!(
            snapshot_lint("x <- 1\nx <- 2\nprint(x)"),
            @r"
        warning: unused_object
         --> <test>:1:1
          |
        1 | x <- 1
          | - Object `x` is defined but never used.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_multiple_unused() {
        assert_snapshot!(
            snapshot_lint("x <- 1\ny <- 2"),
            @r"
        warning: unused_object
         --> <test>:1:1
          |
        1 | x <- 1
          | - Object `x` is defined but never used.
          |
        warning: unused_object
         --> <test>:2:1
          |
        2 | y <- 2
          | - Object `y` is defined but never used.
          |
        Found 2 errors.
        "
        );
    }

    #[test]
    fn test_lint_unused_right_assignment() {
        assert_snapshot!(
            snapshot_lint("1 -> x"),
            @r"
        warning: unused_object
         --> <test>:1:6
          |
        1 | 1 -> x
          |      - Object `x` is defined but never used.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_unused_equals_assignment() {
        assert_snapshot!(
            snapshot_lint("x = 1"),
            @r"
        warning: unused_object
         --> <test>:1:1
          |
        1 | x = 1
          | - Object `x` is defined but never used.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_only_one_of_two_used() {
        assert_snapshot!(
            snapshot_lint("x <- 1\ny <- 2\nprint(x)"),
            @r"
        warning: unused_object
         --> <test>:2:1
          |
        2 | y <- 2
          | - Object `y` is defined but never used.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_unused_in_function_body() {
        assert_snapshot!(
            snapshot_lint("f <- function() {\n  x <- 1\n  y <- 2\n  y\n}"),
            @r"
        warning: unused_object
         --> <test>:2:3
          |
        2 |   x <- 1
          |   - Object `x` is defined but never used.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_unused_with_used_neighbor() {
        assert_snapshot!(
            snapshot_lint("a <- 1\nb <- 2\nc <- a + b\nd <- 99"),
            @r"
        warning: unused_object
         --> <test>:3:1
          |
        3 | c <- a + b
          | - Object `c` is defined but never used.
          |
        warning: unused_object
         --> <test>:4:1
          |
        4 | d <- 99
          | - Object `d` is defined but never used.
          |
        Found 2 errors.
        "
        );
    }

    #[test]
    fn test_lint_nse_read_does_not_count() {
        assert_snapshot!(
            snapshot_lint("x <- 1\nquote(x)"),
            @r"
        warning: unused_object
         --> <test>:1:1
          |
        1 | x <- 1
          | - Object `x` is defined but never used.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_with_assignment_pipe() {
        // should lint: re-assigned `x` isn't used
        assert_snapshot!(
            snapshot_lint("
x <- 1:3
x %<>% sum()"
        ),
            @"
        warning: unused_object
         --> <test>:3:1
          |
        3 | x %<>% sum()
          | - Object `x` is defined but never used.
          |
        Found 1 error.
        "
        );
        // shouldn't lint
        assert_snapshot!(
            snapshot_lint("
x <- 1:3
x %<>% sum()
x + 1"
        ),
            @"All checks passed!"
        );
    }

    #[test]
    fn test_assign() {
        // TODO: this should report env
        // shouldn't lint: env is used as argument to assign()
        expect_no_lint(
            "
f <- function() {
  env <- new.env()
  assign('x', 1 + 1, envir = env)
}
f()",
            "unused_object",
            None,
        );
        // shouldn't lint: we return env, which contains x
        expect_no_lint(
            "
f <- function() {
  env <- new.env()
  assign('x', 1 + 1, envir = env)
  env
}
f()",
            "unused_object",
            None,
        );
        // shouldn't lint: we use env outside the function
        expect_no_lint(
            "
env <- new.env()
f <- function() {
  assign('x', 1 + 1, envir = env)
}
f()
env",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_delayed_assign() {
        // TODO: this should report env
        // shouldn't lint: env is used as argument to delayedAssign()
        expect_no_lint(
            "
f <- function() {
  env <- new.env()
  delayedAssign('x', 1 + 1, assign.env = env)
}
f()",
            "unused_object",
            None,
        );
        // shouldn't lint: we return env, which contains x
        expect_no_lint(
            "
f <- function() {
  env <- new.env()
  delayedAssign('x', 1 + 1, assign.env = env)
  env
}
f()",
            "unused_object",
            None,
        );
        // shouldn't lint: we use env outside the function
        expect_no_lint(
            "
env <- new.env()
f <- function() {
  delayedAssign('x', 1 + 1, assign.env = env)
}
f()
env",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_make_active_binding() {
        // TODO: this should report env
        // shouldn't lint: env is used as argument to makeActiveBinding()
        expect_no_lint(
            "
f <- function() {
  env <- new.env()
  makeActiveBinding('x', \\(x) x, env = env)
}
f()",
            "unused_object",
            None,
        );
        // shouldn't lint: we return env, which contains x
        expect_no_lint(
            "
f <- function() {
  env <- new.env()
  makeActiveBinding('x', \\(x) x, env = env)
  env
}
f()",
            "unused_object",
            None,
        );
        // shouldn't lint: we use env outside the function
        expect_no_lint(
            "
env <- new.env()
f <- function() {
  makeActiveBinding('x', \\(x) x, env = env)
}
f()
env",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_dot_dot_prefix_data_table() {
        expect_no_lint(
            "
cols <- 'a'
dt[, ..cols]
",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_shadowing_after_condition() {
        // `x <- 2` wouldn't run if the first condition is true, so `x <- 1`
        // might be used.
        expect_no_lint(
            "
x <- 1
if (runif(1) < 0.5 || (x <- 2)) {
  print(x)
}",
            "unused_object",
            None,
        );
        // `x <- 2` wouldn't run if the first condition is false, so `x <- 1`
        // might be used.
        expect_no_lint(
            "
x <- 1
if (runif(1) < 0.5 && (x <- 2)) {
  1 + 1
}
x",
            "unused_object",
            None,
        );
    }
}
