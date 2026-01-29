pub(crate) mod blanket_suppression;

// #[cfg(test)]
// mod tests {
//     use crate::utils_test::*;

//     #[test]
//     fn test_no_lint_blanket_suppression() {
//         // We don't handle those args
//         expect_no_lint(
//             "expect_equal(class(x), 'a', info = 'x should have class k')",
//             "blanket_suppression",
//             None,
//         );
//         expect_no_lint(
//             "expect_equal(class(x), 'a', label = 'x class')",
//             "blanket_suppression",
//             None,
//         );
//         expect_no_lint(
//             "expect_equal(class(x), 'a', expected.label = 'target class')",
//             "blanket_suppression",
//             None,
//         );

//         // Those do not work in `blanket_suppression()`.
//         expect_no_lint(
//             "expect_equal(class(x), 'list')",
//             "blanket_suppression",
//             None,
//         );
//         expect_no_lint(
//             "expect_equal(class(x), 'logical')",
//             "blanket_suppression",
//             None,
//         );
//         expect_no_lint(
//             "expect_equal(class(x), 'matrix')",
//             "blanket_suppression",
//             None,
//         );

//         // Not sure if those should be fixed here because if it's an object then
//         // it could contain classes that don't work in `blanket_suppression()`.
//         expect_no_lint("expect_equal(class(x), k)", "blanket_suppression", None);
//         expect_no_lint(
//             "expect_equal(class(x), c('a', 'b')",
//             "blanket_suppression",
//             None,
//         );

//         // Wrong code but no panic
//         expect_no_lint("expect_equal(class(x))", "blanket_suppression", None);
//         expect_no_lint("expect_equal(class())", "blanket_suppression", None);
//         expect_no_lint(
//             "expect_equal(object =, expected =)",
//             "blanket_suppression",
//             None,
//         );
//     }

//     #[test]
//     fn test_lint_blanket_suppression() {
//         use insta::assert_snapshot;
//         let lint_msg = "may fail if `x` gets more classes in the future";

//         expect_lint(
//             "expect_equal(class(x), 'data.frame')",
//             lint_msg,
//             "blanket_suppression",
//             None,
//         );
//         expect_lint(
//             "expect_equal(class(x), \"data.frame\")",
//             lint_msg,
//             "blanket_suppression",
//             None,
//         );
//         expect_lint(
//             "testthat::expect_equal(class(x), 'data.frame')",
//             lint_msg,
//             "blanket_suppression",
//             None,
//         );
//         expect_lint(
//             "expect_equal('data.frame', class(x))",
//             lint_msg,
//             "blanket_suppression",
//             None,
//         );
//         assert_snapshot!(
//             "fix_output",
//             get_fixed_text(
//                 vec![
//                     "expect_equal(class(x), 'data.frame')",
//                     "expect_equal(class(x), \"data.frame\")",
//                     "testthat::expect_equal(class(x), 'data.frame')",
//                     "expect_equal('data.frame', class(x))",
//                 ],
//                 "blanket_suppression",
//                 None,
//             )
//         );
//     }

//     #[test]
//     fn test_blanket_suppression_with_comments_no_fix() {
//         use insta::assert_snapshot;
//         // Should detect lint but skip fix when comments are present
//         expect_lint(
//             "expect_equal(class(x),\n # a comment \n'data.frame')",
//             "may fail if `x` gets more classes in the future",
//             "blanket_suppression",
//             None,
//         );
//         assert_snapshot!(
//             "no_fix_with_comments",
//             get_fixed_text(
//                 vec![
//                     "# leading comment\nexpect_equal(class(x), 'data.frame')",
//                     "expect_equal(class(x),\n # a comment \n'data.frame')",
//                     "expect_equal(class(x), 'data.frame') # trailing comment",
//                 ],
//                 "blanket_suppression",
//                 None
//             )
//         );
//     }
// }
