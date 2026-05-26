# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build, Test, Run

```bash
# Release build (LTO + opt-level 3 configured)
cargo build --release

# Unit tests only (22 tests in normalize, parse modules)
cargo test --lib

# All tests
cargo test

# Check without building
cargo check

# Lint
cargo clippy -- -D warnings
```

Single test: `cargo test test_name_here --lib`

## CLI Commands

```bash
./target/release/bdpm-ingest import [--full] [--file Foo.txt]   # fetch + parse + normalize + import
./target/release/bdpm-ingest check                                  # BLAKE3 hash all files, report changes
./target/release/bdpm-ingest fetch                                  # download all files, print hashes
./target/release/bdpm-ingest poll                                   # HTML listing page date parse, detect changes (no download)
./target/release/bdpm-ingest stats                                  # row counts per table
./target/release/bdpm-ingest logs [--limit N]                       # import history
```

## Architecture

```
Pipeline: fetch → parse → normalize → dedup → import

download/   manifest.rs  — BDPMFile enum (10 files), Encoding, FileSchema, download_path(), target_table()
             state.rs    — StateStore (BLAKE3 hash + size per file), JSON persisted
             listing.rs  — HTML listing page date parser (polling without downloading)
             fetcher.rs  — ureq HTTP client, 3-retry backoff, fetch_text() for HTML

parse/      tab.rs      — TabParser streaming iterator, multiline record handling (SMR/ASMR avis field)
             mod.rs     — parse_file(path, BDPMFile) → Vec<ValidatedRow>

normalize/  mod.rs     — normalize_row dispatcher per BDPMFile
             price.rs   — parse_price_cents (handles "1,466,29" → 146629, 2 commas)
             date.rs    — parse_date_ddmmYYYY, parse_date_YYYYMMDD (range 1900–2100)
             fields.rs  — strip_field, normalize_spaces, normalize_generic_type ("0"→"reference"…)
             html.rs    — strip_avis_html (HTML → plain text for SMR/ASMR avis)
             dedup.rs   — dedup_compo: key=(cis, substance_code, dosage), 4,780 duplicates removed

import/     mod.rs     — run_import orchestrator, insert_sql per table, ImportReport

db/        mod.rs     — init_db (WAL + FK_ON + migrations), optimize_for_bulk_insert, restore_normal_settings
             migrations/001_initial.sql — all 11 tables + CHECK constraints
```

## Key Design Decisions

**BDPMFile** is the central routing type. Every normalizer and INSERT SQL is dispatched from it. The `target_table()` method maps each file to its DB table name; `schema()` returns field count and encoding.

**Windows-1252 encoding** — 7 of 10 files use Windows-1252, 2 use UTF-8, 1 uses Latin-1. The BDPM server returns no charset header. encoding_rs is in Cargo.toml but not yet wired (Phase 1.5). The tab parser currently reads platform-default.

**Curly apostrophes** — byte 0x92 (Windows-1252 U+2019 RIGHT SINGLE QUOTATION MARK) appears in lab names. `normalize_apostrophes()` replaces `\u{2019}` and `\u{2018}` with straight `'` before INSERT. Handled in `normalize/mod.rs`.

**Trailing tab** — CIS_CIP_bdpm has a phantom 14th field (100% trailing empty). `strip_trailing_empty()` in `parse/mod.rs` removes empty trailing fields during parsing. CIS_CIP_Dispo_Spec has empty MIDDLE fields (not trailing) — different pattern.

**Orphan FKs** — SMR/ASMR/GENER reference withdrawn drugs. The `is_orphan` flag is set post-import via UPDATE. `INSERT OR REPLACE` for drugs preserves references.

**CIS_CIP_Dispo_Spec** — availability/stockout file, most frequently updated file (confirmed 19/05/2026). Polled via `bdpm-ingest poll` which parses embedded dates from the BDPM HTML listing page. The server provides no ETag, no Last-Modified, no Content-Length on TXT files.

## rusqlite Patterns (hard-won)

These will break silently if violated:

1. **`transaction()` needs `&mut Connection`** — every caller in the call chain must receive `&mut Connection`, not `&Connection`. If you see `cannot borrow as mutable`, trace back to the caller signature.

2. **`Vec<Option<String>>` does not implement `Params`** — `stmt.execute(params![row.values.as_slice()])` fails. Bind each column individually: `stmt.execute(rusqlite::params![v[0].as_ref().map(...).unwrap_or(""), ...])`.

3. **`CachedStatement` borrows `Transaction`** — `stmt` borrows `tx` for its lifetime. `tx.commit()` moves `tx` while `stmt` still borrows it → E0505. Fix: `drop(stmt)` before `tx.commit()`.

4. **`str::replace(old, new)` — both args must be `&str`, not `char`** — `s.replace('\u{2019}', '\'')` is invalid. Use `s.replace("\u{2019}", "'")`.

5. **`unwrap_or(ns())` where `ns` is a closure** — the closure return type influences `unwrap_or`'s type inference, causing type mismatch. Just use `unwrap_or("")` directly.
