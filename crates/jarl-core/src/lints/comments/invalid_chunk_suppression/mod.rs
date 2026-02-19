pub(crate) mod invalid_chunk_suppression;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "invalid_chunk_suppression", None)
    }

    #[test]
    fn test_pipe_single_line_form_is_flagged() {
        // `#| jarl-ignore-chunk <rule>: <reason>` must be reported.
        insta::assert_snapshot!(snapshot_lint(
            "#| jarl-ignore-chunk any_is_na: legacy code\nany(is.na(x))\n"
        ), @r"
        warning: invalid_chunk_suppression
         --> <test>:1:1
          |
        1 | #| jarl-ignore-chunk any_is_na: legacy code
          | ------------------------------------------- This `jarl-ignore-chunk` comment is wrongly formatted.
          |
          = help: Use the YAML array form instead:
                  #| jarl-ignore-chunk:
                  #|   - <rule>: <reason>
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_hash_single_line_form_is_flagged() {
        // `# jarl-ignore-chunk <rule>: <reason>` must also be reported.
        insta::assert_snapshot!(snapshot_lint(
            "# jarl-ignore-chunk any_is_na: legacy code\nany(is.na(x))\n"
        ), @r"
        warning: invalid_chunk_suppression
         --> <test>:1:1
          |
        1 | # jarl-ignore-chunk any_is_na: legacy code
          | ------------------------------------------ This `jarl-ignore-chunk` comment is wrongly formatted.
          |
          = help: Use the YAML array form instead:
                  #| jarl-ignore-chunk:
                  #|   - <rule>: <reason>
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_yaml_array_form_not_flagged() {
        // The correct YAML array form must not trigger this rule.
        insta::assert_snapshot!(snapshot_lint(
            "any(is.na(x))\n"
        ), @"All checks passed!");
    }
}
