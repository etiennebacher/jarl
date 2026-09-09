//! Shared analysis database backed by oak's salsa stack.
//!
//! jarl's CLI is a one-shot tool over a list of paths, while oak's
//! [`oak_scan`] machinery is built for editor workspace folders. We bridge
//! the two by scanning only roots the invocation actually implies: the
//! **package roots** the linted paths belong to (bounded by `DESCRIPTION`
//! discovery), and the **project roots** the user declared by passing a
//! directory or by having a `jarl.toml` next to the file. The unbounded
//! parent directory of a bare loose script is never one, which keeps a
//! `jarl /tmp/foo.R` invocation from walking all of `/tmp`. A loose script
//! with no root of its own still takes part in cross-file analysis: the lint
//! set itself is its file universe, so it is handed to
//! [`AnalysisDb::cross_file_used_objects`] directly and resolves against the
//! other linted files through explicit `source()` edges.
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
use oak_semantic::semantic_index::{DefinitionKind, SemanticCallKind, SemanticIndex};
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
/// top level, the names it reads *freely* — without definitely binding them
/// anywhere in the file, so they may reference another file — and the
/// `source()` bindings it consumes (a read reaching a
/// [`DefinitionKind::Import`] uses the target file's top-level binding).
struct FileUses {
    path: PathBuf,
    top_defs: Vec<String>,
    free_uses: HashSet<String>,
    /// `(target file, name)` per `Import`-kind definition reached by a use:
    /// this file reads `name` out of `target file` via `source()`.
    import_uses: HashSet<(PathBuf, String)>,
    /// The files this one `source()`s, whatever it reads from them. These are
    /// the edges [`Visibility`] walks to give a sourced file the environment
    /// it runs in.
    source_targets: Vec<PathBuf>,
}

/// Which files a given file can read top-level bindings from.
///
/// Free-use name matching is only sound between files that share an
/// environment; matching across unrelated scripts would count them as readers
/// of each other. Two relations grant it:
///
/// - **A shared environment**, keyed per file by [`Visibility::environment_of`]
///   and decided from the file's path alone. Files with the same key see each
///   other, whatever the load order.
/// - **`source()` inheritance**: a sourced file runs in the environment of the
///   file that sourced it, so it reads that file's top-level bindings. This one
///   is directional, and transitive down a `source()` chain.
///
/// A flat directory of unrelated scripts has neither, so its files stay
/// invisible to each other — R gives no reason to believe they share an
/// environment.
///
/// This mirrors the load contexts of oak's [`File::imports`] without calling
/// it. That query reads every workspace file's semantic index — including,
/// through the reverse `source()` graph, files other than its own — so asking
/// it here would rebuild the whole workspace's indices on this one thread,
/// serializing work the pass below already does on the rayon pool. Everything
/// this type needs is either a path or something the parallel pass already
/// produced.
#[derive(Default)]
struct Visibility {
    /// The environment a file's top-level bindings live in: its package
    /// namespace, its shiny app, or the `R/` directory it is collated into.
    /// Absent for a file in none of those.
    environment_of: HashMap<PathBuf, PathBuf>,
    /// Per file, the files that `source()` it, directly or through a chain.
    sourced_by: HashMap<PathBuf, HashSet<PathBuf>>,
}

impl Visibility {
    /// Whether `reader` can read top-level bindings defined in `definer`.
    fn sees(&self, reader: &Path, definer: &Path) -> bool {
        if let (Some(reader_env), Some(definer_env)) = (
            self.environment_of.get(reader),
            self.environment_of.get(definer),
        ) && reader_env == definer_env
        {
            return true;
        }
        self.sourced_by
            .get(reader)
            .is_some_and(|sourcing| sourcing.contains(definer))
    }

