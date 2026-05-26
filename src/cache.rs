//! Generic in-memory TTL cache.
//!
//! Thread-safe cache with expiration time per entry.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Thread-safe TTL cache.
///
/// Stores `(value, expiry_instant)` per key. Entries are expired on access.
pub struct TtlCache<K, V> {
    inner: Mutex<HashMap<K, (V, Instant)>>,
    ttl: Duration,
}

impl<K, V> TtlCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// Create a new cache with the given default TTL for all entries.
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            ttl,
        }
    }

    /// Get a value if it exists and is not expired.
    /// Returns `None` if the key is missing or the entry has expired.
    pub fn get(&self, key: &K) -> Option<V> {
        let mut inner = self.inner.lock().unwrap();
        if let Some((value, expiry)) = inner.get(key) {
            if Instant::now() < *expiry {
                return Some(value.clone());
            }
            // Expired — remove and return None
            inner.remove(key);
        }
        None
    }

    /// Insert a value with the cache's default TTL.
    pub fn set(&self, key: K, value: V) {
        self.set_with_ttl(key, value, self.ttl);
    }

    /// Insert a value with a custom TTL.
    pub fn set_with_ttl(&self, key: K, value: V, ttl: Duration) {
        let mut inner = self.inner.lock().unwrap();
        inner.insert(key, (value, Instant::now() + ttl));
    }

    /// Check if a key exists and is not stale (expired).
    pub fn is_stale(&self, key: &K) -> bool {
        let inner = self.inner.lock().unwrap();
        match inner.get(key) {
            Some((_, expiry)) => Instant::now() >= *expiry,
            None => true,
        }
    }

    /// Remove a key from the cache.
    pub fn remove(&self, key: &K) {
        let mut inner = self.inner.lock().unwrap();
        inner.remove(key);
    }

    /// Clear all entries.
    pub fn clear(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ttl_cache_basic() {
        let cache = TtlCache::<&str, &str>::new(Duration::from_millis(100));

        cache.set("key1", "value1");
        assert_eq!(cache.get(&"key1"), Some("value1"));

        // Different key returns None
        assert_eq!(cache.get(&"key2"), None);
    }

    #[test]
    fn test_ttl_cache_expiry() {
        let cache = TtlCache::<&str, &str>::new(Duration::from_millis(50));

        cache.set("key1", "value1");
        assert_eq!(cache.get(&"key1"), Some("value1"));

        // Wait for expiry
        std::thread::sleep(Duration::from_millis(60));

        assert_eq!(cache.get(&"key1"), None);
    }

    #[test]
    fn test_ttl_cache_custom_ttl() {
        let cache = TtlCache::<&str, &str>::new(Duration::from_secs(3600));

        cache.set_with_ttl("key1", "value1", Duration::from_millis(50));
        assert_eq!(cache.get(&"key1"), Some("value1"));

        std::thread::sleep(Duration::from_millis(60));

        assert_eq!(cache.get(&"key1"), None);
    }

    #[test]
    fn test_ttl_cache_is_stale() {
        let cache = TtlCache::<&str, &str>::new(Duration::from_millis(50));

        cache.set("key1", "value1");
        assert!(!cache.is_stale(&"key1"));
        assert!(cache.is_stale(&"key2")); // Missing key is stale

        std::thread::sleep(Duration::from_millis(60));

        assert!(cache.is_stale(&"key1"));
    }

    #[test]
    fn test_ttl_cache_remove() {
        let cache = TtlCache::<&str, &str>::new(Duration::from_secs(3600));

        cache.set("key1", "value1");
        assert_eq!(cache.get(&"key1"), Some("value1"));

        cache.remove(&"key1");
        assert_eq!(cache.get(&"key1"), None);
    }

    #[test]
    fn test_ttl_cache_clear() {
        let cache = TtlCache::<&str, &str>::new(Duration::from_secs(3600));

        cache.set("key1", "value1");
        cache.set("key2", "value2");
        assert_eq!(cache.get(&"key1"), Some("value1"));

        cache.clear();
        assert_eq!(cache.get(&"key1"), None);
        assert_eq!(cache.get(&"key2"), None);
    }

    #[test]
    fn test_ttl_cache_string_values() {
        // Test with owned String values
        let cache = TtlCache::<String, String>::new(Duration::from_secs(3600));

        cache.set("key1".to_string(), "value1".to_string());
        assert_eq!(cache.get(&"key1".to_string()), Some("value1".to_string()));

        cache.remove(&"key1".to_string());
        assert_eq!(cache.get(&"key1".to_string()), None);
    }
}