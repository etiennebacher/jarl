pub(crate) mod if_switch;
pub(crate) mod options;

#[cfg(test)]
mod tests {
    use crate::lints::base::if_switch::options::{IfSwitchOptions, ResolvedIfSwitchOptions};
    use crate::rule_options::ResolvedRuleOptions;
    use crate::settings::{LinterSettings, Settings};
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "if_switch", None)
    }

    fn settings_max_branches(max_branches: usize) -> Settings {
        Settings {
            linter: LinterSettings {
                rule_options: ResolvedRuleOptions {
                    if_switch: ResolvedIfSwitchOptions { max_branches },
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_no_lint_if_switch() {
        let rule = "if_switch";
        // Simple if/else
        expect_no_lint("if (x == 'a') 1 else 2", rule, None);
        // Non-character conditions
        expect_no_lint("if (x == 1) 1 else 2", rule, None);
        expect_no_lint(
            "if (x == 1) 1 else if (x == 2) 2 else if (x == 3) 3 else if (x == 4) 4",
            rule,
            None,
        );
        // `!=` anywhere in the chain
        expect_no_lint("if (x == 'a') 1 else if (x != 'b') 2 else 3", rule, None);
        expect_no_lint(
            "if (x == 'a') 1 else if (x == 'b') 2 else if (x != 'c') 3 else if (x == 'd') 4",
            rule,
            None,
        );
        // Several variables
        expect_no_lint("if (x == 'a') 1 else if (y == 'b') 2 else 3", rule, None);
        expect_no_lint(
            "if (x == 'a') 1 else if (x == 'b') 2 else if (y == 'c') 3 else if (x == 'd') 4",
            rule,
            None,
        );
        // Several conditions
        expect_no_lint(
            "if (is.character(x) && x == 'a') 1 else if (x == 'b') 2 else 3",
            rule,
            None,
        );
        // Not enough branches
        expect_no_lint(
            "if (x == 'a') 1 else if (x == 'b') 2 else if (x == 'c') 3",
            rule,
            None,
        );
        expect_no_lint("if (x == 'a') 1 else if (x == 'b') 2", rule, None);
        expect_no_lint("if (x == 'a') 1 else if (x == 'b') 2 else 3", rule, None);
        expect_no_lint("if (x == 'a') 1 else if (x == R'(b)') 2 else 3", rule, None);
        // Empty strings can't be used in switch()
        expect_no_lint(
            "if (x == '') 1 else if (x == 'a') 2 else if (x == 'b') 3 else if (x == 'c') 4",
            rule,
            None,
        );
        expect_no_lint(
            "if (x == \"\") 1 else if (x == \"a\") 2 else if (x == \"b\") 3 else if (x == \"c\") 4",
            rule,
            None,
        );
        expect_no_lint(
            "if (x == 'a') 1 else if (x == '') 2 else if (x == 'b') 3 else if (x == 'c') 4",
            rule,
            None,
        );
        expect_no_lint(
            "if (x == 'a') 1 else if (x == 'b') 2 else if (x == 'c') 3 else if (x == '') 4",
            rule,
            None,
        );
        expect_no_lint(
            "if (x == R\"--()--\") 1 else if (x == \"a\") 2 else if (x == \"b\") 3",
            rule,
            None,
        );
        expect_no_lint(
            "if (x == R\"()\") 1 else if (x == R\"{}\") 2 else if (x == R\"[]\") 3 else if (x == R\"()\") 4",
            rule,
            None,
        );
        // Chain starting with another operator
        expect_no_lint(
            "if (x %in% c('a', 'e')) 1 else if (x == 'b') 2 else if (x == 'c') 3 else if (x == 'd') 4",
            rule,
            None,
        );
        // Both sides are strings
        expect_no_lint(
            "if ('a' == 'a') 1 else if ('a' == 'b') 2 else if ('a' == 'c') 3 else if ('a' == 'd') 4",
            rule,
            None,
        );
        // Non-binary condition
        expect_no_lint(
            "if (x) 1 else if (x == 'b') 2 else if (x == 'c') 3 else if (x == 'd') 4",
            rule,
            None,
        );
        // Existing switch()
        expect_no_lint("switch(x, a = 1, b = 2, 3)", rule, None);
    }

    #[test]
    fn test_lint_if_switch() {
        assert_snapshot!(
            snapshot_lint("if (x == 'a') 1 else if (x == 'b') 2 else if (x == 'c') 3 else if (x == 'd') 4"),
            @"
        warning: if_switch
         --> <test>:1:1
          |
        1 | if (x == 'a') 1 else if (x == 'b') 2 else if (x == 'c') 3 else if (x == 'd') 4
          | ------------------------------------------------------------------------------ Repeated `if`/`else if` equality tests on the same value.
          |
          = help: Use `switch()` instead.
        Found 1 error.
        "
        );
        // With raw strings
        assert_snapshot!(
            snapshot_lint("if (x == 'a') 1 else if (x == r'(b)') 2 else if (x == R'--[c]--') 3 else if (x == 'd') 4"),
            @"
        warning: if_switch
         --> <test>:1:1
          |
        1 | if (x == 'a') 1 else if (x == r'(b)') 2 else if (x == R'--[c]--') 3 else if (x == 'd') 4
          | ---------------------------------------------------------------------------------------- Repeated `if`/`else if` equality tests on the same value.
          |
          = help: Use `switch()` instead.
        Found 1 error.
        "
        );
        // String on the left
        assert_snapshot!(
            snapshot_lint("if ('a' == x) 1 else if (x == 'b') 2 else if ('c' == x) 3 else if (x == 'd') 4"),
            @"
        warning: if_switch
         --> <test>:1:1
          |
        1 | if ('a' == x) 1 else if (x == 'b') 2 else if ('c' == x) 3 else if (x == 'd') 4
          | ------------------------------------------------------------------------------ Repeated `if`/`else if` equality tests on the same value.
          |
          = help: Use `switch()` instead.
        Found 1 error.
        "
        );
        // Expressions as subject
        assert_snapshot!(
            snapshot_lint("if (foo(x) == 'a') 1 else if (foo(x) == 'b') 2 else if (foo(x) == 'c') 3 else if (foo(x) == 'd') 4"),
            @"
        warning: if_switch
         --> <test>:1:1
          |
        1 | if (foo(x) == 'a') 1 else if (foo(x) == 'b') 2 else if (foo(x) == 'c') 3 else if (foo(x) == 'd') 4
          | -------------------------------------------------------------------------------------------------- Repeated `if`/`else if` equality tests on the same value.
          |
          = help: Use `switch()` instead.
        Found 1 error.
        "
        );
        // Comments and whitespace in the subject
        assert_snapshot!(
            snapshot_lint("if (foo(x) == 'a') 1 else if (foo(x # comment\n) == 'b') 2 else if (foo( x ) == 'c') 3 else if (foo(x) == 'd') 4"),
            @"
        warning: if_switch
         --> <test>:1:1
          |
        1 | / if (foo(x) == 'a') 1 else if (foo(x # comment
        2 | | ) == 'b') 2 else if (foo( x ) == 'c') 3 else if (foo(x) == 'd') 4
          | |_________________________________________________________________- Repeated `if`/`else if` equality tests on the same value.
          |
          = help: Use `switch()` instead.
        Found 1 error.
        "
        );
        // A long chain is reported once
        assert_snapshot!(
            snapshot_lint("if (x == 'a') 1 else if (x == 'b') 2 else if (x == 'c') 3 else if (x == 'd') 4"),
            @"
        warning: if_switch
         --> <test>:1:1
          |
        1 | if (x == 'a') 1 else if (x == 'b') 2 else if (x == 'c') 3 else if (x == 'd') 4
          | ------------------------------------------------------------------------------ Repeated `if`/`else if` equality tests on the same value.
          |
          = help: Use `switch()` instead.
        Found 1 error.
        "
        );
        // With a final else
        assert_snapshot!(
            snapshot_lint("if (x == 'a') 1 else if (x == 'b') 2 else if (x == 'c') 3 else if (x == 'd') 4 else 5"),
            @"
        warning: if_switch
         --> <test>:1:1
          |
        1 | if (x == 'a') 1 else if (x == 'b') 2 else if (x == 'c') 3 else if (x == 'd') 4 else 5
          | ------------------------------------------------------------------------------------- Repeated `if`/`else if` equality tests on the same value.
          |
          = help: Use `switch()` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_if_switch_multiple_chains() {
        assert_snapshot!(
            snapshot_lint(
                "{
  if (x == 'a') {
    do_a()
  } else if (x == 'b') {
    do_b()
  } else if (x == 'c') {
    do_c()
  } else if (x == 'd') {
    do_d()
  }
  if (y == 'A') {
    do_A()
  } else if (y == 'B') {
    do_B()
  } else if (y == 'C') {
    do_C()
  } else if (y == 'D') {
    do_D()
  }
  if (z1 == 'a') {
    do_a()
  } else if (z2 == 'b') {
    do_b()
  } else if (z3 == 'c') {
    do_c()
  } else if (x == 'd') {
    do_d()
  }
  if (a == '1') {
    do_1()
  } else if (a == '2') {
    do_2()
  } else if (a == '') {
    do_blank()
  } else if (a == '3') {
    do_3()
  }
}"
            ),
            @"
        warning: if_switch
          --> <test>:2:3
           |
         2 | /   if (x == 'a') {
         3 | |     do_a()
         4 | |   } else if (x == 'b') {
         5 | |     do_b()
        ...  |
         9 | |     do_d()
        10 | |   }
           | |___- Repeated `if`/`else if` equality tests on the same value.
           |
           = help: Use `switch()` instead.
        warning: if_switch
          --> <test>:11:3
           |
        11 | /   if (y == 'A') {
        12 | |     do_A()
        13 | |   } else if (y == 'B') {
        14 | |     do_B()
        ...  |
        18 | |     do_D()
        19 | |   }
           | |___- Repeated `if`/`else if` equality tests on the same value.
           |
           = help: Use `switch()` instead.
        Found 2 errors.
        "
        );
    }

    #[test]
    fn test_fix_if_switch() {
        assert_snapshot!(
            "fix_output",
            get_unsafe_fixed_text(
                vec![
                    "if (x == 'a') 1 else if (x == 'b') 2 else if (x == 'c') 3 else if (x == 'd') 4",
                    "if (x == 'a') 1 else if (x == 'b') 2 else if (x == 'c') 3 else if (x == 'd') 4 else 5",
                    "if ('a' == x) 1 else if (x == 'b') 2 else if ('c' == x) 3 else if (x == 'd') 4",
                    "if (foo(x) == 'a') 1 else if (foo( x ) == 'b') 2 else if (foo(x) == 'c') 3 else if (foo(x) == 'd') 4",
                    "if (x == 'a') {\n  do_a()\n} else if (x == 'b') {\n  do_b()\n} else if (x == 'c') {\n  do_c()\n} else if (x == 'd') {\n  do_d()\n} else {\n  stop('no')\n}",
                    "y <- if (x == 'a') z <- 1 else if (x == 'b') z <- 2 else if (x == 'c') z <- 3 else if (x == 'd') z <- 4",
                    // No fix
                    "if (x == 'a') 1 else if (x == 'b') 2 else if (x == 'c') 3 else if (x == 'd') 4 # comment\n",
                    "if (x == 'a') # comment\n  1 else if (x == 'b') 2 else if (x == 'c') 3 else if (x == 'd') 4",
                    "if (x == 'a') y = 1 else if (x == 'b') y = 2 else if (x == 'c') y = 3 else if (x == 'd') y = 4",
                    "if (x == 'a') 1 else if (x == 'b') 2 else if (x == 'c') 3 else if (x == 'd') 4 else y = 5",
                ],
                "if_switch",
            )
        );
    }

    #[test]
    fn test_if_switch_max_branches() {
        let two_branches = "if (x == 'a') 1 else if (x == 'b') 2";
        let three_branches = "if (x == 'a') 1 else if (x == 'b') 2 else if (x == 'c') 3";
        let four_branches =
            "if (x == 'a') 1 else if (x == 'b') 2 else if (x == 'c') 3 else if (x == 'd') 4";

        assert_snapshot!(
            format_diagnostics_with_settings(two_branches, "if_switch", None, Some(settings_max_branches(2))),
            @"
        warning: if_switch
         --> <test>:1:1
          |
        1 | if (x == 'a') 1 else if (x == 'b') 2
          | ------------------------------------ Repeated `if`/`else if` equality tests on the same value.
          |
          = help: Use `switch()` instead.
        Found 1 error.
        "
        );
        expect_no_lint_with_settings(three_branches, "if_switch", None, settings_max_branches(4));
        expect_no_lint_with_settings(two_branches, "if_switch", None, settings_max_branches(3));
        assert_snapshot!(
            format_diagnostics_with_settings(four_branches, "if_switch", None, Some(settings_max_branches(4))),
            @"
        warning: if_switch
         --> <test>:1:1
          |
        1 | if (x == 'a') 1 else if (x == 'b') 2 else if (x == 'c') 3 else if (x == 'd') 4
          | ------------------------------------------------------------------------------ Repeated `if`/`else if` equality tests on the same value.
          |
          = help: Use `switch()` instead.
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_resolve_max_branches() {
        let resolved = |max_branches| {
            ResolvedIfSwitchOptions::resolve(Some(&IfSwitchOptions { max_branches }))
        };
        assert_snapshot!(
            resolved(Some(1)).unwrap_err().to_string(),
            @"`max-branches` in `[lint.if_switch]` must be at least 2."
        );
        assert_snapshot!(
            resolved(Some(0)).unwrap_err().to_string(),
            @"`max-branches` in `[lint.if_switch]` must be at least 2."
        );
        assert_snapshot!(
            ResolvedIfSwitchOptions::resolve(None).unwrap().max_branches,
            @"4"
        );
    }
}
