# Architecture & Pipeline Analysis: External Reviews vs. Our Plan

> Analysis date: 26 mai 2026
> Sources: 7 external review Markdown files in /external_review/

---

## ARCHITECTURE CONFLICTS

### Conflict 1: Single-Crate vs. 5-Crate Workspace

**External reviews recommend**: A 5-crate workspace architecture:
```
bdpm-core    — shared types, enums, traits, config
bdpm-fetch   — HTTP download, SHA-256, archival
bdpm-parse   — decoding, TSV split, normalization
bdpm-validate — semantic checks, referential integrity, regression detection
bdpm-db      — SQLite, migrations, insertion, logging
```

**Our plan**: Single-crate, no workspace (solo dev, 11 source files, ~150K rows).

**Review reasoning** (from `06_architecture_pipeline_rust.md` lines 7–18):
> "L'architecture se décompose en 5 crates Rust organisés en pipeline séquentiel, chacun avec une responsabilité unique et une interface claire. Cette décomposition permet le développement incrémental, les tests unitaires isolés et la compilation parallèle."

Additionally, `07_roadmap_implementation.md` section 7.1 lists Phase 1 as:
> "[ ] Initialiser le workspace Cargo avec les 5 crates"

**Assessment**: This is the single largest architectural conflict. The 5-crate approach adds complexity (separate Cargo.toml per crate, cross-crate dependencies via `path = "../..."`). For a solo developer with a contained 11-file scope, the overhead may outweigh the benefits. However, if the codebase grows beyond the initial scope, the modularity pays off.

---

### Conflict 2: Async (tokio/reqwest) vs. Sync (ureq/rouille)

**External reviews recommend**: Full async stack throughout:
```toml
# 06_architecture_rust.md lines 610–613
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
```
And throughout the code: `async fn fetch_file(...)`, `tokio::time::sleep()`, `client.get(url).send().await`.

**Our plan**: Pure sync stack — `ureq` + `rusqlite` + `rouille` with NO tokio/async.

**Review reasoning**: Not explicitly stated, but the async pattern is used uniformly across all modules (fetcher, orchestrator, download with retry). This is a pragmatic choice for I/O-bound pipelines with concurrent network operations (downloading 11 files in sequence).

**Assessment**: For a single-user database importer with sequential file downloads (5s delay between files), the async complexity (tokio runtime, async traits, `.await` throughout) likely exceeds its value. However, if we add web API serving (rouille is sync-only), combining async HTTP fetch with sync rouille requires a hybrid approach — or choosing tokio+axum for the API layer.

---

## HASH FUNCTION

### External Reviews: SHA-256

**Used everywhere** — file-level and per-record:

```rust
// 04_strategie_mise_a_jour.md line 175
use sha2::{Sha256, Digest};
fn compute_file_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

// 06_architecture_rust.md line 27
sha2 = "0.10"

// 06_architecture_pipeline_rust.md lines 189–201 (bdpm-fetch dependencies)
sha2 = "0.10"
```

Per-record hashing also proposed in `05_schema_sqlite.md` (line 318):
```sql
UPDATE cis_specialites
SET denomination = ?, forme_pharmaceutique = ?, ..., _import_id = ?
WHERE code_cis = ? AND content_hash != ?;
```

Plus a `_source_hash` column on table `specialites` (`03_schema_sqlite.md` line 48):
```sql
_source_hash TEXT  -- Hash SHA256 de la ligne source
```

### Our Plan: BLAKE3 (file-level only)

**Assessment**: BLAKE3 is 4–10x faster than SHA-256 for bulk hashing (our ~22MB total). This is a genuine performance advantage for our use case. The trade-off: SHA-256 is more ubiquitous, better studied for cryptographic use (though we have no cryptographic requirements here), and is what the review architecture chose for consistency.

**Verdict**: BLAKE3 is appropriate and faster for our non-cryptographic content-identity use case. The reviews' SHA-256 choice reflects a conservative defaults approach but is not necessarily better for this workload.

---

## PIPELINE STAGES

### External Reviews: 5 Stages

