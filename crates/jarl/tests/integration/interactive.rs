use crate::helpers::CliTest;
use crate::helpers::CommandExt;
use crate::helpers::create_commit;
use crate::helpers::git_init;

/// A committed project, so that the version control question doesn't get in
/// the way of the fix questions.
fn clean_repo<'a>(files: impl IntoIterator<Item = (&'a str, &'a str)>) -> anyhow::Result<CliTest> {
    let files: Vec<(&str, &str)> = files.into_iter().collect();
    let case = CliTest::with_files(files.iter().copied())?;
    git_init(case.root())?;
    for (path, _) in files {
        create_commit(&case.root().join(path), case.root())?;
    }
    Ok(case)
}

#[test]
fn test_accept_single_fix() -> anyhow::Result<()> {
    let case = clean_repo([("test.R", "x <- any(is.na(y))\n")])?;

    insta::assert_snapshot!(
        case.command()
            .arg("check")
            .arg(".")
            .arg("--interactive")
            .run_with_stdin("y\n")
            .normalize_os_executable_name(),
        @r"

    success: true
    exit_code: 0
    ----- stdout -----

    test.R:1:6  any_is_na
    `any(is.na(...))` is inefficient.

    ────────────┬───────────────────────────────────────────────────────────────────
        1       │-x <- any(is.na(y))
              1 │+x <- anyNA(y)
    ────────────┴───────────────────────────────────────────────────────────────────

      y accept      apply this fix
      n reject      leave this code as it is
      a accept all  apply this fix and all the remaining ones
      q quit        stop here, keeping the fixes already applied

    Apply this fix? y

    ── Interactive fixes ────────────────────────────
    Applied 1 fix(es), skipped 0.
    ── Summary ──────────────────────────────────────
    All checks passed!

    ----- stderr -----
    "
    );

    insta::assert_snapshot!(case.read_file("test.R")?, @"x <- anyNA(y)");

    Ok(())
}

#[test]
fn test_context_lines_around_the_fix() -> anyhow::Result<()> {
    let case = clean_repo([(
        "test.R",
        "a <- 1\nb <- 2\nc <- 3\nd <- 4\ne <- any(is.na(x))\nf <- 6\ng <- 7\nh <- 8\ni <- 9\n",
    )])?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--interactive")
        .run_with_stdin("n\n");

    // Three unchanged lines on each side, numbered on both, and the rewritten
    // line numbered only on the side it belongs to.
    let frame: Vec<&str> = output
        .stdout
        .lines()
        .skip_while(|line| !line.starts_with('─'))
        .skip(1)
        .take_while(|line| !line.starts_with('─'))
        .collect();

    insta::assert_snapshot!(
        frame.join("\n"),
        @r"
        2     2 │ b <- 2
        3     3 │ c <- 3
        4     4 │ d <- 4
        5       │-e <- any(is.na(x))
              5 │+e <- anyNA(x)
        6     6 │ f <- 6
        7     7 │ g <- 7
        8     8 │ h <- 8
    "
    );

    Ok(())
}

#[test]
fn test_skip_single_fix() -> anyhow::Result<()> {
    let case = clean_repo([("test.R", "x <- any(is.na(y))\n")])?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--interactive")
        .run_with_stdin("n\n");

    assert!(output.stdout.contains("Applied 0 fix(es), skipped 1."));
    insta::assert_snapshot!(case.read_file("test.R")?, @"x <- any(is.na(y))");

    Ok(())
}

#[test]
fn test_accept_then_skip() -> anyhow::Result<()> {
    let case = clean_repo([("test.R", "a <- any(is.na(x))\nb <- any(is.na(yyyy))\n")])?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--interactive")
        .run_with_stdin("y\nn\n");

    assert!(output.stdout.contains("Applied 1 fix(es), skipped 1."));

    // The skipped fix must not be asked about again after the accepted one
    // shifted its offsets.
    assert_eq!(output.stdout.matches("Apply this fix?").count(), 2);

    insta::assert_snapshot!(
        case.read_file("test.R")?,
        @r"
    a <- anyNA(x)
    b <- any(is.na(yyyy))
    "
    );

    Ok(())
}

