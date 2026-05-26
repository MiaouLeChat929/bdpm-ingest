-- Migration 002: Fix SMR/ASMR CHECK constraints
-- SQLite can't ALTER CONSTRAINT; we use app-level validation instead.
-- These CREATE TABLE statements are placeholders that run inside rusqlite_migration's
-- own transaction. The actual fix is handled by dropping and recreating tables
-- in Rust code where we can manage the schema cache.

-- No-op placeholder — real fix is in db/mod.rs
-- Kept so migration numbering stays consecutive
