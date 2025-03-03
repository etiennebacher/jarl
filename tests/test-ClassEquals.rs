mod common;
use common::*;

// #[test]
// fn test_lint_class_equals() {
//     use insta::assert_snapshot;
//     let (lint_output, fix_output) = get_lint_and_fix_text(vec![
//         "is_regression <- class(x) == 'lm'",
//         "if (class(x) == 'character') 1",
//         "is_regression <- 'lm' == class(x)",
//         "is_regression <- \"lm\" == class(x)",
//         // TODO: those two should fix
//         "if ('character' %in% class(x)) 1",
//         "if (class(x) %in% 'character') 1",
//         "if (class(x) != 'character') 1",
//         "x[if (class(x) == 'foo') 1 else 2]",
//         "class(foo(bar(y) + 1)) == 'abc'",
//     ]);
//     assert_snapshot!("lint_output", lint_output);
//     assert_snapshot!("fix_output", fix_output);
// }

// #[test]
// fn test_no_lint_class_equals() {
//     assert!(no_lint("class(x) <- 'character'"));
//     assert!(no_lint("class(x) = 'character'"));
//     assert!(no_lint("identical(class(x), c('glue', 'character'))"));
//     assert!(no_lint("all(sup %in% class(model))"));
//     assert!(no_lint("class(x)[class(x) == 'foo']"));
// }
