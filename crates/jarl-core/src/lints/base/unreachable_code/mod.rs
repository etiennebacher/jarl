pub(crate) mod cfg;
pub(crate) mod unreachable_code;

#[cfg(test)]
mod tests {
    use crate::rule_options::ResolvedRuleOptions;
    use crate::rule_options::unreachable_code::ResolvedUnreachableCodeOptions;
    use crate::rule_options::unreachable_code::UnreachableCodeOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;

    /// Format diagnostics for snapshot testing
    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "unreachable_code", None)
    }

    fn snapshot_lint_with_settings(code: &str, settings: Settings) -> String {
        format_diagnostics_with_settings(code, "unreachable_code", None, Some(settings))
    }

    /// Build a `Settings` with custom `UnreachableCodeOptions`.
    fn settings_with_options(options: UnreachableCodeOptions) -> Settings {
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions {
                    unreachable_code: ResolvedUnreachableCodeOptions::resolve(Some(&options))
                        .unwrap(),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
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
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   x <- 5
          |   ------ This code is unreachable because it appears after a return statement.
          |
        Found 1 error.
        "
        );
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
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:8:3
          |
        8 |   x <- 1
          |   ------ This code is unreachable because the preceding if/else terminates in all branches.
          |
        Found 1 error.
        "
        );
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
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:5:5
          |
        5 |     x <- i
          |     ------ This code is unreachable because it appears after a break statement.
          |
        Found 1 error.
        "
        );
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
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:5:5
          |
        5 |     x <- i
          |     ------ This code is unreachable because it appears after a next statement.
          |
        Found 1 error.
        "
        );
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
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:5:3
          |
        5 | /   y <- 2
        6 | |   z <- 3
          | |________- This code is unreachable because it appears after a return statement.
          |
        Found 1 error.
        "
        );
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
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:5:5
          |
        5 |     x <- 2
          |     ------ This code is unreachable because it appears after a return statement.
          |
        Found 1 error.
        "
        );
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
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r#"
        warning: unreachable_code
         --> <test>:5:10
          |
        5 |     } else {
          |  __________-
        6 | |     x <- 1
        7 | |     "b"
        8 | |   }
          | |___- This code is in a branch that can never be executed.
          |
        Found 1 error.
        "#
        );
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
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r#"
        warning: unreachable_code
         --> <test>:3:14
          |
        3 |     if (FALSE) {
          |  ______________-
        4 | |     x <- 1
        5 | |     "a"
        6 | |   } else {
          | |___- This code is in a branch that can never be executed.
          |
        Found 1 error.
        "#
        );
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
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:3:14
          |
        3 |     if (FALSE) {
          |  ______________-
        4 | |     1 + 1
        ... |
        7 | |     }
        8 | |   } else {
          | |___- This code is in a branch that can never be executed.
          |
        Found 1 error.
        "
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
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
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
           |   ----- This code is unreachable because the preceding if/else terminates in all branches.
           |
        Found 3 errors.
        "
        );
    }

    #[test]
    fn test_if_else_both_return_followed_by_loops() {
        // This should produce exactly 3 diagnostics:
        // 1. x <- 2 (after return in then branch)
        // 2. x <- 3 (after return in else branch)
        // 3. All code after if/else as single diagnostic (not one per loop)
        let code = r#"
foo <- function(bar) {
  if (bar) {
    return(bar) # comment
    x <- 2
  } else {
    return(bar) # comment
    x <- 3
  }
  while (bar) {
    return(bar) # comment
    5 + 3
  }
  repeat {
    return(bar) # comment
    test()
  }
  for (i in 1:3) {
    return(bar) # comment
    5 + 4
  }
}
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
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
        10 | /   while (bar) {
        11 | |     return(bar) # comment
        ...  |
        20 | |     5 + 4
        21 | |   }
           | |___- This code is unreachable because the preceding if/else terminates in all branches.
           |
        Found 3 errors.
        "
        );
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
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   1 + 1
          |   ----- This code is unreachable because it appears after a `stop()` statement (or equivalent).
          |
        Found 1 error.
        "
        );

        let code = r#"
