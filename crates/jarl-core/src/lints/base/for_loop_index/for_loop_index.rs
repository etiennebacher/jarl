use crate::diagnostic::*;
use crate::rule_set::Rule;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct ForLoopIndex;

/// <!-- docs: start -->
/// Version added: 0.0.16
///
/// ## What it does
///
/// Checks whether the index variable in a `for` loop also occurs in its
/// sequence expression.
///
/// ## Why is this bad?
///
/// `for (x in x)` or `for (x in foo(x))` are confusing to read and can lead
/// to errors.
///
/// ## Example
///
/// ```r
/// x <- c(1, 2, 3)
/// for (x in x) {
///   x + 1
/// }
/// ```
///
/// Use instead:
/// ```r
/// x <- c(1, 2, 3)
/// for (i in x) {
///   i + 1
/// }
/// ```
/// <!-- docs: end -->
impl Violation for ForLoopIndex {
    fn rule(&self) -> Rule {
        Rule::ForLoopIndex
    }
    fn body(&self) -> String {
        "Don't re-use any sequence symbols as the index symbol in a for loop.".to_string()
    }
}

pub fn for_loop_index(ast: &RForStatement) -> anyhow::Result<Option<Diagnostic>> {
    let RForStatementFields { variable, sequence, .. } = ast.as_fields();
    let variable = variable?;
    let name = variable.to_trimmed_string();
    let name = name.trim_matches('`');
    let sequence = sequence?;

    let mut walk = sequence.syntax().preorder();
    while let Some(event) = walk.next() {
        let WalkEvent::Enter(node) = event else {
            continue;
        };
        // Labels and qualified names are not sequence variables.
        if matches!(
            node.kind(),
            RSyntaxKind::R_ARGUMENT_NAME_CLAUSE | RSyntaxKind::R_NAMESPACE_EXPRESSION
        ) {
            walk.skip_subtree();
            continue;
        }

        let Some(identifier) = RIdentifier::cast(node) else {
            continue;
        };
        if identifier.to_trimmed_string().trim_matches('`') != name {
            continue;
        }
        // In `x()` and `obj$x`, the bare function name and field selector
        // are not the sequence object. The `obj` expression is still visited.
        if let Some(parent) = identifier.syntax().parent() {
            if let Some(call) = RCall::cast_ref(&parent)
                && call.function()?.syntax() == identifier.syntax()
            {
                continue;
            }
            if let Some(extract) = RExtractExpression::cast_ref(&parent)
                && extract.right()?.syntax() == identifier.syntax()
            {
                continue;
            }
        }
        let range = TextRange::new(variable.range().start(), sequence.range().end());
        return Ok(Some(Diagnostic::new(ForLoopIndex, range, Fix::empty())));
    }

    Ok(None)
}
