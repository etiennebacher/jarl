use std::collections::HashSet;

use crate::diagnostic::*;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub struct UnusedFunctionArguments {
    arg_name: String,
}

/// ## What it does
///
/// Checks for function arguments that are never used in the function body.
///
/// ## Why is this bad?
///
/// Unused arguments can indicate dead code, copy-paste errors, or incomplete
/// refactors. They add noise to the function signature and can confuse readers.
///
/// ## Known limitations
///
/// - Arguments accessed via non-standard evaluation (NSE), e.g.
///   `dplyr::select(df, col_name)`, may be falsely flagged as unused.
/// - Arguments accessed dynamically via `get()`, `do.call()`, or `eval()`
///   won't be detected as used.
/// - The `...` parameter is always skipped.
/// - Glue interpolation is detected heuristically: any R-identifier-like
///   word inside `{...}` in a string is treated as a reference.
///
/// ## Automatically skipped
///
/// - Functions whose body calls `UseMethod()` or `NextMethod()` (S3 generics/methods)
/// - Functions assigned to `.onLoad`, `.onAttach`, `.onUnload`, or `.onDetach`
/// - Functions passed as `error`, `warning`, `message`, or `condition` handlers
///   to `tryCatch()` or `withCallingHandlers()`
///
/// ## Example
///
/// ```r
/// function(x, y) x + 1
/// ```
///
/// Use instead:
/// ```r
/// function(x) x + 1
/// ```
impl Violation for UnusedFunctionArguments {
    fn name(&self) -> String {
        "unused_function_arguments".to_string()
    }
    fn body(&self) -> String {
        format!(
            "Argument \"{}\" is not used in the function body.",
            self.arg_name
        )
    }
}

pub fn unused_function_arguments(func: &RFunctionDefinition) -> anyhow::Result<Vec<Diagnostic>> {
    let body = func.body()?;

    // Skip S3 generics/methods: functions calling UseMethod() or NextMethod()
    if body_contains_dispatch_call(&body) {
        return Ok(Vec::new());
    }

    // Skip functions assigned to R hook names (.onLoad, .onAttach, etc.)
    if is_assigned_to_hook(func) {
        return Ok(Vec::new());
    }

    // Skip tryCatch/withCallingHandlers handler functions
    if is_trycatch_handler(func) {
        return Ok(Vec::new());
    }

    let params = func.parameters()?.items();

    // Collect all identifier names used in the body by walking the syntax tree
    let body_identifiers = collect_body_identifiers(&body);

    let mut diagnostics = Vec::new();

    for param in params {
        let param = param?;
        let param_name_node = param.name()?;

        // Skip `...` and `..1`, `..2`, etc.
        match &param_name_node {
            AnyRParameterName::RDots(_) | AnyRParameterName::RDotDotI(_) => continue,
            AnyRParameterName::RIdentifier(_) => {}
        }

        let param_text = param_name_node.into_syntax().text_trimmed().to_string();

        if !body_identifiers.contains(param_text.as_str()) {
            let range = param.syntax().text_trimmed_range();
            let diagnostic = Diagnostic::new(
                UnusedFunctionArguments { arg_name: param_text },
                range,
                Fix::empty(),
            );
            diagnostics.push(diagnostic);
        }
    }

    Ok(diagnostics)
}

/// Walk the body's syntax tree and collect all `RIdentifier` text values.
///
/// Also collects:
/// - Bare keyword tokens (e.g. `return`, `if`) since R allows keywords as
///   parameter names, and the parser represents bare keyword references as
///   keyword nodes rather than identifiers.
/// - Identifiers referenced via glue interpolation in strings (e.g. `"{x}"`).
fn collect_body_identifiers(body: &AnyRExpression) -> HashSet<String> {
    let mut identifiers = HashSet::new();

    for node in body.syntax().descendants() {
        if let Some(ident) = RIdentifier::cast(node.clone()) {
            identifiers.insert(ident.syntax().text_trimmed().to_string());
        } else if let Some(string_val) = RStringValue::cast(node)
            && let Ok(token) = string_val.value_token()
        {
            let text = token.text_trimmed();
            extract_glue_identifiers(text, &mut identifiers);
        }
    }

    // TODO: remove when tree-sitter bug is fixed
    // https://github.com/r-lib/tree-sitter-r/issues/190
    for token in body
        .syntax()
        .descendants_tokens(biome_rowan::Direction::Next)
    {
        if token.kind() == RSyntaxKind::RETURN_KW {
            identifiers.insert(token.text_trimmed().to_string());
        }
    }

    identifiers
}

