# 05-03 SUMMARY — Tier 3 Normalization

## What was done

### Task 1: Salt prefix/suffix stripping
- `SALT_PREFIXES` array in `src/normalize/fields.rs`: 27 prefixes (chlorhydrate de, sulfate de, sel de, etc.)
- `SALT_SUFFIXES` array: 15 suffixes (dihydrate, trihydrate, monohydrate, anhydre, sodique, calcique, etc.)
- `strip_salt()`: prefix strip (longest-first) + multi-pass suffix strip
- `strip_parens()`: removes parenthetical annotations mid-string
- Applied to `substance_name` field in `normalize_compo()` via `strip_salt()` call
- Unit tests: chlorhydrate de paracétamol → paracétamol, diclofenac sodique → diclofenac, amoxicilline trihydratée → amoxicilline

### Task 2: Post-import validation thresholds
- `validate_thresholds()` in `src/import/mod.rs`: 5 threshold checks run post-import
  - Ghost CIS count (>3,000 → WARN)
  - Substance code cardinality (outside 2,000–3,000 → WARN)
  - Princeps groups (<500 → WARN)
  - Generic name coverage (<50% → WARN, expected on first run)
  - Date coherence (comm_date < auth_date → WARN)
- Results incorporated into ImportReport; logged as WARN, never block pipeline

### Task 3: FTS normalization diacritic stripping
- `fts_normalize()` in `src/normalize/fields.rs`: strips diacritics + removes noise words (de, du, la, le, etc.)
- Applied to FTS insert values in `src/db/fts.rs`
- Search becomes accent-insensitive without modifying stored data
- Original French spellings preserved in API response

### Task 4: Rayon parallelization for CIS_COMPO
- Added `rayon = "1.10"` to Cargo.toml
- Parallel normalization of CIS_COMPO rows: `into_par_iter().map(normalize_row).collect()`
- Results collected via mpsc channel, sorted by original line number, then passed to dedup
- Deterministic output order preserved (verified by test)
- CIS_COMPO 32K rows normalized in parallel

## Crate additions
- `rayon` — data-level parallelism, zero-cost abstraction

## Verification
- `cargo test strip_salt --lib`: passed
- `cargo test --lib`: 177 tests passed
- `cargo clippy -- -D warnings`: clean
- Validation thresholds produce WARN log output on ingest
- Rayon parallel output matches sequential (determinism verified)

## Files modified
- `Cargo.toml` — rayon
- `src/normalize/fields.rs` — SALT_PREFIXES, SALT_SUFFIXES, strip_salt, strip_parens, fts_normalize, NOISE_WORDS
- `src/normalize/mod.rs` — strip_salt applied to substance_name in normalize_compo
- `src/db/fts.rs` — fts_normalize applied to name, lab_name, substance_name columns
- `src/import/mod.rs` — validate_thresholds function, rayon parallel normalization for CIS_COMPO
