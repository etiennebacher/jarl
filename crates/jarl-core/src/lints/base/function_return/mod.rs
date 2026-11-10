pub(crate) mod function_return;
pub(crate) mod options;

#[cfg(test)]
mod tests {
    use crate::lints::base::function_return::options::{
        FunctionReturnOptions, ResolvedFunctionReturnOptions, ReturnMode,
    };
    use crate::rule_options::ResolvedRuleOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "function_return", None)
    }

    fn snapshot_lint_explicit(code: &str) -> String {
        format_diagnostics_with_settings(code, "function_return", None, Some(settings_explicit()))
    }

    fn settings_explicit() -> Settings {
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions {
                    function_return: ResolvedFunctionReturnOptions { mode: ReturnMode::Explicit },
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_no_lint_function_return_implicit() {
        expect_no_lint("x + 1", "function_return", None);
        expect_no_lint("return(x)", "function_return", None);
        expect_no_lint("{\n  return(1)\n}", "function_return", None);
        expect_no_lint("f <- function(x) {\n  x + 1\n}", "function_return", None);
        expect_no_lint("lapply(x, function(x) x + 1)", "function_return", None);
        expect_no_lint("f <- function() {\n  stop(1)\n}", "function_return", None);
        // `return()` not in terminal position
        expect_no_lint(
            "f <- function(x) {\n  return(1)\n  x\n}",
            "function_return",
            None,
        );
        expect_no_lint(
            "f <- function(x) {\n  if (x) return(1)\n  2\n}",
            "function_return",
            None,
        );
        expect_no_lint(
            "f <- function(x) {\n  y <- return(1)\n}",
            "function_return",
            None,
        );
        expect_no_lint(
            "f <- function(x) {\n  return(1) + 1\n}",
            "function_return",
            None,
        );
        expect_no_lint(
            "f <- function(x) {\n  for (i in x) {\n    return(i)\n  }\n}",
            "function_return",
            None,
        );
        expect_no_lint(
            "f <- function() {\n  if (TRUE) {\n    if (TRUE) {\n      return(1)\n    }\n    2\n  } else {\n    3\n  }\n}",
            "function_return",
            None,
        );
        expect_no_lint(
            "f <- function(x) {\n  switch(x,\n    a = ,\n    b = 1\n  )\n}",
            "function_return",
            None,
        );
        // Empty bodies and branches
        expect_no_lint("f <- function() { }", "function_return", None);
        expect_no_lint("f <- function() {\n  # comment\n}", "function_return", None);
        expect_no_lint(
            "f <- function() {\n  if (TRUE) {\n  } else {\n    FALSE\n  }\n}",
            "function_return",
            None,
        );
    }

    #[test]
    fn test_lint_function_return_implicit() {
        assert_snapshot!(
            snapshot_lint("f <- function(x) {\n  return(x + 1)\n}"),
            @"
        warning: function_return
         --> <test>:2:3
          |
        2 |   return(x + 1)
          |   ------------- Explicit `return()` is not needed at the end of a function.
          |
          = help: Remove the `return()` call and keep its value.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint(
                "
f <- function(x) return(x)
g <- \\(x) return(x)
h <- function(x) {
  y <- x
  base::return(y)
  # comment
}"
            ),
            @r"
        warning: function_return
         --> <test>:2:18
          |
        2 | f <- function(x) return(x)
          |                  --------- Explicit `return()` is not needed at the end of a function.
          |
          = help: Remove the `return()` call and keep its value.
        warning: function_return
         --> <test>:3:11
          |
        3 | g <- \(x) return(x)
          |           --------- Explicit `return()` is not needed at the end of a function.
          |
          = help: Remove the `return()` call and keep its value.
        warning: function_return
         --> <test>:6:3
          |
        6 |   base::return(y)
          |   --------------- Explicit `return()` is not needed at the end of a function.
          |
          = help: Remove the `return()` call and keep its value.
        Found 3 errors.
        "
        );
        assert_snapshot!(
            snapshot_lint(
                "
f <- function(x) {
  if (x < 0) {
    if (x == -1) {
      return(TRUE)
    }
    if (x > -10) {
      return(NA)
    } else {
      FALSE
    }
  } else if (x == 0) {
    return(TRUE)
  } else {
    {{ return(1) }}
  }
}"
            ),
            @"
        warning: function_return
         --> <test>:8:7
          |
        8 |       return(NA)
          |       ---------- Explicit `return()` is not needed at the end of a function.
          |
          = help: Remove the `return()` call and keep its value.
        warning: function_return
          --> <test>:13:5
           |
        13 |     return(TRUE)
           |     ------------ Explicit `return()` is not needed at the end of a function.
           |
           = help: Remove the `return()` call and keep its value.
        warning: function_return
          --> <test>:15:8
           |
        15 |     {{ return(1) }}
           |        --------- Explicit `return()` is not needed at the end of a function.
           |
           = help: Remove the `return()` call and keep its value.
        Found 3 errors.
        "
        );
        assert_snapshot!(
            snapshot_lint(
                "
f <- function(x) {
  switch(x,
    a = return(1),
    b = {
      if (TRUE) {
        return(2)
      }
      return(3)
    },
    return(4)
  )
}"
            ),
            @"
        warning: function_return
         --> <test>:4:9
          |
        4 |     a = return(1),
          |         --------- Explicit `return()` is not needed at the end of a function.
          |
          = help: Remove the `return()` call and keep its value.
        warning: function_return
         --> <test>:9:7
          |
        9 |       return(3)
          |       --------- Explicit `return()` is not needed at the end of a function.
          |
          = help: Remove the `return()` call and keep its value.
        warning: function_return
          --> <test>:11:5
           |
        11 |     return(4)
           |     --------- Explicit `return()` is not needed at the end of a function.
           |
           = help: Remove the `return()` call and keep its value.
        Found 3 errors.
        "
        );
        assert_snapshot!(
            snapshot_lint(
                "
f <- function(x) {
  function() {
    return(x + 1)
  }
}
g <- function(y, F = function(z) {
  return(z + 1)
}) {
  F(y) + y
}"
            ),
            @"
        warning: function_return
         --> <test>:4:5
          |
        4 |     return(x + 1)
          |     ------------- Explicit `return()` is not needed at the end of a function.
          |
          = help: Remove the `return()` call and keep its value.
        warning: function_return
         --> <test>:8:3
          |
        8 |   return(z + 1)
          |   ------------- Explicit `return()` is not needed at the end of a function.
          |
          = help: Remove the `return()` call and keep its value.
        Found 2 errors.
        "
        );
    }

    #[test]
    fn test_fix_function_return_implicit() {
        assert_snapshot!(
            "fix_output_implicit",
            get_unsafe_fixed_text(
                vec![
                    "f <- function(x) {\n  return(x + 1)\n}",
                    "f <- function(x) return(x)",
                    "f <- \\(x) return(x)",
                    "f <- function() {\n  return()\n}",
                    "f <- function(x) {\n  if (x) return(1) else return(if (x) 2 else 3)\n}",
                    "f <- function(x) {\n  switch(x, a = return(1), return(2))\n}",
                    "f <- function(x) {\n  return(invisible(x))\n}",
                    "f <- function(x) {\n  return(x <- 1)\n}",
                    // No fix
                    "f <- function(x) {\n  return(# comment\n    x)\n}",
                    "f <- function(x) {\n  return(if (x) 1)\n}",
                    "f <- function(x) {\n  return(for (i in x) i)\n}",
                    "f <- function(x) {\n  return(value = x)\n}",
                ],
                "function_return",
            )
        );
    }

    #[test]
    fn test_no_lint_function_return_explicit() {
        let settings = settings_explicit();

        expect_no_lint_with_settings("x + 1", "function_return", None, settings.clone());
        expect_no_lint_with_settings(
            "if (TRUE) {\n  TRUE\n} else {\n  FALSE\n}",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "f <- function(x) {\n  y <- x + 1\n  return(y)\n}",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "f <- function() {\n  if (x) {\n    return(4)\n  } else if (y) {\n    return(5)\n  } else {\n    return(6)\n  }\n}",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "f <- function() {\n  if (TRUE) return(TRUE)\n}",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "f <- function() {\n  return(TRUE)\n  # comment\n}",
            "function_return",
            None,
            settings.clone(),
        );
        // Inline functions
        expect_no_lint_with_settings(
            "lapply(x, function(x) x + 1)",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "function(x) { sum(x) }",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "function(x) { y <- 1; y }",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "f <- function() { }",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "f <- function() {\n}",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "f <- function() {\n  # comment\n}",
            "function_return",
            None,
            settings.clone(),
        );
        // Exit functions
        expect_no_lint_with_settings(
            "f <- function() {\n  3 + 4\n  stop(1)\n}",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "f <- function() {\n  rlang::abort('bad')\n}",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "f <- function(x) {\n  if (TRUE) {\n    q('n')\n  } else {\n    quit('n')\n  }\n}",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "f <- function(file) {\n  .Call(R_Read, file)\n}",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "print.foo <- function(x, ...) {\n  UseMethod('print')\n}",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "setGeneric('f', function(x) {\n  standardGeneric('f')\n})",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "setMethod('initialize', 'A', function(.Object, ...) {\n  callNextMethod(.Object, ...)\n})",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "warning = function(w) {\n  warn <<- append(warn, conditionMessage(w))\n  invokeRestart('muffleWarning')\n}",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "f <- function(x) {\n  switch(x,\n    a = .Call(a_routine, x),\n    stop('invalid')\n  )\n}",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "f <- function(x) {\n  x %>%\n    mean() %>%\n    return()\n}",
            "function_return",
            None,
            settings.clone(),
        );
        // Package hooks
        expect_no_lint_with_settings(
            ".onLoad <- function(libname, pkgname) {\n  do_setup()\n}",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            ".onAttach = function(libname, pkgname) {\n  1\n}",
            "function_return",
            None,
            settings.clone(),
        );
        expect_no_lint_with_settings(
            "function(libname, pkgname) {\n  1\n} -> .onUnload",
            "function_return",
            None,
            settings.clone(),
        );
    }

    #[test]
    fn test_lint_function_return_explicit() {
        assert_snapshot!(
            snapshot_lint_explicit("f <- function(x) {\n  x + 1\n}"),
            @"
        warning: function_return
         --> <test>:2:3
          |
        2 |   x + 1
          |   ----- Missing explicit `return()` at the end of a function.
          |
          = help: Wrap the returned value in `return()`.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint_explicit(
                "
f <- function() {
  return(1)
  # Test
  3 + 4
}
g <- \\(bar) {
  if (TRUE) return(TRUE)
  5L + 3L
}
h = function(bar) {
  stopifnot(bar == 'd')
}"
            ),
            @"
        warning: function_return
         --> <test>:5:3
          |
        5 |   3 + 4
          |   ----- Missing explicit `return()` at the end of a function.
          |
          = help: Wrap the returned value in `return()`.
        warning: function_return
         --> <test>:9:3
          |
        9 |   5L + 3L
          |   ------- Missing explicit `return()` at the end of a function.
          |
          = help: Wrap the returned value in `return()`.
        warning: function_return
          --> <test>:12:3
           |
        12 |   stopifnot(bar == 'd')
           |   --------------------- Missing explicit `return()` at the end of a function.
           |
           = help: Wrap the returned value in `return()`.
        Found 3 errors.
        "
        );
        assert_snapshot!(
            snapshot_lint_explicit(
                "
f <- function(x) {
  if (x < 0) {
    if (x == -1) {
      return(TRUE)
    }
    if (x > -10) {
      return(NA)
    } else {
      FALSE
    }
  } else if (x == 0) {
    return(TRUE)
  } else if (x == 1) {
  } else {
    if (TRUE) # comment
      FALSE
  }
}"
            ),
            @"
        warning: function_return
          --> <test>:10:7
           |
        10 |       FALSE
           |       ----- Missing explicit `return()` at the end of a function.
           |
           = help: Wrap the returned value in `return()`.
        warning: function_return
          --> <test>:14:22
           |
        14 |     } else if (x == 1) {
           |  ______________________-
        15 | |   } else {
           | |___- Missing explicit `return()` at the end of a function.
           |
           = help: Wrap the returned value in `return()`.
        warning: function_return
          --> <test>:17:7
           |
        17 |       FALSE
           |       ----- Missing explicit `return()` at the end of a function.
           |
           = help: Wrap the returned value in `return()`.
        Found 3 errors.
        "
        );
        assert_snapshot!(
            snapshot_lint_explicit(
                "
f <- function(x) {
  while (x > 4) {
    if (x < 4) {
      return(x)
    }
  }
}
g <- function(x) {
  for (i in seq_len(10)) {
    x %>%
      mean()
  }
}
h <- function(x) {
  y <- x
}"
            ),
            @"
        warning: function_return
         --> <test>:3:3
          |
        3 | /   while (x > 4) {
        4 | |     if (x < 4) {
        5 | |       return(x)
        6 | |     }
        7 | |   }
          | |___- Missing explicit `return()` at the end of a function.
          |
          = help: Wrap the returned value in `return()`.
        warning: function_return
          --> <test>:10:3
           |
        10 | /   for (i in seq_len(10)) {
        11 | |     x %>%
        12 | |       mean()
        13 | |   }
           | |___- Missing explicit `return()` at the end of a function.
           |
           = help: Wrap the returned value in `return()`.
        warning: function_return
          --> <test>:16:3
           |
        16 |   y <- x
           |   ------ Missing explicit `return()` at the end of a function.
           |
           = help: Wrap the returned value in `return()`.
        Found 3 errors.
        "
        );
        assert_snapshot!(
            snapshot_lint_explicit(
                "
f <- function(x) {
  switch(x,
    a = 1,
    b = {
      if (TRUE) {
        return(2)
      }
      3
    },
    return(4)
  )
}"
            ),
            @"
        warning: function_return
         --> <test>:4:9
          |
        4 |     a = 1,
          |         - Missing explicit `return()` at the end of a function.
          |
          = help: Wrap the returned value in `return()`.
        warning: function_return
         --> <test>:9:7
          |
        9 |       3
          |       - Missing explicit `return()` at the end of a function.
          |
          = help: Wrap the returned value in `return()`.
        Found 2 errors.
        "
        );
        assert_snapshot!(
            snapshot_lint_explicit(
                "
f <- function(x) {
  function() {
    x + 1
  }
}
.onLoad <- function(libname, pkgname) {
  function() {
    1
  }
}"
            ),
            @"
        warning: function_return
         --> <test>:3:3
          |
        3 | /   function() {
        4 | |     x + 1
        5 | |   }
          | |___- Missing explicit `return()` at the end of a function.
          |
          = help: Wrap the returned value in `return()`.
        warning: function_return
         --> <test>:4:5
          |
        4 |     x + 1
          |     ----- Missing explicit `return()` at the end of a function.
          |
          = help: Wrap the returned value in `return()`.
        warning: function_return
         --> <test>:9:5
          |
        9 |     1
          |     - Missing explicit `return()` at the end of a function.
          |
          = help: Wrap the returned value in `return()`.
        Found 3 errors.
        "
        );
    }

    #[test]
    fn test_fix_function_return_explicit() {
        assert_snapshot!(
            "fix_output_explicit",
            get_unsafe_fixed_text_with_settings(
                vec![
                    "f <- function(x) {\n  x + 1\n}",
                    "f <- function(x) {\n  if (x) {\n    1\n  } else if (y) 2 else {\n    3\n  }\n}",
                    "f <- function(x) {\n  switch(x, a = 1, 2)\n}",
                    "f <- function(x) {\n  x |>\n    # comment\n    mean()\n}",
                    "f <- function(x) {\n  function() {\n    x\n  }\n}",
                    // No fix
                    "f <- function(x) {\n  y <- x\n}",
                    "f <- function(x) {\n  y = x\n}",
                    "f <- function(x) {\n  for (i in x) print(i)\n}",
                    "f <- function(x) {\n  if (x) {\n  } else {\n    return(1)\n  }\n}",
                ],
                "function_return",
                None,
                Some(settings_explicit()),
            )
        );
    }

    #[test]
    fn test_function_return_invalid_option() {
        let options = FunctionReturnOptions { mode: Some("foo".to_string()) };
        let error = ResolvedFunctionReturnOptions::resolve(Some(&options)).unwrap_err();
        assert_eq!(
            error.to_string(),
            "Invalid value for `mode` in `[lint.function_return]`: \"foo\". Expected \"implicit\" or \"explicit\"."
        );
    }
}
