use clap::Parser;
use jarl::args::Args;
use jarl::logging;
use jarl::output_format;
use jarl::run;
use jarl::status::ExitStatus;
use std::process::ExitCode;

mod args;

fn main() -> ExitCode {
    // Enabled ANSI colors on Windows.
    #[cfg(windows)]
    assert!(colored::control::set_virtual_terminal(true).is_ok());

    let args = Args::parse();

    match run(args) {
        Ok(status) => status.into(),
        Err(err) => {
            use std::io::Write;

            // Use `writeln` instead of `eprintln` to avoid panicking when the stderr pipe is broken.
            let mut stderr = std::io::stderr().lock();

            // This communicates that this isn't a typical error but jarl itself hard-errored for
            // some reason (e.g. failed to resolve the configuration)
            writeln!(stderr, "jarl failed").ok();

            for cause in err.chain() {
                writeln!(stderr, "  Cause: {cause}").ok();
            }

            ExitStatus::Error.into()
        }
    }
}
