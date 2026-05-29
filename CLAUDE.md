# CLAUDE.md — bdpm-ingest

Ingest pipeline for the French public drug database (BDPM). Downloads TSV files from ansm.sante.fr, normalizes into SQLite, exposes via HTTP API.

## Philosophy

**The DB is a pure output. Always rebuilt from scratch. No incremental state, no migration runner.**

```
fetch → raw/*.txt → ingest → bdpm.db (always fresh)
```

Raw files are the cache. No state.json. No BLAKE3 tracking.

## Build & Test

```bash
cargo build --release          # optimized binary (LTO enabled)
cargo test --lib              # unit tests only
cargo test --test integration # integration tests
cargo clippy -- -D warnings    # lint
```

**Use `cargo run --release --` instead of invoking the binary directly.** Shared CARGO_TARGET_DIR can cause stale hard-links.

Single test: `cargo test test_name_here --lib`

## CLI Commands

| Command | Description |
|---------|-------------|
| `fetch` | Download all 10 BDPM files to `raw/` |
| `ingest` | Full rebuild: drop/create DB, import from `raw/`, build FTS5 |
| `serve --addr 127.0.0.1:8080` | HTTP API server (read-only) |
| `poll` | Fetch listing page, print per-file dates, exit |
| `stats` | Row counts per table |
| `logs [--limit N]` | Import history |
| `dump-open-api` | Output OpenAPI YAML |

**Always `ingest` — never `check` or `sync`.** No `--full` or `--file` flags. Import is always full.

## Data Flow

```
fetch          → raw/*.txt (Windows-1252 TSV files)
    ↓
ingest         → bdpm.db (always fresh)
    ├─ DROP + CREATE FTS5 (at start)
    ├─ MITM before drugs (populate atc_code inline)
    └─ triggers sync FTS5 on INSERT
```

Import order matters: MITM must complete before drugs for `drugs.atc_code` to be populated before FTS5 indexing.

## Key Design Decisions

**Homeopathic drugs filtered at import.** `normalize_row()` returns `Option<NormalizedRow>` — `None` for procedure type containing `ENREG HOM` (case-insensitive). Filtered via `filter_map()` in the import loop. Child tables reject orphans via FK constraints.

**FTS5 standalone triggers (no external content).** `DROP TABLE IF EXISTS + CREATE VIRTUAL TABLE` at ingest start. Triggers use explicit `DELETE` + `INSERT`. Not `'rebuild'` command or `content=` mode.

**Windows-1252 encoding.** 7 of 10 files use Windows-1252, 2 use UTF-8, 1 uses Latin-1. BDPM server returns no charset header. Decode at file-open time in `TabParser::from_path()`.

**CIS_bdpm phantom trailing tab.** 13th field is always empty. `strip_one_trailing_empty()` removes exactly 1 trailing empty, preserving middle empty fields (prices/reimb for non-commercialisé rows). Schema field_count: 12.

**HTML entity decoding.** French accented named entities (`&eacute;`, `&oelig;`, etc.) decoded during normalization.

## rusqlite Patterns

These will break silently if violated:

1. **`transaction()` needs `&mut Connection`** — trace back to caller signature if you see `cannot borrow as mutable`.

2. **`Vec<Option<String>>` does not implement `Params`** — bind each column individually via `params![v[0].as_ref().map(...).unwrap_or(""), ...]`.

3. **`CachedStatement` borrows `Transaction`** — `drop(stmt)` before `tx.commit()` to avoid E0505.

4. **`str::replace` — both args must be `&str`** — `s.replace("\u{2019}", "'")` not `s.replace('\u{2019}', '\'')`.

5. **`unwrap_or(ns())` where `ns` is a closure** — type inference issue. Use `unwrap_or("")` directly.

6. **Field count parity** — `normalize_row()` output `.values.len()` MUST match INSERT placeholder count AND params![] binding count. Mismatch panics at runtime. Add test to `test_insert_sql_value_counts_match`.

7. **`str::replace("&amp;", "&amp;")` is a no-op** — verify actual bytes with `xxd` if adding HTML entity decoding. Clippy catches this.

8. **`Option<T>` + `filter_map` for ETL filtering** — idiomatic pattern for "transform or skip". Use `HashSet<String>` pre-scan for two-pass filtering, not `Arc<Mutex<HashSet>>`.

9. **FTS5 column names independent of source table** — With `content=''`, FTS5 column names are arbitrary. Triggers insert by position, not name.

10. **SQLite dynamic typing: empty string is not NULL** — Always filter by type first: `WHERE typeof(dosage_mg) = 'real' AND dosage_mg > N`.

## Parsing Safety Rule

**Every parsing bug or data incoherence fix MUST include a unit test that would have caught the bug.**

- Use the EXACT raw input that triggered the bug
- Test must fail on old code, pass on new code
- For edge case classes, add representative samples from each sub-pattern

## Known Constraints

- BDPM server provides no ETag, no Last-Modified, no Content-Length on TXT files
- CIS_CIP_Dispo_Spec is the most frequently updated file
- BDPMFile enum is the central routing type — all normalizers and INSERT SQL dispatch from it