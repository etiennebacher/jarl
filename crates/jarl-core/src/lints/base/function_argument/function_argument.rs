use crate::diagnostic::*;
use crate::rule_set::Rule;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct FunctionArgument;

/// <!-- docs: start -->
/// Version added: 0.7.0
///
/// ## What it does
///
/// Checks for arguments without defaults that appear after arguments with
/// defaults in function definitions. The `...` argument is ignored.
///
/// ## Why is this bad?
///
/// Placing required arguments before optional arguments makes functions easier
/// to understand and call.
///
/// This rule has no automatic fix because changing the argument order can
/// break existing calls.
///
/// This rule is disabled by default.
///
/// ## Example
///
/// ```r
/// function(x = 1, y) {
///   x + y
/// }
/// ```
///
/// Use instead:
///
/// ```r
/// function(y, x = 1) {
///   x + y
/// }
/// ```
/// <!-- docs: end -->
impl Violation for FunctionArgument {
    fn rule(&self) -> Rule {
        Rule::FunctionArgument
    }

    fn body(&self) -> String {
        "Arguments without defaults should come before arguments with defaults.".to_string()
    }
}

pub fn function_argument(ast: &RFunctionDefinition) -> anyhow::Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let mut seen_default = false;

    for parameter in ast.parameters()?.items() {
        let parameter = parameter?;
        if parameter.default().is_some() {
            seen_default = true;
            continue;
        }

        let name = parameter.name()?;
        // `...` is exempt, but does not reset the ordering of other arguments.
        if !seen_default || matches!(name, AnyRParameterName::RDots(_)) {
            continue;
        }

        diagnostics.push(Diagnostic::new(
            FunctionArgument,
            name.syntax().text_trimmed_range(),
            Fix::empty(),
        ));
    }

    Ok(diagnostics)
}
