//
// Adapted from Ark
// https://github.com/posit-dev/ark/blob/main/crates/ark/src/lsp/inputs/package_description.rs
// 7f9ea95d367712eb40b1669cf317c7a8a71e779b
//
// MIT License - Posit PBC

use std::collections::HashMap;

use anyhow;

/// Parsed DCF file (Debian Control File, e.g. DESCRIPTION). Simple wrapper
/// around the map of fields whose `get()` method returns a `&str` that's easier
/// to work with.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dcf {
    pub fields: HashMap<String, String>,
}

impl Dcf {
    pub fn new() -> Self {
        Dcf { fields: HashMap::new() }
    }

    pub fn parse(input: &str) -> Self {
        Dcf { fields: parse_dcf(input) }
    }

    /// Get a field value by key
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(|s| s.as_str())
    }
}

/// Parsed DESCRIPTION file
#[derive(Clone, Debug)]
pub struct Description {
    pub name: String,
    pub version: String,

    /// `Depends` field. Currently doesn't contain versions.
    pub depends: Vec<String>,

    /// Raw DCF fields
    pub fields: Dcf,
}

impl Default for Description {
    fn default() -> Self {
        Description {
            name: String::new(),
            version: String::new(),
            depends: Vec::new(),
            fields: Dcf::default(),
        }
    }
}

impl Description {
    pub fn get_depend_r_version(contents: &str) -> anyhow::Result<Vec<String>> {
        let fields = Dcf::parse(contents);
        let depends = fields
            .get("Depends")
            .and_then(|deps| {
                let r_dep = deps
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty() && s.starts_with("R "))
                    .map(|s| {
                        if let Some(idx_open) = s.find('(')
                            && let Some(idx_close) = s.find(')')
                        {
                            s[idx_open + 1..idx_close]
                                .to_string()
                                .replace(">=", "")
                                .trim()
                                .to_string()
                        } else {
                            s.to_string()
                        }
                    })
                    .collect::<Vec<String>>();

                Some(r_dep)
            })
            .unwrap_or_default();
        Ok(depends)
    }
}

/// Parse a DCF (Debian Control File) format string into a key-value map.
/// https://www.debian.org/doc/debian-policy/ch-controlfields.html
fn parse_dcf(input: &str) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;

    let mut fields = HashMap::new();
    let mut current_key: Option<String> = None;
    let mut current_value = String::new();

    for line in input.lines() {
        // Indented line: This is a continuation, even if empty
        if line.starts_with(char::is_whitespace) {
            current_value.push_str(line);
            current_value.push('\n');
            continue;
        }

        // Non-whitespace at start and contains a colon: This is a new field
        if !line.is_empty() && line.contains(':') {
            // Save previous field
            if let Some(key) = current_key.take() {
                fields.insert(key, current_value.trim_end().to_string());
            }

            let idx = line.find(':').unwrap();
            let key = line[..idx].trim().to_string();
            let value = line[idx + 1..].trim_start();

            current_key = Some(key);

            current_value.clear();
            current_value.push_str(value);
            current_value.push('\n');

            continue;
        }
    }

    // Finish last field
    if let Some(key) = current_key {
        fields.insert(key, current_value.trim_end().to_string());
    }

    fields
}