#[test]
fn test_skip_then_accept() -> anyhow::Result<()> {
    let case = clean_repo([("test.R", "a <- any(is.na(x))\nb <- any(is.na(yyyy))\n")])?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--interactive")
        .run_with_stdin("n\ny\n");

    assert_eq!(output.stdout.matches("Apply this fix?").count(), 2);

    insta::assert_snapshot!(
        case.read_file("test.R")?,
        @r"
    a <- any(is.na(x))
    b <- anyNA(yyyy)
    "
    );

    Ok(())
}

#[test]
fn test_accept_all() -> anyhow::Result<()> {
    let case = clean_repo([
        ("a.R", "a <- any(is.na(x))\nb <- any(is.na(y))\n"),
        ("b.R", "c <- any(is.na(z))\n"),
    ])?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--interactive")
        .run_with_stdin("a\n");

    // `a` answers every remaining question, in this file and the next.
    assert_eq!(output.stdout.matches("Apply this fix?").count(), 1);
    assert!(output.stdout.contains("Applied 3 fix(es), skipped 0."));

    insta::assert_snapshot!(
        case.read_file("a.R")?,
        @r"
    a <- anyNA(x)
    b <- anyNA(y)
    "
    );
    insta::assert_snapshot!(case.read_file("b.R")?, @"c <- anyNA(z)");

    Ok(())
}

#[test]
fn test_quit_leaves_the_rest_alone() -> anyhow::Result<()> {
    let case = clean_repo([
        ("a.R", "a <- any(is.na(x))\nb <- any(is.na(y))\n"),
        ("b.R", "c <- any(is.na(z))\n"),
    ])?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--interactive")
        .run_with_stdin("y\nq\n");

    assert_eq!(output.stdout.matches("Apply this fix?").count(), 2);
    assert!(output.stdout.contains("Applied 1 fix(es), skipped 0."));

    // Quitting keeps what was already accepted, and still reports the rest.
    insta::assert_snapshot!(
        case.read_file("a.R")?,
        @r"
    a <- anyNA(x)
    b <- any(is.na(y))
    "
    );
    insta::assert_snapshot!(case.read_file("b.R")?, @"c <- any(is.na(z))");
    assert!(output.stdout.contains("Found 2 errors."));

    Ok(())
}

#[test]
fn test_eof_stops_asking() -> anyhow::Result<()> {
    let case = clean_repo([("test.R", "a <- any(is.na(x))\nb <- any(is.na(y))\n")])?;

    // Only one answer for two fixes: the second question hits EOF, which is
    // treated as `q` rather than looping.
    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--interactive")
        .run_with_stdin("y\n");

    assert_eq!(output.stdout.matches("Apply this fix?").count(), 2);
    assert!(output.stdout.contains("Applied 1 fix(es), skipped 0."));

    insta::assert_snapshot!(
        case.read_file("test.R")?,
        @r"
    a <- anyNA(x)
    b <- any(is.na(y))
    "
    );

    Ok(())
}

#[test]
fn test_cascading_fix_is_offered() -> anyhow::Result<()> {
    let case = clean_repo([("test.R", "expect_true(all(!is.na(x)))\n")])?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--interactive")
        .arg("--select")
        .arg("ALL")
        .run_with_stdin("a\n");

    // Fixing the outer negation reveals fixes that only apply to the rewritten
    // code, so accepting everything must converge like `--fix` does.
    assert!(output.stdout.contains("Applied 3 fix(es), skipped 0."));
    insta::assert_snapshot!(case.read_file("test.R")?, @"expect_false(anyNA(x))");

    Ok(())
}

