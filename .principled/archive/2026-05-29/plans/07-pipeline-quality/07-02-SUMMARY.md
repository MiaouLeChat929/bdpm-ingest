# 07-02 SUMMARY — Quarantine table + CIP7 CHECK

## What was done

### Task 1: Quarantine table
- Added `quarantine` table to `src/db/schema.sql` (line 272)
- Columns: id, source_file, source_line, target_table, error_type, error_detail, raw_line, created_at
- 3 indexes: idx_quarantine_file, idx_quarantine_type, idx_quarantine_date
- Single generic table (not per-table) — error_type TEXT for extensibility

### Task 2: CIP7 CHECK constraint
- Added CHECK on `presentations.cip` in `src/db/schema.sql` (line 43)
- Pattern: `CHECK (cip IS NULL OR (LENGTH(cip) = 7 AND cip GLOB '[0-9][0-9][0-9][0-9][0-9][0-9][0-9]'))`
- Enforces exactly 7 ASCII digits at DB level

### Task 3: quarantine_row helper
- Added `quarantine_row()` function in `src/import/mod.rs` (line 18)
- Inserts: source_file, source_line, target_table, error_type, error_detail, raw_line
- Returns silently on failure (does not break import)

### Task 4: Unit tests
- Added `test_quarantine_row_insert` in `src/import/mod.rs`
- Verifies row insertion and error_type retrieval from quarantine table

## Verification
- `cargo test --lib`:177 passed
- `cargo clippy -- -D warnings`: clean
- `cargo run --release -- ingest`: completes, quarantine table exists
- `sqlite3 data/bdpm.db "SELECT COUNT(*) FROM quarantine"`: table exists (0 rows — no rejections yet)

## Files modified
- `src/db/schema.sql` — quarantine table + CIP7 CHECK
- `src/import/mod.rs` — quarantine_row helper + unit test
