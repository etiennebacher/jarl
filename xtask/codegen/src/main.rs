use xtask::{project_root, pushd, Result};

use xtask_codegen::{generate_json_schema, task_command, TaskCommand};

fn main() -> Result<()> {
    let _d = pushd(project_root());
    let result = task_command().fallback_to_usage().run();

    match result {
        TaskCommand::JsonSchema => {
            generate_json_schema()?;
        }
    }

    Ok(())
}
