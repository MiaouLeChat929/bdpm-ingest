# 02-01 — FTS5 Full-Text Search + axum API Scaffold

## Summary

Phase 2 API scaffold delivered. FTS5 search and axum server working.

### Files Created
- `src/db/fts.rs` — FTS5 virtual table + sync triggers
- `src/api/mod.rs` — AppState, run_server, all routes wired
- `src/api/search.rs` — GET /drugs FTS5 search endpoint
- `src/lib.rs` — Added `pub mod api`
- `src/main.rs` — Added `Serve` command variant
- `Cargo.toml` — Added axum, tokio, tower, tower-http, serde, serde_json

### Files Modified
- `src/db/mod.rs` — Calls `fts::create_fts_tables()` after migrations

### Verification
```
cargo build --release          ✓
drugs_fts table                ✓ 15,848 rows
cargo test --lib               ✓ 24/24 passed
GET /health                    ✓ "OK"
GET /drugs?q=DOLIPRANE        ✓ Returns results
```

### Post-commit fix (manual)
- `atc.rs` drug_count: changed `atc_code = ?1` → `atc_code LIKE ?1` for hierarchy prefix match
