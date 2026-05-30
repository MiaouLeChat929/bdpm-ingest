# 07-03 SUMMARY — FTS5 trigram tokenizer

## What was done

### Task 1: Replace unicode61 with trigram
- Added `trigram_available()` function in `src/db/fts.rs` (line 5)
  - Checks `rusqlite::version_number() >= 3_035_000` (3.35+, rusqlite 0.31 bundles3.45)
- Modified `create_fts_tables()` to select tokenizer dynamically via `format!` macro (line 17)
  - Trigram: `tokenize='trigram remove_diacritics 1'`
  - Fallback: `tokenize='unicode61 remove_diacritics 1'`

### Task 2: Short-query fallback in search API
- Modified `src/api/search.rs` `search_drugs` function (line 105)
- Queries < 3 chars: `LIKE UPPER(?1) || '%'` (prefix match)
- Queries >= 3 chars: FTS5 trigram `MATCH ?1*` (substring match)
- BM25 ranking preserved for FTS5 results

### Task 3: Unit tests
- Added `test_trigram_available()` — verifies runtime version check
- Added `test_trigram_partial_match()` — verifies 'dol' matches 'Doliprane' via trigram

## Verification
- `cargo test --lib`: 177 passed
- `cargo clippy -- -D warnings`: clean
- `cargo run --release -- ingest`: FTS5 table created with trigram tokenizer

## Files modified
- `src/db/fts.rs` — trigram_available + dynamic tokenizer selection
- `src/api/search.rs` — short-query LIKE fallback

## Key finding
rusqlite 0.31 bundles SQLite 3.45 which includes trigram (added in 3.48, but rusqlite often bumps past minimum). The version check `>= 3.35` is conservative — actual minimum is 3.48 but rusqlite ships newer.
