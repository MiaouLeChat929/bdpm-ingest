# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build, Test, Run

```bash
# Release build (LTO + opt-level 3 configured)
cargo build --release

# Unit tests (154 tests across all modules — homeopathy filter, dosage parser, HTML entities, API sorting)
cargo test --lib

# Integration tests (34 tests against fresh DB with homeopathy excluded)
cargo test --test integration

# Server tests (19 HTTP endpoint smoke tests)
cargo test --test server_integration

# All tests
cargo test

# Check without building
cargo check

# Lint
cargo clippy -- -D warnings

# Regenerate OpenAPI spec (commit after regenerating)
./target/release/bdpm-ingest dump-open-api > openapi.yaml
```

Single test: `cargo test test_name_here --lib`

## CLI Commands

```bash
./target/release/bdpm-ingest import [--full] [--file Foo.txt]   # fetch + parse + normalize + import
./target/release/bdpm-ingest sync                                # dry-run: detect changed files, print plan
./target/release/bdpm-ingest dispo                               # sync only CIS_CIP_Dispo_Spec.txt
./target/release/bdpm-ingest check                               # BLAKE3 hash all files, report changes
./target/release/bdpm-ingest fetch                               # download all files, print hashes
./target/release/bdpm-ingest poll                                # HTML listing page date parse, detect changes
./target/release/bdpm-ingest stats                               # row counts per table
./target/release/bdpm-ingest logs [--limit N]                    # import history
./target/release/bdpm-ingest serve --addr 127.0.0.1:8080        # HTTP API server
./target/release/bdpm-ingest dump-open-api                       # output OpenAPI YAML to stdout
```

## Architecture

```
Pipeline: fetch → parse → normalize → dedup → import

download/   manifest.rs  — BDPMFile enum (10 files), Encoding, FileSchema, download_path(), target_table()
             state.rs    — StateStore (BLAKE3 hash + size per file), JSON persisted
             listing.rs  — HTML listing page date parser (polling without downloading)
             fetcher.rs  — ureq HTTP client, 3-retry backoff, fetch_text() for HTML

parse/      tab.rs      — TabParser streaming iterator, Windows-1252/UTF-8/Latin-1 via encoding_rs, multiline record handling (SMR/ASMR avis field). Strips UTF-8 BOM before encoding detection.
             mod.rs     — parse_file(path, BDPMFile) → Vec<NormalizedRow>

normalize/  mod.rs     — normalize_row dispatcher per BDPMFile. CIS_MITM drops drug_name (not stored in DB).
             price.rs   — parse_price_cents (handles "1,466,29" → 146629, 2 commas)
             date.rs    — parse_date_ddmmYYYY, parse_date_YYYYMMDD (range 1900–2100)
             fields.rs  — strip_field, normalize_spaces, normalize_generic_type ("0"→"reference"…)
             html.rs    — strip_avis_html (HTML → plain text), decode_html_entities (named + numeric entities), normalize_newlines
             dedup.rs   — dedup_compo: key=(cis, substance_code, dosage), 4,780 duplicates removed

import/     mod.rs     — run_import orchestrator, insert_sql per table, ImportReport

cache/     mod.rs     — TtlCache<K, V>, 6-hour default, Mutex<HashMap>

sync/       mod.rs     — SyncPlan, ChangeReason, detect_changes() [dry-run], run_sync(), run_dispo_sync()
                         All delegate to run_import() — no logic duplication.

db/        mod.rs     — init_db (WAL + FK_ON + migrations), optimize_for_bulk_insert, restore_normal_settings
             fts.rs     — FTS5 virtual table + sync triggers
             migrations/001_initial.sql — all 11 tables + CHECK constraints

api/       mod.rs     — AppState, run_server (axum), all routes wired, health endpoint (JSON: status/last_import/drug_count)
             search.rs  — GET /drugs FTS5 search endpoint
             drugs.rs   — GET /drugs/:cis with presentations + compositions
             safety.rs  — GET /drugs/:cis/safety stub endpoint
             groups.rs  — GET /generic-groups, /generic-groups/:id
             atc.rs     — GET /atc, /atc/:code (LIKE prefix for hierarchy)
             availability.rs — GET /availability?cis=&cip=
             openapi.rs — utoipa OpenApi struct, /openapi.json + /openapi.yaml endpoints

tests/     integration.rs — 34 tests (price, date, normalization, referential integrity, row counts, orphan FKs)
```

