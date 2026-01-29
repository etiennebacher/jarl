use crate::diagnostic::*;
use air_r_syntax::*;
use biome_rowan::{AstNode, SyntaxToken};

pub struct NumericLeadingZero;

/// ## What it does
///
/// Checks for double or complex values with a decimal component and a
/// leading `.`.
///
/// ## Why is this bad?
///
/// While `.1` and `0.1` mean the same thing, the latter is easier to read due
/// to the small size of the `.` glyph.
///
/// ## Example
///
/// ```r
/// x <- .1
/// ```
///
/// Use instead:
/// ```r
/// x <- 0.1
/// ```
impl Violation for NumericLeadingZero {
    fn name(&self) -> String {
        "numeric_leading_zero".to_string()
    }
    fn body(&self) -> String {
        "Include the leading zero for fractional numeric constants.".to_string()
    }
}

pub fn numeric_leading_zero(ast: &AnyRValue) -> anyhow::Result<Option<Diagnostic>> {
    let mut value: SyntaxToken<RLanguage>;
    let mut value_text: &str = "";

    if let Some(double) = ast.as_r_double_value() {
        value = double.value_token()?;
        value_text = value.text_trimmed();
    }
    if let Some(complex) = ast.as_r_complex_value() {
        value = complex.value_token()?;
        value_text = value.text_trimmed();
    };

    if value_text.starts_with(".") {
        let range = ast.syntax().text_trimmed_range();
        let diagnostic = Diagnostic::new(
            NumericLeadingZero,
            range,
            Fix {
                content: format!("0{value_text}"),
                start: range.start().into(),
                end: range.end().into(),
                to_skip: false,
            },
        );
        return Ok(Some(diagnostic));
    }

    Ok(None)
}
