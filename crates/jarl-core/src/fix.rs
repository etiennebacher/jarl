use crate::diagnostic::*;

/// Takes all diagnostics found in a given file and the content of this file,
/// and applies automatic fixes.
///
/// Overlapping fixes are skipped rather than applied, since adjusting their
/// ranges in a single pass is error-prone. The caller is expected to re-lint
/// and re-apply until the content stabilizes (no more fixable diagnostics or
/// no progress made).
pub fn apply_fixes(fixes: &[Diagnostic], contents: &str) -> String {
    let fixes = fixes
        .iter()
        .map(|diagnostic| &diagnostic.fix)
        .collect::<Vec<_>>();

    let old_content = contents;
    let mut new_content = old_content.to_string();
    // Track the end of the last applied fix in original positions so that
    // overlap detection works even when earlier fixes change the content
    // length.
    let mut last_original_end: usize = 0;

    // fix.start()/end() are byte offsets into the source, so the shift applied
    // to later fixes must be a byte delta; a char-count delta misplaces them as
    // soon as an earlier fix touches multi-byte characters.
    let old_length = old_content.len() as i32;
    let mut new_length = old_length;

    for fix in fixes {
        // Skip overlapping fixes; they'll be handled in the next iteration.
        if fix.start() < last_original_end {
            continue;
        }

        let diff_length = new_length - old_length;
        let start = (fix.start() as i32 + diff_length) as usize;
        let end = (fix.end() as i32 + diff_length) as usize;

        new_content.replace_range(start..end, &fix.content);
        new_length = new_content.len() as i32;
        last_original_end = fix.end();
    }

    new_content
}

#[cfg(test)]
mod tests {
    use super::apply_fixes;
    use crate::diagnostic::{Diagnostic, Fix, ViolationData};
    use crate::rule_set::Rule;
    use biome_rowan::{TextRange, TextSize};

    fn fix_at(start: usize, end: usize, content: &str) -> Diagnostic {
        let range = TextRange::new(TextSize::from(start as u32), TextSize::from(end as u32));
        Diagnostic::new(
            ViolationData::new(Rule::AnyIsNa, "test violation".to_string(), None),
            range,
            Fix::new(range, content.to_string(), false),
        )
    }

    #[test]
    fn later_fix_shifted_correctly_when_earlier_fix_drops_multibyte_text() {
        let content = "分数[order(分数)]\nany(is.na(\"数据\"))\n";
        let fixes = [
            fix_at(0, 21, "sort(分数)"),
            fix_at(22, 42, "anyNA(\"数据\")"),
        ];

        assert_eq!(
            apply_fixes(&fixes, content),
            "sort(分数)\nanyNA(\"数据\")\n"
        );
    }

    #[test]
    fn removing_multibyte_comment_line_keeps_later_fix_aligned() {
        let content = "# jarl-ignore seq: 原因一\nx <- 1\nany(is.na(y))\n";
        let fixes = [fix_at(0, 29, ""), fix_at(36, 49, "anyNA(y)")];

        assert_eq!(apply_fixes(&fixes, content), "x <- 1\nanyNA(y)\n");
    }
}
