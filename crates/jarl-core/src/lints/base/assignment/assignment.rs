use crate::diagnostic::*;
use air_r_syntax::*;
use biome_rowan::AstNode;

/// Version added: 0.0.8
///
/// ## What it does
///
/// Checks for consistency of assignment operator.
///
/// ## Why is this bad?
///
/// In most cases using `=` and `<-` is equivalent. Some very popular packages
/// use `=` without problems. This rule only ensures the consistency of the
/// assignment operator in a project.
///
/// Set the following option in `jarl.toml` to use `=` as the preferred operator:
///
/// ```toml
/// [lint.assignment]
/// operator = "=" # or "<-"
/// ```
///
/// ## Example
///
/// ```r
/// x = "a"
/// ```
///
/// Use instead:
/// ```r
/// x <- "a"
/// ```
///
/// ## References
///
/// See:
///
/// - [https://style.tidyverse.org/syntax.html#assignment-1](https://style.tidyverse.org/syntax.html#assignment-1)
pub fn assignment(
    ast: &RBinaryExpression,
    assignment: RSyntaxKind,
) -> anyhow::Result<Option<Diagnostic>> {
    if !can_normalize_to_equal(ast) {
        return Ok(None);
    };

    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();

    let operator = operator?;
    let lhs = left?.into_syntax();
    let rhs = right?.into_syntax();

    let operator_to_check = match assignment {
        RSyntaxKind::ASSIGN => RSyntaxKind::EQUAL,
        RSyntaxKind::EQUAL => RSyntaxKind::ASSIGN,
        _ => unreachable!(),
    };

    if operator.kind() != operator_to_check && operator.kind() != RSyntaxKind::ASSIGN_RIGHT {
        return Ok(None);
    };

    // We don't want the reported range to be the entire binary expression. The
    // range is used in the LSP to highlight lints, but highlighting the entire
    // binary expression would be super annoying for long functions that are
    // assigned using `=`.
    let (range_to_report, msg, replacement) = match operator.kind() {
        RSyntaxKind::EQUAL => {
            let range = TextRange::new(
                lhs.text_trimmed_range().start(),
                operator.text_trimmed_range().end(),
            );
            let message = "Use `<-` for assignment.";
            let fix = format!("{} <- {}", lhs.text_trimmed(), rhs.text_trimmed());
            (range, message, fix)
        }
        RSyntaxKind::ASSIGN => {
            let range = TextRange::new(
                lhs.text_trimmed_range().start(),
                operator.text_trimmed_range().end(),
            );
            let message = "Use `=` for assignment.";
            let fix = format!("{} = {}", lhs.text_trimmed(), rhs.text_trimmed());
            (range, message, fix)
        }
        RSyntaxKind::ASSIGN_RIGHT => {
            let range = TextRange::new(
                operator.text_trimmed_range().start(),
                rhs.text_trimmed_range().end(),
            );
            let (message, fix) = match assignment {
                RSyntaxKind::ASSIGN => {
                    let msg = "Use `<-` for assignment.";
                    let replacement = format!("{} <- {}", rhs.text_trimmed(), lhs.text_trimmed());
                    (msg, replacement)
                }
                RSyntaxKind::EQUAL => {
                    let msg = "Use `=` for assignment.";
                    let replacement = format!("{} = {}", rhs.text_trimmed(), lhs.text_trimmed());
                    (msg, replacement)
                }
                _ => unreachable!(),
            };
            (range, message, fix)
        }
        _ => unreachable!(),
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new("assignment".to_string(), msg.to_string(), None),
        range_to_report,
        Fix {
            content: replacement,
            start: range.start().into(),
            end: range.end().into(),
            to_skip: false,
        },
    );

    Ok(Some(diagnostic))
}

