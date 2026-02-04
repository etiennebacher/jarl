use crate::diagnostic::*;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct ForLoopDupIndex;

/// ## What it does
///
/// Checks whether the index symbol in a `for` loop is already used in a parent
/// `for` loop.
///
/// ## Why is this bad?
///
/// In nested loops, using the same index symbol for several loops can lead to
/// unexpected and incorrect results.
///
/// This rules doesn't have an automatic fix.
///
/// ## Example
///
/// ```r
/// for (x in 1:3) {
///   for (x in 1:4) {
///     print(x + 1)
///   }
/// }
/// ```
///
/// ```r
/// for (x_outer in 1:3) {
///   for (x_inner in 1:4) {
///     print(x_inner + 1)
///   }
/// }
/// ```
impl Violation for ForLoopDupIndex {
    fn name(&self) -> String {
        "for_loop_dup_index".to_string()
    }
    fn body(&self) -> String {
        "This index variable is already used in a parent `for` loop.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Rename this index variable to avoid unexpected results.".to_string())
    }
}

pub fn for_loop_dup_index(ast: &RForStatement) -> anyhow::Result<Option<Diagnostic>> {
    let index = ast.variable()?.to_trimmed_string();

    let has_duplicate_in_ancestor = ast
        .syntax()
        .ancestors()
        // Skip self, otherwise the `for` loop compares to itself.
        .skip(1)
        .filter_map(RForStatement::cast)
        .any(|ancestor_for| {
            ancestor_for
                .variable()
                .map(|v| v.to_trimmed_string() == index)
                .unwrap_or(false)
        });

    if has_duplicate_in_ancestor {
        let range_start = ast.variable()?.range().start();
        let range_end = ast.sequence()?.range().end();
        let range = TextRange::new(range_start, range_end);
        let diagnostic = Diagnostic::new(ForLoopDupIndex, range, Fix::empty());
        Ok(Some(diagnostic))
    } else {
        Ok(None)
    }
}
