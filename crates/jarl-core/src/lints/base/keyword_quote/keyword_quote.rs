use crate::diagnostic::*;
use crate::rule_set::Rule;
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};
use oak_core::syntax_ext::RIdentifierExt;

/// <!-- docs: start -->
/// Version added: 0.8.0
///
/// ## What it does
///
/// Checks for unnecessary quoting of:
///
/// * argument names in function calls, e.g. `c("a" = 1)`;
/// * targets of assignment, e.g. `"foo" <- 1`;
/// * targets of extraction with `$` or `@`, e.g. `x$"foo"`.
///
/// It also reports non-syntactic assignment and extraction targets that use
/// quotes instead of backticks, e.g. `"foo bar" <- 1`.
///
/// ## Why is this bad?
///
/// Quotes and backticks are only needed when a name is not syntactic, i.e.
/// when it contains special characters (such as spaces) or is a reserved word
/// (such as `if` or `TRUE`). In other cases, they add visual noise.
///
/// ## Example
///
/// ```r
/// x <- c("a" = 1, `b` = 2)
/// "foo" <- 1
/// "foo bar" <- 2
/// x$"bar"
/// ```
///
/// Use instead:
/// ```r
/// x <- c(a = 1, b = 2)
/// foo <- 1
/// `foo bar` <- 2
/// x$bar
/// ```
///
/// ## References
///
/// See `?make.names`, `?Reserved` and `?Quotes`.
/// <!-- docs: end -->
pub fn keyword_quote_call(ast: &RCall) -> anyhow::Result<Vec<Diagnostic>> {
    let mut diagnostics = vec![];
    for arg in ast.arguments()?.items().iter().flatten() {
        let Some(name) = arg.name_clause().and_then(|clause| clause.name().ok()) else {
            continue;
        };
        let node = match name {
            AnyRArgumentName::RIdentifier(x) => AnyQuotable::Identifier(x),
            AnyRArgumentName::RStringValue(x) => AnyQuotable::String(x),
            _ => continue,
        };
        // Non-syntactic names can use quotes or backticks in calls.
        if !node.is_syntactic() {
            continue;
        }
        diagnostics.extend(node.diagnostic(
            "Only quote argument names if necessary.",
            "Remove the quotes or backticks around the argument name.",
        ));
    }
    Ok(diagnostics)
}

pub fn keyword_quote_assignment(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let target = match ast.operator()?.kind() {
        RSyntaxKind::ASSIGN | RSyntaxKind::SUPER_ASSIGN | RSyntaxKind::EQUAL => ast.left()?,
        RSyntaxKind::ASSIGN_RIGHT | RSyntaxKind::SUPER_ASSIGN_RIGHT => ast.right()?,
        _ => return Ok(None),
    };
    let node = match target {
        AnyRExpression::RIdentifier(x) => AnyQuotable::Identifier(x),
        AnyRExpression::AnyRValue(AnyRValue::RStringValue(x)) => AnyQuotable::String(x),
        _ => return Ok(None),
    };
    Ok(node.target_diagnostic("assignment"))
}

pub fn keyword_quote_extract(ast: &RExtractExpression) -> anyhow::Result<Option<Diagnostic>> {
    let operator = match ast.operator()?.kind() {
        RSyntaxKind::DOLLAR => "extraction with `$`",
        RSyntaxKind::AT => "extraction with `@`",
        _ => return Ok(None),
    };
    let node = match ast.right()? {
        AnyRSelector::RIdentifier(x) => AnyQuotable::Identifier(x),
        AnyRSelector::RStringValue(x) => AnyQuotable::String(x),
        _ => return Ok(None),
    };
    Ok(node.target_diagnostic(operator))
}

/// A name that may be written with quotes or backticks.
enum AnyQuotable {
    Identifier(RIdentifier),
    String(RStringValue),
}

impl AnyQuotable {
    /// The name without its quotes or backticks, or `None` if it is written
    /// without them.
    fn inner_text(&self) -> Option<String> {
        match self {
            Self::Identifier(x) => x
                .syntax()
                .text_trimmed()
                .to_string()
                .starts_with('`')
                .then(|| x.name_text()),
            Self::String(x) => x.inner_string_text().ok().map(|t| t.text().to_string()),
        }
    }

    fn is_syntactic(&self) -> bool {
        self.inner_text().is_some_and(|x| is_syntactic_name(&x))
    }

    fn range(&self) -> biome_rowan::TextRange {
        match self {
            Self::Identifier(x) => x.syntax().text_trimmed_range(),
            Self::String(x) => x.syntax().text_trimmed_range(),
        }
    }

    /// Diagnostic for the target of an assignment or extraction: syntactic
    /// names shouldn't be quoted, and non-syntactic ones should use backticks.
    fn target_diagnostic(&self, kind: &str) -> Option<Diagnostic> {
        if self.is_syntactic() {
            self.diagnostic(
                &format!("Only quote targets of {kind} if necessary."),
                "Remove the quotes or backticks around the name.",
            )
        } else if matches!(self, Self::String(_)) {
            self.diagnostic(
                "Use backticks to create non-syntactic names, not quotes.",
                "Replace the quotes with backticks.",
            )
        } else {
            None
        }
    }

    fn diagnostic(&self, body: &str, suggestion: &str) -> Option<Diagnostic> {
        let inner = self.inner_text()?;
        let replacement = if is_syntactic_name(&inner) {
            Some(inner)
        } else if inner.contains(['`', '\\']) {
            // Escapes don't necessarily mean the same thing in backticks, so
            // don't try to convert them.
            None
        } else {
            Some(format!("`{inner}`"))
        };
        let range = self.range();
        let fix = match replacement {
            Some(replacement) => Fix::new(range, replacement, false),
            None => Fix::empty(),
        };
        Some(Diagnostic::new(
            ViolationData::new(
                Rule::KeywordQuote,
                body.to_string(),
                Some(suggestion.to_string()),
            ),
            range,
            fix,
        ))
    }
}

/// Whether `name` can be used as a symbol without quotes or backticks.
///
/// Approximation of R's rules (see `?make.names`): letters are
/// locale-dependent in R, here we accept any Unicode alphabetic character.
fn is_syntactic_name(name: &str) -> bool {
    const RESERVED: &[&str] = &[
        "if",
        "else",
        "repeat",
        "while",
        "function",
        "for",
        "in",
        "next",
        "break",
        "TRUE",
        "FALSE",
        "NULL",
        "Inf",
        "NaN",
        "NA",
        "NA_integer_",
        "NA_real_",
        "NA_character_",
        "NA_complex_",
    ];
    if RESERVED.contains(&name) || name == "..." || is_dot_dot_i(name) {
        return false;
    }

    let mut chars = name.chars();
    let valid_start = match chars.next() {
        Some('.') => !chars.next().is_some_and(|c| c.is_ascii_digit()),
        Some(c) => c.is_alphabetic(),
        None => false,
    };
    valid_start
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '.' || c == '_')
}

/// `..1`, `..2`, etc.
fn is_dot_dot_i(name: &str) -> bool {
    name.strip_prefix("..")
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
}
