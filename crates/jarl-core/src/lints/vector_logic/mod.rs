pub(crate) mod vector_logic;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    #[test]
    fn test_no_lint_vector_logic() {
        expect_no_lint("if (x && y) 1", "vector_logic", None);
        expect_no_lint("while (x && y) 1", "vector_logic", None);
        expect_no_lint("if (agg_function(x & y)) 1", "vector_logic", None);
        expect_no_lint("if (DT[x | y, cond]) 1", "vector_logic", None);
        expect_no_lint("if (TRUE && any(TRUE | FALSE)) 1", "vector_logic", None);
    }

    #[test]
    fn test_lint_vector_logic() {
        expect_lint(
            "if (TRUE & FALSE) 1",
            "can lead to conditions of length > 1",
            "vector_logic",
            None,
        );
        expect_lint(
            "if (TRUE | FALSE) 1",
            "can lead to conditions of length > 1",
            "vector_logic",
            None,
        );
        expect_lint(
            "if (TRUE | FALSE & TRUE) 1",
            "can lead to conditions of length > 1",
            "vector_logic",
            None,
        );
        expect_lint(
            "while (TRUE & FALSE) 1",
            "can lead to conditions of length > 1",
            "vector_logic",
            None,
        );
        expect_lint(
            "while (TRUE | FALSE) 1",
            "can lead to conditions of length > 1",
            "vector_logic",
            None,
        );
        expect_lint(
            "if ((x > 1) & (y < 2)) 1",
            "can lead to conditions of length > 1",
            "vector_logic",
            None,
        );
    }
}
