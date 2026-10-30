use crate::diagnostic::*;

/// Takes all diagnostics found in a given file and the content of this file,
/// and applies automatic fixes.
///
/// A fix is all-or-nothing: it is applied only if none of its edits touches an
/// edit already accepted from an earlier fix. Fixes that do conflict are left
/// out rather than applied on stale offsets; the caller is expected to re-lint
/// and re-apply until the content stabilizes (no more fixable diagnostics or no
/// progress made).
pub fn apply_fixes(diagnostics: &[Diagnostic], contents: &str) -> String {
    let mut candidates: Vec<&Diagnostic> = diagnostics
        .iter()
        .filter(|diagnostic| !diagnostic.fix.to_skip && !diagnostic.fix.edits.is_empty())
        .collect();

    // Diagnostics reach us in reporting order, which is neither sorted nor
    // stable across rules. Order the fixes so that conflicts are resolved in
    // favour of the earliest one in the file, and so that two runs on the same
    // input make the same choices.
    candidates.sort_by(|a, b| {
        a.fix
            .start()
            .cmp(&b.fix.start())
            .then_with(|| a.range.start().cmp(&b.range.start()))
            .then_with(|| a.range.end().cmp(&b.range.end()))
            .then_with(|| a.message.rule.name().cmp(b.message.rule.name()))
    });

    let mut accepted: Vec<&Edit> = Vec::new();
    for diagnostic in candidates {
        let edits = &diagnostic.fix.edits;
        if edits
            .iter()
            .any(|edit| accepted.iter().any(|other| conflicts(edit, other)))
        {
            continue;
        }
        accepted.extend(edits);
    }

    // Splicing back-to-front keeps every remaining offset valid, so no offset
    // arithmetic is needed. At a shared start offset the widest edit goes
    // first, which puts an insertion in front of the replacement it abuts.
    accepted.sort_by(|a, b| b.start().cmp(&a.start()).then(b.end().cmp(&a.end())));

    let mut new_content = contents.to_string();
    for edit in accepted {
        new_content.replace_range(edit.start()..edit.end(), &edit.content);
    }

    new_content
}

/// Whether two edits cannot be applied in the same pass.
///
/// Replacements conflict when their ranges intersect. An insertion is an empty
/// range, so it conflicts with a replacement only when it lands strictly inside
/// it — inserting at a boundary is well defined. Two insertions at the same
/// offset conflict because the order between them would be arbitrary.
fn conflicts(a: &Edit, b: &Edit) -> bool {
    match (a.range.is_empty(), b.range.is_empty()) {
        (true, true) => a.start() == b.start(),
        (true, false) => b.start() < a.start() && a.start() < b.end(),
        (false, true) => a.start() < b.start() && b.start() < a.end(),
        (false, false) => a.start() < b.end() && b.start() < a.end(),
    }
}
