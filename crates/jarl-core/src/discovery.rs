//
// Adapted from Air
// https://github.com/posit-dev/air/blob/affa92cd514525c4bab6c8c2ca251ea19414b89f/crates/workspace/src/discovery.rs
//
// MIT License - Posit PBC

use ignore::DirEntry;
use rustc_hash::FxHashSet;
use std::path::Path;
use std::path::PathBuf;

use crate::fs;
use crate::fs::has_r_extension;
use crate::fs::has_rmd_extension;
use crate::settings::Settings;
use crate::toml::find_jarl_toml_in_directory;
use crate::toml::parse_jarl_toml;
use air_workspace::resolve::PathResolver;
use etcetera::BaseStrategy;

/// Default patterns to exclude from linting
/// These match common R project files that should not be linted
pub const DEFAULT_EXCLUDE_PATTERNS: &[&str] = &[
    ".git/",
    "renv/",
    "revdep/",
    "cpp11.R",
    "RcppExports.R",
    "extendr-wrappers.R",
    "import-standalone-*.R",
];

#[derive(Debug)]
pub struct DiscoveredSettings {
    pub directory: PathBuf,
    pub settings: Settings,
    /// Path to the config file that was used
    pub config_path: Option<PathBuf>,
}

/// Get the user config directory for jarl
fn get_user_config_dir() -> Option<PathBuf> {
    let strategy = etcetera::base_strategy::choose_base_strategy().ok()?;
    Some(strategy.config_dir().join("jarl"))
}

/// This is the core function for walking a set of `paths` looking for `jarl.toml`s.
///
/// You typically follow this function up by loading the set of returned path into a
/// [crate::resolve::PathResolver].
///
/// For each `path`, we:
/// - Walk up its ancestors until the user config directory, looking for a `jarl.toml`
/// - If no config found in ancestors, fall back to checking the user config directory
/// - TODO(hierarchical): Walk down its children, looking for nested `jarl.toml`s
pub fn discover_settings<P: AsRef<Path>>(paths: &[P]) -> anyhow::Result<Vec<DiscoveredSettings>> {
    let paths: Vec<PathBuf> = paths.iter().map(fs::normalize_path).collect();

    let mut seen = FxHashSet::default();
    let mut discovered_settings = Vec::with_capacity(paths.len());
    let user_config_dir = get_user_config_dir();

    // Discover all `Settings` across all `paths`, looking up each path's directory tree
    for path in &paths {
        let mut found_config = false;

        for ancestor in path.ancestors() {
            let is_new_ancestor = seen.insert(ancestor);

            if !is_new_ancestor {
                // We already visited this ancestor, we can stop here.
                break;
            }

            if let Some(toml) = find_jarl_toml_in_directory(ancestor) {
                let settings = parse_settings(&toml, ancestor)?;
                discovered_settings.push(DiscoveredSettings {
                    directory: ancestor.to_path_buf(),
                    settings,
                    config_path: Some(toml),
                });
                found_config = true;
                break;
            }

            // Stop at user config directory if we have one
            if let Some(ref config_dir) = user_config_dir
                && ancestor == config_dir
            {
                break;
            }
        }

        // If no config found in ancestors, check user config directory as fallback
        if !found_config
            && let Some(ref config_dir) = user_config_dir
            && seen.insert(config_dir.as_path())
            && let Some(toml) = find_jarl_toml_in_directory(config_dir)
        {
            let settings = parse_settings(&toml, config_dir)?;
            discovered_settings.push(DiscoveredSettings {
                directory: config_dir.clone(),
                settings,
                config_path: Some(toml),
            });
        }
    }

    // TODO(hierarchical): Also iterate into the directories and collect `jarl.toml`
    // found nested withing the directories for hierarchical support

    Ok(discovered_settings)
}

