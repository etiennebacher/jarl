use crate::diagnostic::*;
use crate::rule_set::Rule;
use air_r_syntax::*;
use biome_rowan::AstNode;
use oak_core::syntax_ext::RIdentifierExt;

pub struct ForLoopDupIndex;

/// <!-- docs: start -->
/// Version added: 0.4.0
///
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
/// This rule doesn't have an automatic fix.
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
/// Use instead:
/// ```r
/// for (x_outer in 1:3) {
///   for (x_inner in 1:4) {
///     print(x_inner + 1)
///   }
/// }
/// ```
/// <!-- docs: end -->
impl Violation for ForLoopDupIndex {
    fn rule(&self) -> Rule {
        Rule::ForLoopDupIndex
    }
    fn body(&self) -> String {
        "This index variable is already used in a parent `for` loop.".to_string()
    }
    fn suggestion(&self) -> Option<String> {
        Some("Rename this index variable to avoid unexpected results.".to_string())
    }
}

pub fn for_loop_dup_index(ast: &RForStatement) -> anyhow::Result<Option<Diagnostic>> {
    let variable = ast.variable()?;
    let name = variable.name_text();
    let loop_range = ast.syntax().text_trimmed_range();

    // Skip self and stop at functions, whose loop variables have their own scope.
    for ancestor in ast.syntax().ancestors().skip(1) {
        if RFunctionDefinition::can_cast(ancestor.kind()) {
            break;
        }

        if let Some(outer) = RForStatement::cast(ancestor)
            && outer.variable()?.name_text() == name
            // Loops in the sequence run before this outer loop starts iterating.
            && outer.body()?.range().contains_range(loop_range)
        {
            let range = TextRange::new(variable.range().start(), ast.sequence()?.range().end());
            return Ok(Some(Diagnostic::new(ForLoopDupIndex, range, Fix::empty())));
        }
    }

    Ok(None)
}
