use anyhow::Result;
use colored::Colorize;

use jarl_core::config::suggest_rules;
use jarl_core::rule_docs::rule_doc;
use jarl_core::rule_set::{DefaultStatus, FixStatus, Rule};

use crate::args::RuleCommand;
use crate::status::ExitStatus;

/// Maximum number of "did you mean" rule names printed for an unknown rule.
const MAX_SUGGESTIONS: usize = 5;

pub fn rule(args: RuleCommand) -> Result<ExitStatus> {
    let Some(rule) = Rule::from_name(&args.name) else {
        eprintln!("{}: unknown rule `{}`.", "error".red().bold(), args.name);
        let suggestions = suggest_rules(&args.name);
        if !suggestions.is_empty() {
            let quoted = suggestions
                .iter()
                .take(MAX_SUGGESTIONS)
                .map(|s| format!("`{s}`"))
                .collect::<Vec<_>>()
                .join(", ");
            let remaining = suggestions.len().saturating_sub(MAX_SUGGESTIONS);
            if remaining > 0 {
                eprintln!("  Did you mean: {quoted} (and {remaining} more)?");
            } else {
                eprintln!("  Did you mean: {quoted}?");
            }
        }
        eprintln!("Run `jarl check --help` for how to select rules.");
        return Ok(ExitStatus::Error);
    };

    print!("{}", format_rule(rule));

    Ok(ExitStatus::Success)
}

/// Render a rule's metadata header followed by its embedded documentation.
fn format_rule(rule: Rule) -> String {
    let mut out = String::new();

    out.push_str(&format!("{}\n", rule.name().bold()));

    let categories = rule
        .categories()
        .iter()
        .map(|category| category.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!("{} {categories}\n", "Categories:".bold()));

    let enabled_by_default = match rule.default_status() {
        DefaultStatus::Enabled => "yes",
        DefaultStatus::Disabled => "no",
    };
    out.push_str(&format!(
        "{} {enabled_by_default}\n",
        "Enabled by default:".bold()
    ));

    let fix = match rule.fix_status() {
        FixStatus::None => "not available",
        FixStatus::Safe => "safe",
        FixStatus::Unsafe => "unsafe (requires `--unsafe-fixes`)",
    };
    out.push_str(&format!("{} {fix}\n", "Fix:".bold()));

    if let Some((major, minor, patch)) = rule.minimum_r_version() {
        out.push_str(&format!(
            "{} {major}.{minor}.{patch}\n",
            "Minimum R version:".bold()
        ));
    }

    if let Some(info) = rule.deprecation() {
        out.push_str(&format!(
            "{} deprecated since {}, use `{}` instead\n",
            "Note:".bold(),
            info.version,
            info.replacement
        ));
    }

    match rule_doc(rule.name()) {
        Some(doc) => {
            out.push('\n');
            out.push_str(&strip_quarto(doc));
        }
        None => {
            out.push_str("\nNo detailed documentation is available for this rule yet.\n");
        }
    }

    out
}

/// Strip the Quarto-specific parts of a generated rule doc so it reads cleanly
/// in a terminal: the leading `# <name>` title (shown in the header already)
/// and the `::: {.callout-note …}` "Added in" fence.
fn strip_quarto(doc: &str) -> String {
    let mut lines = Vec::new();

    for line in doc.lines() {
        let trimmed = line.trim_start();

        // Drop the top-level title line (the header already prints the name).
        if line.starts_with("# ") {
            continue;
        }

        // Turn the callout fence into a plain "Added in X" line, dropping the
        // `:::` delimiters.
        if trimmed.starts_with(":::") {
            if let Some(start) = line.find("title=\"") {
                let rest = &line[start + "title=\"".len()..];
                if let Some(end) = rest.find('"') {
                    lines.push(rest[..end].to_string());
                }
            }
            continue;
        }

        lines.push(line.to_string());
    }

    // Collapse the blank lines that surrounded the stripped title/callout.
    let mut result = lines.join("\n");
    while result.starts_with('\n') {
        result.remove(0);
    }
    if !result.ends_with('\n') {
        result.push('\n');
    }
    result
}
