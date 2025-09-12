use crate::diagnostic::*;

/// Takes all diagnostics found in a given file and the content of this file,
/// and applies automatic fixes.
///
/// This returns a boolean indicating whether some fixes were skipped (more on
/// this below), and a String with the modified content.
///
/// ## Overlapping fixes
///
/// When several fixes have overlapping ranges, it becomes hard to apply all of
/// them in a single pass.
/// TODO: it should still be possible since we know the length of the fix so we
/// should be able to update the range of the subsequent fixes. After all this
/// is what I do to update the range of following non-overlapping fixes. The
/// approach below looks quite expensive. This is also how Ruff does it though:
/// https://github.com/astral-sh/ruff/blob/main/crates/ruff_linter/src/linter.rs#L559
///
/// Therefore, the current approach is to signal to the caller function that
/// some fixes were skipped. This caller function then takes care of removing
/// from the list of diagnostics those that have already been addressed, and
/// then re-runs the diagnostic detection to get the new ranges. This loop
/// continues until there are no more skipped fixes.
pub fn apply_fixes(fixes: &[Diagnostic], contents: &str) -> (bool, String) {
    let fixes = fixes
        .iter()
        .map(|diagnostic| &diagnostic.fix)
        .collect::<Vec<_>>();

    let old_content = contents;
    let mut new_content = old_content.to_string();
    let mut last_modified_pos = 0;
    let mut has_skipped_fixes = false;

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

        // We don't know how to handle these cases in a single pass yet.
        if start < last_modified_pos {
            if !has_skipped_fixes {
                has_skipped_fixes = true;
            }
            continue;
        }

        let start_usize = start as usize;
        let end_usize = end as usize;

        new_content.replace_range(start_usize..end_usize, &fix.content);
        new_length = new_content.chars().count() as i32;
        last_modified_pos = end + diff_length;
    }

    (has_skipped_fixes, new_content.to_string())
}
