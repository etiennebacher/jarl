use crate::config::Config;
use anyhow::{Result, bail};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Try to find the git repository root for a given file path.
/// Returns `Some(repo_root)` if found, `None` otherwise (e.g. if git isn't used
/// in the folder or isn't installed).
fn discover_repo(path: &str) -> Option<String> {
    let dir = match Path::new(path).parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };

    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(dir)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Get the list of dirty (modified, untracked, staged) files in a repo.
fn dirty_files(repo_root: &str) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_root)
        .output()?;

    let mut files = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        // porcelain format: "XY filename" where XY is the two-char status
        if let Some(name) = line.get(3..) {
            files.push(name.to_string());
        }
    }
    Ok(files)
}

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
        match discover_repo(path) {
            Some(repo_root) => {
                repo_to_paths
                    .entry(repo_root)
                    .or_default()
                    .push(path.clone());
            }
            None => {
                paths_without_repo.push(path.clone());
            }
        }
    }

    // Check if any paths are not in a repo
    if !paths_without_repo.is_empty() {
        bail!(
            "`jarl check --fix` can potentially perform destructive changes but no \
            Version Control System (e.g. Git) was found on this project, so no fixes \
            were applied.\n\
            Add `--allow-no-vcs` to the call to apply the fixes."
        )
    }

    if config.allow_dirty {
        return Ok(());
    }

    // Check each repository once
    let mut all_dirty_files = Vec::new();

    for repo_root in repo_to_paths.keys() {
        all_dirty_files.extend(dirty_files(repo_root)?);
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
            directory of this project has uncommitted changes, so no fixes were applied.\n\
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
