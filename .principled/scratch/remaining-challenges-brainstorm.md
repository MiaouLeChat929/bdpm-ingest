# Remaining Implementation Challenges — Brainstorm 2026-05-26

## 1. Encoding Phase 1.5
**Decision**: `std::fs::read()` + `encoding_rs` decode, keep sync.
~20 lines. Data is 1-33KB total — no performance concern.
Keep as sync, do NOT go async for this.

## 2. Bulk Insert Performance
**Decision**: Wire `optimize_for_bulk_insert` into `import_file` — highest leverage change.
`synchronous=OFF` + `cache_size=-64000` already defined but unused.
Realistic gain: 2-3x on CIS_COMPO (32K rows).

## 3. FTS5 Schema
**Decision**: One FTS5 virtual table, index name + lab_name + substance_name.
`content='drugs'` (external content FTS5, avoids DB corruption).
BM25 ranking — correct default, don't tune until latency is measurable.

## 4. Same Binary vs Separate Binary
**Decision**: Same binary, `Serve` subcommand added to Command enum.
Cargo.toml already has axum+tokio. Compile cost is one-time.
Runtime overhead is zero when not serving.

## 5. GitHub Actions Push-Back (releasing .db)
**Decision**: `gh release create` + .db as release asset.
NOT git-committed. Use `git lfs track "*.db"` if needed.
LFS free tier: 1GB — ~1.8GB/year for monthly updates, within limits.

## 6. Rusqlite Params Boilerplate
**Decision**: Keep explicit match arms — verbose but correct, zero-cost.
No macro needed at current scale (10 files, ~60 columns).
Revisit if files grow beyond ~20.
