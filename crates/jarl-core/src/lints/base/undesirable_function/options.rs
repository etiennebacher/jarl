use std::collections::{HashMap, HashSet};

use crate::rule_options::resolve_with_extend;

/// Default functions that are considered undesirable.
const DEFAULT_FUNCTIONS: &[&str] = &["browser"];

/// A function name, or a map from one or more function names to suggestions.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum UndesirableFunctionEntry {
    Name(String),
    Message(
        #[cfg_attr(feature = "schemars", schemars(with = "HashMap<String, String>"))]
        HashMap<String, toml::Value>,
    ),
}

/// TOML options for `[lint.undesirable_function]`.
///
/// <!-- docs: start -->
/// Use `functions` to fully replace the default list of undesirable functions.
/// Use `extend-functions` to add to the default list.
/// Entries can be strings or inline tables mapping a function to a custom
/// suggestion.
/// Specifying both is an error.
///
/// ### Default values
///
/// ```toml
/// functions = ["browser"]
/// ```
///
/// ### TOML settings
///
/// ```toml
/// [lint.undesirable_function]
/// # Replace the default list entirely:
/// functions = ["browser", "debug"]
///
/// # Or add to the defaults:
/// extend-functions = ["debug"]
/// ```
/// <!-- docs: end -->
#[derive(Clone, Debug, PartialEq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct UndesirableFunctionOptions {
    pub functions: Option<Vec<UndesirableFunctionEntry>>,
    pub extend_functions: Option<Vec<UndesirableFunctionEntry>>,
}

/// Resolved options for the `undesirable_function` rule, ready for use during
/// linting.
#[derive(Clone, Debug)]
pub struct ResolvedUndesirableFunctionOptions {
    pub functions: HashSet<String>,
    pub messages: HashMap<String, String>,
}

impl ResolvedUndesirableFunctionOptions {
    pub fn resolve(options: Option<&UndesirableFunctionOptions>) -> anyhow::Result<Self> {
        let (base, extend) = match options {
            Some(opts) => (opts.functions.as_ref(), opts.extend_functions.as_ref()),
            None => (None, None),
        };

        let base_names = base.map(|entries| entry_names(entries));
        let extend_names = extend.map(|entries| entry_names(entries));
        let functions = resolve_with_extend(
            base_names.as_ref(),
            extend_names.as_ref(),
            DEFAULT_FUNCTIONS,
            "undesirable_function",
            "functions",
        )?;
        let mut messages = HashMap::new();

        if let Some(entries) = base.or(extend) {
            add_messages(entries, &mut messages)?;
        }

        Ok(Self { functions, messages })
    }
}

fn entry_names(entries: &[UndesirableFunctionEntry]) -> Vec<String> {
    let mut names = Vec::new();

    for entry in entries {
        match entry {
            UndesirableFunctionEntry::Name(function) => names.push(function.clone()),
            UndesirableFunctionEntry::Message(entries) => names.extend(entries.keys().cloned()),
        }
    }

    names
}

fn add_messages(
    entries: &[UndesirableFunctionEntry],
    messages: &mut HashMap<String, String>,
) -> anyhow::Result<()> {
    for entry in entries {
        match entry {
            UndesirableFunctionEntry::Name(function) => validate_function_name(function)?,
            UndesirableFunctionEntry::Message(entries) => {
                for (function, message) in entries {
                    validate_function_name(function)?;
                    let Some(message) = message.as_str() else {
                        anyhow::bail!(
                            "Suggestion for `{function}` in `[lint.undesirable_function]` must be a string."
                        );
                    };
                    if message.trim().is_empty() {
                        anyhow::bail!(
                            "Suggestion for `{function}` in `[lint.undesirable_function]` cannot be empty."
                        );
                    }
                    messages.insert(function.clone(), message.to_string());
                }
            }
        }
    }
    Ok(())
}

fn validate_function_name(function: &str) -> anyhow::Result<()> {
    if function.is_empty() {
        anyhow::bail!("Function name cannot be empty in `[lint.undesirable_function]`.");
    }
    if function.trim() != function {
        anyhow::bail!(
            "Function name `{function}` cannot have leading or trailing whitespace in `[lint.undesirable_function]`."
        );
    }
    let (package, name) = match function.split_once("::") {
        Some((package, name)) if !name.contains("::") => (Some(package), name),
        Some(_) => {
            anyhow::bail!(
                "Function name `{function}` can contain at most one `::` in `[lint.undesirable_function]`."
            )
        }
        None => (None, function),
    };
    let valid_identifier = |value: &str| {
        let mut chars = value.chars();
        chars
            .next()
            .is_some_and(|first| first.is_alphabetic() || first == '.')
            && chars.all(|ch| ch.is_alphanumeric() || ch == '.' || ch == '_')
            && !(value.starts_with('.')
                && value.chars().nth(1).is_some_and(|ch| ch.is_ascii_digit()))
            && !matches!(
                value,
                "if" | "else"
                    | "repeat"
                    | "while"
                    | "function"
                    | "for"
                    | "in"
                    | "next"
                    | "break"
                    | "TRUE"
                    | "FALSE"
                    | "NULL"
                    | "Inf"
                    | "NaN"
                    | "NA"
                    | "NA_integer_"
                    | "NA_real_"
                    | "NA_complex_"
                    | "NA_character_"
                    | "true"
                    | "false"
            )
    };
    if name.is_empty()
        || name.trim() != name
        || !valid_identifier(name)
        || package.is_some_and(|package| !valid_identifier(package))
    {
        anyhow::bail!(
            "Function name `{function}` must be a valid R identifier or use the form `package::name` in `[lint.undesirable_function]`."
        );
    }
    Ok(())
}
