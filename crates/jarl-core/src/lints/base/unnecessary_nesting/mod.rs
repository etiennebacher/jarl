pub(crate) mod unnecessary_nesting;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "unnecessary_nesting", None)
    }

    #[test]
    fn test_no_lint_unnecessary_nesting() {
        expect_no_lint(
            "
if (x && y) {
  1L
}",
            "unnecessary_nesting",
            None,
        );
        expect_no_lint(
            "
if (x) {
  1L
} else if (y) {
  2L
}",
            "unnecessary_nesting",
            None,
        );

        expect_no_lint(
            "
if (x) {
  1L
} else {
  2L
  if (y) {
    3L
  }
}",
            "unnecessary_nesting",
            None,
        );

        expect_no_lint(
            "
if (x) {
  if (y) {
    print('hi')
  }
} else {
    print('hello')
}",
            "unnecessary_nesting",
            None,
        );

        expect_no_lint(
            "
if (if (x) TRUE else FALSE) {
  1L
}",
            "unnecessary_nesting",
            None,
        );

        expect_no_lint(
            "
if (x) {
  y <- x + 1L
  if (y) {
    1L
  }
}",
            "unnecessary_nesting",
            None,
        );

        expect_no_lint(
            "
if ((x && y) || (if (x) TRUE else FALSE)) {
  1L
}",
            "unnecessary_nesting",
            None,
        );

        expect_no_lint(
            "
if (x && a) {
  y <- x + 1L
  if (y || b) {
    1L
  }
}",
            "unnecessary_nesting",
            None,
        );

        expect_no_lint(
            "
if (x && a) {
  y = x + 1L
  if (y || b) {
    1L
  }
}",
            "unnecessary_nesting",
            None,
        );
        expect_no_lint(
            "
if (x) {
  if (y) {
    1L
  }
  y <- x + 1L
}",
            "unnecessary_nesting",
            None,
        );

        expect_no_lint(
            "
if (x) {
  y <- x + 1L
  if (y) {
    1L
  }
  y <- x
}",
            "unnecessary_nesting",
            None,
        );

        expect_no_lint(
            "
if (x) {
  y <- x + 1L
  {
    if (y) {
      1L
    }
  }
}",
            "unnecessary_nesting",
            None,
        );

        expect_no_lint(
            "
if (x) {
  {
     y <- x + 1L
     if (y) {
       1L
     }
  }
}",
            "unnecessary_nesting",
            None,
        );

        expect_no_lint(
            "
if (x) {
  {
    if (y) {
      1L
    }
  }
  y <- x + 1L
}",
            "unnecessary_nesting",
            None,
        );

        expect_no_lint(
            "
if (x) {
  {
    y <- x + 1L
    {
      if (y) {
        1L
      }
    }
  }
}",
            "unnecessary_nesting",
            None,
        );
    }

    #[test]
    fn test_lint_unnecessary_nesting() {
        assert_snapshot!(
            snapshot_lint(
            "
if (x) {
  if (y) {
    1L
  }
}"), @r"
        warning: unnecessary_nesting
         --> <test>:2:1
          |
        2 | / if (x) {
        3 | |   if (y) {
        4 | |     1L
        5 | |   }
        6 | | }
          | |_- There is no need for nested if conditions here.
          |
          = help: Gather the two conditions with `&&` instead.
        Found 1 error.
        "
        );
        assert_snapshot!(
            snapshot_lint(
            "
if (x) {
  if (y) {
    if (z) {
      1L
    }
  }
}"), @r"
        warning: unnecessary_nesting
         --> <test>:2:1
          |
        2 | / if (x) {
        3 | |   if (y) {
        ... |
        7 | |   }
        8 | | }
          | |_- There is no need for nested if conditions here.
          |
          = help: Gather the two conditions with `&&` instead.
        warning: unnecessary_nesting
         --> <test>:3:3
          |
        3 | /   if (y) {
        4 | |     if (z) {
        5 | |       1L
        6 | |     }
        7 | |   }
          | |___- There is no need for nested if conditions here.
          |
          = help: Gather the two conditions with `&&` instead.
        Found 2 errors.
        "
        );

        assert_snapshot!(
            "fix_output",
            get_fixed_text(
                vec![
                    "
if (x) {
  if (y) {
    1L
  }
}",
                    "
if (x) {
  if (y) {
    if (z) {
      1L
    }
  }
}"
                ],
                "unnecessary_nesting",
                None
            )
        );
    }
}
