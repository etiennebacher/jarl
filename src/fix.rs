use crate::message::*;

pub fn apply_fixes(fixes: &[Message], contents: &str) -> String {
    let fixes = fixes.iter().map(|msg| msg.fix()).collect::<Vec<_>>();
    let old_content = contents;
    let mut new_content = old_content.to_string();
    let mut diff_length = 0;
    let mut last_modified_pos = 0;

    let mut unapplied_fixes: Vec<&Fix> = vec![];

    for fix in fixes {
        let mut start: i32 = fix.start.try_into().unwrap();
        let mut end: i32 = fix.end.try_into().unwrap();

        // println!("fix text: {}", fix.content);
        // println!("last_modified_pos: {}", last_modified_pos);
        // println!("original start: {}", start);
        // println!("original end: {}", end);
        // println!("diff_length: {}", diff_length);

        start += diff_length;
        end += diff_length;

        // println!("new start: {}", start);
        // println!("new end: {}", end);

        if start < last_modified_pos {
            unapplied_fixes.push(fix);
            continue;
        }

        // println!("diff_length: {}\n", diff_length);
        diff_length += fix.length_change;
        let start_usize = start as usize;
        let end_usize = end as usize;

        new_content.replace_range(start_usize..end_usize, &fix.content);
        last_modified_pos = end + diff_length;
    }

    // println!("{:?}", unapplied_fixes);

    // println!("fixed output:\n{}", new_content.to_string());
    // old_content.to_string()
    new_content.to_string()
}