```
FETCH → DECODE → PARSE → TRANSFORM → LOAD
```

From `05_pipeline_transformation.md` lines 11–17:
```
FETCH   : Télécharger les fichiers sources
DECODE  : Détecter et convertir l'encodage
PARSE   : Découper les lignes en champs
TRANSFORM: Normaliser les données (dates, etc.)
LOAD    : Insérer en base SQLite
```

### Our Plan: 6 Stages

```
Fetch → Decode → Parse → Validate → Normalize → Import
```

**Key difference**: The reviews include `TRANSFORM` (field-level normalization — dates, prices, HTML cleaning) inside the same stage as parsing, then merge it into LOAD. Our plan separates `Validate` and `Normalize` as distinct stages. The reviews do NOT have a dedicated `Validate` step in their primary pipeline diagram — validation is implied within each stage via `Result`-based error handling (line 204–209, parsing skips malformed lines).

**Verdict**: Functional alignment is high. The stage naming differs but the operations are equivalent. Our split of `Validate` is marginally more explicit.

---

## SQLITE TUNING

### External Reviews (unanimous across files):

```sql
-- 05_schema_sqlite.md lines 287–300
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA cache_size = -64000;  -- 64 MB
PRAGMA temp_store = MEMORY;
PRAGMA foreign_keys = OFF;   -- Allow orphan references
```

Also from `05_pipeline_transformation.md` lines 413–426:
```rust
fn optimize_for_bulk_insert(conn: &Connection) {
    conn.execute_batch("
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = OFF;
        PRAGMA cache_size = -64000;
        PRAGMA temp_store = MEMORY;
    ").unwrap();
}
// After import
fn restore_normal_settings(conn: &Connection) {
    conn.execute_batch("
        PRAGMA synchronous = NORMAL;
        PRAGMA cache_size = -2000;
    ").unwrap();
}
```

**Notable**: The reviews temporarily set `synchronous = OFF` during bulk insert (no disk flush, fastest writes), then restore to `NORMAL` after. This is an additional optimization we did not include in our plan.

### Our Plan: WAL + NORMAL + 64MB cache (same as reviews)

**Verdict**: Strong alignment. The `synchronous = OFF` during bulk insert + restore is a good pattern we should add.

---

## MIGRATION STRATEGY

### External Reviews: `refinery` (SQL-based versioned migrations)

From `06_architecture_pipeline_rust.md` lines 398–419:
```toml
// bdpm-db dependencies
rusqlite = { version = "0.31", features = ["bundled"] }
refinery = { version = "0.8", features = ["rusqlite"] }
```

Migration files in named `migrations/`:
```
migrations/
├── V001__initial_schema.sql
├── V002__add_fulltext_search.sql
└── V003__add_content_hash.sql
```

Also from `07_roadmap_implementation.md` Phase 3 (line 84):
> "[ ] Définir les migrations SQL avec refinery (V001__initial_schema.sql)"

### Our Plan: `rusqlite_migration` (Rust-based embedded migrations)

**Verdict**: Two different migration approaches with the same outcome. The reviews prefer SQL files + `refinery` (migrations are explicit, auditable, diff-friendly). We prefer `rusqlite_migration` (migrations embedded in Rust, versioned via integer sequence). No functional conflict — just implementation style.

---

## CHANGE DETECTION

### External Reviews: SHA-256 + Web Scraping for date

From `04_strategie_mise_a_jour.md` lines 91–98:
> La stratégie repose sur deux piliers :
> 1. Scraper la date de dernière mise à jour affichée sur la page de téléchargement
> 2. Calculer le hash SHA-256 de chaque fichier téléchargé et le comparer à la version précédente

Full architecture diagram (lines 101–139) with 3-step detection:
1. Scrape date from page (lightweight HTML GET)
2. If changed → download all files, compute SHA-256
3. Compare hashes to decide what to import

Additionally, `03_schema_sqlite.md` proposes storing `_source_hash` per table row (line 322–329):
> Pour détecter les modifications, on compare le hash du contenu de chaque enregistrement

### Our Plan: BLAKE3 only (file-level hash per import run)

