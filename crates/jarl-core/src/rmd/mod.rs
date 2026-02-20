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
    fn test_pipe_suppression_not_supported() {
        // `#| jarl-ignore rule: reason` is not a valid suppression comment in
        // Quarto/Rmd: the `#|` prefix is only recognised for `jarl-ignore-chunk`.
        // The directive is silently ignored and the violation is still reported.
        let content = "```{r}\n#| jarl-ignore any_is_na: legacy code\nany(is.na(x))\n```\n";
        let diagnostics = check_rmd(content);
        let violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "any_is_na")
            .collect();
        assert!(
            !violations.is_empty(),
            "#| jarl-ignore must not suppress any_is_na"
        );
    }

    #[test]
    fn test_ignore_chunk_with_rule_suppresses_rule() {
        // The YAML array form should suppress `rule` for ALL expressions in the
        // chunk, not just the next one.
        let content = concat!(
            "```{r}\n",
            "#| jarl-ignore-chunk:\n",
            "#|   - any_is_na: legacy code\n",
            "any(is.na(x))\n",
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
            "YAML array form should suppress any_is_na for the entire chunk"
        );
    }

    #[test]
    fn test_ignore_chunk_does_not_cross_chunk() {
        // A chunk suppression in chunk 1 must NOT suppress violations in chunk 2.
        let content = concat!(
            "```{r}\n",
            "#| jarl-ignore-chunk:\n",
            "#|   - any_is_na: only in this chunk\n",
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
        assert_eq!(
            violations.len(),
            1,
            "chunk suppression should not suppress violations in a different chunk"
        );
    }

    #[test]
    fn test_ignore_chunk_anywhere_in_chunk() {
        // The directive should work even when placed after some expressions,
        // not just at the very top of the chunk.
        let content = concat!(
            "```{r}\n",
            "x <- 1\n",
            "#| jarl-ignore-chunk:\n",
            "#|   - any_is_na: legacy\n",
            "any(is.na(x))\n",
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
            "chunk suppression should suppress even when not at the top of the chunk"
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

    // --- invalid_chunk_suppression: single-line form ---

    #[test]
    fn test_single_line_form_triggers_invalid_chunk_suppression() {
        // `#| jarl-ignore-chunk rule: reason` is not a valid suppression.
        // It fires `invalid_chunk_suppression` and does NOT suppress `any_is_na`.
        let content = "```{r}\n#| jarl-ignore-chunk any_is_na: legacy\nany(is.na(x))\n```\n";
        let diagnostics = check_rmd(content);
        // any_is_na must NOT be suppressed.
        let violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "any_is_na")
            .collect();
        assert!(
            !violations.is_empty(),
            "single-line #| form must not suppress any_is_na"
        );
        // invalid_chunk_suppression fires.
        let warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "invalid_chunk_suppression")
            .collect();
        assert_eq!(
            warnings.len(),
            1,
            "single-line #| form should trigger invalid_chunk_suppression"
        );
    }

    #[test]
    fn test_regular_hash_form_triggers_invalid_chunk_suppression() {
        // `# jarl-ignore-chunk rule: reason` is not a valid suppression:
        // jarl-ignore-chunk requires the YAML array form regardless of prefix.
        let content = "```{r}\n# jarl-ignore-chunk any_is_na: legacy\nany(is.na(x))\n```\n";
        let diagnostics = check_rmd(content);
        // Must NOT suppress any_is_na.
        let violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "any_is_na")
            .collect();
        assert!(!violations.is_empty(), "# form must not suppress any_is_na");
        // Must fire invalid_chunk_suppression.
        let warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "invalid_chunk_suppression")
            .collect();
        assert_eq!(
            warnings.len(),
            1,
            "# form should trigger invalid_chunk_suppression"
        );
    }

    // --- Quarto YAML array form (#| jarl-ignore-chunk: / #|   - rule: reason) ---

    #[test]
    fn test_ignore_chunk_yaml_array_suppresses_rule() {
        // The Quarto-idiomatic YAML array form should suppress the rule for the
        // entire chunk.
        let content = concat!(
            "```{r}\n",
            "#| jarl-ignore-chunk:\n",
            "#|   - any_is_na: legacy code\n",
            "any(is.na(x))\n",
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
            "YAML array form should suppress any_is_na for the entire chunk"
        );
    }

    #[test]
    fn test_ignore_chunk_yaml_array_no_items_is_blanket() {
        // `#| jarl-ignore-chunk:` with no following items should behave like a
        // blanket suppression (does not suppress any_is_na).
        let content = concat!(
            "```{r}\n",
            "#| jarl-ignore-chunk:\n",
            "any(is.na(x))\n",
            "```\n",
        );
        let diagnostics = check_rmd(content);
        let violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "any_is_na")
            .collect();
        assert!(
            !violations.is_empty(),
            "#| jarl-ignore-chunk: with no items should not suppress any_is_na"
        );
    }

    #[test]
    fn test_ignore_chunk_yaml_array_does_not_cross_chunk() {
        // YAML array suppression in chunk 1 must not affect chunk 2.
        let content = concat!(
            "```{r}\n",
            "#| jarl-ignore-chunk:\n",
            "#|   - any_is_na: only in this chunk\n",
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
        assert_eq!(
            violations.len(),
            1,
            "YAML array suppression should not cross chunk boundaries"
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

    #[test]
    fn test_ignore_file_in_first_chunk_no_violations_in_first_chunk() {
        // jarl-ignore-file in the first chunk (no code there) should suppress
        // the rule in other chunks and must NOT trigger outdated_suppression.
        let content = concat!(
            "```{r}\n",
            "# jarl-ignore-file any_is_na: whole document\n",
            "# jarl-ignore-file any_duplicated: whole document\n",
            "```\n",
            "\n",
            "```{r}\n",
            "any(is.na(1))\n",
            "any(duplicated(1))\n",
            "```\n",
        );
        let diagnostics = check_rmd(content);
        // Violations are suppressed.
        let violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "any_is_na" || d.message.name == "any_duplicated")
            .collect();
        assert!(
            violations.is_empty(),
            "jarl-ignore-file should suppress cross-chunk violations"
        );
        // No outdated_suppression should fire either.
        let outdated: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "outdated_suppression")
            .collect();
        assert!(
            outdated.is_empty(),
            "jarl-ignore-file used cross-chunk must not trigger outdated_suppression"
        );
    }

    #[test]
    fn test_ignore_file_in_second_chunk_is_misplaced() {
        // jarl-ignore-file in a non-first R chunk should trigger
        // misplaced_file_suppression and must NOT suppress any violations.
        let content = concat!(
            "```{r}\n",
            "x <- 1\n",
            "```\n",
            "\n",
            "```{r}\n",
            "# jarl-ignore-file any_is_na: should be misplaced\n",
            "any(is.na(1))\n",
            "```\n",
        );
        let diagnostics = check_rmd(content);
        // The violation must still be reported (suppression is misplaced).
        let violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "any_is_na")
            .collect();
        assert_eq!(
            violations.len(),
            1,
            "misplaced jarl-ignore-file must not suppress any_is_na"
        );
        // And misplaced_file_suppression must fire.
        let misplaced: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "misplaced_file_suppression")
            .collect();
        assert_eq!(
            misplaced.len(),
            1,
            "jarl-ignore-file in a non-first chunk must fire misplaced_file_suppression"
        );
    }

    #[test]
    fn test_ignore_file_after_code_in_first_chunk_is_misplaced() {
        // jarl-ignore-file that appears after code in the first chunk is misplaced.
        let content = concat!(
            "```{r}\n",
            "x <- 1\n",
            "# jarl-ignore-file any_is_na: should be misplaced\n",
            "any(is.na(1))\n",
            "```\n",
        );
        let diagnostics = check_rmd(content);
        let violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "any_is_na")
            .collect();
        assert_eq!(
            violations.len(),
            1,
            "misplaced jarl-ignore-file must not suppress any_is_na"
        );
        let misplaced: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "misplaced_file_suppression")
            .collect();
        assert_eq!(
            misplaced.len(),
            1,
            "jarl-ignore-file after code must fire misplaced_file_suppression"
        );
    }

    #[test]
    fn test_ignore_file_valid_when_first_r_chunk_follows_python_chunk() {
        // A Python chunk before the first R chunk does not affect validity:
        // jarl-ignore-file is still accepted in the first R chunk.
        let content = concat!(
            "```{python}\n",
            "x = 1\n",
            "```\n",
            "\n",
            "```{r}\n",
            "# jarl-ignore-file any_is_na: whole document\n",
            "```\n",
            "\n",
            "```{r}\n",
            "any(is.na(1))\n",
            "```\n",
        );
        let diagnostics = check_rmd(content);
        // Violation suppressed.
        let violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "any_is_na")
            .collect();
        assert!(
            violations.is_empty(),
            "jarl-ignore-file in first R chunk should suppress even when preceded by a Python chunk"
        );
        // No misplaced_file_suppression.
        let misplaced: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "misplaced_file_suppression")
            .collect();
        assert!(
            misplaced.is_empty(),
            "first R chunk is valid for jarl-ignore-file regardless of preceding non-R chunks"
        );
        // No outdated_suppression either.
        let outdated: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "outdated_suppression")
            .collect();
        assert!(
            outdated.is_empty(),
            "jarl-ignore-file used cross-chunk must not trigger outdated_suppression"
        );
    }

    #[test]
    fn test_ignore_file_truly_unused_still_reports_outdated() {
        // jarl-ignore-file in the first chunk where the suppressed rule has no
        // violations anywhere must still trigger outdated_suppression.
        let content = concat!(
            "```{r}\n",
            "# jarl-ignore-file any_is_na: whole document\n",
            "```\n",
            "\n",
            "```{r}\n",
            "x <- 1\n",
            "```\n",
        );
        let diagnostics = check_rmd(content);
        let outdated: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.name == "outdated_suppression")
            .collect();
        assert_eq!(
            outdated.len(),
            1,
            "truly unused jarl-ignore-file must still trigger outdated_suppression"
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
