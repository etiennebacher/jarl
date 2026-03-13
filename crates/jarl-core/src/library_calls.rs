//! Extract `library()` and `require()` calls from an R file's AST.
//!
//! A lightweight pre-pass over top-level statements to determine which packages
//! are loaded, and in what order. This information is used to resolve bare
//! function names to packages.

use air_r_syntax::*;
use biome_rowan::AstNode;

use crate::utils::{get_function_name, get_function_namespace_prefix};

/// Extract package names from top-level `library()` and `require()` calls.
///
/// Returns package names in load order. Ignores:
/// - Calls inside function bodies (conditional loading, unreliable).
/// - `library(pkg, character.only = TRUE)` (dynamic, can't resolve).
/// - Calls inside for/while loops (dynamic).
pub fn extract_library_calls(root: &RExpressionList) -> Vec<String> {
    let mut packages = Vec::new();

    for expr in root {
        extract_from_top_level(&expr, &mut packages);
    }

    packages
}

/// Process a single top-level expression.
fn extract_from_top_level(expr: &AnyRExpression, packages: &mut Vec<String>) {
    match expr {
        // Direct call: `library(dplyr)`
        AnyRExpression::RCall(call) => {
            try_extract_library_call(call, packages);
        }
        // Braced block at top level (not inside a function): `{ library(dplyr) }`
        AnyRExpression::RBracedExpressions(braced) => {
            for inner in braced.expressions() {
                extract_from_top_level(&inner, packages);
            }
        }
        // Skip if/else, loops, pipes — conditional loading is unreliable.
        _ => {}
    }
}

/// Try to extract a package name from a `library(pkg)` or `require(pkg)` call.
fn try_extract_library_call(call: &RCall, packages: &mut Vec<String>) {
    let Ok(function) = call.function() else {
        return;
    };
    let fn_name = get_function_name(function.clone());
    let fn_ns = get_function_namespace_prefix(function);

    // Match library() / require() / base::library() / base::require()
    if fn_name != "library" && fn_name != "require" {
        return;
    }
    if let Some(ref ns) = fn_ns
        && ns != "base::"
    {
        return;
    }

    let Ok(args) = call.arguments() else { return };
    let items: Vec<_> = args.items().into_iter().collect();

    // Skip `library(pkg, character.only = TRUE)` — dynamic, can't resolve
    let has_character_only = items.iter().any(|item| {
        item.as_ref().is_ok_and(|arg| {
            arg.name_clause().is_some_and(|nc| {
                nc.name()
                    .is_ok_and(|n| n.to_trimmed_string() == "character.only")
            }) && arg.value().is_some_and(|v| v.to_trimmed_string() == "TRUE")
        })
    });
    if has_character_only {
        return;
    }

    // Get the first positional (unnamed) argument
    let first_arg = items.iter().find_map(|item| {
        item.as_ref().ok().and_then(|arg| {
            if arg.name_clause().is_none() {
                arg.value()
            } else {
                None
            }
        })
    });

    let Some(first_arg) = first_arg else { return };

    // Extract the package name (bare symbol or string literal)
    let pkg_name = extract_package_name(&first_arg);

    if let Some(name) = pkg_name
        && !name.is_empty()
        && !packages.contains(&name)
    {
        packages.push(name);
    }
}

/// Extract a package name from the first argument of `library()`.
///
/// Handles bare symbols (`library(dplyr)`) and string literals (`library("dplyr")`).
fn extract_package_name(expr: &AnyRExpression) -> Option<String> {
    // Bare symbol: `library(dplyr)`
    if let Some(id) = expr.as_r_identifier()
        && let Ok(token) = id.name_token()
    {
        return Some(token.token_text_trimmed().text().to_string());
    }

    // String literal: `library("dplyr")`
    if let AnyRExpression::AnyRValue(value) = expr
        && let AnyRValue::RStringValue(s) = value
    {
        let text = s.to_trimmed_string();
        let unquoted = text.trim_matches('"').trim_matches('\'');
        return Some(unquoted.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use air_r_parser::RParserOptions;

    fn parse_and_extract(code: &str) -> Vec<String> {
        let parsed = air_r_parser::parse(code, RParserOptions::default());
        assert!(!parsed.has_error(), "Parse error in test code: {code}");
        extract_library_calls(&parsed.tree().expressions())
    }

    #[test]
    fn test_basic_library() {
        assert_eq!(parse_and_extract("library(dplyr)"), vec!["dplyr"]);
    }

    #[test]
    fn test_string_library() {
        assert_eq!(parse_and_extract("library(\"dplyr\")"), vec!["dplyr"]);
    }

    #[test]
    fn test_require() {
        assert_eq!(parse_and_extract("require(ggplot2)"), vec!["ggplot2"]);
    }

    #[test]
    fn test_base_prefix() {
        assert_eq!(parse_and_extract("base::library(dplyr)"), vec!["dplyr"]);
    }

    #[test]
    fn test_multiple_libraries() {
        let code = "library(dplyr)\nlibrary(tidyr)\nlibrary(ggplot2)";
        assert_eq!(parse_and_extract(code), vec!["dplyr", "tidyr", "ggplot2"]);
    }

    #[test]
    fn test_no_duplicates() {
        let code = "library(dplyr)\nlibrary(dplyr)";
        assert_eq!(parse_and_extract(code), vec!["dplyr"]);
    }

    #[test]
    fn test_character_only_ignored() {
        let code = "library(pkg, character.only = TRUE)";
        assert!(parse_and_extract(code).is_empty());
    }

    #[test]
    fn test_non_base_namespace_ignored() {
        let code = "mypkg::library(dplyr)";
        assert!(parse_and_extract(code).is_empty());
    }

    #[test]
    fn test_inside_braces() {
        let code = "{\n  library(dplyr)\n}";
        assert_eq!(parse_and_extract(code), vec!["dplyr"]);
    }

    #[test]
    fn test_inside_if_ignored() {
        let code = "if (TRUE) library(dplyr)";
        assert!(parse_and_extract(code).is_empty());
    }

    #[test]
    fn test_empty_file() {
        assert!(parse_and_extract("").is_empty());
    }

    #[test]
    fn test_no_library_calls() {
        assert!(parse_and_extract("x <- 1\ny <- 2").is_empty());
    }
}
