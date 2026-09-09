use std::env;
use std::fs;
use std::path::Path;

/// Embeds the generated rule documentation (`docs/rules/<rule>.md`, produced by
/// `docs/make_docs.R`) into the binary so `jarl rule <name>` can print it.
///
/// The markdown files are the single source used here; they must be regenerated
/// (`just document`) whenever a rule's doc-comment changes. A rule without a
/// matching `.md` simply yields `None` and the CLI falls back to metadata only.
fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let docs_dir = Path::new(&manifest_dir)
        .join("..")
        .join("..")
        .join("docs")
        .join("rules");

    // Re-run when rules are added or removed from the directory.
    println!("cargo:rerun-if-changed={}", docs_dir.display());

    let mut files: Vec<_> = fs::read_dir(&docs_dir)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .collect();
    files.sort();

    let mut arms = String::new();
    for path in &files {
        let name = path.file_stem().unwrap().to_str().unwrap();
        let abs = fs::canonicalize(path).unwrap();
        let abs = abs.to_str().unwrap();
        // Re-run when the content of an existing rule doc changes.
        println!("cargo:rerun-if-changed={abs}");
        arms.push_str(&format!(
            "        {name:?} => Some(include_str!({abs:?})),\n"
        ));
    }

    let generated = format!(
        "/// Returns the generated markdown documentation for a rule, if available.\n\
         pub fn rule_doc(name: &str) -> Option<&'static str> {{\n\
         \x20   match name {{\n\
         {arms}\
         \x20       _ => None,\n\
         \x20   }}\n\
         }}\n"
    );

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("rule_docs.rs");
    fs::write(&dest, generated).unwrap();
}
