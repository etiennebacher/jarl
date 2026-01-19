use colored::Colorize;
use jarl_core::diagnostic::Diagnostic;
use std::{collections::HashMap, path::PathBuf};

use crate::status::ExitStatus;

pub fn print_statistics(
    diagnostics: &[&Diagnostic],
    parent_config_path: Option<PathBuf>,
) -> anyhow::Result<ExitStatus> {
    if diagnostics.is_empty() {
        println!("All checks passed!");
        return Ok(ExitStatus::Success);
    }

    // Hashmap with rule name as key, and (number of occurrences, has_fix) as
    // value.
    let mut hm: HashMap<&String, (usize, bool)> = HashMap::new();

    for diagnostic in diagnostics {
        let rule_name = &diagnostic.message.name;
        hm.entry(rule_name).or_default().0 += 1;
        if diagnostic.has_safe_fix() && !hm.entry(rule_name).or_default().1 {
            hm.entry(rule_name).or_default().1 = true;
        }
    }

    let mut sorted: Vec<_> = hm.iter().collect();
    sorted.sort_by_key(|a| a.1.0);
    sorted.reverse();

    for (key, value) in sorted {
        let star = if value.1 { "*" } else { " " };
        println!(
            "{:>5} [{}] {}",
            value.0.to_string().bold(),
            star,
            key.bold().red()
        );
    }

    println!("\nRules with `[*]` have an automatic fix.");

    // Inform the user if the config file used comes from a parent directory.
    if let Some(config_path) = parent_config_path {
        println!("\nUsed '{}'", config_path.display());
    }

    Ok(ExitStatus::Failure)
}
