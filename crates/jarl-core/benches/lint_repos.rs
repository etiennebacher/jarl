use air_workspace::resolve::PathResolver;
use jarl_core::check::check;
use jarl_core::config::{ArgsConfig, build_config};
use jarl_core::discovery::{DiscoveredSettings, discover_r_file_paths, discover_settings};
use jarl_core::settings::Settings;
use std::path::PathBuf;

fn main() {
    divan::main();
}

/// Benchmark linting a repository directory
#[divan::bench]
fn lint_repository(bencher: divan::Bencher) {
    // Get the repository path from environment variable
    let repo_path =
        std::env::var("BENCH_REPO_PATH").expect("BENCH_REPO_PATH environment variable must be set");

    let path = PathBuf::from(&repo_path);

    bencher.bench_local(|| {
        // Discover settings and paths
        let mut resolver = PathResolver::new(Settings::default());
        let files = vec![path.clone()];

        if let Ok(discovered) = discover_settings(&files) {
            for DiscoveredSettings { directory, settings } in discovered {
                resolver.add(&directory, settings);
            }
        }

        let paths: Vec<PathBuf> = discover_r_file_paths(&files, &resolver, true)
            .into_iter()
            .filter_map(Result::ok)
            .collect();

        if !paths.is_empty() {
            let check_config = ArgsConfig {
                files: vec![path.clone()],
                fix: false,
                unsafe_fixes: false,
                fix_only: false,
                select_rules: String::new(),
                ignore_rules: String::new(),
                min_r_version: None,
                allow_dirty: false,
                allow_no_vcs: true,
                assignment_op: None,
            };

            if let Ok(config) = build_config(&check_config, &resolver, paths) {
                let _results = check(config);
            }
        }
    });
}

/// Benchmark linting with diagnostic counting
#[divan::bench]
fn lint_repository_count_diagnostics(bencher: divan::Bencher) {
    let repo_path =
        std::env::var("BENCH_REPO_PATH").expect("BENCH_REPO_PATH environment variable must be set");

    let path = PathBuf::from(&repo_path);

    bencher
        .counter(divan::counter::ItemsCount::new(|| {
            // Count the number of R files that will be processed
            count_r_files(&path)
        }))
        .bench_local(|| {
            let mut resolver = PathResolver::new(Settings::default());
            let files = vec![path.clone()];

            if let Ok(discovered) = discover_settings(&files) {
                for DiscoveredSettings { directory, settings } in discovered {
                    resolver.add(&directory, settings);
                }
            }

            let paths: Vec<PathBuf> = discover_r_file_paths(&files, &resolver, true)
                .into_iter()
                .filter_map(Result::ok)
                .collect();

            if !paths.is_empty() {
                let check_config = ArgsConfig {
                    files: vec![path.clone()],
                    fix: false,
                    unsafe_fixes: false,
                    fix_only: false,
                    select_rules: String::new(),
                    ignore_rules: String::new(),
                    min_r_version: None,
                    allow_dirty: false,
                    allow_no_vcs: true,
                    assignment_op: None,
                };

                if let Ok(config) = build_config(&check_config, &resolver, paths) {
                    let results = check(config);

                    // Count total diagnostics across all files
                    let total_diagnostics: usize = results
                        .iter()
                        .filter_map(|(_, result)| result.as_ref().ok())
                        .map(|diagnostics| diagnostics.len())
                        .sum();

                    divan::black_box(total_diagnostics);
                }
            }
        });
}

/// Helper function to count R files in a directory recursively
fn count_r_files(path: &PathBuf) -> usize {
    use std::fs;

    let mut count = 0;

    if path.is_file() {
        if let Some(ext) = path.extension() {
            if ext == "R" || ext == "r" {
                return 1;
            }
        }
        return 0;
    }

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();

            if entry_path.is_file() {
                if let Some(ext) = entry_path.extension() {
                    if ext == "R" || ext == "r" {
                        count += 1;
                    }
                }
            } else if entry_path.is_dir() {
                // Skip hidden directories and common non-source directories
                if let Some(name) = entry_path.file_name() {
                    let name_str = name.to_string_lossy();
                    if !name_str.starts_with('.')
                        && name_str != "target"
                        && name_str != "renv"
                        && name_str != "node_modules"
                        && name_str != ".git"
                    {
                        count += count_r_files(&entry_path);
                    }
                }
            }
        }
    }

    count
}
