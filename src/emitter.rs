use air_fs::relativize_path;
use colored::Colorize;
use serde_json;
use std::io::Write;

use crate::diagnostic::Diagnostic;

pub trait Emitter {
    fn emit<W: Write>(&self, writer: &mut W, diagnostics: &Vec<&Diagnostic>) -> anyhow::Result<()>;
}

pub struct ConciseEmitter;

impl Emitter for ConciseEmitter {
    fn emit<W: Write>(&self, writer: &mut W, diagnostics: &Vec<&Diagnostic>) -> anyhow::Result<()> {
        for diagnostic in diagnostics {
            let (row, col) = match diagnostic.location {
                Some(loc) => (loc.row, loc.column),
                None => {
                    unreachable!("Row/col locations must have been parsed successfully before.")
                }
            };
            write!(
                writer,
                "{} [{}:{}] {} {}",
                relativize_path(diagnostic.filename.clone()).white(),
                row,
                col,
                diagnostic.message.name.red(),
                diagnostic.message.body
            )?
        }
        Ok(())
    }
}

pub struct JsonEmitter;

impl Emitter for JsonEmitter {
    fn emit<W: Write>(&self, writer: &mut W, diagnostics: &Vec<&Diagnostic>) -> anyhow::Result<()> {
        serde_json::to_writer_pretty(writer, diagnostics)?;
        Ok(())
    }
}
