//! Simple per-IP rate limiting middleware.
//!
//! Tracks request counts per IP in a sliding 60-second window.
//! Returns 429 Too Many Requests when limit exceeded.

use axum::{
    body::Body,
    extract::connect_info::ConnectInfo,
    http::{Request, Response, StatusCode},
    middleware::Next,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

/// Configuration constants.
const RATE_LIMIT: u32 = 60;
const WINDOW_DURATION: Duration = Duration::from_secs(60);

struct RateLimitEntry {
    window_start: Instant,
    count: u32,
}

static RATE_LIMIT_STORE: OnceLock<RateLimitStore> = OnceLock::new();

/// Get the global rate limit store instance.
pub fn get_store() -> &'static RateLimitStore {
    RATE_LIMIT_STORE.get_or_init(RateLimitStore::new)
}

/// Shared rate limit store - cloneable and stored globally.
#[derive(Clone)]
pub struct RateLimitStore {
    entries: Arc<Mutex<HashMap<String, RateLimitEntry>>>,
}

impl RateLimitStore {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check and update rate limit for the given IP.
    /// Returns Ok(()) if allowed, or Err(retry_after_secs) if limited.
    pub fn check(&self, ip: &str) -> Result<(), u64> {
        let mut state = self.entries.lock().unwrap();
        let now = Instant::now();

        match state.entry(ip.to_string()) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                let rec = e.get_mut();
                if now.duration_since(rec.window_start) >= WINDOW_DURATION {
                    rec.window_start = now;
                    rec.count = 1;
                    Ok(())
                } else {
                    rec.count += 1;
                    if rec.count > RATE_LIMIT {
                        let retry_after = WINDOW_DURATION
                            .saturating_sub(now.duration_since(rec.window_start))
                            .as_secs();
                        Err(retry_after)
                    } else {
                        Ok(())
                    }
                }
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(RateLimitEntry {
                    window_start: now,
                    count: 1,
                });
                Ok(())
            }
        }
    }
}

impl Default for RateLimitStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Rate limiting filter for use with axum's Router::layer().
pub async fn rate_limit_filter(
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    // Get client IP from ConnectInfo extension, or fall back to headers
    let client_ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .or_else(|| {
            // Fall back to X-Forwarded-For or X-Real-IP headers
            request
                .headers()
                .get("x-forwarded-for")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.split(',').next())
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(String::from)
                .or_else(|| {
                    request
                        .headers()
                        .get("x-real-ip")
                        .and_then(|h| h.to_str().ok())
                        .map(String::from)
                })
        })
        .unwrap_or_else(|| "unknown".to_string());

    let store = get_store();

    if let Err(retry_after) = store.check(&client_ip) {
        return Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("Content-Type", "application/json")
            .header("Retry-After", retry_after.to_string())
            .body(Body::from(r#"{"error":"Too Many Requests","message":"Rate limit exceeded. Try again later."}"#))
            .unwrap();
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_allows_initial_request() {
        let store = RateLimitStore::new();
        let result = store.check("192.168.1.1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_rate_limit_tracks_multiple_requests() {
        let store = RateLimitStore::new();
        let ip = "192.168.1.2";

        for _ in 0..RATE_LIMIT - 1 {
            assert!(store.check(ip).is_ok());
        }

        // 60th request should be allowed
        assert!(store.check(ip).is_ok());

        // 61st request should be blocked
        assert!(store.check(ip).is_err());
    }

    #[test]
    fn test_rate_limit_separate_ips() {
        let store = RateLimitStore::new();

        // IP 1 makes 60 requests - should be blocked at 61st
        for _ in 0..RATE_LIMIT {
            assert!(store.check("192.168.1.1").is_ok());
        }
        assert!(store.check("192.168.1.1").is_err());

        // IP 2 should still be allowed
        assert!(store.check("192.168.1.2").is_ok());
    }
}
