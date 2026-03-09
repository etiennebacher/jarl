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
    let mut last_modified_pos = 0;

    let old_length = old_content.chars().count() as i32;
    let mut new_length = old_length;

    for fix in fixes {
        let mut start: i32 = fix.start.try_into().unwrap();
        let mut end: i32 = fix.end.try_into().unwrap();

        // Adjust the range of the fix based on the changes in the contents due
        // to previous fixes.
        let diff_length = new_length - old_length;
        start += diff_length;
        end += diff_length;

        // Skip overlapping fixes; they'll be handled in the next iteration.
        if start < last_modified_pos {
            continue;
        }

        let start_usize = start as usize;
        let end_usize = end as usize;

        new_content.replace_range(start_usize..end_usize, &fix.content);
        new_length = new_content.chars().count() as i32;
        last_modified_pos = end;
    }

    new_content
}
