pub(crate) mod cfg;
pub(crate) mod unreachable_code;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_unreachable_after_return() {
        let code = r#"
foo <- function() {
  return(1)
  x <- 5  # This should be flagged as unreachable
}
"#;
        expect_lint(
            code,
            "unreachable because it appears after a return statement",
            "unreachable_code",
            None,
        );
    }

    #[test]
    fn test_unreachable_after_break() {
        let code = r#"
foo <- function() {
  for (i in 1:10) {
    break
    x <- i  # This should be flagged as unreachable
  }
}
"#;
        expect_lint(
            code,
            "unreachable because it appears after a break statement",
            "unreachable_code",
            None,
        );
    }

    #[test]
    fn test_unreachable_after_next() {
        let code = r#"
foo <- function() {
  for (i in 1:10) {
    next
    x <- i  # This should be flagged as unreachable
  }
}
"#;
        expect_lint(
            code,
            "unreachable because it appears after a next statement",
            "unreachable_code",
            None,
        );
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
    fn test_no_unreachable_loop_with_conditional_break() {
        let code = r#"
foo <- function() {
  for (i in 1:10) {
    if (i == 5) {
      break
    }
    x <- i  # This is reachable
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
  y <- 2  # Unreachable
  z <- 3  # Unreachable
}
"#;
        expect_lint(
            code,
            "unreachable because it appears after a return statement",
            "unreachable_code",
            None,
        );
    }

    #[test]
    fn test_reachable_code_in_loop() {
        let code = r#"
foo <- function() {
  for (i in 1:10) {
    x <- i
    if (x > 5) {
      next
    }
    y <- x + 1  # This is reachable (when x <= 5)
  }
}
"#;
        expect_no_lint(code, "unreachable_code", None);
    }

    #[test]
    fn test_nested_function_with_return() {
        let code = r#"
outer <- function() {
  inner <- function() {
    return(1)
    x <- 2  # Unreachable in inner function
  }
  y <- 3  # Reachable in outer function
}
"#;
        expect_lint(
            code,
            "unreachable because it appears after a return statement",
            "unreachable_code",
            None,
        );
    }

    #[test]
    fn test_dead_branch_if_true() {
        let code = r#"
foo <- function() {
  if (TRUE) {
    "a"
  } else {
    "b"  # Dead branch
  }
}
"#;
        expect_lint(
            code,
            "branch that can never be executed due to a constant condition",
            "unreachable_code",
            None,
        );
    }

    #[test]
    fn test_dead_branch_if_false() {
        let code = r#"
foo <- function() {
  if (FALSE) {
    "a"  # Dead branch
  } else {
    "b"
  }
}
"#;
        expect_lint(
            code,
            "branch that can never be executed due to a constant condition",
            "unreachable_code",
            None,
        );
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
}
