# 03-01 SUMMARY — Sync Engine (absorbed into Phase 01)

## Note: Absorbed into Phase 01 Foundation

The sync engine was not built as a separate Phase 03. Instead, its core functionality was delivered as part of Phase 01:

- **BLAKE3 change detection**: `bdpm-ingest check` compares file hashes before fetching (Phase 01-01)
- **Full-table reload**: `bdpm-ingest ingest` always does a fresh full-table reload — no incremental delta possible with BDPM (confirmed by reference implementation at medicaments-api.giygas.dev)
- **Import log**: `bdpm-ingest logs` shows import history (Phase 01-07)
- **State store**: `StateStore` tracks file hashes and sizes across runs (Phase 01-01)

The design decisions from the archived 03-01 plan were applied:
- Drugs table uses INSERT OR REPLACE — withdrawn drugs are preserved so SMR/ASMR/GENER references are not lost
- All other tables are truncated before reload
- BLAKE3 change detection post-download
- Content-Length as optimization (skip GET if size unchanged)

## Why no separate Phase 03?

BDPM provides no timestamps and no ETag/Last-Modified headers on its TXT files. The sync pattern is always:
```
fetch → raw/*.txt → ingest → bdpm.db (always fresh)
```

There is no row-level delta — only file-level change detection. The "sync engine" is therefore just the ingest pipeline itself with a state store. This was delivered in Phase 01.

## Archived plans

Archived at `archive/2026-05-26/plans/03-sync/`: 03-01-PLAN.md (BLAKE3 + truncate+reload), 03-02-PLAN.md (weekly dispo sync).

## Verification
- `bdpm-ingest check` reports unchanged files correctly
- `bdpm-ingest ingest` rebuilds from scratch each run
- `bdpm-ingest logs --limit N` shows import history

## Files
- `src/download/manifest.rs` — BDPMFile, StateStore
- `src/import/mod.rs` — full ingest pipeline
