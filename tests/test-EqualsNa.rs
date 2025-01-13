mod common;
use common::*;

#[test]
fn test_lint_equals_na() {
    use insta::assert_snapshot;
    let (lint_output, fix_output) = get_lint_and_fix_text(
        "x == NA
x != NA
foo(x(y)) == NA
",
    );
    assert_snapshot!("lint_output", lint_output);
    assert_snapshot!("fix_output", fix_output);
}

#[test]
fn test_no_lint_equals_na() {
    assert!(no_lint("x + NA"));
    assert!(no_lint("x == \"NA\""));
    assert!(no_lint("x == 'NA'"));
}
