use crate::message::Diagnostic;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
struct FileCache {
    last_modified: u64,      // Timestamp of the last modification
    rule_hash: u64,          // Hash of the rules
    checks: Vec<Diagnostic>, // Cached checks
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LinterCache {
    files: HashMap<PathBuf, FileCache>,
    rule_hash: u64,
}

impl LinterCache {
    pub fn new() -> Self {
        LinterCache { files: HashMap::new(), rule_hash: 0 }
    }

    pub fn load_from_disk(cache_path: &Path) -> io::Result<Self> {
        let mut file = File::open(cache_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let cache = bincode::deserialize(&buffer).unwrap_or_else(|_| LinterCache::new());
        Ok(cache)
    }

    pub fn save_to_disk(&self, cache_path: &Path) -> io::Result<()> {
        let serialized = bincode::serialize(self).unwrap();
        let mut file = File::create(cache_path)?;
        file.write_all(&serialized)?;
        Ok(())
    }

    pub fn get_file_last_modified(path: &Path) -> Option<u64> {
        fs::metadata(path)
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .map(|modified| modified.duration_since(UNIX_EPOCH).unwrap().as_secs())
    }

    pub fn hash_rules<T: Hash>(rules: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        rules.hash(&mut hasher);
        hasher.finish()
    }

    pub fn update_rules<T: Hash>(&mut self, rules: &T) {
        self.rule_hash = Self::hash_rules(rules);
    }

    pub fn is_cache_valid(&self, path: &Path, current_rule_hash: u64) -> bool {
        if let Some(file_cache) = self.files.get(path) {
            let current_last_modified = Self::get_file_last_modified(path);
            file_cache.last_modified == current_last_modified.unwrap_or(0)
                && file_cache.rule_hash == current_rule_hash
        } else {
            false
        }
    }

    pub fn get_cached_checks(&self, path: &Path) -> Option<&Vec<Diagnostic>> {
        self.files.get(path).map(|file_cache| &file_cache.checks)
    }

    pub fn update_cache(
        &mut self,
        path: PathBuf,
        checks: &Vec<Diagnostic>,
        current_rule_hash: u64,
    ) {
        let last_modified = Self::get_file_last_modified(&path).unwrap_or(0);
        self.files.insert(
            path,
            FileCache {
                last_modified,
                rule_hash: current_rule_hash,
                checks: checks.to_vec(),
            },
        );
    }
}

// fn main() -> io::Result<()> {
//     let cache_path = PathBuf::from(".flir_cache");

//     // Load cache from disk (or create a new one if it doesn't exist)
//     let mut cache = LinterCache::load_from_disk(&cache_path).unwrap_or_else(|_| LinterCache::new());

//     // Example rules (replace with your actual rules)
//     let rules = vec!["rule1", "rule2"];
//     cache.update_rules(&rules);

//     let file_path = PathBuf::from("example.R");

//     // Check if the cache is valid for the file
//     if cache.is_cache_valid(&file_path, cache.rule_hash) {
//         // Use cached checks
//         if let Some(checks) = cache.get_cached_checks(&file_path) {
//             println!("Cached checks: {:?}", checks);
//         }
//     } else {
//         // Parse the file, find checks, and update the cache
//         let checks = vec!["lint1".to_string(), "lint2".to_string()]; // Replace with actual linting logic
//         cache.update_cache(file_path.clone(), checks.clone(), cache.rule_hash);
//         println!("New checks: {:?}", checks);
//     }

//     // Save the updated cache to disk
//     cache.save_to_disk(&cache_path)?;

//     Ok(())
// }
