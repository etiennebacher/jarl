//! Shared analysis database backed by oak's salsa stack.
//!
//! jarl's CLI is a one-shot tool over a list of paths, while oak's
//! [`oak_scan`] machinery is built for editor workspace folders. We bridge
//! the two by scanning only the **package roots** that the linted paths
//! belong to (bounded by `DESCRIPTION` discovery), never the unbounded
//! parent directory of a loose script. That keeps a `jarl /tmp/foo.R`
//! invocation from walking all of `/tmp`. Loose scripts still take part in
//! cross-file analysis: the lint set itself is their file universe, so they
//! are handed to [`AnalysisDb::cross_file_used_objects`] directly and
//! resolve against each other through explicit `source()` edges.
//!
//! The database is built and queried in jarl's *sequential* pre-pass
//! ([`crate::package::make_package_analysis`]), not the parallel per-file
//! pass: oak's `OakDatabase` is `Send` but not `Sync` (it holds per-thread
//! query state), so it can't be borrowed across rayon workers. The pre-pass
//! uses it to enumerate each package's R files — replacing jarl's hand-rolled
//! filesystem walks — and feeds plain `Send` data to the parallel pass.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use aether_path::FilePath;
use air_r_parser::RParserOptions;
use oak_db::{Db, File, OakDatabase, Package, workspace_files};
use oak_scan::ScanScheduler;
use oak_semantic::semantic_index::{DefinitionKind, SemanticIndex};
use rayon::prelude::*;

use crate::package::find_package_root;

/// One scanned R package: its root directory plus the R source files oak
/// discovered under it, split by load-order classification.
pub struct ScannedPackage {
    /// Package root: the directory containing `DESCRIPTION`.
    pub root: PathBuf,
    /// `R/*.R` files — the package's loadable namespace, in R's load order.
    pub r_files: Vec<PathBuf>,
    /// R files under the package but outside `R/` (`tests/`, `inst/`,
    /// `data-raw/`, ...): analysed but not loaded.
    pub scripts: Vec<PathBuf>,
}

/// A populated, read-only analysis database over the package roots that
/// cover the linted paths.
pub struct AnalysisDb {
    db: OakDatabase,
}

/// One file's contribution to cross-file resolution: the names it binds at
/// top level, the names it reads *freely* — without binding them anywhere
/// in the file, so they reference the package namespace — and the
/// `source()` bindings it consumes (a read reaching a
/// [`DefinitionKind::Import`] uses the target file's top-level binding).
pub(crate) struct FileUses {
    path: PathBuf,
    top_defs: Vec<String>,
    free_uses: HashSet<String>,
    /// `(target file, name)` per `Import`-kind definition reached by a use:
    /// this file reads `name` out of `target file` via `source()`.
    import_uses: HashSet<(PathBuf, String)>,
}

/// Result of the package-wide cross-file pass.
#[derive(Default)]
pub struct CrossFileAnalysis {
    /// Per defining-file (relativized path): top-level object names read from
    /// another file, either through the shared package namespace or through a
    /// `source()` edge.
    pub used: HashMap<PathBuf, HashSet<String>>,
    /// Per-file (relativized path) semantic index, built once here and shared
    /// with the parallel lint pass so it isn't rebuilt.
    pub indices: HashMap<PathBuf, Arc<SemanticIndex>>,
}

impl AnalysisDb {
    /// Scan the package roots covering `paths` into a fresh database.
    ///
    /// Loose scripts (not inside any R package) contribute no root, so
    /// they're simply absent from the database; their per-file analysis
    /// falls back to the standalone index builder. Only files under a
    /// discovered package root are registered, which is exactly the set
    /// that needs cross-file resolution.
    pub fn build(paths: &[PathBuf]) -> Self {
        let mut db = OakDatabase::new();
        let roots = package_roots(paths);
        if !roots.is_empty() {
            let mut scheduler = ScanScheduler::new();
            let editor_owned = HashSet::new();
            let mut requests = scheduler.set_workspace_paths(&mut db, &roots, &editor_owned);
            // Drain synchronously: jarl has no task pool, so run every scan
            // request on this thread and feed follow-ups back until the
            // scheduler is idle (oak_scan's documented out-of-crate pattern).
            while let Some(request) = requests.pop() {
                let completed = request.run();
                requests.extend(scheduler.apply_scan_completed(&mut db, completed, &editor_owned));
            }
        }
        Self { db }
    }

