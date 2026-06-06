use crate::diagnostic::*;
use crate::utils::{
    get_arg_by_position, get_function_name, get_function_namespace_prefix, get_unnamed_args,
    node_contains_comments,
};
use air_r_syntax::*;
use biome_rowan::{AstNode, Direction};

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for coercing a literal to a specific type, e.g. `as.integer(1)` or
/// `as.character(1)`. This also covers the rlang helpers `lgl()`, `int()`,
/// `dbl()` and `chr()`.
///
/// This rule is disabled by default.
///
/// ## Why is this bad?
///
/// Such a coercion is done at runtime even though the result is known
/// statically. Writing the literal value directly (e.g. `1L` instead of
/// `as.integer(1)`) is clearer and avoids the unnecessary computation.
///
/// Only scalar literals are flagged. Vectors such as `as.integer(c(1, 2, 3))`
/// are left alone, since whether to prefer the literal vector is a matter of
/// taste. Numbers written in scientific notation (e.g. `as.integer(1e6)`) are
/// also skipped, as the literal form (`1000000L`) is not obviously better.
///
/// ## Example
///
/// ```r
/// as.integer(1)
/// as.character(1)
/// as.logical("true")
/// rlang::int(1)
/// ```
///
/// Use instead:
/// ```r
/// 1L
/// "1"
/// TRUE
/// 1L
/// ```
pub fn literal_coercion(ast: &RCall) -> anyhow::Result<Option<Diagnostic>> {
    let function = ast.function()?;
    let fn_name = get_function_name(function.clone());
    let fn_ns = get_function_namespace_prefix(function);

    // Determine the target type and which family the function belongs to.
    let (target, is_rlang) = match fn_name.as_str() {
        "as.logical" => (TargetType::Logical, false),
        "as.integer" => (TargetType::Integer, false),
        "as.numeric" | "as.double" => (TargetType::Double, false),
        "as.character" => (TargetType::Character, false),
        "lgl" => (TargetType::Logical, true),
        "int" => (TargetType::Integer, true),
        "dbl" => (TargetType::Double, true),
        "chr" => (TargetType::Character, true),
        _ => return Ok(None),
    };

    // Reject calls that come from an unrelated namespace. Base coercions may be
    // qualified with `base::`, rlang helpers with `rlang::`.
    if let Some(ref ns) = fn_ns {
        let expected = if is_rlang { "rlang::" } else { "base::" };
        if ns != expected {
            return Ok(None);
        }
    }

    let args = ast.arguments()?.items();

    // For the rlang helpers, only a single scalar literal is unambiguous. A
    // trailing empty argument (`int(1, )`, from `list2()` construction) is
    // tolerated, so we look at the non-empty unnamed arguments.
    //
    // For the base `as.<type>()` functions we only examine the first argument
    // (the object being coerced), ignoring any extra arguments such as a format
    // string.
    let value = if is_rlang {
        let values: Vec<AnyRExpression> = get_unnamed_args(&args)
            .into_iter()
            .filter_map(|arg| arg.value())
            .collect();
        if values.len() != 1 {
            return Ok(None);
        }
        values.into_iter().next().unwrap()
    } else {
        let Some(first) = get_arg_by_position(&args, 1) else {
            return Ok(None);
        };
        let Some(value) = first.value() else {
            return Ok(None);
        };
        value
    };

    let Some(literal) = parse_literal(&value) else {
        return Ok(None);
    };

    let Some(result) = coerce(target, &literal) else {
        return Ok(None);
    };

    let call_text = reconstruct_call_text(ast.syntax());
    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "literal_coercion".to_string(),
            format!("This coercion can be simplified."),
            Some(format!("Use `{}` instead of `{}`.", result, call_text)),
        ),
        range,
        Fix {
            content: result,
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    );
    Ok(Some(diagnostic))
}

#[derive(Clone, Copy)]
enum TargetType {
    Logical,
    Integer,
    Double,
    Character,
}

enum Literal {
    /// A double literal, with its parsed value.
    Double(f64),
    /// An integer literal (e.g. `1L`), with its parsed value.
    Integer(i64),
    /// A string literal, with its (unquoted) content.
    String(String),
    /// A logical literal (`TRUE`/`FALSE`).
    Logical(bool),
    /// `NA`.
    Na,
}

