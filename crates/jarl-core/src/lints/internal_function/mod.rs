pub(crate) mod internal_function;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_internal_function() {
        expect_no_lint("foo::bar()", "internal_function", None);
        expect_no_lint("foo::bar", "internal_function", None);
    }

    #[test]
    fn test_lint_internal_function() {
        expect_lint(
            "foo:::bar()",
            "Use public functions via `::` instead",
            "internal_function",
            None,
        );
        expect_lint(
            "foo:::bar",
            "Use public functions via `::` instead",
            "internal_function",
            None,
        );
    }
}
