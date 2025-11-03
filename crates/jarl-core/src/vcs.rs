use crate::config::Config;
use anyhow::{Result, bail};
use std::env;
use std::path::Path;

fn in_git_repo(path: &String) -> bool {
    let path = Path::new(path);
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir().unwrap_or_default().join(path)
    };

    // If path is a file, use its parent directory for discovery
    let discovery_path = if abs_path.is_file() {
        abs_path.parent().unwrap_or(&abs_path)
    } else {
        &abs_path
    };

    GitRepo::discover(discovery_path).is_ok()
}

pub struct GitRepo;

impl GitRepo {
    pub fn init(path: &Path) -> Result<GitRepo> {
        gix::init(path)?;
        Ok(GitRepo)
    }
    pub fn discover(path: &Path) -> Result<gix::Repository, gix::discover::Error> {
        gix::discover(path)
    }
}

pub fn check_version_control(path: &String, config: &Config) -> Result<()> {
    if config.allow_no_vcs {
        return Ok(());
    }
    if !in_git_repo(path) {
        // Do not add too many line breaks here so that the text wraps the terminal
        // width.
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

    let mut dirty_files = Vec::new();

    let path_buf = Path::new(path);
    let abs_path = if path_buf.is_absolute() {
        path_buf.to_path_buf()
    } else {
        env::current_dir().unwrap_or_default().join(path_buf)
    };

    // If path is a file, use its parent directory for discovery
    let discovery_path = if abs_path.is_file() {
        abs_path.parent().unwrap_or(&abs_path)
    } else {
        &abs_path
    };

    if let Ok(repo) = gix::discover(discovery_path) {
        let platform = repo.status(gix::progress::Discard)?;

        for item in platform.into_iter(None)? {
            let item = item?;

            // Collect any files that have changes (worktree or index)
            if let gix::status::Item::IndexWorktree(worktree_item) = item {
                let path_bytes = worktree_item.rela_path();
                if let Ok(path_str) = std::str::from_utf8(path_bytes.as_ref()) {
                    dirty_files.push(path_str.to_string());
                }
            }
        }
    }

    if dirty_files.is_empty() {
        return Ok(());
    }

    let mut files_list = String::new();
    for file in dirty_files {
        files_list.push_str("  * ");
        files_list.push_str(&file);
        files_list.push_str(" (dirty)\n");
    }

    // Do not add too many line breaks here so that the text wraps the terminal
    // width.
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
