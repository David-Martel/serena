//! LSP response caching
//!
//! Provides caching mechanisms for LSP responses to reduce redundant
//! language server queries and improve performance.

use dashmap::DashMap;
use serde_json::Value;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

/// A cache key for LSP requests
#[derive(Clone, Debug, Eq)]
struct CacheKey {
    method: String,
    params: String, // JSON-serialized params
}

impl PartialEq for CacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.method == other.method && self.params == other.params
    }
}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.method.hash(state);
        self.params.hash(state);
    }
}

/// A cached value with timestamp
struct CachedValue {
    value: Value,
    cached_at: Instant,
}

/// LSP response cache
///
/// Caches LSP responses using a DashMap for concurrent access.
/// Cache entries have a configurable TTL (time-to-live).
pub struct LspCache {
    cache: DashMap<CacheKey, CachedValue>,
    ttl: Duration,
}

impl LspCache {
    /// Create a new LSP cache with default TTL of 5 minutes
    pub fn new() -> Self {
        Self::with_ttl(Duration::from_secs(300))
    }

    /// Create a new LSP cache with a custom TTL
    ///
    /// # Arguments
    /// * `ttl` - Time-to-live for cache entries
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            cache: DashMap::new(),
            ttl,
        }
    }

    /// Get a cached value if it exists and hasn't expired
    ///
    /// # Arguments
    /// * `method` - The LSP method name
    /// * `params` - The request parameters (will be serialized to JSON for key)
    ///
    /// # Returns
    /// The cached value if found and valid, or `None`
    pub fn get(&self, method: &str, params: &Value) -> Option<Value> {
        let key = CacheKey {
            method: method.to_string(),
            params: params.to_string(),
        };

        if let Some(entry) = self.cache.get(&key) {
            // Check if cache entry has expired
            if entry.cached_at.elapsed() < self.ttl {
                return Some(entry.value.clone());
            } else {
                // Remove expired entry
                drop(entry);
                self.cache.remove(&key);
            }
        }

        None
    }

    /// Insert a value into the cache
    ///
    /// # Arguments
    /// * `method` - The LSP method name
    /// * `params` - The request parameters
    /// * `value` - The response value to cache
    pub fn insert(&self, method: &str, params: &Value, value: Value) {
        let key = CacheKey {
            method: method.to_string(),
            params: params.to_string(),
        };

        let cached_value = CachedValue {
            value,
            cached_at: Instant::now(),
        };

        self.cache.insert(key, cached_value);
    }

    /// Clear all cached entries
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Remove expired entries from the cache
    ///
    /// This is called automatically during get operations, but can
    /// also be called manually for cleanup.
    pub fn prune_expired(&self) {
        let now = Instant::now();
        self.cache
            .retain(|_, v| now.duration_since(v.cached_at) < self.ttl);
    }

    /// Get the number of cached entries
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Invalidate cache entries for a specific method
    ///
    /// # Arguments
    /// * `method` - The LSP method name to invalidate
    pub fn invalidate_method(&self, method: &str) {
        self.cache.retain(|k, _| k.method != method);
    }
}

impl Default for LspCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_cache_insert_and_get() {
        let cache = LspCache::new();
        let params = json!({"uri": "file:///test.rs"});
        let value = json!({"symbols": []});

        cache.insert("textDocument/documentSymbol", &params, value.clone());

        let retrieved = cache.get("textDocument/documentSymbol", &params);
        assert_eq!(retrieved, Some(value));
    }

    #[test]
    fn test_cache_miss() {
        let cache = LspCache::new();
        let params = json!({"uri": "file:///test.rs"});

        let retrieved = cache.get("textDocument/documentSymbol", &params);
        assert_eq!(retrieved, None);
    }

    #[test]
    fn test_cache_expiration() {
        let cache = LspCache::with_ttl(Duration::from_millis(10));
        let params = json!({"uri": "file:///test.rs"});
        let value = json!({"symbols": []});

        cache.insert("textDocument/documentSymbol", &params, value.clone());

        // Should be cached
        assert!(cache.get("textDocument/documentSymbol", &params).is_some());

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(20));

        // Should be expired
        assert!(cache.get("textDocument/documentSymbol", &params).is_none());
    }

    #[test]
    fn test_cache_clear() {
        let cache = LspCache::new();
        let params = json!({"uri": "file:///test.rs"});
        let value = json!({"symbols": []});

        cache.insert("textDocument/documentSymbol", &params, value);
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_invalidate_method() {
        let cache = LspCache::new();

        let params1 = json!({"uri": "file:///test1.rs"});
        let params2 = json!({"uri": "file:///test2.rs"});
        let value = json!({"symbols": []});

        cache.insert("textDocument/documentSymbol", &params1, value.clone());
        cache.insert("textDocument/definition", &params2, value.clone());

        assert_eq!(cache.len(), 2);

        cache.invalidate_method("textDocument/documentSymbol");

        assert_eq!(cache.len(), 1);
        assert!(cache.get("textDocument/documentSymbol", &params1).is_none());
        assert!(cache.get("textDocument/definition", &params2).is_some());
    }
}