    /// The underlying salsa database, for cross-file queries.
    pub fn db(&self) -> &dyn Db {
        &self.db
    }

    /// The registered [`File`] for `path`, if it was scanned in.
    pub fn file_for_path(&self, path: &Path) -> Option<File> {
        let file_path = FilePath::from_path_buf(path.to_path_buf())?;
        self.db.file_by_path(&file_path)
    }

    /// Every R package oak scanned, with its R-source file paths.
    ///
    /// This is the database-backed replacement for jarl's manual package
    /// discovery: oak's scan already walked each package root (honouring
    /// `.gitignore`, applying R's flat-`R/` load rule), so the file sets
    /// come straight from the salsa graph instead of a second filesystem walk.
    pub fn packages(&self) -> Vec<ScannedPackage> {
        let db = self.db();
        let mut seen: HashSet<Package> = HashSet::new();
        let mut packages = Vec::new();
        for file in workspace_files(db) {
            let Some(package) = file.package(db) else {
                continue;
            };
            if !seen.insert(package) {
                continue;
            }
            let Some(root) = package
                .description_path(db)
                .as_path()
                .and_then(|path| path.parent())
                .map(|dir| dir.as_std_path().to_path_buf())
            else {
                continue;
            };
            packages.push(ScannedPackage {
                root,
                r_files: file_paths(db, package.files(db)),
                scripts: file_paths(db, package.scripts(db)),
            });
        }
        packages
    }

    /// For each analyzed file, the set of its top-level object names that are
    /// read from *another* file — through the package namespace or through a
    /// `source()` edge.
    ///
    /// A package's R files share one namespace, so a top-level binding defined
    /// in one file and read in another is used even when its own file never
    /// reads it. For every file we collect, from its per-file index, the names
    /// it defines at top level and the names it reads *freely* — uses with no
    /// binding anywhere in the file, which therefore reference the package
    /// namespace (this is the same `reaching_definitions().is_empty()` test
    /// oak's `resolve_at` uses to decide local-vs-cross-file). A top-level
    /// definition is cross-file-used when another file reads its name freely.
    ///
    /// `script_paths` are the linted R files living outside any package. They
    /// don't share a namespace with anything, so they skip the free-use
    /// matching above and participate only through the `source()` edges below:
    /// a read reaching a `DefinitionKind::Import` marks the *target* file's
    /// binding used, chasing forwards when the target itself sources the real
    /// definer.
    ///
    /// This avoids per-use `File::resolve_at`, which has to run on a single
    /// thread because the salsa db is `!Sync`. The index work here is db-free
    /// and runs on the rayon pool; only the cheap final merge is sequential.
    ///
    /// The per-file indices built here are returned alongside the use map: the
    /// parallel lint pass reuses them via [`PackageAnalysis::file_indices`]
    /// instead of rebuilding each file's index a second time. They're built
    /// with the real [`jarl_semantic::JarlImportsResolver`] (not the no-op one)
    /// so they're identical to what the lint pass would build.
    ///
    /// Keyed by relativized file path to match the lint's per-file lookup.
    pub fn cross_file_used_objects(&self, script_paths: &[PathBuf]) -> CrossFileAnalysis {
        let db = self.db();

        // Collect just the paths up front. `path()` is a salsa query that needs
        // the (`!Sync`) db, but it touches no disk, so this sequential loop is
        // cheap. Reading the file contents — the part that scales with file
        // count — is deferred to the parallel pass below. Package files take
        // part in namespace-based matching; loose scripts (outside every
        // scanned root) are appended as source-edge-only participants.
        let mut paths: Vec<(PathBuf, bool)> = workspace_files(db)
            .iter()
            .filter_map(|&file| {
                let path = file
                    .path(db)
                    .as_path()
                    .map(|p| p.as_std_path().to_path_buf())?;
                Some((PathBuf::from(crate::fs::relativize_path(&path)), true))
            })
            .collect();
        let scanned: HashSet<&PathBuf> = paths.iter().map(|(path, _)| path).collect();
        let scripts: Vec<(PathBuf, bool)> = script_paths
            .iter()
            .map(|path| PathBuf::from(crate::fs::relativize_path(path)))
            .filter(|path| !scanned.contains(path))
            .map(|path| (path, false))
            .collect();
        paths.extend(scripts);

        // Per file, in parallel: read the source, parse, build the index, and
        // read off the file's top-level definitions and its free uses. None of
        // this needs the db, so it's the rayon-friendly bulk of the work. The
        // index is kept (shared with the lint pass), so building it here is not
        // throwaway work. Reading from disk here (rather than via the db's
        // `source_text`) is what lets the read run in parallel; in the one-shot
        // CLI the disk is the source of truth, so the two are equivalent.
        let built: Vec<(Arc<SemanticIndex>, FileUses)> = self
            .universe_paths()
            .par_iter()
            .filter_map(|(path, in_package)| {
                let source = std::fs::read_to_string(path).ok()?;
                let parsed = air_r_parser::parse(&source, RParserOptions::default());
                if parsed.has_error() {
                    return None;
                }
                let index = oak_semantic::build_index(
                    &parsed.tree(),
                    jarl_semantic::JarlImportsResolver::new(path.clone()),
                );
                let uses = collect_file_uses(path.clone(), &index, *in_package);
                Some((Arc::new(index), uses))
            })
            .collect();

        let indices = built
            .iter()
            .map(|(index, file)| (file.path.clone(), Arc::clone(index)))
            .collect();
        let uses: Vec<FileUses> = built.into_iter().map(|(_, file)| file).collect();
        let used = merge_cross_file(&uses);
        CrossFileAnalysis { used, indices }
    }