foo <- function() {
  abort("a")
  1 + 1
}
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   1 + 1
          |   ----- This code is unreachable because it appears after a `stop()` statement (or equivalent).
          |
        Found 1 error.
        "
        );

        let code = r#"
foo <- function() {
  .Defunct("a")
  1 + 1
}
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   1 + 1
          |   ----- This code is unreachable because it appears after a `stop()` statement (or equivalent).
          |
        Found 1 error.
        "
        );

        let code = r#"
foo <- function() {
  cli_abort("a")
  1 + 1
}
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   1 + 1
          |   ----- This code is unreachable because it appears after a `stop()` statement (or equivalent).
          |
        Found 1 error.
        "
        );

        let code = r#"
foo <- function() {
  if (x > 0) {
    cli_abort("a")
  }
  1 + 1
}
"#;
        expect_no_lint(code, "unreachable_code", None);

        let code = r#"
foo <- function() {
  bar$stop()
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

    #[test]
    fn test_nested_if_with_all_branches_returning() {
        let code = r#"
foo <- function(x) {
  if (x) {
    if (y) {
      print("hi")
    }
    return(1)
  } else {
    if (is.null(z)) {
      print("hello")
    }
    return(2)
  }

  return(3)
}
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
          --> <test>:15:3
           |
        15 |   return(3)
           |   --------- This code is unreachable because the preceding if/else terminates in all branches.
           |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_unreachable_after_return_with_comment() {
        let code = r#"
foo <- function(x) {
  #
  return(x)
  1 + 1
}
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:5:3
          |
        5 |   1 + 1
          |   ----- This code is unreachable because it appears after a return statement.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_reachable_function_starting_with_return() {
        let code = r#"
foo <- function(x) {
  return_foo(x)
  1 + 1
}
"#;
        expect_no_lint(code, "unreachable_code", None);
    }

    #[test]
    fn test_function_shortcut_is_handled() {
        let code = r#"
foo <- \(x) {
  return(x)
  1 + 1
}
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   1 + 1
          |   ----- This code is unreachable because it appears after a return statement.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_unreachable_after_semicolon() {
        let code = r#"
foo <- function(x) {
  return(
    y^2
  ); 3 + 1
}
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:5:6
          |
        5 |   ); 3 + 1
          |      ----- This code is unreachable because it appears after a return statement.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_unreachable_true_plus_or() {
        let code = r#"
foo <- function(x) {
  if (TRUE | x) {
    1
  } else {
    2
  }
  if (TRUE || x) {
    1
  } else {
    2
  }
  if (x | TRUE) {
    1
  } else {
    2
  }
  if (x || TRUE) {
    1
  } else {
    2
  }
  if (TRUE || (x || TRUE)) {
    1
  } else {
    2
  }
}
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:5:10
          |
        5 |     } else {
          |  __________-
        6 | |     2
        7 | |   }
          | |___- This code is in a branch that can never be executed.
          |
        warning: unreachable_code
          --> <test>:10:10
           |
        10 |     } else {
           |  __________-
        11 | |     2
        12 | |   }
           | |___- This code is in a branch that can never be executed.
           |
        warning: unreachable_code
          --> <test>:15:10
           |
        15 |     } else {
           |  __________-
        16 | |     2
        17 | |   }
           | |___- This code is in a branch that can never be executed.
           |
        warning: unreachable_code
          --> <test>:20:10
           |
        20 |     } else {
           |  __________-
        21 | |     2
        22 | |   }
           | |___- This code is in a branch that can never be executed.
           |
        warning: unreachable_code
          --> <test>:25:10
           |
        25 |     } else {
           |  __________-
        26 | |     2
        27 | |   }
           | |___- This code is in a branch that can never be executed.
           |
        Found 5 errors.
        "
        );
    }

    #[test]
    fn test_unreachable_false_plus_and() {
        let code = r#"
foo <- function(x) {
  if (FALSE & x) {
    1
  } else {
    2
  }
  if (FALSE && x) {
    1
  } else {
    2
  }
  if (x & FALSE) {
    1
  } else {
    2
  }
  if (x && FALSE) {
    1
  } else {
    2
  }
  if (FALSE & (x && FALSE)) {
    1
  } else {
    2
  }
}
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:3:18
          |
        3 |     if (FALSE & x) {
          |  __________________-
        4 | |     1
        5 | |   } else {
          | |___- This code is in a branch that can never be executed.
          |
        warning: unreachable_code
          --> <test>:8:19
           |
         8 |     if (FALSE && x) {
           |  ___________________-
         9 | |     1
        10 | |   } else {
           | |___- This code is in a branch that can never be executed.
           |
        warning: unreachable_code
          --> <test>:13:18
           |
        13 |     if (x & FALSE) {
           |  __________________-
        14 | |     1
        15 | |   } else {
           | |___- This code is in a branch that can never be executed.
           |
        warning: unreachable_code
          --> <test>:18:19
           |
        18 |     if (x && FALSE) {
           |  ___________________-
        19 | |     1
        20 | |   } else {
           | |___- This code is in a branch that can never be executed.
           |
        warning: unreachable_code
          --> <test>:23:29
           |
        23 |     if (FALSE & (x && FALSE)) {
           |  _____________________________-
        24 | |     1
        25 | |   } else {
           | |___- This code is in a branch that can never be executed.
           |
        Found 5 errors.
        "
        );
    }

    // Top-level unreachable code tests

    #[test]
    fn test_top_level_dead_branch() {
        let code = r#"
if (TRUE) {
  x <- 1
} else {
  y <- 2
}
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:4:8
          |
        4 |   } else {
          |  ________-
        5 | |   y <- 2
        6 | | }
          | |_- This code is in a branch that can never be executed.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_top_level_after_break_in_loop() {
        let code = r#"
for (i in 1:10) {
  break
  x <- i
}
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   x <- i
          |   ------ This code is unreachable because it appears after a break statement.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_top_level_after_next_in_loop() {
        let code = r#"
for (i in 1:10) {
  next
  x <- i
}
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   x <- i
          |   ------ This code is unreachable because it appears after a next statement.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_top_level_after_stop() {
        let code = r#"
stop("error")
x <- 1
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:3:1
          |
        3 | x <- 1
          | ------ This code is unreachable because it appears after a `stop()` statement (or equivalent).
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_top_level_nested_after_stop_in_if() {
        // AfterStop inside an if statement SHOULD be reported
        let code = r#"
if (condition) {
  stop("error")
  x <- 1
}
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   x <- 1
          |   ------ This code is unreachable because it appears after a `stop()` statement (or equivalent).
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_top_level_after_branch_terminating() {
        let code = r#"
if (condition) {
  stop("a")
} else {
  stop("b")
}
x <- 1
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:7:1
          |
        7 | x <- 1
          | ------ This code is unreachable because the preceding if/else terminates in all branches.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_top_level_nested_branch_terminating() {
        // AfterBranchTerminating inside an if statement SHOULD be reported
        let code = r#"
if (outer_condition) {
  if (inner_condition) {
    stop("a")
  } else {
    stop("b")
  }
  x <- 1
}
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:8:3
          |
        8 |   x <- 1
          |   ------ This code is unreachable because the preceding if/else terminates in all branches.
          |
        Found 1 error.
        "
        );
    }

    // ---- Rule-specific config tests ----

    #[test]
    fn test_stopping_functions_replaces_defaults() {
        // With custom stopping-functions = ["my_stop"], only "my_stop" stops.
        // Default "stop" should no longer trigger unreachable code.
        let settings = settings_with_options(UnreachableCodeOptions {
            stopping_functions: Some(vec!["my_stop".to_string()]),
            extend_stopping_functions: None,
        });

        // "stop" is NOT in the custom list -> no longer considered stopping
        let code = r#"
foo <- function() {
  stop("error")
  1 + 1
}
"#;
        expect_no_lint_with_settings(code, "unreachable_code", None, settings.clone());

        // "my_stop" IS in the custom list -> triggers unreachable code
        let code = r#"
foo <- function() {
  my_stop("error")
  1 + 1
}
"#;
        insta::assert_snapshot!(
            snapshot_lint_with_settings(code, settings),
            @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   1 + 1
          |   ----- This code is unreachable because it appears after a `stop()` statement (or equivalent).
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_extend_stopping_functions_adds_to_defaults() {
        // extend-stopping-functions = ["my_stop"] -> defaults + "my_stop"
        let settings = settings_with_options(UnreachableCodeOptions {
            stopping_functions: None,
            extend_stopping_functions: Some(vec!["my_stop".to_string()]),
        });

        // "my_stop" is in the extended list -> triggers unreachable code
        let code = r#"
foo <- function() {
  my_stop("error")
  1 + 1
}
"#;
        insta::assert_snapshot!(
            snapshot_lint_with_settings(code, settings.clone()),
            @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   1 + 1
          |   ----- This code is unreachable because it appears after a `stop()` statement (or equivalent).
          |
        Found 1 error.
        "
        );

        // Default "stop" still works
        let code = r#"
foo <- function() {
  stop("error")
  1 + 1
}
"#;
        insta::assert_snapshot!(
            snapshot_lint_with_settings(code, settings),
            @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   1 + 1
          |   ----- This code is unreachable because it appears after a `stop()` statement (or equivalent).
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_extend_stopping_functions_top_level() {
        // Custom stopping functions should also work at top level
        let settings = settings_with_options(UnreachableCodeOptions {
            stopping_functions: None,
            extend_stopping_functions: Some(vec!["my_stop".to_string()]),
        });

        let code = r#"
my_stop("fatal")
x <- 1
"#;
        insta::assert_snapshot!(
            snapshot_lint_with_settings(code, settings),
            @r"
        warning: unreachable_code
         --> <test>:3:1
          |
        3 | x <- 1
          | ------ This code is unreachable because it appears after a `stop()` statement (or equivalent).
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_namespaced_stopping_function() {
        // The CFG builder strips namespace prefixes, so "abort" in the stopping
        // functions list matches both `abort(...)` and `rlang::abort(...)`.
        let code = r#"
foo <- function() {
  rlang::abort("error")
  1 + 1
}
"#;
        insta::assert_snapshot!(
            snapshot_lint(code),
            @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   1 + 1
          |   ----- This code is unreachable because it appears after a `stop()` statement (or equivalent).
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_namespaced_custom_stopping_function() {
        // A custom stopping function also works when called with a namespace prefix.
        let settings = settings_with_options(UnreachableCodeOptions {
            stopping_functions: None,
            extend_stopping_functions: Some(vec!["my_stop".to_string()]),
        });

        let code = r#"
foo <- function() {
  mypkg::my_stop("error")
  1 + 1
}
"#;
        insta::assert_snapshot!(
            snapshot_lint_with_settings(code, settings),
            @r"
        warning: unreachable_code
         --> <test>:4:3
          |
        4 |   1 + 1
          |   ----- This code is unreachable because it appears after a `stop()` statement (or equivalent).
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_namespaced_value_in_config_does_not_match_plain_call() {
        // If the user puts "mypkg::myfun" in the config, only the function name
        // is matched (i.e. "myfun"), so a plain call to `myfun(...)` should NOT
        // match "mypkg::myfun".
        let settings = settings_with_options(UnreachableCodeOptions {
            stopping_functions: Some(vec!["mypkg::myfun".to_string()]),
            extend_stopping_functions: None,
        });

        let code = r#"
foo <- function() {
  myfun("error")
  1 + 1
}
"#;
        expect_no_lint_with_settings(code, "unreachable_code", None, settings);
    }
}
