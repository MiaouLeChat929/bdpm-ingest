# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build, Test, Run

```bash
# Release build (LTO + opt-level 3 configured)
cargo build --release

# Unit tests (homeopathy filter, dosage parser, HTML entities, API sorting, FTS5 sanitization)
cargo test --lib

# Integration tests (normalization, row counts, referential integrity, FTS5 search)
cargo test --test integration

# Server tests (HTTP endpoint smoke tests — search, drug detail, ATC, MITM)
cargo test --test server_integration

# All tests
cargo test

# Check without building
cargo check

# Lint
cargo clippy -- -D warnings

# Regenerate OpenAPI spec (commit after regenerating)
cargo run --release -- dump-open-api > openapi.yaml
```

Single test: `cargo test test_name_here --lib`

**IMPORTANT: always use `cargo run --release --` instead of `./target/release/bdpm-ingest`.** With a shared `CARGO_TARGET_DIR`, cargo hard-links the binary into `./target/release/`, but this link can go stale — the project-local binary won't update even after `cargo build --release` succeeds. `cargo run` always uses the fresh binary from the shared target. If you must use the binary directly, verify the timestamp with `stat target/release/bdpm-ingest | grep Modify` after building.

## CLI Commands

```bash
./target/release/bdpm-ingest import [--full] [--file Foo.txt]   # fetch + parse + normalize + import
./target/release/bdpm-ingest sync                                # dry-run: detect changed files, print plan
./target/release/bdpm-ingest dispo                               # sync only CIS_CIP_Dispo_Spec.txt
./target/release/bdpm-ingest fts-rebuild                          # rebuild FTS5 full-text search index
./target/release/bdpm-ingest check                               # BLAKE3 hash all files, report changes
./target/release/bdpm-ingest fetch                               # download all files, print hashes
./target/release/bdpm-ingest poll                                # HTML listing page date parse, detect changes
./target/release/bdpm-ingest stats                               # row counts per table
./target/release/bdpm-ingest logs [--limit N]                    # import history
./target/release/bdpm-ingest serve --addr 127.0.0.1:8080        # HTTP API server
./target/release/bdpm-ingest dump-open-api                       # output OpenAPI YAML to stdout
```

## Dev Workflow

**Always use `--full` during development.** The DB is recreated from scratch on each import — no incremental state, no migration runner. `init_db()` runs the consolidated `schema.sql` (all tables, indexes, constraints in one file) and creates FTS5 via `fts::create_fts_tables()`. There are no migration files; the schema is a single source of truth at `src/db/schema.sql`.

## Architecture

```
Pipeline: fetch → parse → normalize → dedup → import

Key types: BDPMFile (central routing enum, 10 files), NormalizedRow (table + values)
Key modules: download/ (fetch + state + listing), parse/ (TabParser, encoding detection),
             normalize/ (row transformers, dedup, price/date/HTML fields),
             import/ (orchestrator, bulk insert), db/ (SQLite + FTS5, consolidated schema.sql),
             api/ (axum HTTP server, search, drugs, ATC, availability),
             sync/ (change detection)
```

## Key Design Decisions

**BDPMFile** is the central routing type. Every normalizer and INSERT SQL is dispatched from it. The `target_table()` method maps each file to its DB table name; `schema()` returns field count and encoding.

**Windows-1252 encoding** — 7 of 10 files use Windows-1252, 2 use UTF-8, 1 uses Latin-1. The BDPM server returns no charset header. encoding_rs is wired via `std::fs::read()` + decode at file-open time in `TabParser::from_path()`.

**Trailing tab** — CIS_CIP_bdpm has a phantom trailing tab creating a 13th awk field (always empty). `strip_one_trailing_empty()` removes exactly 1 trailing empty, preserving middle empty fields (prices/reimb fields for non-commercialisé rows). Schema field_count: 12. Short rows (8 fields) are padded with empty strings by the >= half-threshold logic in `parse_file()`.

**TabParser multiline logic** — when a CIS-code line triggers record emission, the current line is pushed to buffer FIRST, then the previous buffer is emitted. Old code lost the triggering line on even positions (50% of rows discarded). On EOF, buffered record is flushed. Covered by 2 unit tests.

**Orphan FKs** — SMR/ASMR/GENER/safety_alerts reference withdrawn drugs. The `is_orphan` flag is set post-import via UPDATE. `INSERT OR REPLACE` for drugs preserves references.

**FTS5 standalone (no external content)** — FTS5 uses a standalone table (no `content='drugs'`). Triggers (`drugs_ai`, `drugs_ad`, `drugs_au`) handle all sync. Previous external-content approach broke because FTS5 column names (`name_clean`, `substance_name_clean`) didn't match `drugs` table columns. `rebuild_fts()` does DELETE+re-INSERT (not the `'rebuild'` command which requires content sync). FTS columns: `cis`, `name_raw`, `name`, `atc_code`, `form`, `lab_name`.

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

6. **Field count parity: normalizer VALUES must match INSERT placeholders** — When adding a new file normalizer or modifying an existing one, `normalize_row()` output `.values.len()` MUST equal `insert_sql()` placeholder count AND the `stmt.execute(rusqlite::params![...])` binding count. A mismatch panics at runtime. Always add a test case to `test_insert_sql_value_counts_match`. Debug: check normalizer values vec (does it drop any fields from raw input?) vs SQL column list vs params![] binding.

7. **`str::replace("&amp;", "&amp;")` is a no-op — check for self-replacing calls** — The Rust source file may render `&amp;`, `&lt;`, `&gt;` HTML entities as their character equivalents (`&`, `<`, `>`) in string literals, making `s.replace("&amp;", "&amp;")` compile and run as a silent no-op. When adding HTML entity decoding, verify the actual bytes with `xxd` or a hex dump, not just visual inspection. Clippy catches this with `no_effect_replace`.

8. **`Option<T>` + `filter_map` for ETL filtering** — The idiomatic Rust pattern for "transform or skip" at normalize time. `normalize_row() -> Option<NormalizedRow>` returning `None` for filtered rows, consumed via `filter_map()` in the import loop. For two-pass filtering (exclude CIS from file A based on file B), use a pre-scan `HashSet<String>`, not `Arc<Mutex<HashSet>>`.

9. **FTS5 column names must be independent of source table** — With `content=''`, FTS5 column names are arbitrary (no mapping to source table). With `content='table'`, FTS5 column names MUST match source table column names exactly or all queries fail with "no such column: T.X". Triggers insert by position, not name, so trigger column lists can use `new.any_column_name` regardless of FTS column names.

10. **SQLite dynamic typing: empty string is not NULL** — SQLite columns declared `REAL` accept any type. When `dosage_mg` is `None`, the binding `unwrap_or("")` inserts an empty string, not SQL NULL. Comparisons like `WHERE dosage_mg > 10000` match empty strings as text (truthy in some collations). Always filter by type first: `WHERE typeof(dosage_mg) = 'real' AND dosage_mg > N`.

## Parsing Safety Rule

**Every parsing bug or data incoherence fix MUST include a unit test that would have caught the bug.** This is non-negotiable — regression prevention is mandatory for all parser/normalizer changes.

- The test must use the EXACT raw input that triggered the bug, demonstrating the wrong output before fix and correct output after
- For edge case classes (e.g., "all dosage strings containing `M.U.I.`"), add at least representative samples from each sub-pattern
- Run `cargo test --lib` before committing to confirm the test fails on old code and passes on new code