// Entirely copied from https://github.com/posit-dev/air/pull/502
// ===============================================================
//
// Can we safely normalize `<-` to `=`?
//
// In R, it is always safe to normalize `=` to `<-`, but the reverse is not true.
//
// For example, it may change the semantic meaning:
//
// ```r
// # Expression `x <- 1` as unnamed argument to function `f`
// f(x <- 1)
//
// # Named argument `x` with value `1` to function `f`
// f(x = 1)
// ```
//
// Or it may become a syntax error:
//
// ```r
// # Expression `x <- 1`'s result is used as the `condition`
// if (x <- 1) this
//
// # Syntax error
// if (x = 1) this
// ```
//
// To handle this precisely, we looked for all usage of `expr_or_assign_or_help` in the
// R `gram.y` grammar. This is the only place that `EQ_ASSIGN` is used in an assignment
// context. Each usage of `expr_or_assign_or_help` in `gram.y` is replicated below,
// ensuring that we capture all possible places that `=` is allowed.
// https://github.com/wch/r-source/blob/b7e27523048d135e3a02560e51cb266702bd49c1/src/main/gram.y#L453-L455
//
// Note how `LEFT_ASSIGN` is instead part of `expr`, making usage of it more permissive
// than `EQ_ASSIGN` (a superset, really). This is why we can always replace `=` with
// `<-`.
// https://github.com/wch/r-source/blob/b7e27523048d135e3a02560e51cb266702bd49c1/src/main/gram.y#L497
//
// `EQ_ASSIGN` is also used in `sub:` and `formlist:`, but these correspond to named
// arguments (i.e. `fn(x = 1)`) and parameters with defaults (i.e. `function(x = 1) {}`)
// respectively, so are not relevant usages for us (and in fact those are locations where
// normalizing to `=` is not allowed).
// https://github.com/wch/r-source/blob/b7e27523048d135e3a02560e51cb266702bd49c1/src/main/gram.y#L549-L564
fn can_normalize_to_equal(node: &RBinaryExpression) -> bool {
    let node = node.syntax();

    let Some(parent) = node.parent() else {
        // Should not happen, should always be at least contained in an RRoot's
        // RExpressionList. We return the conservative `false` if we somehow get here.
        return false;
    };
    let parent_kind = parent.kind();

    // i.e. top level `x <- 1` to `x = 1`
    // i.e. `{ x <- 1 }` to `{ x = 1 }`
    if RExpressionList::can_cast(parent_kind) {
        return true;
    }

    // i.e. `(x <- 1)` to `(x = 1)`
    if RParenthesizedExpression::can_cast(parent_kind) {
        return true;
    }

    // i.e. `? x <- 1` to `? x = 1`
    if let Some(parent) = RUnaryExpression::cast_ref(&parent)
        && let Ok(operator) = parent.operator()
        && operator.kind() == RSyntaxKind::WAT
    {
        return true;
    }

    // i.e. `x <- y <- 1` to `x = y = 1`
    // i.e. `x = y <- 1` to `x = y = 1`, but notably `x <- y = 1` is a parse error.
    // i.e. `x <- 1 ? y` to `x = 1 ? y`
    // i.e. `y ? x <- 1` to `y ? x = 1`
    //
    // We recurse through `can_normalize_to_equal()` in these cases because the parent
    // binary expression must also be in a position where `=` would be valid. These would
    // all result in parse errors or change semantic meaning if we didn't check this:
    //
    // i.e. `f(x <- y <- 1)` to `f(x <- y = 1)` gives parse error
    // i.e. `if (x <- y <- 1) z` to `if (x <- y = 1) z` gives parse error
    //
    // i.e. `f(x ? y <- 1)` to `f(x ? y = 1)` gives parse error
    // i.e. `if(x ? y <- 1) z` to `if(x ? y = 1) z` gives parse error
    //
    // i.e. `f(x <- 1 ? y)` to `f(x = 1 ? y)` changes semantic meaning
    // i.e. `if(x <- 1 ? y) z` to `if(x = 1 ? y) z` gives parse error
    if let Some(parent) = RBinaryExpression::cast_ref(&parent)
        && let Ok(operator) = parent.operator()
        && matches!(
            operator.kind(),
            RSyntaxKind::ASSIGN | RSyntaxKind::EQUAL | RSyntaxKind::WAT
        )
    {
        return can_normalize_to_equal(&parent);
    }

    // i.e. `function(x) x <- 1` to `function(x) x = 1`
    if let Some(parent) = RFunctionDefinition::cast_ref(&parent)
        && let Ok(body) = parent.body()
        && body.syntax() == node
    {
        return true;
    }

    // i.e. `if (cond) x <- 1` to `if (cond) x = 1`
    if let Some(parent) = RIfStatement::cast_ref(&parent)
        && let Ok(consequence) = parent.consequence()
        && consequence.syntax() == node
    {
        return true;
    }

    // i.e. `if (cond) x else y <- 1` to `if (cond) x else y = 1`
    if let Some(parent) = RElseClause::cast_ref(&parent)
        && let Ok(alternative) = parent.alternative()
        && alternative.syntax() == node
    {
        return true;
    }

    // i.e. `for(i in 1:5) x <- 1` to `for(i in 1:5) x = 1`
    if let Some(parent) = RForStatement::cast_ref(&parent)
        && let Ok(body) = parent.body()
        && body.syntax() == node
    {
        return true;
    }

    // i.e. `while(cond) x <- 1` to `while(cond) x = 1`
    if let Some(parent) = RWhileStatement::cast_ref(&parent)
        && let Ok(body) = parent.body()
        && body.syntax() == node
    {
        return true;
    }

    // i.e. `repeat x <- 1` to `repeat x = 1`
    if let Some(parent) = RRepeatStatement::cast_ref(&parent)
        && let Ok(body) = parent.body()
        && body.syntax() == node
    {
        return true;
    }

    false
}
