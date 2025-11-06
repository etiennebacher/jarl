//! Codegen tools for generating Syntax and AST definitions. Derived from Rust analyzer's codegen
//!
mod r_json_schema;

use bpaf::Bpaf;

pub use self::r_json_schema::generate_json_schema;

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub enum TaskCommand {
    #[bpaf(command, long("json-schema"))]
    JsonSchema,
}