## Key Design Decisions

**BDPMFile** is the central routing type. Every normalizer and INSERT SQL is dispatched from it. The `target_table()` method maps each file to its DB table name; `schema()` returns field count and encoding.

**Windows-1252 encoding** — 7 of 10 files use Windows-1252, 2 use UTF-8, 1 uses Latin-1. The BDPM server returns no charset header. encoding_rs is wired via `std::fs::read()` + decode at file-open time in `TabParser::from_path()`.

**Trailing tab** — CIS_CIP_bdpm has a phantom trailing tab creating a 13th awk field (always empty). `strip_one_trailing_empty()` removes exactly 1 trailing empty, preserving middle empty fields (prices/reimb fields for non-commercialisé rows). Schema field_count: 12. Short rows (8 fields) are padded with empty strings by the >= half-threshold logic in `parse_file()`.

**TabParser multiline logic** — when a CIS-code line triggers record emission, the current line is pushed to buffer FIRST, then the previous buffer is emitted. Old code lost the triggering line on even positions (50% of rows discarded). On EOF, buffered record is flushed. Covered by 2 unit tests.

**Orphan FKs** — SMR/ASMR/GENER/safety_alerts reference withdrawn drugs. The `is_orphan` flag is set post-import via UPDATE. `INSERT OR REPLACE` for drugs preserves references.

**FTS5 external content** — `INSERT OR REPLACE` on the drugs table does implicit DELETE+INSERT. The `content_rowid='rowid'` mapping means FTS index entries use SQLite rowid as key. When REPLACE reassigns rowids, orphaned FTS entries can accumulate. The `rebuild_fts()` function in `fts.rs` is the safety net — call it after full imports. SQLite FTS5 external content tables do not support REPLACE conflict handling natively (converts to ABORT). Triggers fire correctly but rowid stability under REPLACE is the risk.

**Homeopathic drugs filtered at import** — Homeopathic drugs are excluded during normalization, not via post-import SQL.
- Filter: `normalize_row()` returns `Option<NormalizedRow>` — `None` when CIS_bdpm `procedure_type` (field 5) contains "ENREG HOM" (case-insensitive)
- Caught: 1,319 drugs (81.4% of all homeopathics) via official ANSM "Enreg homeo (Proc. Nat.)" classification
- Remaining: ~138 drugs with homeopathic dilutions (CH/DH) in compositions but standard procedure types — accepted residual (< 1% of DB)
- Architecture: `filter_map()` in import loop silently drops `None` rows; FK constraints in child tables naturally reject orphans
- API queries use `INNER JOIN` (not `LEFT JOIN`) for groups and availability to prevent surfacing orphan rows
- DB state: 14,529 drugs (from 15,848 raw), zero `ENREG HOM` entries in drugs table
- FTS5: rebuild after import; search returns zero results for homeopathic terms (BOIRON, dilution, etc.)

**CIS_bdpm field mapping** — The 12 tab-delimited fields are:
  f[0]=cis, f[1]=name, f[2]=form, f[3]=route, f[4]=auth_status, f[5]=procedure_type,
  f[6]=comm_status, f[7]=auth_date, f[8]=alert_type (85% empty),
  f[9]=eu_number (e.g. "EU/1/01/185"), f[10]=lab_name (e.g. " BOIRON"),
  f[11]=is_patent ("Oui"/"Non")
  WARNING: fields 9-11 are shifted: f[9]=eu_number, f[10]=lab_name, f[11]=is_patent (not sequential).

**CIS_CIP_Dispo_Spec** — availability/stockout file, most frequently updated file (confirmed 19/05/2026). Polled via `bdpm-ingest poll` which parses embedded dates from the BDPM HTML listing page. The server provides no ETag, no Last-Modified, no Content-Length on TXT files.

## rusqlite Patterns (hard-won)

These will break silently if violated:

1. **`transaction()` needs `&mut Connection`** — every caller in the call chain must receive `&mut Connection`, not `&Connection`. If you see `cannot borrow as mutable`, trace back to the caller signature.

