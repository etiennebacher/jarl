use crate::message::*;

pub fn apply_fixes(fixes: &[Message], contents: &str) -> (bool, String) {
    let fixes = fixes
        .iter()
        .map(|msg| match msg {
            Message::AnyDuplicated { fix, .. }
            | Message::AnyIsNa { fix, .. }
            | Message::TrueFalseSymbol { fix, .. } => fix,
        })
        .collect::<Vec<_>>();
    let old_content = contents;
    let mut new_content = old_content.to_string();
    let mut diff_length = 0;
    let mut last_modified_pos = 0;
    let mut has_skipped_fixes = false;

    for fix in fixes {
        let mut start: i32 = fix.start.try_into().unwrap();
        let mut end: i32 = fix.end.try_into().unwrap();

        start += diff_length;
        end += diff_length;

        if start < last_modified_pos {
            if !has_skipped_fixes {
                has_skipped_fixes = true;
            }
            continue;
        }

        diff_length += fix.length_change;
        let start_usize = start as usize;
        let end_usize = end as usize;

        new_content.replace_range(start_usize..end_usize, &fix.content);
        last_modified_pos = end + diff_length;
    }

    (has_skipped_fixes, new_content.to_string())
}
