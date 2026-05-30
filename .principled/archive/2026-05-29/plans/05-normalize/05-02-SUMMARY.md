# 05-02 SUMMARY — Tier 2 Normalization

## What was done

### Task 1: Form canonicalization
- Added `phf` crate to Cargo.toml
- `FORM_CANONICAL` phf_map in `src/normalize/fields.rs`: 20+ high-frequency forms → canonical codes (CPR, GEL, INJ, SOL, COL, SUP, POM, PATCH, GRAN, POW, etc.)
- `canonicalize_form()` function with unit tests
- **Accent mismatch fix**: added NFD diacritic stripping before lookup — `"comprimé"` matches key `"comprime"`
- Extended with top BDPM forms: comprimé pelliculé, gélule, solution pour perfusion, crème, etc.

### Task 2: Route canonicalization
- `ROUTE_CANONICAL` phf_map in `src/normalize/fields.rs`: 16 entries
- `canonicalize_route()` with semicolon-delimited multi-value support
- NFD diacritic stripping before lookup
- Extended with: intraveineuse, sous-cutanée, ophtalmique, inhalée, nasale, buccale, articulaire

### Task 3: Lab family canonicalization
- `LAB_FAMILY_MAP` phf_map in `src/normalize/fields.rs`: 20+ entries
- 8 lab families covered: VIATRIS, MYLAN+ACTAVIS, BIOGARAN, ARROW, TEVA, SANOFI, SERVIER, SANDOZ
- `strip_lab_suffix()`: strips LABORATOIRES, PHARMA, SAS, SARL, GMBH, LTD, SA, FRANCE, HEALTH, SANTE
- `lab_name_canonical` column added to drugs table and INSERT SQL
- FTS sync triggers updated to include `lab_name_canonical`

### Task 4: CIS_CPD prescription condition flags
- Created `src/normalize/cpd.rs` with `CpdFlags` struct (6 booleans)
- 6 regex patterns: liste_i, liste_ii, stupefiant, hospitalier, dentaire, reserve_hopital
- `prescription_flags` table added to schema.sql
- `INSERT OR IGNORE` in import loop populates both `prescription_rules` and `prescription_flags`
- `CpdFlags` exposed from `src/normalize/mod.rs`

## Crate additions
- `phf` — compile-time perfect-hash map, zero heap allocation

## Verification
- `cargo test canonicalize_form --lib`: passed
- `cargo test canonicalize_route --lib`: passed
- `cargo test canonicalize_lab --lib`: passed
- `cargo test cpd --lib`: passed
- `cargo test --lib`: 165+ tests passed
- `cargo clippy -- -D warnings`: clean

## Files modified
- `Cargo.toml` — phf crate
- `src/normalize/fields.rs` — FORM_CANONICAL, ROUTE_CANONICAL, LAB_FAMILY_MAP, canonicalize_form/route/lab, strip_lab_suffix
- `src/normalize/mod.rs` — CpdFlags export
- `src/normalize/cpd.rs` — new file, CpdFlags struct and patterns
- `src/db/schema.sql` — prescription_flags table, lab_name_canonical column, FTS trigger updates
- `src/import/mod.rs` — lab_name_canonical INSERT, prescription_flags INSERT OR IGNORE
