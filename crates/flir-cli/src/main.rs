use clap::Parser;
use flir_cli::args::Args;
use flir_cli::logging;
use flir_cli::output_format;
use flir_cli::run;
use flir_cli::status::ExitStatus;
use std::process::ExitCode;

mod args;

fn main() -> ExitCode {
    let args = Args::parse();

    match run(args) {
        Ok(status) => status.into(),
        Err(err) => {
            use std::io::Write;

            // Use `writeln` instead of `eprintln` to avoid panicking when the stderr pipe is broken.
            let mut stderr = std::io::stderr().lock();

            // This communicates that this isn't a typical error but flir itself hard-errored for
            // some reason (e.g. failed to resolve the configuration)
            writeln!(stderr, "flir failed").ok();

            for cause in err.chain() {
                writeln!(stderr, "  Cause: {cause}").ok();
            }

            ExitStatus::Error.into()
        }
    }
}
