use air_r_parser::RParserOptions;

use flir::cache::LinterCache;
use flir::check_ast::*;
use flir::fix::*;
use flir::message::*;
use flir::utils::parse_rules;

use clap::{arg, Parser};
use rayon::prelude::*;
use std::default;
use std::fs;
use std::path::{Path, PathBuf};
// use std::time::Instant;
use anyhow::{Context, Result};
use walkdir::WalkDir;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(
    author,
    name = "flir",
    about = "Flint: Find and Fix Lints in R Code",
    after_help = "For help with a specific command, see: `flir help <command>`."
)]
struct Args {
    #[arg(
        short,
        long,
        default_value = ".",
        help = "The directory in which to check or fix lints."
    )]
    dir: String,
    #[arg(
        short,
        long,
        default_value = "false",
        help = "Automatically fix issues detected by the linter."
    )]
    fix: bool,
    #[arg(
        short,
        long,
        default_value = "",
        help = "Names of rules to include, separated by a comma (no spaces)."
    )]
    rules: String,
    #[arg(long, env = "FLIR_CACHE_DIR", help_heading = "Miscellaneous")]
    cache_dir: Option<PathBuf>,
}

/// This is my first rust crate
fn main() -> Result<()> {
    // let start = Instant::now();
    let args = Args::parse();

    let r_files = WalkDir::new(args.dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path().extension() == Some(std::ffi::OsStr::new("R"))
                || e.path().extension() == Some(std::ffi::OsStr::new("r"))
        })
        .map(|e| e.path().to_path_buf())
        .collect::<Vec<_>>();

    let rules = parse_rules(&args.rules);
    let rules_hashed = LinterCache::hash_rules(&rules);

    let mut cache = if let Some(cache_dir) = args.cache_dir {
        LinterCache::load_from_disk(&cache_dir)?
    } else {
        LinterCache::new()
    };

    // let r_files = vec![Path::new("demo/foo.R").to_path_buf()];

    let parser_options = RParserOptions::default();
    let result: Result<Vec<Diagnostic>, anyhow::Error> = r_files
        .par_iter()
        .map(|file| {
            let mut checks: Vec<Diagnostic>;
            let skip_cache = cache.is_cache_valid(file, rules_hashed);

            println!("skip_cache: {}", skip_cache);

            if skip_cache {
                checks = cache.get_cached_checks(file).unwrap().to_vec();
            } else {
                let mut has_skipped_fixes = true;
                loop {
                    // Add file context to the read error
                    let contents = fs::read_to_string(Path::new(file))
                        .with_context(|| format!("Failed to read file: {}", file.display()))?;

                    // Add file context to the get_checks error
                    checks = get_checks(&contents, file, parser_options, rules.clone())
                        .with_context(|| {
                            format!("Failed to get checks for file: {}", file.display())
                        })?;

                    if !has_skipped_fixes || !args.fix {
                        break;
                    }

                    let (new_has_skipped_fixes, fixed_text) = apply_fixes(&checks, &contents);
                    has_skipped_fixes = new_has_skipped_fixes;

                    // Add file context to the write error
                    fs::write(file, fixed_text)
                        .with_context(|| format!("Failed to write file: {}", file.display()))?;
                }

                cache.update_cache(file.into(), &checks, rules_hashed);
            }
            cache.save_to_disk(&file)?;

            if !args.fix && !checks.is_empty() {
                for message in &checks {
                    println!("{}", message);
                }
            }

            Ok(checks)
        })
        .flat_map(|result| match result {
            Ok(checks) => checks.into_par_iter().map(Ok).collect::<Vec<_>>(),
            Err(e) => vec![Err(e)],
        })
        .collect();

    match result {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{:?}", e);
        }
    };

    Ok(())
    // let duration = start.elapsed();
    // println!("Checked files in: {:?}", duration);
}
