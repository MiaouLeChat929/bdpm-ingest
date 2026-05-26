//! HTTP fetcher with retry backoff for the BDPM server.
//!
//! Retry strategy: 3 attempts with 5s → 10s → 30s backoff.
//! User-Agent set to identify the client to the BDPM server.

use std::io::Read;
use std::path::Path;
use std::time::Duration;

/// BDPM file downloader using the synchronous `ureq` HTTP client.
#[derive(Debug)]
pub struct Fetcher {
    user_agent: String,
}

impl Fetcher {
    /// Create a new fetcher with the default client identification.
    pub fn new() -> Self {
        Self {
            user_agent: "bdpm-ingest/0.1 (Rust)".to_string(),
        }
    }

    /// Fetch a file from `url` and save it to `dest_dir/{filename}`.
    ///
    /// Returns the raw bytes on success so callers can compute BLAKE3 hash.
    /// Retries up to 3 times with exponential backoff (5s, 10s, 30s).
    ///
    /// # Errors
    /// Returns an error if all 3 attempts fail.
    pub fn fetch(&self, url: &str, dest_dir: &Path) -> anyhow::Result<Vec<u8>> {
        // Extract filename from URL for disk destination.
        let filename = url.split('/').last().unwrap_or("unknown");
        let dest = dest_dir.join(filename);

        let backoffs = [5, 10, 30];

        for attempt in 1..=3 {
            match self.fetch_once(url) {
                Ok((bytes, content_len)) => {
                    // Save to disk after successful fetch.
                    std::fs::write(&dest, &bytes)?;
                    tracing::info!(
                        "Downloaded {} ({} bytes) → {}",
                        filename,
                        content_len,
                        dest.display()
                    );
                    return Ok(bytes);
                }
                Err(e) => {
                    if attempt < 3 {
                        let backoff = backoffs[attempt - 1];
                        tracing::warn!(
                            "Fetch attempt {}/3 failed for {}: {}. Retrying in {}s.",
                            attempt,
                            filename,
                            e,
                            backoff
                        );
                        std::thread::sleep(Duration::from_secs(backoff));
                    } else {
                        anyhow::bail!(
                            "Failed to fetch {} after 3 attempts: {}",
                            url,
                            e
                        );
                    }
                }
            }
        }

        // Unreachable — loop always returns or errors.
        unreachable!()
    }

    /// Single HTTP GET attempt without retry.
    fn fetch_once(&self, url: &str) -> anyhow::Result<(Vec<u8>, usize)> {
        let agent = ureq::Agent::new();

        let response = agent
            .get(url)
            .set("User-Agent", &self.user_agent)
            // Disable Accept-Encoding so we get raw bytes, not gzip
            .set("Accept-Encoding", "identity")
            .timeout(Duration::from_secs(60))
            .call()?;

        // Content-Length from header (0 if not present).
        let content_len: usize = response
            .header("Content-Length")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        // Consume the response body as raw bytes.
        let mut reader = response.into_reader();
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;

        Ok((bytes, content_len))
    }

    /// Fetch a URL and return the response body as a UTF-8 string.
    ///
    /// Used for HTML listing pages (no file save, just text extraction).
    /// Same retry/backoff strategy as `fetch`.
    pub fn fetch_text(&self, url: &str) -> anyhow::Result<String> {
        let backoffs = [5, 10, 30];

        for attempt in 1..=3 {
            match self.fetch_raw(url) {
                Ok(bytes) => {
                    // The listing page is served as HTML — decode assuming UTF-8
                    // (French accents are in UTF-8 on the listing page).
                    match String::from_utf8(bytes) {
                        Ok(text) => return Ok(text),
                        Err(e) => {
                            // Fall back to Latin-1 if UTF-8 fails (some servers mislabel).
                            let bytes = e.into_bytes();
                            let latin1_lossy = String::from_utf8_lossy(&bytes);
                            tracing::debug!("Listing page decoded as Latin-1 (UTF-8 failed)");
                            return Ok(latin1_lossy.into_owned());
                        }
                    }
                }
                Err(e) => {
                    if attempt < 3 {
                        let backoff = backoffs[attempt - 1];
                        tracing::warn!(
                            "fetch_text attempt {}/3 failed for {}: {}. Retrying in {}s.",
                            attempt, url, e, backoff
                        );
                        std::thread::sleep(Duration::from_secs(backoff));
                    } else {
                        anyhow::bail!("fetch_text failed for {} after 3 attempts: {}", url, e);
                    }
                }
            }
        }
        unreachable!()
    }

    /// Raw fetch returning bytes (used internally).
    fn fetch_raw(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        let agent = ureq::Agent::new();
        let response = agent
            .get(url)
            .set("User-Agent", &self.user_agent)
            .set("Accept-Encoding", "identity")
            .timeout(Duration::from_secs(30))
            .call()?;
        let mut reader = response.into_reader();
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

impl Default for Fetcher {
    fn default() -> Self {
        Self::new()
    }
}