    /// Record, per file, every file that `source()`s it — directly, or through
    /// a chain, since a file two hops down still runs in the environment the
    /// chain started in.
    fn add_source_inheritance(&mut self, built: &[(Arc<SemanticIndex>, FileUses)]) {
        let mut targets_of: HashMap<&Path, &[PathBuf]> = HashMap::new();
        for (_, file) in built {
            if !file.source_targets.is_empty() {
                targets_of.insert(file.path.as_path(), &file.source_targets);
            }
        }
        if targets_of.is_empty() {
            return;
        }

        // Depth-first from every sourcing file, carrying it down the chain.
        // Revisiting a file under the same sourcer adds nothing, which both
        // prunes diamonds and terminates `source()` cycles.
        for &sourcer in targets_of.keys() {
            let mut stack: Vec<&Path> = targets_of[sourcer].iter().map(PathBuf::as_path).collect();
            while let Some(sourced) = stack.pop() {
                if !self
                    .sourced_by
                    .entry(sourced.to_path_buf())
                    .or_default()
                    .insert(sourcer.to_path_buf())
                {
                    continue;
                }
                if let Some(deeper) = targets_of.get(sourced) {
                    stack.extend(deeper.iter().map(PathBuf::as_path));
                }
            }
        }
    }
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
    /// Memo of `source()` target indices populated by the resolvers above,
    /// seeded with each file's own (untruncated) index, and shared with the
    /// lint pass.
    pub source_index_cache: jarl_semantic::SourceIndexCache,
}

