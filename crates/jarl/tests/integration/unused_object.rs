use crate::helpers::{CliTest, CommandExt};

#[test]
fn test_relative_source_resolves_next_to_the_file() -> anyhow::Result<()> {
    // `main.R` redefines `x`, sources a helper that also defines `x`, then
    // reads `x`. R's `source()` evaluates the helper in the calling
    // environment, so the helper's `x` wins and `main.R`'s own `x <- 2` is
    // dead — but only if the sourced path resolves, which makes the
    // diagnostic a direct probe of that resolution.
    //
    // The linted path (`sub/main.R`) is relative to the CWD, so resolution
    // walks anchors from the file's own directory upwards. Here the helper
    // sits next to the file, i.e. the first anchor.
    let case = CliTest::with_files([
        ("sub/helper.R", "x <- 1\n"),
        ("sub/main.R", "x <- 2\nsource(\"helper.R\")\nprint(x)\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("sub/main.R")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_object
     --> sub/main.R:1:1
      |
    1 | x <- 2
      | - Object `x` is defined but never used.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_relative_source_resolves_at_an_ancestor_directory() -> anyhow::Result<()> {
    // Same, but the helper only exists one anchor up (at the CWD), the way a
    // script run from the project root would find it.
    let case = CliTest::with_files([
        ("helper.R", "x <- 1\n"),
        ("sub/main.R", "x <- 2\nsource(\"helper.R\")\nprint(x)\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("sub/main.R")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_object
     --> sub/main.R:1:1
      |
    1 | x <- 2
      | - Object `x` is defined but never used.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_relative_source_that_resolves_nowhere_is_ignored() -> anyhow::Result<()> {
    // No anchor between the file and the CWD has `nope.R`: resolution fails
    // and `print(x)` reads `main.R`'s own binding, so nothing is reported.
    let case = CliTest::with_files([
        ("helper.R", "x <- 1\n"),
        ("sub/main.R", "x <- 2\nsource(\"nope.R\")\nprint(x)\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("sub/main.R")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_exported_alias_not_flagged() -> anyhow::Result<()> {
    // `summarize_each <- summarise_each` is a typical alias-style export. The
    // RHS isn't a function literal, so the existing function-def filter
    // doesn't suppress it; we rely on the NAMESPACE export list instead.
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        ("NAMESPACE", "export(summarize_each)\n"),
        (
            "R/aliases.R",
            "summarise_each <- function(x) x\nsummarize_each <- summarise_each\n",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_source_cycle_terminates() -> anyhow::Result<()> {
    // `p.R` and `q.R` source each other. Resolution must terminate, and the
    // read of `k` in `p.R` still consumes `q.R`'s binding.
    let case = CliTest::with_files([
        ("p.R", "source(\"q.R\")\nprint(k)\n"),
        ("q.R", "source(\"p.R\")\nk <- 1\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_object_used_in_another_package_file_not_flagged() -> anyhow::Result<()> {
    // A top-level object defined in one file ("foo") and read in another is
    // used even though its own file never reads it.
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "NAMESPACE",
            "# Generated by roxygen2: do not edit by hand\n",
        ),
        ("R/foo1.R", "foo <- new.env(parent = emptyenv())\n"),
        ("R/foo2.R", "out <- list(\"foo\" = foo)\nprint(out)\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_object_read_under_conditional_shadow_not_flagged() -> anyhow::Result<()> {
    // In "foo2.R", if `cond` is false then "the_const" is read from "foo1.R"
    // so we don't report the definition in "foo1.R" as unused.
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "NAMESPACE",
            "# Generated by roxygen2: do not edit by hand\n",
        ),
        ("R/foo1.R", "the_const <- 42\n"),
        (
            "R/foo2.R",
            "compute <- function(cond) {\n  if (cond) the_const <- 2\n  the_const\n}\n",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_object_read_by_sourcing_script_not_flagged() -> anyhow::Result<()> {
    // Loose scripts (no DESCRIPTION anywhere): `a.R` sources `b.R` and then
    // reads `x`, so `b.R`'s binding is consumed cross-file and not unused.
    let case = CliTest::with_files([("b.R", "x <- 1\n"), ("a.R", "source(\"b.R\")\nprint(x)\n")])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_source_path_anchored_at_cwd_not_flagged() -> anyhow::Result<()> {
    // R resolves `source("b.R")` against `getwd()`, so a script in a subfolder
    // run from the project root reads the root's `b.R`. Nothing exists next to
    // `foo/a.R`, so resolution falls back to the CWD anchor.
    let case = CliTest::with_files([
        ("b.R", "x <- 1\n"),
        ("foo/a.R", "source(\"b.R\")\nprint(x)\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_source_path_anchored_at_linted_folder_not_flagged() -> anyhow::Result<()> {
    // Scripts of a project run from its root anchor `source()` there, even
    // when the project sits below jarl's CWD: `foo/sub/a.R` sources `b.R`
    // living at `foo/`, and `jarl check foo` runs from the parent. The
    // ancestor walk between the file's directory and the CWD finds it.
    let case = CliTest::with_files([
        ("foo/b.R", "x <- 1\n"),
        ("foo/sub/a.R", "source(\"b.R\")\nprint(x)\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("foo")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_object_read_through_source_chain_not_flagged() -> anyhow::Result<()> {
    // Transitive chain: `a.R` sources `b.R`, which sources `c.R`. The read of
    // `z` in `a.R` reaches `c.R`'s binding through the forwarded exports.
    let case = CliTest::with_files([
        ("c.R", "z <- 1\n"),
        ("b.R", "source(\"c.R\")\n"),
        ("a.R", "source(\"b.R\")\nprint(z)\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_object_unused_across_package_files_is_flagged() -> anyhow::Result<()> {
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        (
            "NAMESPACE",
            "# Generated by roxygen2: do not edit by hand\n",
        ),
        ("R/foo1.R", "helper_obj <- new.env(parent = emptyenv())\n"),
        ("R/foo2.R", "x <- 1\nprint(x)\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @r"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_object
     --> R/foo1.R:1:1
      |
    1 | helper_obj <- new.env(parent = emptyenv())
      | ---------- Object `helper_obj` is defined but never used.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_object_not_read_by_sourcing_script_is_flagged() -> anyhow::Result<()> {
    // Sourcing alone doesn't consume a binding: `a.R` runs `b.R` but never
    // reads `x`, so `x` is still unused.
    let case = CliTest::with_files([
        ("b.R", "x <- 1\n"),
        ("a.R", "source(\"b.R\")\nprint(\"hi\")\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @r"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_object
     --> b.R:1:1
      |
    1 | x <- 1
      | - Object `x` is defined but never used.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_free_name_in_unrelated_script_does_not_suppress() -> anyhow::Result<()> {
    // Unlike package files, loose scripts share no namespace: `e.R` reads a
    // free `y` but never sources `d.R`, so `d.R`'s `y` stays unused.
    let case = CliTest::with_files([("d.R", "y <- 1\n"), ("e.R", "print(y)\n")])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @r"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_object
     --> d.R:1:1
      |
    1 | y <- 1
      | - Object `y` is defined but never used.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_unexported_alias_is_flagged() -> anyhow::Result<()> {
    // Same code, but no NAMESPACE export — `summarize_each` is dead.
    let case = CliTest::with_files([
        (
            "DESCRIPTION",
            "Package: testpkg\nTitle: Test\nVersion: 0.0.1\n",
        ),
        ("NAMESPACE", ""),
        (
            "R/aliases.R",
            "summarise_each <- function(x) x\nsummarize_each <- summarise_each\n",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @r"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_object
     --> R/aliases.R:2:1
      |
    2 | summarize_each <- summarise_each
      | -------------- Object `summarize_each` is defined but never used.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_object_used_in_non_package_r_directory_not_flagged() -> anyhow::Result<()> {
    // No DESCRIPTION, but an `R/` directory: R collates those files
    // alphabetically into one environment, the same convention as a package
    // without `Collate:`. So `R/b.R` reads `helper` out of `R/a.R`.
    let case = CliTest::with_files([
        ("R/a.R", "helper <- 1\n"),
        ("R/b.R", "compute <- function() helper\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg(".")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_interpolation_honoured_when_sourced_file_attaches_the_package() -> anyhow::Result<()> {
    // `{x}` is only a read when glue is in reach. Here `main.R` never mentions
    // glue itself: `setup.R` attaches it, and R's `source()` runs that
    // `library()` on the global search path, so the caller sees it too.
    let case = CliTest::with_files([
        ("setup.R", "library(glue)\n"),
        (
            "main.R",
            "source(\"setup.R\")\nx <- 1\nprint(glue(\"{x}\"))\n",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("main.R")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_interpolation_not_honoured_when_sourced_file_attaches_nothing() -> anyhow::Result<()> {
    // Control for the test above: same shape, but the sourced file attaches no
    // package. Nothing puts glue in reach, so `"{x}"` stays literal text and
    // `x` is reported.
    let case = CliTest::with_files([
        ("setup.R", "options(digits = 3)\n"),
        (
            "main.R",
            "source(\"setup.R\")\nx <- 1\nprint(glue(\"{x}\"))\n",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("main.R")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @r"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_object
     --> main.R:2:1
      |
    2 | x <- 1
      | - Object `x` is defined but never used.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_interpolation_honoured_through_a_source_chain() -> anyhow::Result<()> {
    // The attach travels the whole chain: `a.R` sources `b.R`, which sources
    // `c.R`, which attaches glue. `b.R` forwards what it sees, so `a.R`
    // reaches glue two hops away.
    let case = CliTest::with_files([
        ("c.R", "library(glue)\n"),
        ("b.R", "source(\"c.R\")\n"),
        ("a.R", "source(\"b.R\")\nx <- 1\nprint(glue(\"{x}\"))\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("a.R")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_interpolation_not_honoured_for_a_lazy_attach_in_a_sourced_file() -> anyhow::Result<()> {
    // A `library()` inside a function body only attaches when that body runs,
    // which sourcing the file does not do. So glue is not in reach and `x` is
    // still reported.
    let case = CliTest::with_files([
        ("setup.R", "setup <- function() library(glue)\n"),
        (
            "main.R",
            "source(\"setup.R\")\nx <- 1\nprint(glue(\"{x}\"))\n",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("main.R")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @r"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_object
     --> main.R:2:1
      |
    2 | x <- 1
      | - Object `x` is defined but never used.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_cli_markup_honoured_when_sourced_file_attaches_cli() -> anyhow::Result<()> {
    // Same reach rule for the other interpolation dialect: cli's inline markup
    // in `cli_abort()` reads `x` only because `setup.R` attaches cli.
    let case = CliTest::with_files([
        ("setup.R", "library(cli)\n"),
        (
            "main.R",
            "source(\"setup.R\")\nx <- 1\ncli_abort(\"bad value: {x}\")\n",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("main.R")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_data_table_prefix_honoured_when_sourced_file_attaches_data_table() -> anyhow::Result<()> {
    // The package-in-reach gate applies to every package idiom, not just
    // interpolation: `..cols` only reads `cols` because `setup.R` attaches
    // data.table.
    let case = CliTest::with_files([
        ("setup.R", "library(data.table)\n"),
        (
            "main.R",
            "source(\"setup.R\")\ncols <- c(\"a\")\nprint(dt[, ..cols])\n",
        ),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("main.R")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @"

    success: true
    exit_code: 0
    ----- stdout -----
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    Ok(())
}

#[test]
fn test_non_package_r_directory_not_scanned_for_a_file_argument() -> anyhow::Result<()> {
    // Same layout as above, but linting a single file declares no project, so
    // the sibling that reads `helper` is never scanned and can't vouch for it.
    // This is what keeps `jarl check /tmp/foo.R` from walking `/tmp`.
    let case = CliTest::with_files([
        ("R/a.R", "helper <- 1\n"),
        ("R/b.R", "compute <- function() helper\n"),
    ])?;

    insta::assert_snapshot!(
        &mut case
            .command()
            .arg("check")
            .arg("R/a.R")
            .arg("--select")
            .arg("unused_object")
            .run()
            .normalize_os_executable_name(),
        @r"

    success: false
    exit_code: 1
    ----- stdout -----
    warning: unused_object
     --> R/a.R:1:1
      |
    1 | helper <- 1
      | ------ Object `helper` is defined but never used.
      |


    ── Summary ──────────────────────────────────────
    Found 1 error.

    ----- stderr -----
    "
    );

    Ok(())
}
