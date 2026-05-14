use crate::diagnostic::*;
use crate::rule_options::pipe_consistency::PreferredPipe;
use crate::utils::node_contains_comments;
use air_r_syntax::*;
use biome_rowan::{AstNode, Direction, TextRange};

/// Version added: 0.6.0
///
/// ## What it does
///
/// Reports cases where both pipes (`%>%` or `|>`) are used. By default, the
/// base pipe `|>` is preferred but this can be changed in the configuration
/// file.
///
/// ## Why is this bad?
///
/// This simply ensures that pipe usage is consistent. There are a few cases
/// where both pipes are not equivalent, and therefore where this rule doesn't
/// report diagnostics:
///
/// - if the RHS of a `%>%` uses `.` as an unnamed argument, then it is not
///   reported because there is no equivalent in base R (the `_` placeholder
///   only works for named arguments).
///
/// - if the RHS of a `%>%` uses `.` several times, then it is not reported
///   because there is no equivalent in base R (the `_` placeholder can only be
///   used once in the RHS).
///
/// This rule is available only for R >= 4.2 (the `_` placeholder was
/// introduced in 4.2, even though `|>` itself was introduced in 4.1), and it
/// has an unsafe fix due to some specificities of the native pipe (e.g. it
/// doesn't work when `+()` is on the RHS).
///
/// ## Example
///
/// ```r
/// data %>%
///   transform(a = x / 2) |>
///   plot()
/// ```
///
/// Use instead:
/// ```r
/// data |>
///   transform(a = x / 2) |>
///   plot()
/// ```
///
/// ## References
///
/// See `?pipeOp`
pub fn pipe_consistency(
    ast: &RBinaryExpression,
    preferred: PreferredPipe,
) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left: _, operator, right } = ast.as_fields();
    let operator = operator?;
    let right = right?;

    let kind = operator.kind();
    let is_base_pipe = kind == RSyntaxKind::PIPE;
    let is_magrittr_pipe = kind == RSyntaxKind::SPECIAL && operator.text_trimmed() == "%>%";

    if !is_base_pipe && !is_magrittr_pipe {
        return Ok(None);
    }

    let preferred_is_base = matches!(preferred, PreferredPipe::Base);
    if is_base_pipe && preferred_is_base {
        return Ok(None);
    }

    let preferred_is_magrittr = matches!(preferred, PreferredPipe::Magrittr);
    if is_magrittr_pipe && preferred_is_magrittr {
        return Ok(None);
    }

    // For `%>%` -> `|>` conversions, the `.` placeholder needs special care:
    // - if it appears as an unnamed argument, base R has no equivalent
    // - if it appears more than once, base R only allows a single `_`
    // In both cases the rule reports nothing rather than emit something that
    // would change semantics.
    let placeholder_replacement = if is_magrittr_pipe {
        match find_dot_placeholder(&right) {
            DotPlaceholder::Unsupported => return Ok(None),
            DotPlaceholder::None => None,
            DotPlaceholder::OneNamed(range) => Some(range),
        }
    } else {
        // `|>` -> `%>%`: a single `_` always has a `.` equivalent.
        find_underscore_placeholder(&right)
    };

    let (new_op, new_placeholder, body, suggestion) = if preferred_is_base {
        (
            "|>",
            "_",
            "`%>%` is inconsistent with the preferred pipe `|>`.",
            "Use `|>` instead.",
        )
    } else {
        (
            "%>%",
            ".",
            "`|>` is inconsistent with the preferred pipe `%>%`.",
            "Use `%>%` instead.",
        )
    };

    let bin_range = ast.syntax().text_trimmed_range();
    let bin_start: u32 = bin_range.start().into();
    let mut content = ast.to_trimmed_string();

    let op_range = operator.text_trimmed_range();
    let mut edits: Vec<(usize, usize, &str)> = Vec::with_capacity(2);
    edits.push((
        (u32::from(op_range.start()) - bin_start) as usize,
        (u32::from(op_range.end()) - bin_start) as usize,
        new_op,
    ));
    if let Some(r) = placeholder_replacement {
        edits.push((
            (u32::from(r.start()) - bin_start) as usize,
            (u32::from(r.end()) - bin_start) as usize,
            new_placeholder,
        ));
    }
    // Apply edits from the back so earlier offsets remain valid.
    edits.sort_by_key(|e| std::cmp::Reverse(e.0));
    for (start, end, replacement) in edits {
        content.replace_range(start..end, replacement);
    }

    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "pipe_consistency".to_string(),
            body.to_string(),
            Some(suggestion.to_string()),
        ),
        op_range,
        Fix {
            content,
            start: bin_range.start().into(),
            end: bin_range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );

    Ok(Some(diagnostic))
}

enum DotPlaceholder {
    /// No `.` placeholder in the RHS.
    None,
    /// Exactly one `.` placeholder, used as a named top-level argument.
    OneNamed(TextRange),
    /// `.` is used in a way that can't be converted to `_`: unnamed, nested,
    /// or appearing more than once.
    Unsupported,
}

/// Inspect the RHS of `%>%` and classify its use of the `.` placeholder.
fn find_dot_placeholder(rhs: &AnyRExpression) -> DotPlaceholder {
    let dot_tokens: Vec<_> = rhs
        .syntax()
        .descendants_tokens(Direction::Next)
        .filter(|t| t.kind() == RSyntaxKind::IDENT && t.text_trimmed() == ".")
        .collect();

    match dot_tokens.len() {
        0 => DotPlaceholder::None,
        1 => {
            let token = &dot_tokens[0];
            // Find the enclosing argument; if `.` isn't a direct argument
            // value the conversion would require restructuring.
            let mut node = token.parent();
            let mut arg = None;
            while let Some(n) = node {
                if let Some(a) = RArgument::cast(n.clone()) {
                    arg = Some(a);
                    break;
                }
                node = n.parent();
            }
            let Some(arg) = arg else {
                return DotPlaceholder::Unsupported;
            };
            // Only support `.` as the direct value of a named argument.
            let value = match arg.value() {
                Some(v) => v,
                None => return DotPlaceholder::Unsupported,
            };
            let value_text = value.to_trimmed_string();
            if value_text != "." {
                return DotPlaceholder::Unsupported;
            }
            if arg.name_clause().is_none() {
                return DotPlaceholder::Unsupported;
            }
            DotPlaceholder::OneNamed(token.text_trimmed_range())
        }
        _ => DotPlaceholder::Unsupported,
    }
}

/// Locate a single `_` placeholder in the RHS of `|>` if present. Base R
/// guarantees at most one and only as a named argument, so any occurrence is
/// safe to convert to `.` for magrittr.
fn find_underscore_placeholder(rhs: &AnyRExpression) -> Option<TextRange> {
    rhs.syntax()
        .descendants_tokens(Direction::Next)
        .find(|t| t.kind() == RSyntaxKind::IDENT && t.text_trimmed() == "_")
        .map(|t| t.text_trimmed_range())
}
