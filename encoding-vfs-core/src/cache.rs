use encoding_rs::Encoding;
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Cache entry with TTL
#[derive(Clone)]
struct CacheEntry {
    encoding: &'static Encoding,
    inserted_at: Instant,
}

/// Cache for file encoding results.
/// Key: file path, Value: detected encoding + timestamp
pub struct EncodingCache {
    cache: DashMap<PathBuf, CacheEntry>,
    ttl: Duration,
    #[allow(dead_code)]
    max_entries: u64,
}

impl EncodingCache {
    pub fn new(max_entries: u64, ttl_seconds: u64) -> Self {
        Self {
            cache: DashMap::new(),
            ttl: Duration::from_secs(ttl_seconds),
            max_entries,
        }
    }

    /// Get cached encoding for a file, or None if not cached or expired
    pub fn get(&self, path: &Path) -> Option<&'static Encoding> {
        self.cache.get(path).and_then(|entry| {
            if entry.inserted_at.elapsed() < self.ttl {
                Some(entry.encoding)
            } else {
                None // Expired
            }
        })
    }

    /// Cache the detected encoding for a file
    pub fn insert(&self, path: &Path, encoding: &'static Encoding) {
        self.cache.insert(path.to_path_buf(), CacheEntry {
            encoding,
            inserted_at: Instant::now(),
        });
    }

    /// Remove cached encoding for a file (e.g., after file modification)
    pub fn invalidate(&self, path: &Path) {
        self.cache.remove(path);
    }

    /// Clear all cached encodings
    pub fn invalidate_all(&self) {
        self.cache.clear();
    }

    /// Get number of cached entries
    pub fn entry_count(&self) -> usize {
        self.cache.len()
    }
}
