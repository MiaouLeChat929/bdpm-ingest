# 07-01 SUMMARY — PRAGMA optimize + orphan detection + FK disable

## What was done

### Task 1: PRAGMA optimize post-import
- Added `PRAGMA optimize=0x10002` at the end of `run_ingest` in `src/import/mod.rs` (line 299)
- Added `PRAGMA optimize=0x10002` to `restore_normal_settings` in `src/db/mod.rs`
- `0x10002` = auto-mode (`0x02`) + force-scan-all-tables (`0x10000`) — correct for a fresh-import DB with no query history

### Task 2: Comprehensive orphan detection
- Added `check_all_orphans(conn)` function in `src/import/mod.rs` (line 102)
- Checks 8 child tables using NOT EXISTS pattern: presentations, compositions, generic_groups, prescription_rules, prescription_flags, availability, safety_alerts, mitm
- Called at end of `run_ingest` before PRAGMA optimize
- Logs WARN for orphan counts > 0, INFO for 0

### Task 3 (follow-up): FK disable during bulk load
- Added `PRAGMA foreign_keys = OFF` before the import transaction in `import_file`
- Added `PRAGMA foreign_keys = ON` after commit + quarantine inserts
- Eliminates FK overhead per row during INSERT; orphan validation runs post-import via `check_all_orphans`
- Fixes intermittent `FOREIGN KEY constraint failed` errors on CIS_bdpm/CIS_CPD

## Verification
- `cargo test --lib`: 177 passed
- `cargo clippy -- -D warnings`: clean
- `cargo build --release`: clean

## Files modified
- `src/import/mod.rs` — FK disable, check_all_orphans, quarantine wiring
- `src/db/mod.rs` — PRAGMA optimize added to restore_normal_settings
