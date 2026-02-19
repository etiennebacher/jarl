pub mod extraction;
pub use extraction::{RCodeChunk, extract_r_chunks};

#[cfg(test)]
mod tests {
    use std::fs;

    use air_workspace::resolve::PathResolver;
    use tempfile::Builder;

    use crate::check::get_checks;
    use crate::config::{ArgsConfig, build_config};
    use crate::diagnostic::Diagnostic;
    use crate::settings::Settings;

    /// Run `get_checks` on a temporary `.Rmd` file with the default rule set.
    fn check_rmd(content: &str) -> Vec<Diagnostic> {
        let temp_file = Builder::new()
            .prefix("test-jarl")
            .suffix(".Rmd")
            .tempfile()
            .unwrap();
        fs::write(&temp_file, content).expect("Failed to write content");

        let path = temp_file.path().to_path_buf();
        let check_config = ArgsConfig {
            files: vec![path.clone()],
            fix: false,
            unsafe_fixes: false,
            fix_only: false,
            select: String::new(),
            extend_select: String::new(),
            ignore: String::new(),
            min_r_version: None,
            allow_dirty: false,
            allow_no_vcs: true,
            assignment: None,
        };

        let resolver = PathResolver::new(Settings::default());
        let config = build_config(&check_config, &resolver, vec![path.clone()])
            .expect("Failed to build config");

        get_checks(content, &path, &config).expect("get_checks failed")
    }

    // --- Lint detection ---

    #[test]
    fn test_lint_fires_with_correct_line_number() {
        // The lint is inside a chunk that starts after a 5-line YAML header.
        // Line 1-3: YAML, line 4: blank, line 5: fence, line 6: code.
        let content = "---\ntitle: \"Test\"\n---\n\n```{r}\nany(is.na(x))\n```\n";
        let diagnostics = check_rmd(content);

        let any_is_na: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "any_is_na")
            .collect();
        assert_eq!(
            any_is_na.len(),
            1,
            "expected exactly one any_is_na diagnostic"
        );

        let loc = any_is_na[0]
            .location
            .as_ref()
            .expect("location should be set");
        assert_eq!(
            loc.row(),
            6,
            "diagnostic should be on line 6 of the original Rmd"
        );
        assert_eq!(loc.column(), 0);
    }

    #[test]
    fn test_no_lint_for_clean_chunk() {
        let content = "```{r}\nx <- 1\n```\n";
        let diagnostics = check_rmd(content);
        let violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "any_is_na")
            .collect();
        assert!(violations.is_empty());
    }

    // --- Per-rule suppression ---

    #[test]
    fn test_pipe_suppression_works() {
        // `#| jarl-ignore rule: reason` should suppress that rule in the chunk.
        let content = "```{r}\n#| jarl-ignore any_is_na: legacy code\nany(is.na(x))\n```\n";
        let diagnostics = check_rmd(content);
        let violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "any_is_na")
            .collect();
        assert!(
            violations.is_empty(),
            "#| jarl-ignore should suppress any_is_na"
        );
    }

    #[test]
    fn test_ignore_chunk_with_rule_suppresses_rule() {
        // `#| jarl-ignore-chunk rule: reason` should suppress `rule` for the
        // expression it precedes, just like `# jarl-ignore rule: reason`.
        let content = "```{r}\n#| jarl-ignore-chunk any_is_na: legacy code\nany(is.na(x))\n```\n";
        let diagnostics = check_rmd(content);
        let violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "any_is_na")
            .collect();
        assert!(
            violations.is_empty(),
            "#| jarl-ignore-chunk rule: reason should suppress any_is_na"
        );
    }

    #[test]
    fn test_ignore_chunk_without_rule_does_not_suppress() {
        // `#| jarl-ignore-chunk` without a rule name must NOT suppress any
        // diagnostic. It is a blanket suppression and should leave any_is_na
        // visible while itself triggering a BlanketSuppression lint.
        let content = "```{r}\n#| jarl-ignore-chunk\nany(is.na(x))\n```\n";
        let diagnostics = check_rmd(content);
        let violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "any_is_na")
            .collect();
        assert!(
            !violations.is_empty(),
            "#| jarl-ignore-chunk without a rule should not suppress any_is_na"
        );
    }

    #[test]
    fn test_hash_suppression_works() {
        // `# jarl-ignore rule: reason` (standard form) should also work in Rmd chunks.
        let content = "```{r}\n# jarl-ignore any_is_na: legacy code\nany(is.na(x))\n```\n";
        let diagnostics = check_rmd(content);
        let violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "any_is_na")
            .collect();
        assert!(
            violations.is_empty(),
            "# jarl-ignore should suppress any_is_na in Rmd"
        );
    }

    // --- Parse errors ---

    #[test]
    fn test_parse_error_chunk_silently_skipped() {
        // A chunk with a syntax error should be skipped without surfacing a ParseError.
        let content = "```{r}\n1 +\n```\n";
        // get_checks should succeed (Ok), not return a ParseError.
        let temp_file = Builder::new()
            .prefix("test-jarl")
            .suffix(".Rmd")
            .tempfile()
            .unwrap();
        fs::write(&temp_file, content).unwrap();
        let path = temp_file.path().to_path_buf();
        let check_config = ArgsConfig {
            files: vec![path.clone()],
            fix: false,
            unsafe_fixes: false,
            fix_only: false,
            select: String::new(),
            extend_select: String::new(),
            ignore: String::new(),
            min_r_version: None,
            allow_dirty: false,
            allow_no_vcs: true,
            assignment: None,
        };
        let resolver = PathResolver::new(Settings::default());
        let config = build_config(&check_config, &resolver, vec![path.clone()]).unwrap();
        let result = get_checks(content, &path, &config);
        assert!(
            result.is_ok(),
            "parse error in a chunk should not bubble up as Err"
        );
        assert!(result.unwrap().is_empty());
    }

    // --- Non-R blocks skipped ---

    #[test]
    fn test_display_block_not_linted() {
        // ```r without braces is a display block and should not be linted.
        let content = "```r\nany(is.na(x))\n```\n";
        let diagnostics = check_rmd(content);
        let violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "any_is_na")
            .collect();
        assert!(violations.is_empty());
    }

    // --- File-level suppression across chunks ---

    #[test]
    fn test_ignore_file_applies_cross_chunk() {
        // jarl-ignore-file in the first chunk should suppress the rule in all chunks.
        let content = concat!(
            "```{r}\n",
            "# jarl-ignore-file any_is_na: whole document\n",
            "any(is.na(x))\n",
            "```\n",
            "\n",
            "```{r}\n",
            "any(is.na(y))\n",
            "```\n",
        );
        let diagnostics = check_rmd(content);
        let violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "any_is_na")
            .collect();
        assert!(
            violations.is_empty(),
            "jarl-ignore-file should suppress across all chunks"
        );
    }

    // --- No autofix ---

    #[test]
    fn test_no_fix_applied() {
        // All diagnostics from Rmd files must have Fix::empty() (to_skip = true).
        let content = "```{r}\nany(is.na(x))\n```\n";
        let diagnostics = check_rmd(content);
        for diag in &diagnostics {
            assert!(
                diag.fix.to_skip,
                "fix.to_skip must be true for Rmd diagnostic '{}'",
                diag.message.name
            );
        }
    }
}
