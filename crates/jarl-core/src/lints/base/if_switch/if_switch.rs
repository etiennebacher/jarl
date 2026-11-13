use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::node_contains_comments;
use air_r_syntax::*;
use biome_rowan::AstNode;

/// <!-- docs: start -->
/// Version added: 0.8.0
///
/// ## What it does
///
/// Checks for chains of `if`/`else if` statements that all compare the same
/// expression to a string with `==`, e.g.
/// `if (x == "a") 1 else if (x == "b") 2 else if (x == "c") 3`.
///
/// ## Why is this bad?
///
/// `switch()` is clearer than repeated `if`/`else` tests on the same value.
///
/// This rule has an unsafe automatic fix which wraps the compared expression
/// in `as.character()`. Without it, `switch()` would select numbers and logicals
/// by position, not by name, e.g. `switch(3, a = "x", b = "y", c = "z")` returns
/// `"z"` (the third branch). Factors would likewise be matched by their
/// integer codes instead of their labels.
///
/// This fix is unsafe because `NA` values make an `if` chain error, but `switch()`
/// silently falls back to the default (or returns an invisible `NULL`).
/// `NA_character_` even matches a branch named `"NA"`.
///
/// Empty strings are not reported because `switch()` cannot have an empty
/// string as an argument name.
///
/// ## Examples
///
/// ```r
/// if (x == "a") {
///   1
/// } else if (x == "b") {
///   2
/// } else if (x == "c") {
///   3
/// }
/// ```
///
/// Use instead:
/// ```r
/// switch(as.character(x), a = 1, b = 2, c = 3)
/// ```
///
/// ## References
///
/// See `?switch`.
/// <!-- docs: end -->
pub fn if_switch(ast: &RIfStatement, max_branches: usize) -> anyhow::Result<Option<Diagnostic>> {
    // Only look at the start of a chain, so that a chain is reported once.
    if ast.syntax().parent().and_then(RElseClause::cast).is_some() {
        return Ok(None);
    }

    let mut subject: Option<(String, String)> = None;
    // `name = value` arguments of the `switch()` call, and its default value.
    let mut arms: Vec<(String, AnyRExpression)> = Vec::new();
    let mut default: Option<AnyRExpression> = None;
    let mut current = Some(ast.clone());

    while let Some(if_stmt) = current {
        let condition = if_stmt.condition()?;
        let Some(binary) = condition.as_r_binary_expression() else {
            return Ok(None);
        };
        if binary.operator()?.kind() != RSyntaxKind::EQUAL2 {
            return Ok(None);
        }

        let (left, right) = (binary.left()?, binary.right()?);
        let (this_subject, string) = match (string_value(&left), string_value(&right)) {
            (Some(value), None) if !value.is_empty() => (right, left),
            (None, Some(value)) if !value.is_empty() => (left, right),
            _ => return Ok(None),
        };

        // Compare token by token so that whitespace and comments don't matter.
        let this_key = this_subject
            .syntax()
            .descendants_tokens(biome_rowan::Direction::Next)
            .map(|token| token.text_trimmed().to_string())
            .collect::<String>();
        match &subject {
            Some((key, _)) if *key != this_key => return Ok(None),
            Some(_) => {}
            None => {
                subject = Some((this_key, this_subject.syntax().text_trimmed().to_string()));
            }
        }
        arms.push((
            string.syntax().text_trimmed().to_string(),
            if_stmt.consequence()?,
        ));

        current = None;
        if let Some(alternative) = if_stmt
            .else_clause()
            .and_then(|else_clause| else_clause.alternative().ok())
        {
            match alternative.as_r_if_statement() {
                Some(next) => current = Some(next.clone()),
                None => default = Some(alternative),
            }
        }
    }

    if arms.len() < max_branches {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();
    let fix = match (&subject, build_switch(&subject, &arms, &default)) {
        (Some(_), Some(replacement)) => {
            Fix::new(range, replacement, node_contains_comments(ast.syntax()))
        }
        _ => Fix::empty(),
    };

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::IfSwitch,
            "Repeated `if`/`else if` equality tests on the same value.".to_string(),
            Some("Use `switch()` instead.".to_string()),
        ),
        range,
        fix,
    )))
}

/// Build the `switch()` call equivalent to the chain. Returns `None` if a
/// branch can't be an argument: an `=` assignment would be parsed as a named
/// argument.
fn build_switch(
    subject: &Option<(String, String)>,
    arms: &[(String, AnyRExpression)],
    default: &Option<AnyRExpression>,
) -> Option<String> {
    let (_, subject) = subject.as_ref()?;
    // A non-character `switch()` input selects by position instead of by name,
    // e.g. `x <- 3; switch(x, a = "first", b = "second", c = "third")` returns
    // "third".
    let subject = format!("as.character({subject})");

    let is_equal_assignment = |expr: &AnyRExpression| {
        expr.as_r_binary_expression()
            .and_then(|binary| binary.operator().ok())
            .is_some_and(|op| op.kind() == RSyntaxKind::EQUAL)
    };
    if arms.iter().any(|(_, value)| is_equal_assignment(value))
        || default.as_ref().is_some_and(is_equal_assignment)
    {
        return None;
    }

    let mut args: Vec<String> = arms
        .iter()
        .map(|(name, value)| format!("{name} = {}", value.syntax().text_trimmed()))
        .collect();
    if let Some(default) = default {
        args.push(default.syntax().text_trimmed().to_string());
    }

    if args.iter().any(|arg| arg.contains('\n')) {
        Some(format!(
            "switch(\n  {subject},\n  {}\n)",
            args.join(",\n  ")
        ))
    } else {
        Some(format!("switch({subject}, {})", args.join(", ")))
    }
}

/// The content of `expr` if it is a string literal.
fn string_value(expr: &AnyRExpression) -> Option<String> {
    match expr {
        AnyRExpression::AnyRValue(AnyRValue::RStringValue(s)) => {
            Some(s.inner_string_text().ok()?.text().to_string())
        }
        _ => None,
    }
}
