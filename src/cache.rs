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
    last_modified: u64, // Timestamp of the last modification
    rule_hash: u64,     // Hash of the rules
    lints: Vec<String>, // Cached lints
}

#[derive(Debug, Serialize, Deserialize)]
struct LinterCache {
    files: HashMap<PathBuf, FileCache>,
    rule_hash: u64,
}

impl LinterCache {
    fn new() -> Self {
        LinterCache { files: HashMap::new(), rule_hash: 0 }
    }

    fn load_from_disk(cache_path: &Path) -> io::Result<Self> {
        let mut file = File::open(cache_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let cache = bincode::deserialize(&buffer).unwrap_or_else(|_| LinterCache::new());
        Ok(cache)
    }

    fn save_to_disk(&self, cache_path: &Path) -> io::Result<()> {
        let serialized = bincode::serialize(self).unwrap();
        let mut file = File::create(cache_path)?;
        file.write_all(&serialized)?;
        Ok(())
    }

    fn get_file_last_modified(path: &Path) -> Option<u64> {
        fs::metadata(path)
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .map(|modified| modified.duration_since(UNIX_EPOCH).unwrap().as_secs())
    }

    fn hash_rules<T: Hash>(rules: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        rules.hash(&mut hasher);
        hasher.finish()
    }

    fn update_rules<T: Hash>(&mut self, rules: &T) {
        self.rule_hash = Self::hash_rules(rules);
    }

    fn is_cache_valid(&self, path: &Path, current_rule_hash: u64) -> bool {
        if let Some(file_cache) = self.files.get(path) {
            let current_last_modified = Self::get_file_last_modified(path);
            file_cache.last_modified == current_last_modified.unwrap_or(0)
                && file_cache.rule_hash == current_rule_hash
        } else {
            false
        }
    }

    fn get_cached_lints(&self, path: &Path) -> Option<&Vec<String>> {
        self.files.get(path).map(|file_cache| &file_cache.lints)
    }

    fn update_cache(&mut self, path: PathBuf, lints: Vec<String>, current_rule_hash: u64) {
        let last_modified = Self::get_file_last_modified(&path).unwrap_or(0);
        self.files.insert(
            path,
            FileCache { last_modified, rule_hash: current_rule_hash, lints },
        );
    }
}

fn main() -> io::Result<()> {
    let cache_path = PathBuf::from(".flir_cache");

    // Load cache from disk (or create a new one if it doesn't exist)
    let mut cache = LinterCache::load_from_disk(&cache_path).unwrap_or_else(|_| LinterCache::new());

    // Example rules (replace with your actual rules)
    let rules = vec!["rule1", "rule2"];
    cache.update_rules(&rules);

    let file_path = PathBuf::from("example.R");

    // Check if the cache is valid for the file
    if cache.is_cache_valid(&file_path, cache.rule_hash) {
        // Use cached lints
        if let Some(lints) = cache.get_cached_lints(&file_path) {
            println!("Cached lints: {:?}", lints);
        }
    } else {
        // Parse the file, find lints, and update the cache
        let lints = vec!["lint1".to_string(), "lint2".to_string()]; // Replace with actual linting logic
        cache.update_cache(file_path.clone(), lints.clone(), cache.rule_hash);
        println!("New lints: {:?}", lints);
    }

    // Save the updated cache to disk
    cache.save_to_disk(&cache_path)?;

    Ok(())
}
