use crate::config::Config;
use anyhow::{Result, bail};
use std::collections::HashMap;
use std::path::Path;

/// Check version control status once for multiple paths.
///
/// The ideal case would be that we know that all paths are either not tracked
/// by VCS or part of the same repo. However, it is completely possible that
/// Jarl is called from a directory where subdirs are different R projects, some
/// not covered by VCS, some covered by VCS but dirty, and some clean.
///
/// Therefore, we cannot just take the first path, check if it's covered by VCS
/// and then get the statuses of all our paths in this repo. We have to loop
/// through paths. This doesn't necessarily result in a big perf hit: what takes
/// time is to get the statuses of the paths, so we limit the calls to statuses
/// by grouping files per repo first. Then, we go through the repos to get the
/// statuses (only once per repo).
pub fn check_version_control(paths: &[String], config: &Config) -> Result<()> {
    if config.allow_no_vcs {
        return Ok(());
    }

    // Group paths by their repository root
    let mut repo_to_paths: HashMap<String, Vec<String>> = HashMap::new();
    let mut paths_without_repo: Vec<String> = Vec::new();

    for path in paths {
        match git2::Repository::discover(Path::new(path)) {
            Ok(repo) => {
                // Get the repository root path as a key
                let repo_path = repo
                    .path()
                    .parent()
                    .and_then(|p| p.to_str())
                    .unwrap_or("")
                    .to_string();
                repo_to_paths
                    .entry(repo_path)
                    .or_default()
                    .push(path.clone());
            }
            Err(_) => {
                paths_without_repo.push(path.clone());
            }
        }
    }

    // Check if any paths are not in a repo
    if !paths_without_repo.is_empty() {
        bail!(
            "`jarl check --fix` can potentially perform destructive changes but no \
            Version Control System (e.g. Git) was found on this project, so no fixes \
            were applied. \n\
            Add `--allow-no-vcs` to the call to apply the fixes."
        )
    }

    if config.allow_dirty {
        return Ok(());
    }

    // Check each repository once
    let mut all_dirty_files = Vec::new();

    for (repo_path, _paths) in repo_to_paths {
        let repo = git2::Repository::discover(Path::new(&repo_path))?;

        let mut repo_opts = git2::StatusOptions::new();
        repo_opts.include_ignored(false);
        repo_opts.include_untracked(true);

        for status in repo.statuses(Some(&mut repo_opts))?.iter() {
            if let Some(path) = status.path() {
                match status.status() {
                    git2::Status::CURRENT => (),
                    _ => {
                        all_dirty_files.push(path.to_string());
                    }
                };
            }
        }
    }

    if !all_dirty_files.is_empty() {
        let mut files_list = String::new();
        for file in &all_dirty_files {
            files_list.push_str("  * ");
            files_list.push_str(file);
            files_list.push_str(" (dirty)\n");
        }

        bail!(
            "`jarl check --fix` can potentially perform destructive changes but the working \
            directory of this project has uncommitted changes, so no fixes were applied. \n\
            To apply the fixes, either add `--allow-dirty` to the call, or commit the changes \
            to these files:\n\
             \n\
             {}\n\
             ",
            files_list
        );
    }

    Ok(())
}
