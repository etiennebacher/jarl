mod common;
use common::*;

// #[test]
// fn test_lint_any_na() {
//     use insta::assert_snapshot;
//     let (lint_output, fix_output) = get_lint_and_fix_text(vec![
//         "any(is.na(x))",
//         "any(is.na(foo(x)))",
//         "any(is.na(x), na.rm = TRUE)",
//     ]);
//     assert_snapshot!("lint_output", lint_output);
//     assert_snapshot!("fix_output", fix_output);
// }

// #[test]
// fn test_no_lint_any_na() {
//     assert!(no_lint("y <- any(x)"));
//     assert!(no_lint("y <- is.na(x)"));
//     assert!(no_lint("y <- any(!is.na(x))"));
//     assert!(no_lint("y <- any(!is.na(foo(x)))"))
// }