**Assessment**: Our approach is simpler and faster (BLAKE3 hash vs. SHA-256 scraping + hashing). We skip the web-page scraping step. If the BDPM page is the only signal, web scraping is valuable; but per-file hash comparison is sufficient for our incremental update use case.

**Verdict**: Our approach is sufficient. The web-scraping signal adds complexity for marginal benefit unless the target server is slow or bandwidth-constrained.

---

## IMPORT STRATEGY

### External Reviews: DELETE + INSERT + Soft Delete (`_is_active`)

From `05_schema_sqlite.md` lines 304–349:

**Phase 1** (Insert new records):
```sql
INSERT INTO cis_specialites (code_cis, denomination, ...)
VALUES (?, ?, ...)
ON CONFLICT(code_cis) DO NOTHING;
```

**Phase 2** (Update modified records — per-row hash comparison):
```sql
UPDATE cis_specialites
SET denomination = ?, forme_pharmaceutique = ?, ..., _import_id = ?
WHERE code_cis = ? AND content_hash != ?;
```

**Phase 3** (Soft delete — mark missing records as inactive):
```sql
UPDATE cis_specialites
SET _is_active = 0, _import_id = ?
WHERE code_cis NOT IN (SELECT code_cis FROM temp_new_data)
  AND _is_active = 1;
```

**Transactional wrapper**:
```sql
BEGIN TRANSACTION;
-- Phase 1: Insertions
-- Phase 2: Mises à jour
-- Phase 3: Suppressions logiques
-- Enregistrement dans import_log
COMMIT;
```

Also from `05_pipeline_transformation.md` lines 358–404:
```rust
fn import_specialites(conn: &Connection, rows: &[Vec<String>]) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM specialites", [])?;  // Clear old data
    // Then batch INSERT ...
    tx.commit()?;
}
```

### Our Plan: Per-File Transaction + `INSERT OR REPLACE` (upsert)

**Assessment**: There is a significant functional difference:
- **Reviews' approach**: `DELETE FROM table` (full truncate of that table's data), then batch `INSERT`. This means the table is briefly empty during the transaction. For `specialites` (central table), this is risky — `presentations`, `compositions`, etc. reference it via FK during that window.
- **Reviews' approach also** includes a 3-phase strategy with per-row content hash comparison.
- **Our approach**: `INSERT OR REPLACE` preserves historical references since it is an upsert (insert if new, replace if exists). No table goes empty. No `_is_active` soft-delete column needed.

**Trade-off**: Our approach is simpler and safer (no empty-table window, no orphan risk from truncated table). The reviews' approach is more conservative (explicit soft-delete, full control over which rows are updated vs. inserted). For a single-user database importer, our approach is pragmatic.

**Additional review complexity**: The reviews recommend `_is_active` columns on every table (`_is_active INTEGER NOT NULL DEFAULT 1` — see `03_schema_sqlite.md` lines 58, 91, 116, etc.). This adds ~1 column per table and query complexity (`AND _is_active = 1` in every SELECT). We recommend omitting this for simplicity unless we build a versioned history feature.

**Verdict**: Our `INSERT OR REPLACE` approach is simpler and safer for our single-user case. The reviews' 3-phase DELETE/INSERT/soft-delete strategy is more conservative but unnecessary for our scope. We should add `_is_active` only if we build a change-history feature.

---

## DEPENDENCY VERSIONS

### Crates recommended across all reviews:

| Crate | Version | Usage |
|-------|---------|-------|
| `rusqlite` | `0.31` (bundled) | SQLite database |
| `sha2` | `0.10` | Hashing |
| `reqwest` | `0.12` | HTTP client |
| `tokio` | `1` (full) | Async runtime |
| `chrono` | `0.4` | Date/time |
| `regex` | `1` | HTML/text patterns |
| `clap` | `4` (derive) | CLI |
| `tracing` / `tracing-subscriber` | `0.1` / `0.3` | Structured logging |
| `serde` / `serde_json` | `1` | Serialization |
| `encoding_rs` | `0.8` | Text encoding |
| `anyhow` / `thiserror` | `1` | Error handling |
| `refinery` | `0.8` (rusqlite) | DB migrations |
| `scraper` | `0.19`–`0.20` | HTML parsing |
| `strum` | (latest) | Enum derive macros |
| `rust_decimal` | `1` | Decimal arithmetic |

