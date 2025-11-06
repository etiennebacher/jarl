use std::path::PathBuf;

const ROOT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");

pub fn generate_json_schema() -> anyhow::Result<()> {
    let schema = json_schema()?;
    let schema_path = schema_path();
    std::fs::write(schema_path, schema.as_bytes())?;
    Ok(())
}

fn json_schema() -> anyhow::Result<String> {
    let schema = schemars::schema_for!(jarl_core::toml::TomlOptions);
    let schema = serde_json::to_string_pretty(&schema)?;
    Ok(schema)
}

fn schema_path() -> PathBuf {
    PathBuf::from(ROOT_DIR)
        .join("artifacts")
        .join("jarl.schema.json")
}