impl AnalysisDb {
    /// Scan the roots covering `paths` into a fresh database.
    ///
    /// Two kinds of root are registered. Every `DESCRIPTION` directory
    /// covering `paths` is one, found by walking up from each linted file.
    /// `project_roots` adds the directories the user declared explicitly (see
    /// [`crate::config::Config::project_roots`]), which is what lets files in
    /// a non-package project be scanned as a unit.
    ///
    /// A loose script with neither — a bare file argument in an unconfigured
    /// directory — contributes no root and is simply absent from the database;
    /// its per-file analysis falls back to the standalone index builder. That
    /// is what keeps `jarl /tmp/foo.R` from walking all of `/tmp`.
    pub fn build(paths: &[PathBuf], project_roots: &[PathBuf]) -> Self {
        let mut db = OakDatabase::new();
        let roots = scan_roots(paths, project_roots);
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
    /// read from *another* file — through a shared environment or through a
    /// `source()` edge.
    ///
    /// Files that share an environment — a package's namespace, a collated
    /// `R/` directory, a shiny app's support files — see each other's
    /// top-level bindings, so a binding defined in one and read in another is
    /// used even when its own file never reads it. For every file we collect,
    /// from its per-file index, the names it defines at top level and the
    /// names it reads *freely* — uses the file doesn't definitely bind, which
    /// therefore may reference another file (this is the same
    /// `use_is_bound()` test oak's `resolve_at` uses to decide
    /// local-vs-cross-file). A top-level definition is cross-file-used when a
    /// file that can see it reads its name freely; [`Visibility`] decides
    /// which pairs qualify.
    ///
    /// `script_paths` are the linted R files outside every scanned root. They
    /// have no database entry and so no visibility, and participate only
    /// through the `source()` edges below: a read reaching a
    /// `DefinitionKind::Import` marks the *target* file's binding used,
    /// chasing forwards when the target itself sources the real definer.
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

        // Collect the paths and each file's environment up front. Both need the
        // (`!Sync`) db but touch no disk, so this sequential loop is cheap.
        // Reading the file contents — the part that scales with file count —
        // is deferred to the parallel pass below. Loose scripts (outside every
        // scanned root) have no `File`, so they get no environment and
        // participate through `source()` edges alone.
        let mut paths: Vec<PathBuf> = Vec::new();
        let mut packaged: Vec<(PathBuf, PathBuf)> = Vec::new();
        let mut app_dirs: HashSet<PathBuf> = HashSet::new();
        for &file in workspace_files(db) {
            let Some(path) = relative_file_path(db, file) else {
                continue;
            };
            match file.package(db) {
                Some(package) => {
                    if let Some(root) = package_root_path(db, package) {
                        packaged.push((path.clone(), root));
                    }
                }
                None => {
                    if let Some(dir) = shiny_app_dir(db, file, &path) {
                        app_dirs.insert(dir);
                    }
                }
            }
            paths.push(path);
        }

        // Now that the app directories are known, every scanned file's
        // environment can be settled. A package file's is its namespace; the
        // rest go by directory layout.
        let mut visibility = Visibility::default();
        for (path, root) in packaged {
            visibility.environment_of.insert(path, root);
        }
        for path in &paths {
            if visibility.environment_of.contains_key(path) {
                continue;
            }
            if let Some(environment) = script_environment(path, &app_dirs) {
                visibility.environment_of.insert(path.clone(), environment);
            }
        }

        let scanned: HashSet<&PathBuf> = paths.iter().collect();
        let scripts: Vec<PathBuf> = script_paths
            .iter()
            .map(|path| PathBuf::from(air_fs::relativize_path(path)))
            .filter(|path| !scanned.contains(path))
            .collect();
        paths.extend(scripts);

        // Per file, in parallel: read the source, parse, build the index, and
        // read off the file's top-level definitions and its free uses. None of
        // this needs the db, so it's the rayon-friendly bulk of the work. The
        // index is kept (shared with the lint pass), so building it here is not
        // throwaway work. Reading from disk here (rather than via the db's
        // `source_text`) is what lets the read run in parallel; in the one-shot
        // CLI the disk is the source of truth, so the two are equivalent.
        let source_index_cache = jarl_semantic::SourceIndexCache::new();
        let built: Vec<(Arc<SemanticIndex>, FileUses)> = paths
            .par_iter()
            .filter_map(|path| {
                let source = std::fs::read_to_string(path).ok()?;
                let parsed = air_r_parser::parse(&source, RParserOptions::default());
                if parsed.has_error() {
                    return None;
                }
                let index = oak_semantic::build_index(
                    &parsed.tree(),
                    jarl_semantic::JarlImportsResolver::with_cache(
                        path.clone(),
                        source_index_cache.clone(),
                    ),
                );
                let uses = collect_file_uses(path.clone(), &index);
                Some((Arc::new(index), uses))
            })
            .collect();

        // Seed the memo with each file's own index. These are the untruncated
        // builds, so they overwrite any cycle-truncated sub-build a resolver
        // chain may have cached for the same file, and the lint pass resolves
        // `source()` targets without re-indexing them.
        for (index, file) in &built {
            let key = std::path::absolute(&file.path).unwrap_or_else(|_| file.path.clone());
            source_index_cache.insert(key, Arc::clone(index));
        }

        // The `source()` graph the parallel pass read off each index completes
        // the visibility relation: a sourced file runs in its sourcer's
        // environment and so reads its top-level bindings.
        visibility.add_source_inheritance(&built);

        // Only a name defined at top level somewhere can be the target of a
        // cross-file read, so indexing by definer drops free uses of locals,
        // library symbols and base functions on lookup. A name usually has a
        // single definer, which keeps the match below linear in the free uses
        // rather than quadratic in the file count.
        let mut definers_of: HashMap<&str, Vec<&PathBuf>> = HashMap::new();
        for (_, file) in &built {
            for name in &file.top_defs {
                definers_of
                    .entry(name.as_str())
                    .or_default()
                    .push(&file.path);
            }
        }

        // A top-level definition is used when some *other* file that can see
        // it reads its name freely.
        let mut used: HashMap<PathBuf, HashSet<String>> = HashMap::new();
        for (_, reader) in &built {
            for name in &reader.free_uses {
                let Some(definers) = definers_of.get(name.as_str()) else {
                    continue;
                };
                for definer in definers {
                    if **definer != reader.path && visibility.sees(&reader.path, definer) {
                        used.entry((*definer).clone())
                            .or_default()
                            .insert(name.clone());
                    }
                }
            }
        }

        // `source()` edges: a read that reaches an `Import`-kind definition
        // consumes the target file's top-level binding, so mark it used there.
        // A target may forward the name from a file it sources itself; chase
        // those `Import`-kind exports to the real definer (jarl's parallel of
        // oak_db's `File::collect_exports`), marking every hop. The visited
        // set makes `source()` cycles terminate.
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
            for (_, def) in index.export(&name) {
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
        CrossFileAnalysis { used, indices, source_index_cache }
    }
}

/// Collect a file's top-level definitions, its free uses, and its `source()`
/// import uses from its index.
///
/// A use is *free* when the file doesn't definitely bind it — the same
/// `use_is_bound()` test oak's `resolve_at` uses before falling back to
/// cross-file resolution. Reaching definitions already fold in enclosing-scope
/// captures, so a closure reading an outer local counts as bound, not free.
///
/// Not definitely bound is weaker than unbound: a conditional local
/// (`if (cond) x <- 2; x`) or a `<<-` write leaves some path reaching the use
/// unbound, so the read can still resolve through the package namespace even
/// though a definition in this file reaches it. Counting those as free keeps
/// the cross-file side conservative in the *used* direction.
///
/// A use reaching a [`DefinitionKind::Import`] instead reads a binding that
/// `source()` injected from another file, so it's recorded as an import use
/// of `(target file, name)`.
///
/// Every file reports both sides unconditionally; which pairs may be matched
/// against each other is [`Visibility`]'s job. A file no one can see simply
/// never matches, and contributes through its import uses alone.
fn collect_file_uses(path: PathBuf, index: &SemanticIndex) -> FileUses {
    let top_defs: Vec<String> = index
        .exports()
        .keys()
        .map(|name| name.to_string())
        .collect();

    let mut free_uses: HashSet<String> = HashSet::new();
    let mut import_uses: HashSet<(PathBuf, String)> = HashSet::new();
    for scope in index.scope_ids() {
        let symbols = index.symbols(scope);
        for (use_id, use_site) in index.uses(scope).iter() {
            for (def_scope, def_id) in index.reaching_definitions(scope, use_id) {
                let def = &index.definitions(def_scope)[def_id];
                let DefinitionKind::Import { file: url, name, .. } = def.kind() else {
                    continue;
                };
                let Some(target) = import_target_path(url) else {
                    continue;
                };
                import_uses.insert((target, name.clone()));
            }
            if !index.use_is_bound(scope, use_id) {
                free_uses.insert(symbols.symbol(use_site.symbol()).name().to_string());
            }
        }
    }

    // `source()` calls oak resolved to a file. Only the annotated calls are
    // here, so a shadowed `source` or a non-literal `local =` argument — which
    // doesn't run the target in this file's environment — is already excluded.
    let source_targets: Vec<PathBuf> = index
        .semantic_calls()
        .iter()
        .filter_map(|call| match call.kind() {
            SemanticCallKind::Source { resolved, .. } => resolved.as_ref(),
            _ => None,
        })
        .filter_map(import_target_path)
        .collect();

    FileUses {
        path,
        top_defs,
        free_uses,
        import_uses,
        source_targets,
    }
}

/// Convert an `Import` definition's file URL back to the relativized path
/// that keys [`CrossFileAnalysis`]'s maps. [`jarl_semantic::JarlImportsResolver`]
/// builds these URLs from absolutized paths, so the round-trip through
/// `to_file_path` + relativize lands on the same key as the linted file's.
fn import_target_path(url: &url::Url) -> Option<PathBuf> {
    let path = url.to_file_path().ok()?;
    Some(PathBuf::from(air_fs::relativize_path(path)))
}

/// The relativized path of a database [`File`], in the form that keys
/// [`CrossFileAnalysis`]'s maps. `None` when the file's URL has no filesystem
/// path (e.g. a virtual document).
fn relative_file_path(db: &dyn Db, file: File) -> Option<PathBuf> {
    let path = file.path(db).as_path()?.as_std_path().to_path_buf();
    Some(PathBuf::from(air_fs::relativize_path(&path)))
}

/// The RStudio entry-point filenames a shiny app is recognized by, each with
/// the call that has to appear in it. Shiny matches the filenames
/// case-insensitively, unlike the exact `R/` convention.
const SHINY_ENTRY_FILES: [(&str, &str); 3] = [
    ("app.R", "shinyApp"),
    ("ui.R", "shinyUI"),
    ("server.R", "shinyServer"),
];

/// The directory `path` makes a shiny app root, i.e. its own directory when it
/// is one of the app's entry points. `None` for every other file.
///
/// Text matching on the source, the same test oak applies: it accepts the
/// occasional `shinyApp` written in a comment, which only ever widens an app's
/// support set. Reads the file's text but not its semantic index, so it stays
/// off the expensive path this module avoids.
fn shiny_app_dir(db: &dyn Db, file: File, path: &Path) -> Option<PathBuf> {
    let name = path.file_name()?.to_str()?;
    let (_, marker) = SHINY_ENTRY_FILES
        .iter()
        .find(|(entry, _)| name.eq_ignore_ascii_case(entry))?;
    file.source_text(db)
        .contains(marker)
        .then(|| path.parent())
        .flatten()
        .map(Path::to_path_buf)
}

/// The environment a file outside every package shares with its neighbours,
/// from directory layout alone. `None` for a loose script, which has no reason
/// to share one with anything.
///
/// Two layouts qualify, both mirroring the load contexts oak's [`File::imports`]
/// derives:
///
/// - **An `R/` directory.** R sources its files alphabetically into one
///   environment, the convention a package without `Collate:` follows. The
///   directory name is case-sensitive, like R's package scanner.
/// - **A shiny app.** `shiny::runApp()` evaluates `global.R` and the adjacent
///   `R/` directory into the environment its entry point runs in, so the app's
///   files all resolve against each other. A script the app doesn't load — one
///   sitting next to `app.R` under some other name — is not part of it.
fn script_environment(path: &Path, app_dirs: &HashSet<PathBuf>) -> Option<PathBuf> {
    let dir = path.parent()?;
    if dir.file_name() == Some(std::ffi::OsStr::new("R")) {
        // An app's `R/` collates into the app rather than on its own, so that
        // its files also reach `global.R` and the entry point.
        return match dir.parent() {
            Some(app) if app_dirs.contains(app) => Some(app.to_path_buf()),
            _ => Some(dir.to_path_buf()),
        };
    }
    (app_dirs.contains(dir) && is_shiny_app_file(path)).then(|| dir.to_path_buf())
}

/// Whether a file sitting directly in a shiny app directory is one the app
/// loads: an entry point, or the `global.R` that `loadSupport()` evaluates
/// before the app's own code.
fn is_shiny_app_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.eq_ignore_ascii_case("global.R")
        || SHINY_ENTRY_FILES
            .iter()
            .any(|(entry, _)| name.eq_ignore_ascii_case(entry))
}

