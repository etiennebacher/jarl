use crate::checker::Checker;
use crate::diagnostic::*;
use crate::utils::{get_arg_by_name, get_arg_by_position, get_function_name};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

pub struct StringsAsFactors;

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for calls to `data.frame()` that contain a statically identifiable
/// character column but do not explicitly set `stringsAsFactors`. This rule
/// only applies when the project's minimum supported R version is known and is
/// below R 4.0.0.
///
/// ## Why is this bad?
///
/// Before R 4.0.0, `data.frame()` converted strings to factors by default. From
/// R 4.0.0 onward, strings remain character vectors by default. Code supporting
/// versions on both sides of this change can therefore return columns with
/// different types depending on the R version used.
///
/// This rule does not provide an automatic fix because either `TRUE` or `FALSE`
/// can be the intended value of `stringsAsFactors`.
///
/// ## Example
///
/// ```r
/// data.frame(x = "a")
/// ```
///
/// Use one of the following instead:
/// ```r
/// data.frame(x = "a", stringsAsFactors = TRUE)
/// data.frame(x = "a", stringsAsFactors = FALSE)
/// ```
///
/// ## References
///
/// See `?data.frame`
/// and the [R Core discussion](https://developer.r-project.org/Blog/public/2020/02/16/stringsasfactors/).
impl Violation for StringsAsFactors {
    fn name(&self) -> String {
        "strings_as_factors".to_string()
    }

    fn body(&self) -> String {
        "`data.frame()` can create different column types before and after R 4.0 when `stringsAsFactors` is omitted."
            .to_string()
    }

    fn suggestion(&self) -> Option<String> {
        Some(
            "Specify `stringsAsFactors = TRUE` or `stringsAsFactors = FALSE` explicitly."
                .to_string(),
        )
    }
}

pub fn strings_as_factors(ast: &RCall, checker: &Checker) -> anyhow::Result<Option<Diagnostic>> {
    // This rule only applies when the minimum R version is known and is below 4.0.0.
    if checker
        .minimum_r_version
        .is_none_or(|version| version >= (4, 0, 0))
    {
        return Ok(None);
    }

    // Check if the call is to `data.frame()`.
    let RCallFields { function, arguments } = ast.as_fields();
    if get_function_name(function?) != "data.frame" {
        return Ok(None);
    }

    // Check if `stringsAsFactors` is explicitly set.
    let arguments = arguments?.items();
    if get_arg_by_name(&arguments, "stringsAsFactors").is_some() {
        return Ok(None);
    }

    for argument in arguments.iter().filter_map(|argument| argument.ok()) {
        let is_row_names = argument
            .name_clause()
            .and_then(|name_clause| name_clause.name().ok())
            .is_some_and(|name| name.to_string().trim() == "row.names");

        if is_row_names {
            continue;
        }

        let Some(value) = argument.value() else {
            continue;
        };

        if is_known_character_expression(value)? {
            return Ok(Some(Diagnostic::new(
                StringsAsFactors,
                ast.syntax().text_trimmed_range(),
                Fix::empty(),
            )));
        }
    }

    Ok(None)
}

const KNOWN_CHARACTER_FUNCTIONS: &[&str] = &[
    "character",
    "as.character",
    "paste",
    "sprintf",
    "format",
    "formatC",
    "prettyNum",
    "toString",
    "encodeString",
];

fn is_known_character_expression(expression: AnyRExpression) -> anyhow::Result<bool> {
    if is_string_literal(&expression) {
        return Ok(true);
    }

    let Some(call) = expression.as_r_call() else {
        return Ok(false);
    };
    let function = call.function()?;

    match get_function_name(function).as_str() {
        "c" => is_literal_character_combine(call),
        "rep" => rep_starts_with_known_character(call),
        function => Ok(KNOWN_CHARACTER_FUNCTIONS.contains(&function)),
    }
}

fn is_literal_character_combine(call: &RCall) -> anyhow::Result<bool> {
    let arguments = call.arguments()?;

    let mut has_string = false;
    for argument in arguments
        .items()
        .iter()
        .filter_map(|argument| argument.ok())
    {
        let Some(value) = argument.value() else {
            continue;
        };

        match &value {
            AnyRExpression::AnyRValue(value) if value.as_r_string_value().is_some() => {
                has_string = true;
            }
            AnyRExpression::AnyRValue(value) if value.as_r_bogus_value().is_none() => {}
            AnyRExpression::RFalseExpression(_)
            | AnyRExpression::RInfExpression(_)
            | AnyRExpression::RNaExpression(_)
            | AnyRExpression::RNanExpression(_)
            | AnyRExpression::RNullExpression(_)
            | AnyRExpression::RTrueExpression(_) => {}
            _ => return Ok(false),
        }
    }

    Ok(has_string)
}

fn rep_starts_with_known_character(call: &RCall) -> anyhow::Result<bool> {
    let arguments = call.arguments()?;
    let Some(first_argument) = get_arg_by_position(&arguments.items(), 1) else {
        return Ok(false);
    };
    let Some(value) = first_argument.value() else {
        return Ok(false);
    };

    if is_string_literal(&value) {
        return Ok(true);
    }

    let Some(call) = value.as_r_call() else {
        return Ok(false);
    };
    let Ok(function) = call.function() else {
        return Ok(false);
    };
    if get_function_name(function) != "c" {
        return Ok(false);
    }

    is_literal_character_combine(call)
}

fn is_string_literal(expression: &AnyRExpression) -> bool {
    expression
        .as_any_r_value()
        .is_some_and(|value| value.as_r_string_value().is_some())
}