/// Extract all identifier-like words from glue interpolation patterns
/// in a string literal.
///
/// For each `{...}` block (skipping escaped `{{`), extracts all R-identifier-like
/// tokens from the content. This handles both simple references like `{x}` and
/// complex expressions like `{x + 1}` or `{paste0(a, b)}`.
fn extract_glue_identifiers(s: &str, identifiers: &mut HashSet<String>) {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'{' {
            // Check for escaped brace `{{`
            if i + 1 < len && bytes[i + 1] == b'{' {
                i += 2;
                continue;
            }

            // Find the closing `}`
            if let Some(close) = s[i + 1..].find('}') {
                let content = &s[i + 1..i + 1 + close];
                extract_r_identifiers_from_expr(content, identifiers);
                i = i + 1 + close + 1;
            } else {
                break;
            }
        } else {
            i += 1;
        }
    }
}

/// Extract all R-identifier-like words from an expression string.
///
/// Splits on non-identifier characters and collects each valid R identifier.
/// For example, `"x + 1"` yields `["x"]`, `"paste0(a, b)"` yields
/// `["paste0", "a", "b"]`.
fn extract_r_identifiers_from_expr(s: &str, identifiers: &mut HashSet<String>) {
    let mut start = None;

    for (i, c) in s.char_indices() {
        let is_ident_char = c.is_alphanumeric() || c == '.' || c == '_';
        match (is_ident_char, start) {
            (true, None) => start = Some(i),
            (false, Some(s_idx)) => {
                let word = &s[s_idx..i];
                if is_r_identifier_start(word) {
                    identifiers.insert(word.to_string());
                }
                start = None;
            }
            _ => {}
        }
    }

    // Handle trailing identifier
    if let Some(s_idx) = start {
        let word = &s[s_idx..];
        if is_r_identifier_start(word) {
            identifiers.insert(word.to_string());
        }
    }
}

/// Check if a word starts like a valid R identifier (not a number).
fn is_r_identifier_start(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let first = s.chars().next().unwrap();
    first.is_alphabetic() || first == '.' || first == '_'
}

/// Check if the function body contains a call to `UseMethod()` or `NextMethod()`.
fn body_contains_dispatch_call(body: &AnyRExpression) -> bool {
    for node in body.syntax().descendants() {
        if let Some(call) = RCall::cast(node)
            && let Ok(function) = call.function()
        {
            let name = function.into_syntax().text_trimmed().to_string();
            if name == "UseMethod" || name == "NextMethod" {
                return true;
            }
        }
    }
    false
}

/// Check if this function definition is assigned to an R hook name.
///
/// Looks at the parent AST to find patterns like:
///   `.onLoad <- function(libname, pkgname) { ... }`
///   `.onLoad = function(libname, pkgname) { ... }`
fn is_assigned_to_hook(func: &RFunctionDefinition) -> bool {
    const HOOK_NAMES: &[&str] = &[".onLoad", ".onAttach", ".onUnload", ".onDetach"];

    let parent = match func.syntax().parent() {
        Some(p) => p,
        None => return false,
    };

    // The function definition is the RHS of a binary expression (assignment)
    if let Some(binary) = RBinaryExpression::cast(parent)
        && let Ok(left) = binary.left()
    {
        let name = left.into_syntax().text_trimmed().to_string();
        return HOOK_NAMES.contains(&name.as_str());
    }

    false
}

/// Check if this function is a handler argument to `tryCatch()` or
/// `withCallingHandlers()`.
///
/// Detects patterns like:
///   `tryCatch(expr, error = function(e) ...)`
///   `withCallingHandlers(expr, warning = function(w) ...)`
fn is_trycatch_handler(func: &RFunctionDefinition) -> bool {
    const HANDLER_NAMES: &[&str] = &["error", "warning", "message", "condition", "interrupt"];
    const HANDLER_FUNCTIONS: &[&str] = &["tryCatch", "try_fetch", "withCallingHandlers"];

    // Walk up: function_definition -> R_ARGUMENT (value) -> R_ARGUMENT_LIST -> R_CALL_ARGUMENTS -> R_CALL
    let arg_node = match func.syntax().parent() {
        Some(p) if p.kind() == RSyntaxKind::R_ARGUMENT => p,
        _ => return false,
    };

    // Check the argument name
    if let Some(argument) = RArgument::cast(arg_node.clone()) {
        let fields = argument.as_fields();
        if let Some(name_clause) = &fields.name_clause {
            if let Ok(name) = name_clause.name() {
                let arg_name = name.to_trimmed_string();
                if !HANDLER_NAMES.contains(&arg_name.as_str()) {
                    return false;
                }
            } else {
                return false;
            }
        } else {
            return false;
        }
    } else {
        return false;
    }

    // Walk up to the call: R_ARGUMENT -> R_ARGUMENT_LIST -> R_CALL_ARGUMENTS -> R_CALL
    let call_node = arg_node
        .parent() // R_ARGUMENT_LIST
        .and_then(|n| n.parent()) // R_CALL_ARGUMENTS
        .and_then(|n| n.parent()); // R_CALL

    if let Some(call_node) = call_node
        && let Some(call) = RCall::cast(call_node)
        && let Ok(function) = call.function()
    {
        let name = function.into_syntax().text_trimmed().to_string();
        return HANDLER_FUNCTIONS.contains(&name.as_str());
    }

    false
}
