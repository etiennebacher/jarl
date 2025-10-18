use crate::util::CargoResult;
use cargo_util::ProcessBuilder;
use cargo_util::paths;
use std::path::Path;

fn in_git_repo(path: &Path, cwd: &Path) -> bool {
    if let Ok(repo) = GitRepo::discover(path, cwd) {
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
    pub fn init(path: &Path, _: &Path) -> CargoResult<GitRepo> {
        git2::Repository::init(path)?;
        Ok(GitRepo)
    }
    pub fn discover(path: &Path, _: &Path) -> Result<git2::Repository, git2::Error> {
        git2::Repository::discover(path)
    }
}

fn check_version_control(gctx: &GlobalContext, opts: &FixOptions) -> CargoResult<()> {
    // if opts.allow_no_vcs {
    //     return Ok(());
    // }
    // if !existing_vcs_repo(gctx.cwd(), gctx.cwd()) {
    //     bail!(
    //         "no VCS found for this package and `cargo fix` can potentially \
    //          perform destructive changes; if you'd like to suppress this \
    //          error pass `--allow-no-vcs`"
    //     )
    // }

    if opts.allow_dirty && opts.allow_staged {
        return Ok(());
    }

    let mut dirty_files = Vec::new();
    let mut staged_files = Vec::new();
    if let Ok(repo) = git2::Repository::discover(gctx.cwd()) {
        let mut repo_opts = git2::StatusOptions::new();
        repo_opts.include_ignored(false);
        repo_opts.include_untracked(true);
        for status in repo.statuses(Some(&mut repo_opts))?.iter() {
            if let Some(path) = status.path() {
                match status.status() {
                    git2::Status::CURRENT => (),
                    git2::Status::INDEX_NEW
                    | git2::Status::INDEX_MODIFIED
                    | git2::Status::INDEX_DELETED
                    | git2::Status::INDEX_RENAMED
                    | git2::Status::INDEX_TYPECHANGE => {
                        if !opts.allow_staged {
                            staged_files.push(path.to_string())
                        }
                    }
                    _ => {
                        if !opts.allow_dirty {
                            dirty_files.push(path.to_string())
                        }
                    }
                };
            }
        }
    }

    if dirty_files.is_empty() && staged_files.is_empty() {
        return Ok(());
    }

    let mut files_list = String::new();
    for file in dirty_files {
        files_list.push_str("  * ");
        files_list.push_str(&file);
        files_list.push_str(" (dirty)\n");
    }
    for file in staged_files {
        files_list.push_str("  * ");
        files_list.push_str(&file);
        files_list.push_str(" (staged)\n");
    }

    bail!(
        "the working directory of this package has uncommitted changes, and \
         `cargo fix` can potentially perform destructive changes; if you'd \
         like to suppress this error pass `--allow-dirty`, \
         or commit the changes to these files:\n\
         \n\
         {}\n\
         ",
        files_list
    );
}
