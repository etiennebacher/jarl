use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context as _;
use tempfile::TempDir;

use super::binary::binary_path;

pub struct CliTest {
    _temp_dir: TempDir,
    project_dir: PathBuf,
}

impl CliTest {
    pub fn new() -> anyhow::Result<Self> {
        let temp_dir = TempDir::new()?;
        let project_dir = temp_dir.path().to_path_buf();

        Ok(Self { _temp_dir: temp_dir, project_dir })
    }

    pub fn with_file(path: impl AsRef<Path>, content: &str) -> anyhow::Result<Self> {
        let case = Self::new()?;
        case.write_file(path, content)?;
        Ok(case)
    }

    pub fn with_files<'a>(
        files: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> anyhow::Result<Self> {
        let case = Self::new()?;
        for (path, content) in files {
            case.write_file(path, content)?;
        }
        Ok(case)
    }

    pub fn write_file(&self, path: impl AsRef<Path>, content: &str) -> anyhow::Result<()> {
        let path = self.project_dir.join(path);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory `{}`", parent.display()))?;
        }

        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write file `{}`", path.display()))?;

        Ok(())
    }

    pub fn read_file(&self, path: impl AsRef<Path>) -> anyhow::Result<String> {
        let path = self.project_dir.join(path);
        std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read file `{}`", path.display()))
    }

    pub fn root(&self) -> &Path {
        &self.project_dir
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new(binary_path());
        command.current_dir(&self.project_dir);

        // Prevent host environment from affecting tests
        command.env("NO_COLOR", "1");
        command.env("R_HOME", std::env::temp_dir());

        command
    }
}
