pub(crate) mod empty_assignment;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_lint_empty_assignment() {
        let expected_message = "Assign NULL explicitly or";

        expect_lint("x <- {}", expected_message, "empty_assignment");
        expect_lint("x = { }", expected_message, "empty_assignment");
        expect_lint("{ } -> x", expected_message, "empty_assignment");
        expect_lint("x <- {\n}", expected_message, "empty_assignment");
        expect_lint("env$obj <- {}", expected_message, "empty_assignment");
    }

    #[test]
    fn test_no_lint_empty_assignment() {
        expect_no_lint("x <- { 3 + 4 }", "empty_assignment");
        expect_no_lint("x = if (x > 1) { 3 + 4 }", "empty_assignment");
        expect_no_lint("{ 3 + 4 } -> x", "empty_assignment");
        expect_no_lint("x <- function() { }", "empty_assignment");
    }
}