2. **`Vec<Option<String>>` does not implement `Params`** — `stmt.execute(params![row.values.as_slice()])` fails. Bind each column individually: `stmt.execute(rusqlite::params![v[0].as_ref().map(...).unwrap_or(""), ...])`.

3. **`CachedStatement` borrows `Transaction`** — `stmt` borrows `tx` for its lifetime. `tx.commit()` moves `tx` while `stmt` still borrows it → E0505. Fix: `drop(stmt)` before `tx.commit()`.

4. **`str::replace(old, new)` — both args must be `&str`, not `char`** — `s.replace('\u{2019}', '\'')` is invalid. Use `s.replace("\u{2019}", "'")`.

5. **`unwrap_or(ns())` where `ns` is a closure** — the closure return type influences `unwrap_or`'s type inference, causing type mismatch. Just use `unwrap_or("")` directly.

6. **Field count parity: normalizer VALUES must match INSERT placeholders** — When adding a new file normalizer or modifying an existing one, `normalize_row()` output `.values.len()` MUST equal `insert_sql()` placeholder count AND the `stmt.execute(rusqlite::params![...])` binding count. A mismatch panics at runtime with "Wrong number of parameters passed to query".

7. **`str::replace("&amp;", "&amp;")` is a no-op — check for self-replacing calls** — The Rust source file may render `&amp;`, `&lt;`, `&gt;` HTML entities as their character equivalents (`&`, `<`, `>`) in string literals, making `s.replace("&amp;", "&amp;")` compile and run as a silent no-op. When adding HTML entity decoding, verify the actual bytes with `xxd` or a hex dump, not just visual inspection. Clippy catches this with `no_effect_replace`.

8. **`Option<T>` + `filter_map` for ETL filtering** — The idiomatic Rust pattern for "transform or skip" at normalize time. `normalize_row() -> Option<NormalizedRow>` returning `None` for filtered rows, consumed via `filter_map()` in the import loop. For two-pass filtering (exclude CIS from file A based on file B), use a pre-scan `HashSet<String>`, not `Arc<Mutex<HashSet>>`.

6. **Field count parity: normalizer VALUES must match INSERT placeholders** — When adding a new file normalizer or modifying an existing one, `normalize_row()` output `.values.len()` MUST equal `insert_sql()` placeholder count AND the `stmt.execute(rusqlite::params![...])` binding count. A mismatch panics at runtime with "Wrong number of parameters passed to query". Always add a test case to `test_insert_sql_value_counts_match` when modifying normalizers. To debug: check normalizer values vec (does it drop any fields from raw input?) vs SQL column list vs params![] binding.

7. **`str::replace("&amp;", "&amp;")` is a no-op — check for self-replacing calls** — The Rust source file may render `&amp;`, `&lt;`, `&gt;` HTML entities as their character equivalents (`&`, `<`, `>`) in string literals, making `s.replace("&amp;", "&amp;")` compile and run as a silent no-op. When adding HTML entity decoding, verify the actual bytes with `xxd` or a hex dump, not just visual inspection. Clippy catches this with `no_effect_replace`.

## Rust Compilation Shortcuts (from global CLAUDE.md)

Fix Rust compilation errors at the tool level without re-running cargo:

- **Quick error scan**: `cargo check 2>&1 | grep -E "^error"` — no context overhead
- **Missing trait impl**: grep for the actual trait in `~/.cargo/registry/src/`, find the concrete impl
- **Byte-level corruption** in source (e.g. `\x5c` literal bytes): Python binary replace:
  ```python
  data = open('src/file.rs', 'rb').read()
  data = data.replace(b'\x5c&', b'&')  # fix literal backslash+ampersand in tokens
  open('src/file.rs', 'wb').write(data)
  ```
  The Edit tool's escaping layer can produce literal `\x5c` bytes in `&`-prefixed Rust expressions. Byte-level fix is the only solution.
- **`?` on wrong type**: check if the return type changed (e.g. `init_db` returning `Connection` vs `Result<Connection>`)
- **`std::io::Read` not imported**: `use std::io::Read`
- **`IntoRustString` for Response`**: ureq 2.x → `into_reader()` + `read_to_end()`
- **`unwrap_or(ns())`** where `ns()` is a closure returning `&str`: the closure return type influences `unwrap_or`, causing type mismatch → just use `unwrap_or("")`