### Our plan dependency coverage:

| Aspect | Reviews | Our Plan | Status |
|--------|---------|----------|--------|
| HTTP | `reqwest 0.12` | `ureq` (sync) | DIFFERENT |
| SQLite | `rusqlite 0.31` ✓ | `rusqlite` ✓ | ALIGNED |
| Hash | `sha2 0.10` | `blake3` | DIFFERENT (better) |
| Encoding | `encoding_rs 0.8` ✓ | `encoding_rs` ✓ | ALIGNED |
| Primitives | `chrono 0.4` ✓ | `chrono` ✓ | ALIGNED |
| Decimal | `rust_decimal 1` | `f64` | OPTIONAL (f64 fine for prices) |
| CLI | `clap 4` ✓ | N/A (no CLI plan) | MISSING from our plan |

### Notable missing from our plan:
- **CLI** (`clap 4`): The reviews build a CLI binary (`bdpm-importer import --full`, `check-updates`, `validate`, `stats`, `export`). We have no CLI plan — only a database-only usecase.
- **Structured logging** (`tracing`): Our plan has no logging strategy specified.
- **Error handling** (`anyhow`/`thiserror`): Our plan has no error strategy specified.

---

## SUMMARY OF CONFLICTS

| Topic | Reviews | Our Plan | Severity |
|-------|---------|----------|----------|
| Crate structure | 5-crate workspace | Single-crate | **HIGH** — architectural |
| HTTP client | reqwest + tokio (async) | ureq (sync) | **HIGH** — if we want API later (rouille is sync-only) |
| Hash function | SHA-256 | BLAKE3 | **LOW** — BLAKE3 is faster |
| Pipeline stages | FETCH→DECODE→PARSE→TRANSFORM→LOAD | Fetch→Decode→Parse→Validate→Normalize→Import | **LOW** — naming only |
| SQLite PRAGMAs | WAL + NORMAL + 64MB + ref docs | Same + should add `synchronous=OFF` during bulk | **MEDIUM** — missing optimize/restore pattern |
| Migrations | refinery (SQL files) | rusqlite_migration | **NONE** — equivalent |
| Change detection | SHA-256 + web scraping | BLAKE3 (file-level) | **MEDIUM** — we lack web-page signal |
| Import strategy | DELETE + INSERT + soft delete | INSERT OR REPLACE | **MEDIUM** — our approach is safer for single-user |
| Decimal handling | rust_decimal | f64 | **NONE** — f64 fine for this use |
| CLI | clap 4 | None | **MEDIUM** — CLI is useful for ops |
| Logging/Errors | tracing, anyhow/thiserror | None | **LOW** — add passively |

---

## RECOMMENDED CHANGES TO OUR PLAN

1. **Add CLI layer** (`clap`, `tracing`): Even a simple CLI (import, validate, stats) improves usability over raw library calls.
2. **Add `synchronous=OFF` bulk-insert pattern**: Temporarily set during import, restore to NORMAL after — as shown in `05_pipeline_transformation.md` lines 413–426.
3. **Consider tokio+reqwest if we add HTTP API layer**: Since `rouille` is sync-only and future phases may want an async web API (Phase 6 of reviews: actix-web/axum). A tokio runtime now future-proofs this.
4. **Add structured error handling**: Use `thiserror` for domain errors and `anyhow` for application-level errors.
5. **Skip `_is_active` soft-delete columns**: Not needed for our single-user use case unless we add versioning.
6. **Keep single-crate structure**: For 11 files and ~150K rows, 5 crates add more overhead than they solve for a solo developer. Re-evaluate if the scope expands.
7. **Encoding strategy alignment**: Our UTF-8-first-then-CP1252 heuristic matches the review pattern (reviewed in `06_architecture_rust.md` section 4.3), good to keep.
