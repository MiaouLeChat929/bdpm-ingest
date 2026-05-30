# 06-01 SUMMARY — Data Quality Fixes

## What was done

### Task 1: Fix CPD FK failure — INSERT OR IGNORE
- Changed `INSERT OR REPLACE INTO prescription_flags` to `INSERT OR IGNORE INTO prescription_flags` in `src/import/mod.rs`
- 20 orphan CIS codes (homeopathy-filtered or withdrawn) no longer cause FK violations
- CIS_CPD now imports successfully; `prescription_rules` and `prescription_flags` both populated

### Task 2: Fix COMPO dedup key to match PK
- Changed dedup key in `src/normalize/dedup.rs` from `(cis, substance_code, dosage)` to `(cis, substance_code, seq)`
- PK on compositions is `(cis, substance_code, seq)` — dedup key now matches
- ~4,763 valid rows recovered (same substance+dosage, different pharmaceutical form)
- Added `test_dedup_same_dosage_different_seq` to verify fix

### Task 3: Fix dilution regex false positive
- Removed `X` from DILUTION_RE alternation: `(?i)\b\d+\s*(?:CH|DH|K|LM)\b`
- Gene/cell therapy products restored (TECARTUS, YESCARTA, KYMRIAH, ZOLGENSMA, etc.)
- `2 X 100 000 000 cellules` no longer matched as homeopathic dilution

### Task 4: Wire form/route/lab canonicalization
- `canonicalize_form()` wired into `normalize_cis_bdpm()` — form column canonicalized at import
- `canonicalize_route()` wired — route column canonicalized
- `canonicalize_lab()` wired — lab_name column canonicalized
- NFD diacritic stripping applied before FORM_CANONICAL and ROUTE_CANONICAL lookups

### Task 5: Add missing salt suffixes
- Extended SALT_SUFFIXES with: sodique, calcique, potassique, mésylate, fumarate, succinate, camphosulfonate, pamoate, émésylate, ésilate, xaforate, cilexétil, arginine, sel de sodium

### Task 6: Extend route canonicalization
- Extended ROUTE_CANONICAL with: intraveineuse, sous-cutanée, ophtalmique, inhalée, nasale, buccale, articulaire
- `canonicalize_route()` handles semicolon-delimited multi-value routes

## Verification
- `cargo test dedup --lib`: passed (including new same-dosage-different-seq test)
- `cargo test dilution --lib`: passed (false-positive test added)
- `cargo test strip_salt --lib`: passed
- `cargo test --lib`: 177 tests passed
- `cargo clippy -- -D warnings`: clean
- Full ingest: CPD tables populated, compositions > 27,000 rows, gene therapy products present

## Files modified
- `src/import/mod.rs` — INSERT OR IGNORE for prescription_flags, lab_name_canonical in INSERT
- `src/normalize/dedup.rs` — dedup key changed to seq, new test
- `src/normalize/mod.rs` — DILUTION_RE X removed, canonicalize_form/route/lab wired into normalize_cis_bdpm
- `src/normalize/fields.rs` — extended SALT_SUFFIXES, extended ROUTE_CANONICAL, diacritic-stripped lookups
