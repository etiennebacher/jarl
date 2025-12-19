use colored::Colorize;
use jarl_core::diagnostic::Diagnostic;
use std::collections::HashMap;

use crate::status::ExitStatus;

pub fn print_statistics(diagnostics: &[&Diagnostic]) -> anyhow::Result<ExitStatus> {
    if diagnostics.len() == 0 {
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

    return Ok(ExitStatus::Failure);
}
