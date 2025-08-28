use crate::diagnostic::*;

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

        let diff_length = new_length - old_length;

        start += diff_length;
        end += diff_length;

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
