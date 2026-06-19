//! Shared analysis database backed by oak's salsa stack.
//!
//! jarl's CLI is a one-shot tool over a list of paths, while oak's
//! [`oak_scan`] machinery is built for editor workspace folders. We bridge
//! the two by scanning only the **package roots** that the linted paths
//! belong to (bounded by `DESCRIPTION` discovery), never the unbounded
//! parent directory of a loose script. That keeps a `jarl /tmp/foo.R`
//! invocation from walking all of `/tmp`.
//!
//! The database is built once in [`crate::check::check`] and shared
//! read-only across the per-file parallel pass: `OakDatabase` is `Send +
//! Sync` and its tracked queries take `&dyn Db`, so cross-file lookups
//! (`File::semantic_index`, `Package::resolve`, `oak_ide::find_references`)
//! run concurrently without cloning storage.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use aether_path::FilePath;
use oak_db::{Db, File, OakDatabase};
use oak_scan::ScanScheduler;

use crate::package::find_package_root;

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
}

/// The deduplicated set of package roots (directories containing a
/// `DESCRIPTION`) for `paths`, with nested roots collapsed to their
/// outermost ancestor so each tree is scanned once.
fn package_roots(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = paths
        .iter()
        .filter_map(|path| find_package_root(path))
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