/// Parse [Settings] from a given `jarl.toml`
// TODO(hierarchical): Allow for an `extends` option in `jarl.toml`, which will make things
// more complex, but will be very useful once we support hierarchical configuration as a
// way of "inheriting" most top level configuration while slightly tweaking it in a nested directory.
fn parse_settings(toml: &Path, root_directory: &Path) -> anyhow::Result<Settings> {
    let options = parse_jarl_toml(toml)?;
    let settings = options
        .into_settings(root_directory)
        .map_err(|err| anyhow::anyhow!("Invalid configuration in {}:\n{err}", toml.display()))?;
    Ok(settings)
}

type DiscoveredFiles = Vec<Result<PathBuf, ignore::Error>>;

/// For each provided `path`, recursively search for any R files within that `path`
/// that match our inclusion criteria
///
/// NOTE: Make sure that the inclusion criteria that guide `path` discovery are also
/// consistently applied to [discover_settings()].
pub fn discover_r_file_paths<P: AsRef<Path>>(
    paths: &[P],
    resolver: &PathResolver<Settings>,
    use_linter_settings: bool,
    no_default_exclude: bool,
) -> DiscoveredFiles {
    let paths: Vec<PathBuf> = paths.iter().map(fs::normalize_path).collect();

    let Some((first_path, paths)) = paths.split_first() else {
        // No paths provided
        return Vec::new();
    };

    let mut builder = ignore::WalkBuilder::new(first_path);

    for path in paths {
        builder.add(path);
    }

    // TODO: Make these configurable options (possibly just one?)
    // Right now we explicitly call them even though they are `true` by default
    // to remind us to expose them.
    //
    // "This toggles, as a group, all the filters that are enabled by default"
    // builder.standard_filters(true)
    builder.hidden(true);
    builder.parents(true);
    builder.ignore(false);
    builder.git_ignore(true);
    builder.git_global(true);
    builder.git_exclude(true);

    // Add exclude patterns from settings if linter settings should be used
    if use_linter_settings {
        // Build custom ignore patterns
        let mut patterns = Vec::new();

        // Default root directory if no settings found
        let mut root = Path::new(".");

        if let Some(settings_item) = resolver.items().first() {
            let settings = settings_item.value();
            root = settings_item.path();

            // Add custom exclude patterns from jarl.toml
            if let Some(exclude_patterns) = &settings.linter.exclude {
                for pattern in exclude_patterns {
                    patterns.push(pattern.as_str());
                }
            }
            if settings.linter.default_exclude.unwrap_or(true) {
                // Add default exclude patterns
                patterns.extend_from_slice(DEFAULT_EXCLUDE_PATTERNS);
            }
        } else if !no_default_exclude {
            // Add default exclude patterns
            patterns.extend_from_slice(DEFAULT_EXCLUDE_PATTERNS);
        }

        // If we have patterns, create an override and add it to the builder
        if !patterns.is_empty() {
            let mut override_builder = ignore::overrides::OverrideBuilder::new(root);
            for pattern in patterns {
                // Add as negation pattern (exclude)
                if let Err(e) = override_builder.add(&format!("!{pattern}")) {
                    tracing::warn!("Failed to add exclude pattern '{}': {}", pattern, e);
                }
            }
            if let Ok(overrides) = override_builder.build() {
                builder.overrides(overrides);
            }
        }
    }

    // Prefer `available_parallelism()`, with a max of 12 threads
    builder.threads(
        std::thread::available_parallelism()
            .map_or(1, std::num::NonZeroUsize::get)
            .min(12),
    );

    let walker = builder.build_parallel();

    // Run the `WalkParallel` to collect all R files.
    let state = FilesState::new();
    let mut visitor_builder = FilesVisitorBuilder::new(&state);
    walker.visit(&mut visitor_builder);

    state.finish()
}

/// Shared state across the threads of the walker
struct FilesState {
    files: std::sync::Mutex<DiscoveredFiles>,
}

impl FilesState {
    fn new() -> Self {
        Self { files: std::sync::Mutex::new(Vec::new()) }
    }

    fn finish(self) -> DiscoveredFiles {
        self.files.into_inner().unwrap()
    }
}

/// Object capable of building a [FilesVisitor]
///
/// Implements the `build()` method of [ignore::ParallelVisitorBuilder], which
/// [ignore::WalkParallel] utilizes to create one [FilesVisitor] per thread.
struct FilesVisitorBuilder<'state> {
    state: &'state FilesState,
}

