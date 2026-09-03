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

/// What the user decided about a single fix in interactive mode.
pub enum FixDecision {
    Accept,
    Skip,
    Quit,
}

/// Asks the user whether to apply each fix. Implemented by the CLI, which owns
/// all terminal I/O; the core only calls back into it.
pub trait FixPrompt {
    fn ask(
        &mut self,
        path: &str,
        contents: &str,
        diagnostic: &Diagnostic,
    ) -> anyhow::Result<FixDecision>;

    /// Asked once when the working tree is dirty or untracked. `false` means
    /// no fix is offered for the rest of the run.
    fn confirm_vcs(&mut self, status: &crate::vcs::VcsStatus) -> anyhow::Result<bool>;

    /// True once the user quit, so remaining files fall back to lint-only.
    fn aborted(&self) -> bool {
        false
    }
}
