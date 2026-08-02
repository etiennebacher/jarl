pub(crate) mod unused_object;
#[cfg(test)]
mod tests {
    use crate::utils_test::*;
    use insta::assert_snapshot;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "unused_object", None)
    }

    /// Renders the `unused_object` diagnostics produced by linting `main_path`
    /// (already written to disk), formatted for snapshot comparison.
    fn snapshot_unused_object_at(main_path: &std::path::Path, main: &str) -> String {
        use crate::check::check;
        use crate::config::ArgsConfig;
        use crate::diagnostic::render_diagnostic;
        use annotate_snippets::Renderer;

        let args = ArgsConfig {
            files: vec![main_path.to_path_buf()],
            fix: false,
            unsafe_fixes: false,
            fix_only: false,
            select: "unused_object".to_string(),
            extend_select: String::new(),
            ignore: String::new(),
            min_r_version: None,
            allow_dirty: false,
            allow_no_vcs: true,
            assignment: None,
        };
        let config = crate::config::build_config(&args, None, vec![main_path.to_path_buf()])
            .expect("build config");

        let diagnostics: Vec<_> = check(config)
            .into_iter()
            .find_map(|(_, result)| result.ok())
            .unwrap_or_default();

        if diagnostics.is_empty() {
            return "All checks passed!".to_string();
        }
        let renderer = Renderer::plain();
        let mut output = String::new();
        for diagnostic in &diagnostics {
            let rendered = render_diagnostic(
                main,
                "<test>",
                &diagnostic.message.name,
                diagnostic,
                &renderer,
            );
            output.push_str(&format!("{}\n", rendered));
        }
        output.push_str(&format!(
            "Found {} error{}.",
            diagnostics.len(),
            if diagnostics.len() == 1 { "" } else { "s" }
        ));
        output
    }

    /// Lints `main.R` inside a fresh tempdir after populating that directory
    /// with the named (filename, content) pairs, and renders diagnostics as
    /// a snapshot string. Used for `source()` resolution tests where the
    /// sourced file lives next to the linted file.
    fn snapshot_lint_with_sourced_files(main: &str, files: &[(&str, &str)]) -> String {
        use std::fs;

        let dir = tempfile::tempdir().expect("create tempdir");
        let main_path = dir.path().join("main.R");
        fs::write(&main_path, main).expect("write main.R");
        for (name, content) in files {
            fs::write(dir.path().join(name), content).expect("write sourced file");
        }
        snapshot_unused_object_at(&main_path, main)
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
    fn test_no_lint_chained_function_definition() {
        // Every name in a chained function assignment is a function binding, so
        // the function-definition exemption covers the whole chain.
        expect_no_lint("x <- y <- function() {}", "unused_object", None);
    }

    #[test]
    fn test_lint_chained_non_function_assignment() {
        assert_snapshot!(snapshot_lint("x <- y <- 1"), @r"
        warning: unused_object
         --> <test>:1:6
          |
        1 | x <- y <- 1
          |      - Object `y` is defined but never used.
          |
        warning: unused_object
         --> <test>:1:1
          |
        1 | x <- y <- 1
          | - Object `x` is defined but never used.
          |
        Found 2 errors.
        ");
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
    fn test_no_lint_custom_operator_used() {
        // oak doesn't model `1 %op% 2` as a use of the `%op%` binding, so a
        // custom infix operator defined via a non-function RHS would otherwise
        // look unused.
        expect_no_lint(
            "f <- function() {}\n`%op%` <- f\n1 %op% 2",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_lint_custom_operator_never_used() {
        // A custom operator defined but never used at a call site is still
        // reported.
        assert_snapshot!(snapshot_lint("`%op%` <- 42\nprint(1)"), @"
        warning: unused_object
         --> <test>:1:1
          |
        1 | `%op%` <- 42
          | ------ Object `%op%` is defined but never used.
          |
        Found 1 error.
        ");
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
    fn test_no_lint_string_interpolation_nested_call() {
        expect_no_lint("x <- 1\nglue::glue(\"{mean(x)}\")", "unused_object", None);
    }

    #[test]
    fn test_no_lint_string_interpolation_dollar_access() {
        // `x` is referenced (used); `a` is a field name, not a binding.
        expect_no_lint(
            "x <- list(a = 1)\nglue::glue(\"{x$a}\")",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_glue_basic() {
        expect_no_lint("x <- 1\nglue(\"this is {x}\")", "unused_object", None);
    }

    #[test]
    fn test_no_lint_glue_custom_delimiters() {
        expect_no_lint(
            "x <- 1\nglue(\"<x>\", .open = \"<\", .close = \">\")",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_glue_custom_multichar_delimiters() {
        expect_no_lint(
            "x <- 1\nglue(\"<<x>>\", .open = \"<<\", .close = \">>\")",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_glue_custom_delimiters_raw_string() {
        expect_no_lint(
            "x <- 1\nglue(r\"([x])\", .open = \"[\", .close = \"]\")",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_str_glue_default_delimiters() {
        expect_no_lint("x <- 1\nstr_glue(\"{x}\")", "unused_object", None);
    }

    #[test]
    fn test_no_lint_glue_sql_default_delimiters() {
        expect_no_lint(
            "col <- 1\nglue_sql(\"SELECT {col}\", .con = con)",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_lint_glue_custom_delimiters_unrelated_object() {
        assert_snapshot!(
            snapshot_lint("x <- 1\nglue(\"[a]\", .open = \"[\", .close = \"]\")"),
            @r#"
        warning: unused_object
         --> <test>:1:1
          |
        1 | x <- 1
          | - Object `x` is defined but never used.
          |
        Found 1 error.
        "#
        );
    }

    #[test]
    fn test_no_lint_glue_same_char_delimiters() {
        // `.open` and `.close` are the same character, so nesting is
        // impossible and the closing `|` must not be read as another opener.
        expect_no_lint(
            "
x <- 1
glue(\"|x|\", .open = \"|\", .close = \"|\")",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_lint_interpolation_does_not_revive_later_reassignment() {
        // The `glue` read resolves to the preceding `foo`; the later, unread
        // reassignment of the same name is still reported unused.
        assert_snapshot!(
            snapshot_lint("
foo <- \"a\"
glue(\"{foo}\")

foo <- \"b\""),
        @r#"
        warning: unused_object
         --> <test>:5:1
          |
        5 | foo <- "b"
          | --- Object `foo` is defined but never used.
          |
        Found 1 error.
        "#
        );
    }

    #[test]
    fn test_no_lint_interpolation_reads_branch_assignments() {
        // Both `if`/`else` arms assign `x` in the same scope and both reach the
        // later `glue` read through branching control flow, so neither is unused.
        expect_no_lint(
            "if (a) {\n  x <- \"a\"\n} else {\n  x <- \"b\"\n}\nglue(\"{x}\")",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_interpolation_captures_enclosing_definition() {
        // The interpolated read is a closure capture evaluated later, so the
        // top-level `prefix` it reads stays used even though it is defined
        // after the function.
        expect_no_lint(
            "
f <- function() glue(\"{prefix}\")
prefix <- \"info\"
f()",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_cli_interpolation() {
        expect_no_lint("x <- 1\ncli_abort(\"{x}\")", "unused_object", None);
        expect_no_lint("x <- 1\ncli_warn(\"{x}\")", "unused_object", None);
    }

    #[test]
    fn test_no_lint_cli_markup_with_interpolation() {
        expect_no_lint("x <- 1\ncli_abort(\"{.field {x}}\")", "unused_object", None);
    }

    #[test]
    fn test_no_lint_cli_nested_markup_with_interpolation() {
        expect_no_lint(
            "x <- 1\ncli_abort(\"{.strong {.emph {x}}}\")",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_lint_cli_interpolation_does_not_revive_later_reassignment() {
        // Same position-aware resolution as glue: the `cli_abort` markup read
        // resolves to the preceding `foo`, so the later reassignment is unused.
        assert_snapshot!(
            snapshot_lint("foo <- \"a\"\ncli_abort(\"{.field {foo}}\")\n\nfoo <- \"b\""),
            @r#"
        warning: unused_object
         --> <test>:4:1
          |
        4 | foo <- "b"
          | --- Object `foo` is defined but never used.
          |
        Found 1 error.
        "#
        );
    }

    #[test]
    fn test_no_lint_cli_interpolation_captures_enclosing_definition() {
        // A cli markup read inside a closure captures the enclosing `prefix`
        // even though it is defined after the function.
        expect_no_lint(
            "f <- function() cli_abort(\"{.field {prefix}}\")\nprefix <- \"info\"\nf()",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_cli_namespaced() {
        expect_no_lint(
            "x <- 1\ncli::cli_abort(\"{.val {x}}\")",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_cli_bullets_vector() {
        expect_no_lint(
            "path <- \"f\"\ncli_abort(c(\"Can't find {.file {path}}\", \"i\" = \"check it\"))",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_cli_other_families() {
        expect_no_lint("x <- 1\ncli_text(\"{.emph {x}}\")", "unused_object", None);
        expect_no_lint("x <- 1\ncli_alert_info(\"{x}\")", "unused_object", None);
        expect_no_lint(
            "x <- 1\nformat_inline(\"{.field {x}}\")",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_lint_cli_markup_literal_text() {
        // In `{.field x}`, `x` is literal styled text, not an interpolation.
        assert_snapshot!(
            snapshot_lint("x <- 1\ncli_abort(\"{.field x}\")"),
            @r#"
        warning: unused_object
         --> <test>:1:1
          |
        1 | x <- 1
          | - Object `x` is defined but never used.
          |
        Found 1 error.
        "#
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
            "x <- 1
            y <- 2
            z <- x + y
            print(z)",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_used_in_nested_call() {
        expect_no_lint(
            "
        x <- 1
        print(mean(x))",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_local_scope() {
        expect_no_lint(
            "
        local({
          x <- 1
          print(x)
        })",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_with_unresolved_refs_in_function_def_resolved_later() {
        expect_no_lint(
            "
        f <- function() x
        x <- 1",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_closure_reads_redefined_variable() {
        // Both definitions of `x` are read by `f()` at different call sites.
        expect_no_lint(
            "
        x <- 1
        f <- function() x
        f()
        x <- 2
        f()",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_nested_closure_reads_redefined_variable() {
        // Same as test_no_lint_closure_reads_redefined_variable but nested.
        expect_no_lint(
            "
        foo <- function() {
            x <- 1
            f <- function() x
            f()
            x <- 2
            f()
        }",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_closure_only_called_after_redefinition() {
        // `x <- 1` is technically dead here (`f()` only runs after `x <- 2`),
        // but oak's enclosing snapshot captures the union of `x`'s definitions
        // for the closure, so `x <- 1` counts as used. We accept this
        // conservative answer (no false positives) rather than re-deriving
        // call-site-sensitive capture ourselves.
        expect_no_lint(
            "
x <- 1
f <- function() x
x <- 2
f()",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_nested_closure_callback() {
        // x is captured by f2 and used via lapply (not a direct call).
        expect_no_lint(
            "
        f <- function() {
            x <- 1
            f2 <- function(i) {
                i == x
            }
            lapply(1:2, f2)
        }",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_anonymous_closure_callback() {
        // x is captured by an anonymous function passed to lapply.
        expect_no_lint(
            "
        x <- 1
        lapply(1, function() x)",
            "unused_object",
            None,
        );
        // Same but nested inside a function.
        expect_no_lint(
            "
        f <- function() {
            x <- 1
            lapply(1, function() x)
        }",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_returned_closure() {
        // x is captured by f2, which is returned from f. f2 could be called
        // by f's caller, so x must be considered used.
        //
        // This happens in function factories, see for instance `string_magic_alias()`
        // in stringmagic.
        expect_no_lint(
            "
        f <- function() {
            x <- 1
            f2 <- function() x
            f2
        }",
            "unused_object",
            None,
        );
        // Same but with an anonymous function as the return value.
        expect_no_lint(
            "
        f <- function() {
            x <- 1
            function() x
        }",
            "unused_object",
            None,
        );
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
    fn test_bquote_unquote_counts_as_use() {
        // `bquote(x)` quotes `x`, so its value is unused -> report.
        assert_snapshot!(
            snapshot_lint("x <- 1\nbquote(x)"),
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
        // `bquote(.(x))` unquotes (evaluates) `x`, so it is used -> no report.
        expect_no_lint("x <- 1\nbquote(.(x))", "unused_object", None);
        // The unquoted use resolves to the latest definition, so the shadowed
        // `x <- 1` is still dead and reported.
        assert_snapshot!(
            snapshot_lint("x <- 1\nx <- 2\nbquote(.(x))"),
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
    fn test_bquote_where_arg_counts_as_use() {
        expect_no_lint(
            "env <- as.environment(list(x = 1))\nbquote(.(x), env)",
            "unused_object",
            None,
        );

        expect_no_lint(
            "env <- as.environment(list(x = 1))\nbquote(where = env, .(x))",
            "unused_object",
            None,
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
    fn test_no_lint_pipe_target_used_in_glue() {
        // The `%<>%` binding is read only inside a glue string, which oak's
        // use maps don't see; `SemanticInfo` interpolation tracking does.
        expect_no_lint(
            "x <- 1\nx %<>% as.character()\nglue(\"{x}\")",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_pipe_target_used_in_cli() {
        expect_no_lint(
            "x <- 1\nx %<>% as.character()\ncli::cli_inform(\"{x}\")",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_pipe_target_used_in_dotdot() {
        // `..cols` is a synthetic use of `cols`.
        expect_no_lint(
            "cols <- 1\ncols %<>% rev()\ndplyr::select(df, ..cols)",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_pipe_target_used_in_do_call() {
        // `do.call("f", …)` marks `f` synthetically used.
        expect_no_lint(
            "f <- 1\nf %<>% identity()\ndo.call(\"f\", list())",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_no_lint_pipe_target_captured_in_nested_glue() {
        // The pipe target is read by an interpolation inside a nested closure,
        // evaluated later, so it stays used regardless of textual order.
        expect_no_lint(
            "x <- 1\nx %<>% sum()\ng <- function() glue(\"{x}\")\ng()",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_lint_pipe_target_glue_before_reassignment() {
        // The glue read sees the pre-pipe `x`; the `%<>%` rebinding is never
        // read afterward, so it is still reported (position-aware, like a plain
        // reassignment).
        assert_snapshot!(
            snapshot_lint("
x <- 1
glue(\"{x}\")
x %<>% sum()"),
            @"
        warning: unused_object
         --> <test>:4:1
          |
        4 | x %<>% sum()
          | - Object `x` is defined but never used.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_assign() {
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

    #[test]
    fn test_object_used_in_next_iteration() {
        expect_no_lint(
            "
for (i in 1:3) {
  out <- f(i, x)
  x <- nrow(out)
}",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_function_def_default_arg_value() {
        expect_no_lint(
            "
default <- 'a'
f <- function(arg = default) {}",
            "unused_object",
            None,
        );
        expect_no_lint(
            "
f <- function(arg = default) {}
default <- 'a'",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_unused_for_loop_index_not_reported() {
        expect_no_lint(
            "
for (i in 1:2) {
    print('hello')
}",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_rm_in_on_exit() {
        expect_no_lint(
            "
        f <- function() {
            on.exit({
                x <- 1
                rm(x)
            })
        }",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_assignment_inside_nse_is_not_definition() {
        // `x <- 2` inside `quote()` is quoted code, not a real assignment.
        expect_no_lint("as.call(quote(x <- 2))", "unused_object", None);
        expect_no_lint("substitute(y <- 1)", "unused_object", None);
    }

    #[test]
    fn test_assignment_inside_alist_is_not_definition() {
        // `alist()` stores its arguments unevaluated (as if describing
        // function arguments), so `x <- 1` is captured code, not a real
        // assignment of `x`.
        expect_no_lint("alist(x <- 1)", "unused_object", None);
    }

    #[test]
    fn test_mention_inside_alist_is_not_used() {
        assert_snapshot!(snapshot_lint("x <- 1\nalist(x)"), @"
        warning: unused_object
         --> <test>:1:1
          |
        1 | x <- 1
          | - Object `x` is defined but never used.
          |
        Found 1 error.
        ");
    }

    #[test]
    fn test_nse_evaluated_argument_counts_as_use() {
        // `substitute`'s `env` argument is evaluated, not quoted, so the read
        // of `env` keeps the binding alive.
        expect_no_lint(
            "env <- as.environment(list(x = 1))\nsubstitute(x, env = env)",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_nse_assignment_does_not_shadow_real_definition() {
        // The quoted `x <- 2` must not kill the real `x <- 1`; `print(x)`
        // reads the live binding (which is still `1`).
        expect_no_lint(
            "x <- 1\nsubstitute(x <- 2)\nprint(x)",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_nse_in_same_call() {
        expect_no_lint(
            "
        x <- 1
        f(x, substitute('a'))",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_special_functions_use_quoted_objects() {
        expect_no_lint(
            "
        f <- mean
        do.call('f', list(x = 1:3))",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_equal_in_formula_is_not_definition() {
        expect_no_lint(
            "
        a ~ b + (c = 1)",
            "unused_object",
            None,
        );
    }

    #[test]
    fn test_object_used_in_formula_is_used() {
        expect_no_lint(
            "
        X <- 2
        lm(1 ~ X)",
            "unused_object",
            None,
        );
    }

    // ---------------------------------------------------------------
    // source() cross-file resolution
    // ---------------------------------------------------------------

    #[test]
    fn test_no_lint_sourced_file_reads_var() {
        // `x` looks unused in main.R, but the sourced helper reads it, so
        // the binding is consumed at the source() call site.
        assert_snapshot!(
            snapshot_lint_with_sourced_files(
                "x <- 1\nsource(\"helper.R\")\n",
                &[("helper.R", "print(x + 1)")],
            ),
            @"All checks passed!"
        );
    }

    #[test]
    fn test_lint_sourced_file_does_not_read_var() {
        // The sourced helper doesn't reference `y`, so it's still unused.
        assert_snapshot!(
            snapshot_lint_with_sourced_files(
                "y <- 1\nsource(\"helper.R\")\n",
                &[("helper.R", "print(1)")],
            ),
            @r"
        warning: unused_object
         --> <test>:1:1
          |
        1 | y <- 1
          | - Object `y` is defined but never used.
          |
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_lint_sourced_file_missing_does_not_suppress() {
        // No helper.R on disk: resolution silently fails and we fall back
        // to the regular unused-object check.
        assert_snapshot!(
            snapshot_lint_with_sourced_files("x <- 1\nsource(\"missing.R\")\n", &[]),
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
    fn test_no_lint_sourced_file_absolute_path_outside_project() {
        // The sourced file lives in a separate tempdir, referenced by an
        // absolute path. Resolution should follow the path verbatim rather
        // than joining it under the linted file's directory.
        use std::fs;

        let project_dir = tempfile::tempdir().expect("create project tempdir");
        let external_dir = tempfile::tempdir().expect("create external tempdir");

        let helper_path = external_dir.path().join("helper.R");
        fs::write(&helper_path, "print(x + 1)").expect("write helper.R");

        let main = format!(
            "x <- 1\nsource(\"{}\")\n",
            helper_path.to_str().expect("utf-8 path")
        );
        let main_path = project_dir.path().join("main.R");
        fs::write(&main_path, &main).expect("write main.R");

        assert_snapshot!(snapshot_unused_object_at(&main_path, &main), @"All checks passed!");
    }
}
