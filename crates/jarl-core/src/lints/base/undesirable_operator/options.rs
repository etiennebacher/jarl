use std::collections::{HashMap, HashSet};

use crate::rule_options::resolve_with_extend;

/// Default operators that are considered undesirable.
const DEFAULT_OPERATORS: &[&str] = &["->>", ":::", "<<-"];

/// An operator, or a map from one operator to a custom message.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum UndesirableOperatorEntry {
    Name(String),
    Message(
        #[cfg_attr(feature = "schemars", schemars(with = "HashMap<String, String>"))]
        HashMap<String, toml::Value>,
    ),
}

/// <!-- docs: start -->
/// Use `operators` to fully replace the default list of undesirable operators.
/// Use `extend-operators` to add to the default list.
/// Specifying both is an error.
///
/// Entries can be strings or inline tables mapping an operator to a custom
/// message. Operator names in inline tables must be quoted.
///
/// ### Default values
///
/// ```toml
/// operators = ["->>", ":::", "<<-"]
/// ```
///
/// ### TOML settings
///
/// ```toml
/// [lint.undesirable_operator]
/// # Replace the default list entirely:
/// operators = [":::", "%in%"]
///
/// # Or add to the defaults:
/// extend-operators = ["%in%"]
///
/// # Or add to the defaults, with optional messages:
/// extend-operators = [{ "%notin%" = "Use `!(x %in% y)` instead." }]
/// ```
/// <!-- docs: end -->
#[derive(Clone, Debug, PartialEq, Default, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct UndesirableOperatorOptions {
    pub operators: Option<Vec<UndesirableOperatorEntry>>,
    pub extend_operators: Option<Vec<UndesirableOperatorEntry>>,
}

/// Resolved options for the `undesirable_operator` rule, ready for use during
/// linting.
#[derive(Clone, Debug)]
pub struct ResolvedUndesirableOperatorOptions {
    pub operators: HashSet<String>,
    pub messages: HashMap<String, String>,
}

impl ResolvedUndesirableOperatorOptions {
    pub fn resolve(options: Option<&UndesirableOperatorOptions>) -> anyhow::Result<Self> {
        let (base, extend) = match options {
            Some(opts) => (opts.operators.as_ref(), opts.extend_operators.as_ref()),
            None => (None, None),
        };

        let base_names = base.map(|entries| entry_names(entries));
        let extend_names = extend.map(|entries| entry_names(entries));
        let operators = resolve_with_extend(
            base_names.as_ref(),
            extend_names.as_ref(),
            DEFAULT_OPERATORS,
            "undesirable_operator",
            "operators",
        )?;
        let mut messages = HashMap::new();
        if let Some(entries) = base.or(extend) {
            add_messages(entries, &mut messages)?;
        }

        Ok(Self { operators, messages })
    }
}

fn entry_names(entries: &[UndesirableOperatorEntry]) -> Vec<String> {
    let mut names = Vec::new();
    for entry in entries {
        match entry {
            UndesirableOperatorEntry::Name(operator) => names.push(operator.clone()),
            UndesirableOperatorEntry::Message(entries) => names.extend(entries.keys().cloned()),
        }
    }
    names
}

fn add_messages(
    entries: &[UndesirableOperatorEntry],
    messages: &mut HashMap<String, String>,
) -> anyhow::Result<()> {
    for entry in entries {
        if let UndesirableOperatorEntry::Message(entries) = entry {
            for (operator, message) in entries {
                validate_operator_name(operator)?;
                let Some(message) = message.as_str() else {
                    anyhow::bail!(
                        "Message for `{operator}` in `[lint.undesirable_operator]` must be a string."
                    );
                };
                if message.trim().is_empty() {
                    anyhow::bail!(
                        "Message for `{operator}` in `[lint.undesirable_operator]` cannot be empty."
                    );
                }
                messages.insert(operator.clone(), message.to_string());
            }
        } else if let UndesirableOperatorEntry::Name(operator) = entry {
            validate_operator_name(operator)?;
        }
    }
    Ok(())
}

fn validate_operator_name(operator: &str) -> anyhow::Result<()> {
    if operator.is_empty() {
        anyhow::bail!("Operator name cannot be empty in `[lint.undesirable_operator]`.");
    }
    if operator.trim() != operator {
        anyhow::bail!(
            "Operator name `{operator}` cannot have leading or trailing whitespace in `[lint.undesirable_operator]`."
        );
    }
    Ok(())
}