/// The root directory of a [`Package`], i.e. the directory holding its
/// `DESCRIPTION`.
fn package_root_path(db: &dyn Db, package: Package) -> Option<PathBuf> {
    package
        .description_path(db)
        .as_path()
        .and_then(|path| path.parent())
        .map(|dir| dir.as_std_path().to_path_buf())
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

/// The deduplicated set of directories to scan: the package roots
/// (directories containing a `DESCRIPTION`) covering `paths`, plus the
/// user-declared `project_roots`. Nested roots are collapsed to their
/// outermost ancestor so each tree is scanned once.
///
/// Paths are absolutized against the working directory first: oak's scanner
/// keys files by `file://` URL and rejects relative paths, and walking up a
/// relative path like `R/foo.R` would otherwise resolve the root to an empty
/// (cwd-relative) path the scanner can't register.
fn scan_roots(paths: &[PathBuf], project_roots: &[PathBuf]) -> Vec<PathBuf> {
    let cwd = std::env::current_dir().ok();
    let absolutize = |path: &PathBuf| -> Option<PathBuf> {
        if path.is_absolute() {
            Some(path.clone())
        } else {
            Some(cwd.as_ref()?.join(path))
        }
    };
    let mut roots: Vec<PathBuf> = paths
        .iter()
        .filter_map(|path| find_package_root(&absolutize(path)?))
        .chain(project_roots.iter().filter_map(absolutize))
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
