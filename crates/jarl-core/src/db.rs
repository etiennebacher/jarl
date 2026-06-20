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

use aether_path::FilePath;
use air_r_parser::RParserOptions;
use oak_db::{Db, File, OakDatabase, Package, workspace_files};
use oak_scan::ScanScheduler;
use oak_semantic::ScopeId;
use oak_semantic::semantic_index::SemanticIndex;

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
    /// reads it. We enumerate every use in every scanned file and resolve it
    /// with oak's cross-file [`File::resolve_at`]: a use that binds to a
    /// file-scope definition in a *different* file is a cross-file read of that
    /// package-level object. The defining file's name is recorded against the
    /// read so the lint can suppress it.
    ///
    /// Keyed by relativized file path to match the lint's per-file lookup.
    pub fn cross_file_used_objects(&self) -> HashMap<PathBuf, HashSet<String>> {
        let db = self.db();

        // Build a throwaway index per file to enumerate use sites, and collect
        // the package's top-level binding names. Only those names can be the
        // target of a cross-file read, so probing just their uses skips the
        // locals, package functions, and library symbols that dominate a file.
        let mut indices: Vec<(File, SemanticIndex)> = Vec::new();
        let mut candidates: HashSet<String> = HashSet::new();
        for &file in workspace_files(db) {
            let parsed =
                air_r_parser::parse(file.source_text(db).as_str(), RParserOptions::default());
            if parsed.has_error() {
                continue;
            }
            let index =
                oak_semantic::build_index(&parsed.tree(), oak_semantic::NoopImportsResolver);
            candidates.extend(index.exports().keys().map(|name| name.to_string()));
            indices.push((file, index));
        }

        let file_scope = ScopeId::from(0);
        let mut used: HashMap<PathBuf, HashSet<String>> = HashMap::new();
        for (file, index) in &indices {
            for scope in index.scope_ids() {
                let symbols = index.symbols(scope);
                for (_use_id, use_site) in index.uses(scope).iter() {
                    let name = symbols.symbol(use_site.symbol()).name();
                    if !candidates.contains(name) {
                        continue;
                    }
                    for def in file.resolve_at(db, use_site.range().start()) {
                        // Only top-level bindings are shared across the package,
                        // and a read in the binding's own file is already
                        // handled by the per-file analysis.
                        if def.scope(db) != file_scope || def.file(db) == *file {
                            continue;
                        }
                        let Some(path) = def
                            .file(db)
                            .path(db)
                            .as_path()
                            .map(|p| p.as_std_path().to_path_buf())
                        else {
                            continue;
                        };
                        used.entry(PathBuf::from(crate::fs::relativize_path(&path)))
                            .or_default()
                            .insert(def.name(db).text(db).as_str().to_string());
                    }
                }
            }
        }
        used
    }
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
