pub(crate) mod invalid_chunk_suppression;

#[cfg(test)]
mod tests {
    use crate::utils_test::*;

    fn snapshot_lint(code: &str) -> String {
        format_diagnostics(code, "invalid_chunk_suppression", None)
    }

    #[test]
    fn test_single_line_form_is_flagged() {
        // The old `#| jarl-ignore-chunk <rule>: <reason>` form should be reported.
        insta::assert_snapshot!(snapshot_lint(
            "#| jarl-ignore-chunk any_is_na: legacy code\nany(is.na(x))\n"
        ), @r"
        warning: invalid_chunk_suppression
         --> <test>:1:1
          |
        1 | #| jarl-ignore-chunk any_is_na: legacy code
          | ------------------------------------------- This `#| jarl-ignore-chunk` comment uses the single-line form.
          |
          = help: Use the YAML array form instead:
                  #| jarl-ignore-chunk:
                  #|   - <rule>: <reason>
        Found 1 error.
        "
        );
    }

    #[test]
    fn test_regular_hash_form_not_flagged() {
        // `# jarl-ignore-chunk <rule>: <reason>` (without `|`) is still valid.
        insta::assert_snapshot!(snapshot_lint(
            "# jarl-ignore-chunk any_is_na: legacy code\nany(is.na(x))\n"
        ), @"All checks passed!");
    }

    #[test]
    fn test_yaml_array_form_not_flagged() {
        // The correct YAML array form must not trigger this rule.
        // (The header line is intercepted before reaching `process_comment`.)
        insta::assert_snapshot!(snapshot_lint(
            "any(is.na(x))\n"
        ), @"All checks passed!");
    }
}
