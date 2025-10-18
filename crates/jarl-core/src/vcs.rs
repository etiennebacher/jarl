use crate::config::Config;
use anyhow::{Result, bail};
use std::path::Path;

fn in_git_repo(path: &String) -> bool {
    let path = Path::new(path);
    if let Ok(repo) = GitRepo::discover(path) {
        // Don't check if the working directory itself is ignored.
        if repo.workdir().map_or(false, |workdir| workdir == path) {
            true
        } else {
            !repo.is_path_ignored(path).unwrap_or(false)
        }
    } else {
        false
    }
}

pub struct GitRepo;

impl GitRepo {
    pub fn init(path: &Path) -> Result<GitRepo> {
        git2::Repository::init(path)?;
        Ok(GitRepo)
    }
    pub fn discover(path: &Path) -> Result<git2::Repository, git2::Error> {
        git2::Repository::discover(path)
    }
}

pub fn check_version_control(path: &String, config: &Config) -> Result<()> {
    if config.allow_no_vcs {
        return Ok(());
    }
    if !in_git_repo(&path) {
        bail!(
            "no Version Control System (e.g. Git) found for this package and \
            `jarl check --fix` can potentially perform destructive changes; if \
            you'd like to suppress this error pass `--allow-no-vcs`"
        )
    }

    if config.allow_dirty {
        return Ok(());
    }

    let mut dirty_files = Vec::new();
    let mut _staged_files: Vec<String> = Vec::new();
    if let Ok(repo) = git2::Repository::discover(path) {
        let mut repo_opts = git2::StatusOptions::new();
        repo_opts.include_ignored(false);
        repo_opts.include_untracked(true);
        for status in repo.statuses(Some(&mut repo_opts))?.iter() {
            if let Some(path) = status.path() {
                match status.status() {
                    git2::Status::CURRENT => (),
                    // TODO: add an arg --allow-staged?
                    // git2::Status::INDEX_NEW
                    // | git2::Status::INDEX_MODIFIED
                    // | git2::Status::INDEX_DELETED
                    // | git2::Status::INDEX_RENAMED
                    // | git2::Status::INDEX_TYPECHANGE => {
                    //     if !opts.allow_staged {
                    //         staged_files.push(path.to_string())
                    //     }
                    // }
                    _ => {
                        if !config.allow_dirty {
                            dirty_files.push(path.to_string())
                        }
                    }
                };
            }
        }
    }

    if dirty_files.is_empty() && _staged_files.is_empty() {
        return Ok(());
    }

    let mut files_list = String::new();
    for file in dirty_files {
        files_list.push_str("  * ");
        files_list.push_str(&file);
        files_list.push_str(" (dirty)\n");
    }
    for file in _staged_files {
        files_list.push_str("  * ");
        files_list.push_str(&file);
        files_list.push_str(" (staged)\n");
    }

    bail!(
        "the working directory of this package has uncommitted changes, and \
         `jarl check --fix` can potentially perform destructive changes; if \
         you'd like to suppress this error pass `--allow-dirty`, \
         or commit the changes to these files:\n\
         \n\
         {}\n\
         ",
        files_list
    );
}
