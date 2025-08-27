use serde_json;
use std::io::Write;

use crate::message::Diagnostic;

pub trait Emitter {
    fn emit<W: Write>(&self, writer: &mut W, diagnostics: &Vec<&Diagnostic>) -> anyhow::Result<()>;
}

pub struct ConciseEmitter;

impl Emitter for ConciseEmitter {
    fn emit<W: Write>(
        &self,
        _writer: &mut W,
        diagnostics: &Vec<&Diagnostic>,
    ) -> anyhow::Result<()> {
        for diagnostic in diagnostics {
            println!("{diagnostic}")
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
