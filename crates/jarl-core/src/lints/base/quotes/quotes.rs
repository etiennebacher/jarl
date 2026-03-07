use crate::diagnostic::*;
use crate::rule_options::quotes::PreferredQuote;
use air_r_syntax::*;
use biome_rowan::AstNode;

#[derive(Debug)]
enum ParsedString<'a> {
    // Normal strings
    Standard {
        quote: char,
        content: &'a str,
    },
    // Cases of r"(content)" and variations (R v4.0)
    Raw {
        raw_prefix: char,
        quote: char,
        content: &'a str,
        open: char,
        close: char,
        dashes: &'a str,
    },
}

impl ParsedString<'_> {
    fn quote(&self) -> char {
        match self {
            Self::Standard { quote, .. } | Self::Raw { quote, .. } => *quote,
        }
    }

    fn content(&self) -> &str {
        match self {
            Self::Standard { content, .. } | Self::Raw { content, .. } => content,
        }
    }
}

/// ## What it does
///
/// Checks for consistency of quote delimiters in string literals.
/// This rule is disabled by default.
///
/// ## Why is this bad?
///
/// Using a consistent quote delimiter improves readability.
///
/// By default, this rule expects double quotes (`"`). To prefer single quotes,
/// set this in `jarl.toml`:
///
/// ```toml
/// [lint.quotes]
/// quote = "single"
/// ```
///
/// For regular strings, this rule allows the opposite quote when needed to
/// avoid escaping the preferred quote.
///
/// Raw strings follow the same rule and allow the use of the opposite quote
/// for readability and to prevent early termination.
///
/// ## Example
///
/// ```r
/// x <- 'hello'
/// print(r'-('hello')-')
/// ```
///
/// Use instead:
/// ```r
/// x <- "hello"
/// print(r"-('hello')-")
/// ```
pub fn quotes(
    ast: &AnyRValue,
    preferred_quote: PreferredQuote,
) -> anyhow::Result<Option<Diagnostic>> {
    let string = unwrap_or_return_none!(ast.as_r_string_value());

    let token = string.value_token()?;
    let text = token.text_trimmed();

    // Malformed raw strings like `r'(hello]'` are parsed as identifier (`r`)
    // followed by a regular string literal. Skip these invalid forms.
    if is_malformed_raw_string(ast, text) {
        return Ok(None);
    }

    let parsed = unwrap_or_return_none!(parse_string(text));

    let quote_char = preferred_quote.as_char();

    if parsed.quote() == quote_char {
        return Ok(None);
    }

    // Allow the non-preferred quote when escaping would be needed
    // eg. 'R says "Hello world" ...'` vs `"R says \"Hello world\" ..."`
    // Skip cases in raw strings where using preferred quote reduces readability
    // e.g. `'r("rawstring")'` vs `"r("rawstring")"`.
    // Also skips cases where switching to the preferred quote results in invalid syntax
    // e.g. r'(abc)"def)' becomes `r"(abc)"def)"`
    if parsed.content().contains(quote_char) {
        return Ok(None);
    }

    let replacement = match &parsed {
        ParsedString::Standard { content, .. } => {
            format!("{quote_char}{content}{quote_char}")
        }
        ParsedString::Raw { raw_prefix, content, open, close, dashes, .. } => {
            format!("{raw_prefix}{quote_char}{dashes}{open}{content}{close}{dashes}{quote_char}")
        }
    };

    let range = ast.syntax().text_trimmed_range();
    let diagnostic = Diagnostic::new(
        ViolationData::new(
            "quotes".to_string(),
            quote_message(preferred_quote).to_string(),
            None,
        ),
        range,
        Fix {
            content: replacement,
            start: range.start().into(),
            end: range.end().into(),
            to_skip: false,
        },
    );

    Ok(Some(diagnostic))
}

fn is_malformed_raw_string(ast: &AnyRValue, text: &str) -> bool {
    if !text.starts_with(['"', '\'']) {
        return false;
    }

    let Some(prev) = ast.syntax().prev_sibling_or_token() else {
        return false;
    };

    if !matches!(prev.kind(), RSyntaxKind::R_IDENTIFIER | RSyntaxKind::IDENT)
        || !matches!(prev.to_string().trim(), "r" | "R")
    {
        return false;
    }
    // Check that it is actually raw string (r"hi" vs r "hi" or r \n "hi")
    prev.text_trimmed_range().end() == ast.syntax().text_trimmed_range().start()
}

fn quote_message(preferred_quote: PreferredQuote) -> &'static str {
    match preferred_quote {
        PreferredQuote::Double => "Only use double-quotes.",
        PreferredQuote::Single => "Only use single-quotes.",
    }
}

fn parse_string(text: &str) -> Option<ParsedString<'_>> {
    parse_standard_string(text).or_else(|| parse_raw_string(text))
}

fn parse_standard_string(text: &str) -> Option<ParsedString<'_>> {
    let quote = text.chars().next().filter(|c| matches!(c, '"' | '\''))?;
    let content = text[1..].strip_suffix(quote)?;
    Some(ParsedString::Standard { quote, content })
}

fn parse_raw_string(text: &str) -> Option<ParsedString<'_>> {
    // Raw strings with the form:
    // r"( content )"
    // R'---[ content ]---'
    let raw_prefix = text.chars().next()?;
    if raw_prefix != 'r' && raw_prefix != 'R' {
        return None;
    }
    let rest = text.strip_prefix(raw_prefix)?;

    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = rest.strip_prefix(quote)?;

    let after_dashes = rest.trim_start_matches('-');
    let leading_dashes = &rest[..rest.len() - after_dashes.len()];

    let open_brace = after_dashes.chars().next()?;
    let close_brace = match open_brace {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        _ => return None,
    };
    let body_and_suffix = after_dashes.strip_prefix(open_brace)?;
    let expected_closing_fence = format!("{close_brace}{leading_dashes}{quote}");
    // If the closing side has fewer/more dashes (R syntax error), `strip_suffix()` returns none
    // and no lint.
    let content = body_and_suffix.strip_suffix(&expected_closing_fence)?;

    Some(ParsedString::Raw {
        raw_prefix,
        quote,
        content,
        open: open_brace,
        close: close_brace,
        dashes: leading_dashes,
    })
}
