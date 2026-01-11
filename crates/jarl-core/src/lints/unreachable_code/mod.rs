pub(crate) mod cfg;
pub(crate) mod unreachable_code;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    /// Format diagnostics for snapshot testing
    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "unreachable_code", None)
    }

    #[test]
    fn test_no_unreachable_simple_function() {
        let code = r#"
foo <- function() {
  x <- 1
  y <- 2
  return(x + y)
}
"#;
        expect_no_lint(code, "unreachable_code", None);
    }

    #[test]
    fn test_no_unreachable_conditional_return() {
        let code = r#"
foo <- function(x) {
  if (x > 0) {
    return(1)
  } else {
    return(-1)
  }
}
"#;
        expect_no_lint(code, "unreachable_code", None);
    }

    #[test]
    fn test_unreachable_after_return() {
        let code = r#"
foo <- function() {
  return(1)
  x <- 5
}
"#;
        insta::assert_snapshot!(snapshot_lint(code), @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   x <- 5
          |   ------ This code is unreachable because it appears after a return statement.
          |
        Found 1 error.
        ");
    }

    #[test]
    fn test_unreachable_conditional_return() {
        let code = r#"
foo <- function(x) {
  if (x > 0) {
    return(1)
  } else {
    return(-1)
  }
  x <- 1
}
"#;
        expect_no_lint(code, "unreachable_code", None);
    }

    #[test]
    fn test_unreachable_after_break() {
        let code = r#"
foo <- function() {
  for (i in 1:10) {
    break
    x <- i
  }
}
"#;
        insta::assert_snapshot!(snapshot_lint(code), @r"
        warning: unreachable_code
         --> <test>:5:5
          |
        5 |     x <- i
          |     ------ This code is unreachable because it appears after a break statement.
          |
        Found 1 error.
        ");
    }

    #[test]
    fn test_unreachable_after_next() {
        let code = r#"
foo <- function() {
  for (i in 1:10) {
    next
    x <- i
  }
}
"#;
        insta::assert_snapshot!(snapshot_lint(code), @r"
        warning: unreachable_code
         --> <test>:5:5
          |
        5 |     x <- i
          |     ------ This code is unreachable because it appears after a next statement.
          |
        Found 1 error.
        ");
    }

    #[test]
    fn test_no_unreachable_loop_with_conditional_break() {
        let code = r#"
foo <- function() {
  for (i in 1:10) {
    if (i == 5) {
      break
    }
    x <- i
  }
}
"#;
        expect_no_lint(code, "unreachable_code", None);
    }

    #[test]
    fn test_no_unreachable_loop_with_conditional_next() {
        let code = r#"
foo <- function() {
  for (i in 1:10) {
    x <- i
    if (x > 5) {
      next
    }
    y <- x + 1
  }
}
"#;
        expect_no_lint(code, "unreachable_code", None);
    }

    #[test]
    fn test_multiple_statements_after_return() {
        let code = r#"
foo <- function() {
  x <- 1
  return(x)
  y <- 2
  z <- 3
}
"#;
        insta::assert_snapshot!(snapshot_lint(code), @r"
        warning: unreachable_code
         --> <test>:5:3
          |
        5 | /   y <- 2
        6 | |   z <- 3
          | |________- This code is unreachable because it appears after a return statement.
          |
        Found 1 error.
        ");
    }

    #[test]
    fn test_nested_function_with_return() {
        let code = r#"
outer <- function() {
  inner <- function() {
    return(1)
    x <- 2
  }
  y <- 3
}
"#;
        insta::assert_snapshot!(snapshot_lint(code), @r"
        warning: unreachable_code
         --> <test>:5:5
          |
        5 |     x <- 2
          |     ------ This code is unreachable because it appears after a return statement.
          |
        Found 1 error.
        ");
    }

    #[test]
    fn test_dead_branch_if_true() {
        let code = r#"
foo <- function() {
  if (TRUE) {
    "a"
  } else {
    x <- 1
    "b"
  }
}
"#;
        insta::assert_snapshot!(snapshot_lint(code), @r#"
        warning: unreachable_code
         --> <test>:5:10
          |
        5 |     } else {
          |  __________-
        6 | |     x <- 1
        7 | |     "b"
        8 | |   }
          | |___- This code is in a branch that can never be executed due to a constant condition.
          |
        Found 1 error.
        "#);
    }

    #[test]
    fn test_dead_branch_if_false() {
        let code = r#"
foo <- function() {
  if (FALSE) {
    x <- 1
    "a"
  } else {
    "b"
  }
}
"#;
        insta::assert_snapshot!(snapshot_lint(code), @r#"
        warning: unreachable_code
         --> <test>:3:14
          |
        3 |     if (FALSE) {
          |  ______________-
        4 | |     x <- 1
        5 | |     "a"
        6 | |   } else {
          | |___- This code is in a branch that can never be executed due to a constant condition.
          |
        Found 1 error.
        "#);
    }

    #[test]
    fn test_dead_branch_with_nested_code() {
        let code = r#"
foo <- function(bar) {
  if (FALSE) {
    1 + 1
    if (a) {
      2 + 2
    }
  } else {
    3 + 3
  }
}
"#;
        insta::assert_snapshot!(snapshot_lint(code), @r"
        warning: unreachable_code
         --> <test>:3:14
          |
        3 |     if (FALSE) {
          |  ______________-
        4 | |     1 + 1
        ... |
        7 | |     }
        8 | |   } else {
          | |___- This code is in a branch that can never be executed due to a constant condition.
          |
        Found 1 error.
        ");
    }

    #[test]
    fn test_dead_branch_t_and_f_are_not_true_and_false() {
        let code = r#"
foo <- function() {
  if (F) {
    "a"
  } else {
    "b"
  }
}
"#;
        expect_no_lint(code, "unreachable_code", None);
        let code = r#"
foo <- function() {
  if (T) {
    "a"
  } else {
    "b"
  }
}
"#;
        expect_no_lint(code, "unreachable_code", None);
    }

    #[test]
    fn test_if_else_both_with_unreachable() {
        let code = r#"
foo <- function(bar) {
  if (bar) {
    return(bar)
    x <- 2
  } else {
    return(bar)
    x <- 3
  }
  1 + 1
}
"#;
        insta::assert_snapshot!(snapshot_lint(code), @r"
        warning: unreachable_code
         --> <test>:5:5
          |
        5 |     x <- 2
          |     ------ This code is unreachable because it appears after a return statement.
          |
        warning: unreachable_code
         --> <test>:8:5
          |
        8 |     x <- 3
          |     ------ This code is unreachable because it appears after a return statement.
          |
        warning: unreachable_code
          --> <test>:10:3
           |
        10 |   1 + 1
           |   ----- This code is in a branch that can never be executed due to a constant condition.
           |
        Found 3 errors.
        ");
    }

    #[test]
    fn test_no_dead_branch_variable_condition() {
        let code = r#"
foo <- function(x) {
  if (x > 0) {
    "a"
  } else {
    "b"
  }
}
"#;
        expect_no_lint(code, "unreachable_code", None);
    }

    #[test]
    fn test_code_after_stop_and_variants() {
        let code = r#"
foo <- function() {
  stop("a")
  1 + 1
}
"#;
        insta::assert_snapshot!(snapshot_lint(code), @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   1 + 1
          |   ----- This code is unreachable because it appears after a `stop()` statement (or equivalent).
          |
        Found 1 error.
        ");

        let code = r#"
foo <- function() {
  abort("a")
  1 + 1
}
"#;
        insta::assert_snapshot!(snapshot_lint(code), @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   1 + 1
          |   ----- This code is unreachable because it appears after a `stop()` statement (or equivalent).
          |
        Found 1 error.
        ");

        let code = r#"
foo <- function() {
  cli_abort("a")
  1 + 1
}
"#;
        insta::assert_snapshot!(snapshot_lint(code), @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   1 + 1
          |   ----- This code is unreachable because it appears after a `stop()` statement (or equivalent).
          |
        Found 1 error.
        ");

        let code = r#"
foo <- function() {
  if (x > 0) {
    cli_abort("a")
  }
  1 + 1
}
"#;
        expect_no_lint(code, "unreachable_code", None);
    }

    #[test]
    fn test_repeat_with_break_in_braced_expression() {
        let code = r#"
foo <- function() {
  repeat {
    ({
      if (1 == 1) break
    })
  }
  print("here")
}
"#;
        expect_no_lint(code, "unreachable_code", None);
    }

    #[test]
    fn test_repeat_with_break_in_nested_braces() {
        let code = r#"
foo <- function() {
  repeat {
    {
      if (1 == 1) break
    }
  }
  print("here")
}
"#;
        expect_no_lint(code, "unreachable_code", None);
    }
}
