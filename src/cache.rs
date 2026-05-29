//! In-memory TTL cache using moka.
//!
//! Thread-safe cache with time-to-idle expiration.

use moka::sync::Cache;
use std::time::Duration;

pub type SearchCache = Cache<String, Vec<crate::api::search::DrugSearchResult>>;

pub fn create_search_cache() -> SearchCache {
    Cache::builder()
        .max_capacity(1000)
        .time_to_idle(Duration::from_secs(300))
        .build()
}