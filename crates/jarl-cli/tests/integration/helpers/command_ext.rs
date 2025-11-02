use std::fmt::Display;
use std::process::Command;
use std::process::ExitStatus;

pub trait CommandExt {
    /// Executes the command as a child process, waiting for it to finish and collecting all of its output.
    ///
    /// Like [Command::output], but also collects arguments
    ///
    /// The [Output] has a suitable [Display] method for capturing with insta
    fn run(&mut self) -> Output;
}

/// Like [std::process::Output], but augmented with `arguments` and a few extra methods
pub struct Output {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
    pub arguments: String,
}

impl Output {
    /// Normalize executable name for cross OS snapshot stability
    pub fn normalize_os_executable_name(self) -> Self {
        Self {
            status: self.status,
            stdout: self.stdout.replace("jarl.exe", "jarl"),
            stderr: self.stderr.replace("jarl.exe", "jarl"),
            arguments: self.arguments.replace("jarl.exe", "jarl"),
        }
    }

    /// Normalize temporary file paths for snapshot stability
    pub fn normalize_temp_paths(self) -> Self {
        use regex::Regex;

        // Match temporary directory paths with trailing separator:
        // - Linux: /tmp/.tmpXXXXXX/
        // - macOS: /var/folders/.../T/.tmpXXXXXX/ or /private/var/folders/.../
        // - Windows: C:\Users\...\AppData\Local\Temp\...\
        let temp_path_regex =
            Regex::new(r"(?:/private)?/(?:tmp|var/folders/[^/]+/[^/]+/T)/\.tmp[A-Za-z0-9]+/|C:\\Users\\[^\\]+\\AppData\\Local\\Temp\\[^\\]+\\")
                .unwrap();

        Self {
            status: self.status,
            stdout: temp_path_regex
                .replace_all(&self.stdout, "[TEMP_DIR]/")
                .to_string(),
            stderr: temp_path_regex
                .replace_all(&self.stderr, "[TEMP_DIR]/")
                .to_string(),
            arguments: self.arguments,
        }
    }
}

/// Strip ANSI escape codes from a string
fn strip_ansi_escape_codes(s: &str) -> String {
    // This regex matches ANSI escape sequences
    use regex::Regex;
    let ansi_regex = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    ansi_regex.replace_all(s, "").to_string()
}

impl CommandExt for Command {
    fn run(&mut self) -> Output {
        // Augment `std::process::Output` with the arguments
        let output = self.output().unwrap();

        // Go ahead and turn these into `String`
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        let arguments: Vec<String> = self
            .get_args()
            .map(|x| x.to_string_lossy().into_owned())
            .collect();

        let arguments = arguments.join(" ");

        Output { status: output.status, stdout, stderr, arguments }
    }
}

impl Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Strip ANSI codes and normalize path separators for readable snapshots
        let stdout = strip_ansi_escape_codes(&self.stdout).replace("\\", "/");
        let stderr = strip_ansi_escape_codes(&self.stderr).replace("\\", "/");

        f.write_fmt(format_args!(
            "
success: {:?}
exit_code: {}
----- stdout -----
{}
----- stderr -----
{}
----- args -----
{}",
            self.status.success(),
            self.status.code().unwrap_or(1),
            stdout,
            stderr,
            self.arguments,
        ))
    }
}