    /// The package-namespace file universe: every R file oak scanned, relativized
    /// to match the lint's per-file path keys.
    ///
    /// `path()` is a salsa query that needs the (`!Sync`) db, but it touches no
    /// disk, so this loop is cheap; reading file contents — the part that scales
    /// with file count — is left to the parallel callers.
    pub(crate) fn universe_paths(&self) -> Vec<PathBuf> {
        let db = self.db();
        workspace_files(db)
            .iter()
            .filter_map(|&file| {
                let path = file
                    .path(db)
                    .as_path()
                    .map(|p| p.as_std_path().to_path_buf())?;
                Some(PathBuf::from(crate::fs::relativize_path(&path)))
            })
            .collect()
    }
}

/// From every file's top-level definitions and free uses, compute for each
/// defining file the set of its top-level names read by *another* file in the
/// same package.
///
/// A package's R files share one namespace, so a top-level binding defined in
/// one file and read freely in another is used even when its own file never
/// reads it.
pub(crate) fn merge_cross_file(files: &[FileUses]) -> HashMap<PathBuf, HashSet<String>> {
    // Only a name defined at top level somewhere in the package can be the
    // target of a cross-file read, so we ignore free uses of locals, library
    // symbols, and base functions.
    let candidates: HashSet<&str> = files
        .iter()
        .flat_map(|file| file.top_defs.iter().map(String::as_str))
        .collect();

    // For each candidate name, the files that read it freely.
    let mut readers_of: HashMap<&str, Vec<&PathBuf>> = HashMap::new();
    for file in files {
        for name in &file.free_uses {
            if candidates.contains(name.as_str()) {
                readers_of.entry(name).or_default().push(&file.path);
            }
        }
    }

    // A top-level definition is used when some *other* file reads its name.
    let mut used: HashMap<PathBuf, HashSet<String>> = HashMap::new();
    for file in files {
        for name in &file.top_defs {
            let Some(readers) = readers_of.get(name.as_str()) else {
                continue;
            };
            if readers.iter().any(|reader| **reader != file.path) {
                used.entry(file.path.clone())
                    .or_default()
                    .insert(name.clone());
            }
        }

        // `source()` edges: a read that reaches an `Import`-kind definition
        // consumes the target file's top-level binding, so mark it used there.
        // A target may forward the name from a file it sources itself; chase
        // those `Import`-kind exports to the real definer (jarl's parallel of
        // oak_db's `File::collect_exports`), marking every hop. The visited
        // set makes `source()` cycles terminate. `exports()` is recomputed
        // per hop, but chains are short and edges few, so a per-file cache
        // isn't worth it.
        let index_by_path: HashMap<&Path, &SemanticIndex> = built
            .iter()
            .map(|(index, file)| (file.path.as_path(), index.as_ref()))
            .collect();
        let mut pending: Vec<(PathBuf, String)> = built
            .iter()
            .flat_map(|(_, file)| file.import_uses.iter().cloned())
            .collect();
        let mut chased: HashSet<(PathBuf, String)> = HashSet::new();
        while let Some((path, name)) = pending.pop() {
            if !chased.insert((path.clone(), name.clone())) {
                continue;
            }
            used.entry(path.clone()).or_default().insert(name.clone());
            let Some(index) = index_by_path.get(path.as_path()) else {
                continue;
            };
            for &(_, def) in index.exports().get(name.as_str()).into_iter().flatten() {
                let DefinitionKind::Import { file: url, name: forwarded, .. } = def.kind() else {
                    continue;
                };
                let Some(target) = import_target_path(url) else {
                    continue;
                };
                pending.push((target, forwarded.clone()));
            }
        }

        let indices = built
            .iter()
            .map(|(index, file)| (file.path.clone(), Arc::clone(index)))
            .collect();
        CrossFileAnalysis { used, indices }
    }
    used
}