/// Extract a scalar literal from an expression, or `None` if it is not a literal
/// we can statically coerce (identifiers, calls, ranges, vectors, scientific
/// notation, ...).
fn parse_literal(expr: &AnyRExpression) -> Option<Literal> {
    if let Some(value) = expr.as_any_r_value() {
        if let Some(double) = value.as_r_double_value() {
            let text = double.value_token().ok()?.token_text_trimmed();
            let text = text.text();
            // Skip scientific notation: the literal form is not clearly better.
            if text.contains('e') || text.contains('E') {
                return None;
            }
            return text.parse::<f64>().ok().map(Literal::Double);
        }
        if let Some(integer) = value.as_r_integer_value() {
            let text = integer.value_token().ok()?.token_text_trimmed();
            let digits = text.text().trim_end_matches(['L', 'l']);
            return digits.parse::<i64>().ok().map(Literal::Integer);
        }
        if let Some(string) = value.as_r_string_value() {
            let text = string.value_token().ok()?.token_text_trimmed();
            return strip_string_quotes(text.text()).map(Literal::String);
        }
        // Complex and other values are not coerced.
        return None;
    }

    if expr.as_r_na_expression().is_some() {
        return Some(Literal::Na);
    }
    if expr.as_r_true_expression().is_some() {
        return Some(Literal::Logical(true));
    }
    if expr.as_r_false_expression().is_some() {
        return Some(Literal::Logical(false));
    }

    None
}

/// Compute the literal that the coercion produces, formatted as R source.
/// Returns `None` if we don't want to take a stand on the result.
fn coerce(target: TargetType, literal: &Literal) -> Option<String> {
    let result = match target {
        TargetType::Logical => match literal {
            Literal::Double(v) => bool_literal(*v != 0.0),
            Literal::Integer(v) => bool_literal(*v != 0),
            Literal::Logical(b) => bool_literal(*b),
            Literal::String(s) => match s.as_str() {
                "T" | "TRUE" | "true" | "True" => "TRUE".to_string(),
                "F" | "FALSE" | "false" | "False" => "FALSE".to_string(),
                _ => "NA".to_string(),
            },
            Literal::Na => "NA".to_string(),
        },
        TargetType::Integer => match literal {
            Literal::Double(v) => integer_literal(*v),
            Literal::Integer(v) => format!("{v}L"),
            Literal::Logical(b) => bool_as_int_literal(*b),
            Literal::String(s) => match s.parse::<f64>() {
                Ok(v) => integer_literal(v),
                Err(_) => "NA_integer_".to_string(),
            },
            Literal::Na => "NA_integer_".to_string(),
        },
        TargetType::Double => match literal {
            Literal::Double(v) => format!("{v}"),
            Literal::Integer(v) => format!("{v}"),
            Literal::Logical(b) => if *b { "1" } else { "0" }.to_string(),
            Literal::String(s) => match s.parse::<f64>() {
                Ok(v) => format!("{v}"),
                Err(_) => "NA_real_".to_string(),
            },
            Literal::Na => "NA_real_".to_string(),
        },
        TargetType::Character => match literal {
            Literal::Double(v) => format!("\"{v}\""),
            Literal::Integer(v) => format!("\"{v}\""),
            Literal::Logical(b) => if *b { "\"TRUE\"" } else { "\"FALSE\"" }.to_string(),
            Literal::String(s) => format!("\"{s}\""),
            Literal::Na => "NA_character_".to_string(),
        },
    };
    Some(result)
}

fn bool_literal(b: bool) -> String {
    if b { "TRUE" } else { "FALSE" }.to_string()
}

fn bool_as_int_literal(b: bool) -> String {
    if b { "1L" } else { "0L" }.to_string()
}

/// Format a double coerced to integer. `as.integer()` truncates towards zero and
/// returns `NA` when the value falls outside the 32-bit integer range.
fn integer_literal(v: f64) -> String {
    let truncated = v.trunc();
    if truncated.is_finite() && (-2_147_483_647.0..=2_147_483_647.0).contains(&truncated) {
        format!("{}L", truncated as i64)
    } else {
        "NA_integer_".to_string()
    }
}

/// Remove the surrounding quotes from a standard string literal. Returns `None`
/// for raw strings or anything that doesn't look like a quoted literal.
fn strip_string_quotes(text: &str) -> Option<String> {
    let mut chars = text.chars();
    let quote = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = text.strip_prefix(quote)?;
    rest.strip_suffix(quote).map(|s| s.to_string())
}

/// Rebuild the call text from its tokens, dropping whitespace and comments so
/// the message reads as a clean one-liner (e.g. `int(1,)`).
fn reconstruct_call_text(node: &RSyntaxNode) -> String {
    let mut out = String::new();
    for token in node.descendants_tokens(Direction::Next) {
        out.push_str(token.text_trimmed());
    }
    out
}