impl<'state> FilesVisitorBuilder<'state> {
    fn new(state: &'state FilesState) -> Self {
        Self { state }
    }
}

impl<'state> ignore::ParallelVisitorBuilder<'state> for FilesVisitorBuilder<'state> {
    /// Constructs the per-thread [FilesVisitor], called for us by `ignore`
    fn build(&mut self) -> Box<dyn ignore::ParallelVisitor + 'state> {
        Box::new(FilesVisitor { files: vec![], state: self.state })
    }
}

/// Object that implements [ignore::ParallelVisitor]'s `visit()` method
///
/// A files visitor has its `visit()` method repeatedly called. It modifies its own
/// synchronous state by pushing to its thread specific `files` while visiting. On `Drop`,
/// the collected `files` are appended to the global set of `state.files`.
struct FilesVisitor<'state> {
    files: DiscoveredFiles,
    state: &'state FilesState,
}

impl ignore::ParallelVisitor for FilesVisitor<'_> {
    /// Visit a file in the tree
    ///
    /// Visiting a file requires two actions:
    /// - Deciding whether or not to accept the file
    /// - Deciding whether or not to `WalkState::Continue` or `WalkState::Skip`
    ///
    /// ## Importance of `WalkState::Skip`
    ///
    /// We only return `WalkState::Skip` when we reject a file due to our `exclude`
    /// criteria, but this case is extremely important. It is a nice optimization because
    /// if we reject `renv/` then we never look at `renv/activate.R` at all, but it also
    /// affects the behavior of `exclude` in general. With `exclude = ["renv/"]`,
    /// `matches("renv")` of course returns `true`, but `matches("renv/activate.R")`
    /// returns `false`. This means that in order to correctly implement the `exclude`
    /// behavior, we absolutely cannot recurse into `renv/` after we reject it, otherwise
    /// we will blindly accept its children unless we run `matches()` on each parent
    /// directory of `"renv/activate.R"` as well, which would be wasteful and expensive.
    fn visit(&mut self, result: std::result::Result<DirEntry, ignore::Error>) -> ignore::WalkState {
        // Determine if `ignore` gave us a valid `result` or not
        let entry = match result {
            Ok(entry) => entry,
            Err(error) => {
                // Store error but continue walking
                self.files.push(Err(error));
                return ignore::WalkState::Continue;
            }
        };

        let path = entry.path();

        // An entry is explicit if it was provided directly, not discovered by looking into a directory
        let is_explicit = entry.depth() == 0;
        let is_directory = entry.file_type().is_none_or(|ft| ft.is_dir());

        if is_explicit && !is_directory {
            // Accept explicitly provided files, regardless of exclusion/inclusion
            // criteria (including extension). This is the user supplying `air linter
            // file.R`. Note we don't do this for directories, i.e. `air linter renv`
            // should do nothing since we have a default `exclude` for `renv/`.
            tracing::trace!(
                "Included file due to explicit provision {path}",
                path = path.display()
            );
            self.files.push(Ok(entry.into_path()));
            return ignore::WalkState::Continue;
        }

        // Check if this is an R or Rmd/Qmd file
        if !is_directory && (has_r_extension(path) || has_rmd_extension(path)) {
            tracing::trace!("Included R file {path}", path = path.display());
            self.files.push(Ok(entry.into_path()));
            return ignore::WalkState::Continue;
        }

        // Didn't accept this file, just keep going
        tracing::trace!(
            "Excluded file due to fallthrough {path}",
            path = path.display()
        );
        ignore::WalkState::Continue
    }
}

impl Drop for FilesVisitor<'_> {
    fn drop(&mut self) {
        // Lock the global shared set of `files`
        // Unwrap: If we can't lock the mutex then something is very wrong
        let mut files = self.state.files.lock().unwrap();

        // Transfer files gathered on this thread to the global set
        if files.is_empty() {
            *files = std::mem::take(&mut self.files);
        } else {
            files.append(&mut self.files);
        }
    }
}