/// Collect a file's top-level definitions, its free uses, and its `source()`
/// import uses from its index.
///
/// A use is *free* when no definition reaches it within the file — the same
/// `reaching_definitions().is_empty()` test oak's `resolve_at` uses before
/// falling back to cross-file resolution. Reaching definitions already fold in
/// enclosing-scope captures, so a closure reading an outer local counts as
/// bound, not free.
///
/// A use reaching a [`DefinitionKind::Import`] instead reads a binding that
/// `source()` injected from another file, so it's recorded as an import use
/// of `(target file, name)`.
///
/// `in_package` gates the namespace-based side (`top_defs` / `free_uses`):
/// loose scripts share no namespace, so name matching would count unrelated
/// scripts as readers of each other. They contribute import uses only.
pub(crate) fn collect_file_uses(path: PathBuf, index: &SemanticIndex, in_package: bool) -> FileUses {
    let top_defs: Vec<String> = if in_package {
        index
            .exports()
            .keys()
            .map(|name| name.to_string())
            .collect()
    } else {
        Vec::new()
    };

    let mut free_uses: HashSet<String> = HashSet::new();
    let mut import_uses: HashSet<(PathBuf, String)> = HashSet::new();
    for scope in index.scope_ids() {
        let symbols = index.symbols(scope);
        for (use_id, use_site) in index.uses(scope).iter() {
            let mut reached = false;
            for (def_scope, def_id) in index.reaching_definitions(scope, use_id) {
                reached = true;
                let def = &index.definitions(def_scope)[def_id];
                let DefinitionKind::Import { file: url, name, .. } = def.kind() else {
                    continue;
                };
                let Some(target) = import_target_path(url) else {
                    continue;
                };
                import_uses.insert((target, name.clone()));
            }
            if !reached && in_package {
                free_uses.insert(symbols.symbol(use_site.symbol()).name().to_string());
            }
        }
    }

    FileUses { path, top_defs, free_uses, import_uses }
}

/// Convert an `Import` definition's file URL back to the relativized path
/// that keys [`CrossFileAnalysis`]'s maps. [`jarl_semantic::JarlImportsResolver`]
/// builds these URLs from absolutized paths, so the round-trip through
/// `to_file_path` + relativize lands on the same key as the linted file's.
fn import_target_path(url: &url::Url) -> Option<PathBuf> {
    let path = url.to_file_path().ok()?;
    Some(PathBuf::from(crate::fs::relativize_path(path)))
}

/// Resolve a list of database [`File`]s to their filesystem paths, dropping
/// any whose URL has no filesystem path (e.g. virtual documents).
fn file_paths(db: &dyn Db, files: &[File]) -> Vec<PathBuf> {
    files
        .iter()
        .filter_map(|file| {
            file.path(db)
                .as_path()
                .map(|p| p.as_std_path().to_path_buf())
        })
        .collect()
}

/// The deduplicated set of package roots (directories containing a
/// `DESCRIPTION`) for `paths`, with nested roots collapsed to their
/// outermost ancestor so each tree is scanned once.
///
/// Paths are absolutized against the working directory first: oak's scanner
/// keys files by `file://` URL and rejects relative paths, and walking up a
/// relative path like `R/foo.R` would otherwise resolve the root to an empty
/// (cwd-relative) path the scanner can't register.
fn package_roots(paths: &[PathBuf]) -> Vec<PathBuf> {
    let cwd = std::env::current_dir().ok();
    let mut roots: Vec<PathBuf> = paths
        .iter()
        .filter_map(|path| {
            let absolute = if path.is_absolute() {
                path.clone()
            } else {
                cwd.as_ref()?.join(path)
            };
            find_package_root(&absolute)
        })
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    roots.sort();

    let mut outermost: Vec<PathBuf> = Vec::new();
    for root in roots {
        if !outermost.iter().any(|ancestor| root.starts_with(ancestor)) {
            outermost.push(root);
        }
    }
    outermost
}
