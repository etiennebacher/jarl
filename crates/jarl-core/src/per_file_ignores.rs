use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::rule_set::Rule;

/// A single `per-file-ignores` entry: a glob pattern paired with the rules to
/// ignore in the files it matches.
#[derive(Clone, Debug)]
struct PerFileIgnore {
    /// Matcher built from the (un-negated) glob pattern, rooted at the
    /// `jarl.toml` directory.
    matcher: Gitignore,
    /// Whether the pattern was negated with a leading `!`. When `true`, the
    /// entry applies to files that do *not* match `matcher`.
    negated: bool,
    /// Rules to ignore in matching files.
    rules: Vec<Rule>,
}

/// Resolved `[lint.per-file-ignores]` configuration. Holds compiled glob
/// matchers so that the rules ignored for a given file can be looked up
/// cheaply during linting.
#[derive(Clone, Debug, Default)]
pub struct PerFileIgnores {
    /// Directory the patterns are resolved against (the `jarl.toml` directory).
    root: PathBuf,
    entries: Vec<PerFileIgnore>,
}

impl PerFileIgnores {
    /// Build a [PerFileIgnores] from already-resolved `(pattern, rules)` pairs.
    ///
    /// Rule-name validation and group expansion are expected to have happened
    /// before this point (see `crate::toml`). A leading `!` in a pattern marks
    /// it as negated.
    pub fn new(root: &Path, entries: Vec<(String, Vec<Rule>)>) -> anyhow::Result<Self> {
        let mut compiled = Vec::with_capacity(entries.len());

        for (pattern, rules) in entries {
            let (negated, bare) = match pattern.strip_prefix('!') {
                Some(rest) => (true, rest.to_string()),
                None => (false, pattern.clone()),
            };

            // Mirror the directory handling used for `include`/`exclude`: a
            // trailing slash targets a directory's contents.
            let glob = if bare.ends_with('/') {
                format!("{bare}**")
            } else {
                bare
            };

            let mut builder = GitignoreBuilder::new(root);
            builder.add_line(None, &glob).map_err(|e| {
                anyhow::anyhow!("Invalid `per-file-ignores` pattern '{pattern}': {e}")
            })?;
            let matcher = builder.build().map_err(|e| {
                anyhow::anyhow!("Invalid `per-file-ignores` pattern '{pattern}': {e}")
            })?;

            compiled.push(PerFileIgnore { matcher, negated, rules });
        }

        Ok(Self { root: root.to_path_buf(), entries: compiled })
    }

    /// Whether any per-file ignore was configured.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return the set of rules to ignore for `path`.
    ///
    /// `path` should be the file's absolute (normalized) path so that it can be
    /// made relative to the configuration root before matching.
    pub fn ignored_rules(&self, path: &Path) -> HashSet<Rule> {
        let relative = path.strip_prefix(&self.root).unwrap_or(path);

        let mut ignored = HashSet::new();
        for entry in &self.entries {
            let matched = entry.matcher.matched(relative, false).is_ignore();
            // Plain patterns apply when the file matches; negated patterns
            // apply when it does not.
            if matched != entry.negated {
                ignored.extend(entry.rules.iter().copied());
            }
        }
        ignored
    }
}
