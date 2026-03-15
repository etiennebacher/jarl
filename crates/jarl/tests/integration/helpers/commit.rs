use std::path::Path;
use std::process::Command;

pub fn create_commit(file_path: &Path, repo_dir: &Path) -> anyhow::Result<()> {
    let file_name = file_path
        .file_name()
        .expect("file_path must have a file name");

    Command::new("git")
        .args(["add", &file_name.to_string_lossy()])
        .current_dir(repo_dir)
        .output()?;

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_dir)
        .output()?;

    Ok(())
}

pub fn git_init(dir: &Path) -> anyhow::Result<()> {
    Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()?;

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir)
        .output()?;

    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()?;

    Ok(())
}
