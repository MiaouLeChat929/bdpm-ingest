//! Persistent state store for BLAKE3 hash tracking across sync runs.
//!
//! On first run: no stored state, everything needs update.
//! On subsequent runs: compare BLAKE3 hash to detect file changes.
//!
//! Stored as JSON at `$DATA_DIR/state.json`. Never exposes raw file bytes.

use std::collections::HashMap;
use std::path::Path;

use crate::download::BDPMFile;

/// Per-file download state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileState {
    /// BLAKE3 hash of the downloaded file content (40-char hex).
    pub content_hash: String,
    /// File size in bytes from Content-Length header.
    pub size_bytes: u64,
    /// ISO-8601 timestamp when the file was last downloaded.
    pub downloaded_at: String,
}

/// Root of the persisted state file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct StateStore {
    /// Per-filename state entries.
    #[serde(rename = "files")]
    pub files: HashMap<String, FileState>,
}

impl StateStore {
    /// Load an existing state file or return a fresh empty store.
    pub fn load_or_create(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            tracing::debug!("No existing state at {}, starting fresh", path.display());
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        match serde_json::from_str::<StateStore>(&content) {
            Ok(state) => {
                tracing::debug!(
                    "Loaded state: {} files tracked",
                    state.files.len()
                );
                Ok(state)
            }
            Err(e) => {
                tracing::warn!("Corrupt state file at {}: {}. Resetting.", path.display(), e);
                Ok(Self::default())
            }
        }
    }

    /// Persist the current state to disk (pretty-printed JSON).
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        tracing::debug!("State saved to {}", path.display());
        Ok(())
    }

    /// Returns true if the file needs downloading or re-processing.
    ///
    /// Reasons to return true:
    /// - First run (no stored state for this file)
    /// - Hash mismatch (content changed on server)
    /// - Size mismatch (incomplete previous download)
    pub fn needs_update(&self, file: &BDPMFile, hash: &str, size: u64) -> bool {
        match self.files.get(file.filename()) {
            Some(prev) => prev.content_hash != hash || prev.size_bytes != size,
            None => true,
        }
    }

    /// Record that a file was successfully downloaded with the given hash and size.
    pub fn mark_updated(&mut self, file: &BDPMFile, hash: &str, size: u64) {
        self.files.insert(
            file.filename().to_string(),
            FileState {
                content_hash: hash.to_string(),
                size_bytes: size,
                downloaded_at: time::OffsetDateTime::now_utc().to_string(),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn temp_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    fn temp_state_path(tmpdir: &TempDir) -> PathBuf {
        tmpdir.path().join("state.json")
    }

    // Test 1: load_or_create returns default when no file exists
    #[test]
    fn test_load_or_create_no_file() {
        let tmpdir = temp_dir();
        let path = temp_state_path(&tmpdir);

        let store = StateStore::load_or_create(&path).unwrap();
        assert!(store.files.is_empty());
    }

    // Test 2: load_or_create resets to default on corrupt JSON
    #[test]
    fn test_load_or_create_corrupt_json() {
        let tmpdir = temp_dir();
        let path = temp_state_path(&tmpdir);

        // Write corrupt JSON
        std::fs::write(&path, "not valid json {{{{").unwrap();

        let store = StateStore::load_or_create(&path).unwrap();
        assert!(store.files.is_empty());
    }

    // Test 3: save then load produces identical state
    #[test]
    fn test_save_and_load_roundtrip() {
        let tmpdir = temp_dir();
        let path = temp_state_path(&tmpdir);

        let mut store = StateStore::load_or_create(&path).unwrap();
        store.mark_updated(&BDPMFile::CIS_bdpm, "abc123", 1024);

        store.save(&path).unwrap();

        let loaded = StateStore::load_or_create(&path).unwrap();
        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.files["CIS_bdpm.txt"].content_hash, "abc123");
        assert_eq!(loaded.files["CIS_bdpm.txt"].size_bytes, 1024);
    }

    // Test 4: needs_update returns true when no prior state
    #[test]
    fn test_needs_update_no_prior_state() {
        let store = StateStore::default();
        assert!(store.needs_update(&BDPMFile::CIS_bdpm, "abc123", 1024));
    }

    // Test 5: needs_update returns true when hash differs
    #[test]
    fn test_needs_update_hash_changed() {
        let mut store = StateStore::default();
        store.mark_updated(&BDPMFile::CIS_bdpm, "old_hash", 1024);

        assert!(store.needs_update(&BDPMFile::CIS_bdpm, "new_hash", 1024));
    }

    // Test 6: needs_update returns true when size differs
    #[test]
    fn test_needs_update_size_changed() {
        let mut store = StateStore::default();
        store.mark_updated(&BDPMFile::CIS_bdpm, "abc123", 1024);

        assert!(store.needs_update(&BDPMFile::CIS_bdpm, "abc123", 2048));
    }

    // Test 7: needs_update returns false when hash and size match
    #[test]
    fn test_needs_update_identical() {
        let mut store = StateStore::default();
        store.mark_updated(&BDPMFile::CIS_bdpm, "abc123", 1024);

        assert!(!store.needs_update(&BDPMFile::CIS_bdpm, "abc123", 1024));
    }

    // Test 8: mark_updated prevents needs_update from returning true
    #[test]
    fn test_mark_updated_prevents_needs_update() {
        let mut store = StateStore::default();
        let hash = "final_hash";
        let size = 4096;

        store.mark_updated(&BDPMFile::CIS_bdpm, hash, size);
        assert!(!store.needs_update(&BDPMFile::CIS_bdpm, hash, size));
    }
}