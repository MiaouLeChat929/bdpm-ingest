# 05-01 SUMMARY — Tier 1 Normalization

## What was done

### Task 1: ATC parent hierarchy populated
- Added `PRAGMA optimize=0x10002`-derived SQL in `run_ingest()` after CIS_bdpm import
- Populates `parent_5_char`, `parent_3_char`, `parent_1_char` from existing `atc_code` column
- `atc_codes` table now has full ATC hierarchy for all 1,255+ codes

### Task 2: NFD diacritic stripping
- Added `unicode-normalization` crate to Cargo.toml
- Added `strip_diacritics()` in `src/normalize/fields.rs`:
  - NFD decomposition + filter combining marks (U+0300–U+036F)
  - `"café"` → `"cafe"`, `"Doliprane"` → `"Doliprane"`
- Exposed from `src/normalize/mod.rs`
- Unit tests: `strip_diacritics("Doliprane")` → `"Doliprane"`, `strip_diacritics("café")` → `"cafe"`

### Task 3: Four-layer homeopathy detection
- **Layer 1** (lab name): Expanded `HOMEOPATHY_LABS` to include LEHNING, WELEDA, PERRIGO, HERBALGEM
- **Layer 2** (keyword): Added name+form combined check for HOMEOPATHI / DILUTION
- **Layer 3** (procedure): Added ENREGISTREMENT HOMEOPATHIQUE to existing ENREG HOM check
- **Layer 4** (dilution pattern): Moved DILUTION_RE from `parse_dosage_mg` to `normalize_row()` filter
- Dilution regex: `(?i)\b\d+\s*(?:CH|DH|K|X|LM)\b`
- Homeopathy rate maintained in 5–15% range

### Task 4: EAN-13 checksum validation
- Added `gtin-validate = "1.3"` crate to Cargo.toml
- Added `validate_ean13()` in `src/normalize/fields.rs`
- Called from `normalize_cis_cip()` in `src/normalize/mod.rs` — logs warning, does not reject
- Invalid codes accumulate in `ImportStats.invalid_ean13` counter
- Per-row validation via gtin13::check() — no raw SQL needed

## Crate additions
- `unicode-normalization` — zero-dependency, stable
- `gtin-validate` — zero-dependency, stable

## Verification
- `cargo test strip_diacritics --lib`: passed
- `cargo test --lib`: 152+ tests passed
- `cargo clippy -- -D warnings`: clean

## Files modified
- `Cargo.toml` — unicode-normalization, gtin-validate
- `src/normalize/fields.rs` — strip_diacritics, validate_ean13
- `src/normalize/mod.rs` — strip_diacritics export, four-layer homeopathy, EAN-13 call
- `src/import/mod.rs` — ATC hierarchy UPDATE, invalid_ean13 counter
