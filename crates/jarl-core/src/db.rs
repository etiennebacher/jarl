//! Shared analysis database backed by oak's salsa stack.
//!
//! jarl's CLI is a one-shot tool over a list of paths, while oak's
//! [`oak_scan`] machinery is built for editor workspace folders. We bridge
//! the two by scanning only the **package roots** that the linted paths
//! belong to (bounded by `DESCRIPTION` discovery), never the unbounded
//! parent directory of a loose script. That keeps a `jarl /tmp/foo.R`
//! invocation from walking all of `/tmp`.
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
use oak_semantic::semantic_index::SemanticIndex;
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
/// top level, and the names it reads *freely* — without binding them anywhere
/// in the file, so they reference the package namespace.
struct FileUses {
    path: PathBuf,
    top_defs: Vec<String>,
    free_uses: HashSet<String>,
}

/// Result of the package-wide cross-file pass.
#[derive(Default)]
pub struct CrossFileAnalysis {
    /// Per defining-file (relativized path): top-level object names read from
    /// another file in the same package.
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

    /// For each scanned file, the set of its top-level object names that are
    /// read from *another* file in the same package.
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
    pub fn cross_file_used_objects(&self) -> CrossFileAnalysis {
        let db = self.db();

        // Pull each file's path + source up front: both are salsa queries that
        // need the (`!Sync`) db, so doing it here lets the parse + index pass
        // below run in parallel.
        let sources: Vec<(PathBuf, String)> = workspace_files(db)
            .iter()
            .filter_map(|&file| {
                let path = file
                    .path(db)
                    .as_path()
                    .map(|p| p.as_std_path().to_path_buf())?;
                let rel = PathBuf::from(crate::fs::relativize_path(&path));
                Some((rel, file.source_text(db).clone()))
            })
            .collect();

        // Per file, in parallel: parse, build the index, and read off the
        // file's top-level definitions and its free uses. No db access, so this
        // is the rayon-friendly bulk of the work. The index is kept (shared
        // with the lint pass), so building it here is not throwaway work.
        let built: Vec<(Arc<SemanticIndex>, FileUses)> = sources
            .par_iter()
            .filter_map(|(path, source)| {
                let parsed = air_r_parser::parse(source, RParserOptions::default());
                if parsed.has_error() {
                    return None;
                }
                let index = oak_semantic::build_index(
                    &parsed.tree(),
                    jarl_semantic::JarlImportsResolver::new(path.clone()),
                );
                let uses = collect_file_uses(path.clone(), &index);
                Some((Arc::new(index), uses))
            })
            .collect();

        // Only a name defined at top level somewhere in the package can be the
        // target of a cross-file read, so we ignore free uses of locals,
        // library symbols, and base functions.
        let candidates: HashSet<&str> = built
            .iter()
            .flat_map(|(_, file)| file.top_defs.iter().map(String::as_str))
            .collect();

        // For each candidate name, the files that read it freely.
        let mut readers_of: HashMap<&str, Vec<&PathBuf>> = HashMap::new();
        for (_, file) in &built {
            for name in &file.free_uses {
                if candidates.contains(name.as_str()) {
                    readers_of.entry(name).or_default().push(&file.path);
                }
            }
        }

        // A top-level definition is used when some *other* file reads its name.
        let mut used: HashMap<PathBuf, HashSet<String>> = HashMap::new();
        for (_, file) in &built {
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
        }

        let indices = built
            .iter()
            .map(|(index, file)| (file.path.clone(), Arc::clone(index)))
            .collect();
        CrossFileAnalysis { used, indices }
    }
}

/// Collect a file's top-level definitions and its free uses from its index.
///
/// A use is *free* when no definition reaches it within the file — the same
/// `reaching_definitions().is_empty()` test oak's `resolve_at` uses before
/// falling back to cross-file resolution. Reaching definitions already fold in
/// enclosing-scope captures, so a closure reading an outer local counts as
/// bound, not free.
fn collect_file_uses(path: PathBuf, index: &SemanticIndex) -> FileUses {
    let top_defs: Vec<String> = index
        .exports()
        .keys()
        .map(|name| name.to_string())
        .collect();

    let mut free_uses: HashSet<String> = HashSet::new();
    for scope in index.scope_ids() {
        let symbols = index.symbols(scope);
        for (use_id, use_site) in index.uses(scope).iter() {
            if index.reaching_definitions(scope, use_id).next().is_some() {
                continue;
            }
            free_uses.insert(symbols.symbol(use_site.symbol()).name().to_string());
        }
    }

    FileUses { path, top_defs, free_uses }
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