#[test]
fn test_unsafe_fix_is_marked() -> anyhow::Result<()> {
    let case = clean_repo([("test.R", "if (!all.equal(x, y)) 1\n")])?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--interactive")
        .arg("--unsafe-fixes")
        .run_with_stdin("y\n");

    assert!(output.stdout.contains("all_equal [unsafe]"));
    insta::assert_snapshot!(
        case.read_file("test.R")?,
        @"if (!isTRUE(all.equal(x, y))) 1"
    );

    Ok(())
}

#[test]
fn test_multibyte_content() -> anyhow::Result<()> {
    let case = clean_repo([("test.R", "x <- \"héllo\"\ny <- any(is.na(zé))\n")])?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--interactive")
        .run_with_stdin("y\n");

    assert!(output.stdout.contains("+y <- anyNA(zé)"));
    insta::assert_snapshot!(
        case.read_file("test.R")?,
        @r#"
    x <- "héllo"
    y <- anyNA(zé)
    "#
    );

    Ok(())
}

#[test]
fn test_dirty_repo_declined() -> anyhow::Result<()> {
    let case = clean_repo([("test.R", "x <- any(is.na(y))\n")])?;
    case.write_file("dirt.R", "z <- 1\n")?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--interactive")
        .run_with_stdin("n\n");

    assert!(
        output
            .stdout
            .contains("this project has uncommitted changes")
    );
    assert!(output.stdout.contains("Go through fixes anyway?"));
    // Saying no skips the fixes but still reports the violations.
    assert!(!output.stdout.contains("Apply this fix?"));
    assert!(output.stdout.contains("Found 1 error."));
    insta::assert_snapshot!(case.read_file("test.R")?, @"x <- any(is.na(y))");

    Ok(())
}

#[test]
fn test_dirty_repo_accepted() -> anyhow::Result<()> {
    let case = clean_repo([("test.R", "x <- any(is.na(y))\n")])?;
    case.write_file("dirt.R", "z <- 1\n")?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--interactive")
        .run_with_stdin("y\ny\n");

    assert!(output.stdout.contains("Go through fixes anyway?"));
    insta::assert_snapshot!(case.read_file("test.R")?, @"x <- anyNA(y)");

    Ok(())
}

#[test]
fn test_allow_dirty_skips_the_question() -> anyhow::Result<()> {
    let case = clean_repo([("test.R", "x <- any(is.na(y))\n")])?;
    case.write_file("dirt.R", "z <- 1\n")?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--interactive")
        .arg("--allow-dirty")
        .run_with_stdin("y\n");

    assert!(!output.stdout.contains("Go through fixes anyway?"));
    insta::assert_snapshot!(case.read_file("test.R")?, @"x <- anyNA(y)");

    Ok(())
}

#[test]
fn test_no_vcs_asks() -> anyhow::Result<()> {
    let case = CliTest::with_file("test.R", "x <- any(is.na(y))\n")?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--interactive")
        .run_with_stdin("y\ny\n");

    assert!(
        output
            .stdout
            .contains("no Version Control System (e.g. Git) was found")
    );
    insta::assert_snapshot!(case.read_file("test.R")?, @"x <- anyNA(y)");

    Ok(())
}

#[test]
fn test_rmd_is_never_fixed() -> anyhow::Result<()> {
    let case = clean_repo([(
        "test.Rmd",
        "---\ntitle: t\n---\n\n```{r}\nany(is.na(x))\n```\n",
    )])?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--interactive")
        .run_with_stdin("y\n");

    assert!(!output.stdout.contains("Apply this fix?"));

    Ok(())
}

#[test]
fn test_generated_file_is_skipped() -> anyhow::Result<()> {
    let case = clean_repo([("test.R", "# Generated by me\nany(is.na(x))\n")])?;

    let output = case
        .command()
        .arg("check")
        .arg(".")
        .arg("--interactive")
        .run_with_stdin("y\n");

    assert!(!output.stdout.contains("Apply this fix?"));

    Ok(())
}
