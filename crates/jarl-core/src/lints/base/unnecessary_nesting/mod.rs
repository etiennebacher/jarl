pub(crate) mod unnecessary_nesting;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

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
        use insta::assert_snapshot;
        let msg = "no need for nested if conditions";

        expect_lint(
            "
if (x) {
  if (y) {
    1L
  }
}",
            msg,
            "unnecessary_nesting",
            None,
        );
        expect_lint(
            "
if (x) {
  if (y) {
    if (z) {
      1L
    }
  }
}",
            msg,
            "unnecessary_nesting",
            None,
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
