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

    let old_length = old_content.chars().count() as i32;
    let mut new_length = old_length;

    for fix in fixes {
        // Skip overlapping fixes; they'll be handled in the next iteration.
        if fix.start < last_original_end {
            continue;
        }

        let diff_length = new_length - old_length;
        let start = (fix.start as i32 + diff_length) as usize;
        let end = (fix.end as i32 + diff_length) as usize;

        new_content.replace_range(start..end, &fix.content);
        new_length = new_content.chars().count() as i32;
        last_original_end = fix.end;
    }

    new_content
}
