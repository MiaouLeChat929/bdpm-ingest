# Schema, Encoding, and Data Format Critique

**Comparing our plan (01-01 through 01-05, BRIEF.md) against external analyses (format_doc, analyse_technique, etude_faisabilite, feasibility_body)**

---

## CRITICAL (must fix before execution)

### C1: Encoding — Windows-1252 vs ISO-8859-1 mismatch

**What the external sources say:**
- format_doc.md: "The file is encoded in **Windows-1252** (CP1252), which is a single-byte superset of ISO-8859-1... Accented characters... are valid and expected. Any parser must handle Windows-1252 decoding correctly. Using UTF-8 by default will corrupt characters like `é`, `è`, `à`, `ç`, `œ`, etc."
- format_doc.md Section 3.1: "Character 0x92 (right single quotation mark, U+2019) appears **massivement** in HAS files: 29,704 occurrences in CIS_HAS_ASMR and 22,253 in CIS_HAS_SMR. Decoding these files in ISO-8859-1 produces **invisible control characters** instead of expected apostrophes."
- analyse_technique.md Table: Lists CIS_bdpm, CIS_COMPO, CIS_CPD, CIS_GENER, CIS_HAS_SMR, CIS_HAS_ASMR, CIS_MITM all as "Latin-1" but then describes encoding_rs handling "Windows-1252/Latin-1 files" with explicit curly apostrophe replacement guidance.
- etude_faisabilite.md Section 4.1 encoding table: Maps CIS_bdpm, CIS_COMPO, CIS_GENER, CIS_HAS_SMR, CIS_HAS_ASMR, CIS_MITM to "cp1252 (Windows-1252)".
- etude_faisabilite.md: "Decoding as UTF-8 will corrupt characters... 0x92 = right single quote (U+2019), **not valid in latin-1**. 29,704 occurrences in CIS_HAS_ASMR alone."

**What our plan says:**
- BRIEF.md: "8 files ISO-8859-1; 2 files UTF-8"
- 01-01-PLAN.md: `Encoding { Latin1, Utf8 }` enum — no Windows-1252 variant
- 01-02-PLAN.md: "Decode using `encoding_rs::ISO_8859_1` or `UTF_8`"
- 01-02-PLAN.md: "handles Windows-1252 residuals natively: `\x92 → U+2019`" — this is factually wrong. ISO-8859-1 maps 0x92 to U+0092 (control character), NOT U+2019. Windows-1252 maps 0x92 to U+2019.

**Concrete recommendation:**
1. Add `Windows1252` variant to the `Encoding` enum in `src/download/manifest.rs`
2. Use `encoding_rs::WINDOWS_1252` for 7 files: CIS_bdpm, CIS_COMPO, CIS_CPD, CIS_GENER, CIS_HAS_SMR, CIS_HAS_ASMR, CIS_MITM
3. Fix the incorrect claim in 01-02-PLAN.md: `encoding_rs::ISO_8859_1` does NOT handle `\x92 → U+2019`. You need `encoding_rs::WINDOWS_1252` for that.
4. The curly apostrophe normalization (`U+2019 → U+0027`) should still happen after Windows-1252 decoding, but the decoding itself must be Windows-1252.

**Impact if not fixed:** 52,000+ apostrophe-like characters in HAS files decoded as invisible control characters, causing text content corruption and broken search functionality.

---

### C2: File Format — Our plan says TSV, format_doc says fixed-width

**What the external sources say:**
- format_doc.md Section 1.1: "Each line in a BDPM file consists of **fixed-width columns** (champs fixes, not délimités par des séparateurs). The document does not explicitly specify column widths — those must be inferred from the documentation or reverse-engineered from sample data."
- format_doc.md Section 1.3: "Fields are not separated by semicolons, tabs, or commas. They are purely positional. No escaping mechanism."
- etude_faisabilite.md Section 1.1: "Inspection confirms the **separator is uniformly the tab character**... no file contains a header row, and no field delimiter (quotes) is used."
- analyse_technique.md: "Stage 3: TSV Parsing — Split on tab characters"

**Concrete recommendation:**
Two of three external sources disagree on this fundamental format. etude_faisabilite (768-line study) and analyse_technique (933-line analysis) both confirm tab-separated. format_doc (9-page review) claims fixed-width. Given that 10 files have variable-length fields (drug names > 255 chars, avis text up to 2019 chars), fixed-width is structurally impossible. format_doc appears to be an outlier or a misinterpretation.

**Do not change your tab-split approach.** But add a validation note: if a line does not contain any tab characters, flag it as malformed (support for both interpretations).

---

### C3: Dispo Status Codes — Our plan says CHECK IN (1, 4), external says (1, 2, 3, 4)

**What the external sources say:**
- format_doc.md Section 2.6 (CIS_CIPS): Describes CIP code as 7-digit unique identifier for commercial packaging, with multiple CIPs per CIS.
- analyse_technique.md (ruptures_stock table): "CHECK(identifiant_disponibilite BETWEEN 1 AND 4)"
- etude_faisabilite.md Section 3.2 (cis_disponibilite): "code_statut INTEGER — CHECK IN (1, 2, 3, 4)"
- feasibility_body.md Section 3: "CHECK IN (1, 2, 3, 4)"

**What our plan says:**
- BRIEF.md: `status_type INTEGER — CHECK IN (1, 4)`
- 01-05-PLAN.md: No explicit CHECK constraint defined for `status_type` in the import function

**Concrete recommendation:**
Change the CHECK constraint from `CHECK IN (1, 4)` to `CHECK IN (1, 2, 3, 4)`. Status codes 2 and 3 are valid in the source data. Rejecting them silently (as None) would lose legitimate stock status information.

---

## WARNING (should fix, non-blocking)

### W1: CIS_CIP_Dispo_Spec.txt encoding — conflict between sources

**What the external sources say:**
- BRIEF.md: "CIS_CIP_Dispo_Spec.txt — UTF-8"
- analyse_technique.md file inventory: "CIS_CIP_Dispo_Spec.txt — latin-1 — CRLF"
- etude_faisabilite.md encoding table: "CIS_CIP_Dispo_Spec — latin-1"
- etude_faisabilite.md Section 4.1: "latin-1 (ISO-8859-1): CIS_CIP_Dispo_Spec — 0xE0 = à grave. Decoding as UTF-8 fails."

**Concrete recommendation:**
Use latin-1 (not UTF-8) for CIS_CIP_Dispo_Spec.txt. Two sources outvote our plan. latin-1 handles 0xE0 (à) correctly; UTF-8 would fail on it.

---

### W2: CIS_CIP_bdpm.txt encoding — conflict between sources

**What the external sources say:**
- BRIEF.md: "CIS_CIP_bdpm.txt — UTF-8"
- analyse_technique.md file inventory: "CIS_CIP_bdpm.txt — UTF-8 — LF"
- etude_faisabilite.md encoding table: "CIS_CIP_bdpm.txt — utf-8"

All three external sources agree on UTF-8. Our plan matches. No change needed, but document why this file differs from the others (it contains accented French text but stored as UTF-8 rather than Windows-1252).

---

### W3: Line endings — two files use LF, not CRLF everywhere

**What the external sources say:**
- analyse_technique.md "Critical Issues": "Inconsistent line endings: 9 files CRLF, 2 files LF, 1 file mixed"
- analyse_technique.md file inventory: CIS_CIP_bdpm.txt = LF, CIS_InfoImportantes.txt = LF, CIS_CPD = Mixed
- etude_faisabilite.md: "CIS_CIP_bdpm.txt — LF; CIS_InfoImportantes.txt — LF; CIS_CPD — Mixed (CR+CR+LF)"
- format_doc.md: "Each line ends with: CRLF on Windows, LF on Unix/Linux"

**What our plan says:**
- BRIEF.md: "CRLF everywhere: all files use Windows-style `\r\n` terminators"

**Concrete recommendation:**
Our plan is incorrect for at least 3 files. The CRLF strip logic is already in the plan (Task 1 of 01-02), which correctly handles this. The issue is the framing in BRIEF.md which states "CRLF everywhere" — update to reflect the actual mixed line ending reality.

---

### W4: Multi-line records — format_doc describes continuation lines

**What the external sources say:**
- format_doc.md Section 3.3: "The document describes a critical anomaly: certain fields (primarily `INDICATIONS` and `COMPOSITION`) may contain embedded line breaks within what should be a single logical record. This can cause parsers that split on newlines to incorrectly create multiple records from a single drug entry."
- format_doc.md Section 4.6: "Recommended algorithm for handling multi-line fields: if line starts with valid CIS code → new record; else → append to previous record as continuation"

**What our plan says:**
- 01-02-PLAN.md Task 1: "Empty lines: skip" — doesn't mention multi-line record handling
- No mention of continuation-line detection in any plan file

**Concrete recommendation:**
Implement the continuation-line algorithm from format_doc in the tab parser:
```
if line.strip().is_empty():
    continue
elif line starts with 9-digit CIS code:
    emit previous record; start new one
else:
    append to current record (join with space)
```
This handles the embedded line breaks in long avis and indications fields.

---

### W5: import_id traceability — our plan omits the foreign key pattern

**What the external sources say:**
- etude_faisabilite.md: "Each table includes `_import_id` referencing `import_log` for traceability"
- feasibility_body.md: All 11 tables show `_import_id INTEGER REFERENCES import_log(id)`

**What our plan says:**
- BRIEF.md: Only defines the `import_log` table. Tables like `drugs`, `presentations`, etc. have no `_import_id` column.
- 01-05-PLAN.md: Writes to `import_log` table but doesn't add `_import_id` foreign keys to data tables.

**Concrete recommendation:**
Add `_import_id INTEGER REFERENCES import_log(id)` column to all data tables. This enables audit queries like "which import brought in this record?" and "replay imports in order". The import orchestrator should capture the `import_log.id` and pass it to each per-file import function.

---

## SUGGESTION (nice-to-have improvement)

### S1: Price storage — rust_decimal vs INTEGER cents

**What the external sources say:**
- etude_faisabilite.md Section 7.2 crate table: "`rust_decimal` (optional wrapper over f64) — avoids floating-point precision issues in financial calculations"
- etude_faisabilite.md Section 4.4: "Convert during import into REAL columns" (f64)

**What our plan says:**
- BRIEF.md: "Prices are stored as `24,34` → 2434 (cents). Integer arithmetic avoids floating-point errors."
- 01-04-PLAN.md: `parse_price_cents` returns `Option<i64>` with integer cents

**Concrete recommendation:**
Our plan (INTEGER cents) is the more robust approach for exact arithmetic. The external feasibility suggests REAL (f64) but also notes `rust_decimal` as the better option. Keep the INTEGER cents approach — it's simpler and avoids floating-point precision issues. Document this as an explicit design decision that diverges from the feasibility study's recommendation.

---

### S2: ATC hierarchy — 5-level derivation missing from plan

**What the external sources say:**
- format_doc.md Section 2.4: "ATC classification system is hierarchical (5 levels: ATC classes → subgroups → groups → chemical subgroups → substance)"
- format_doc.md Section 5.2: "Observed: Some codes are 9 characters with an additional level (e.g., `N02BE01B`)"

**What our plan says:**
- BRIEF.md: `atc_codes` table has `parent_5_char`, `parent_3_char`, `parent_1_char` — correct
- But no explicit handling of the 9-character extended codes

**Concrete recommendation:**
Add explicit derivation logic for the 5-level ATC hierarchy when parsing CIS_MITM.txt. Validate ATC codes against `[A-Z]\d{2}[A-Z]{2}\d{2}` pattern (7-char). Log and flag 9-character codes as extended variant without blocking import.

---

### S3: Schema drift detection — field count should be dynamic

**What the external sources say:**
- format_doc.md Section 5.1: "Column Count Inconsistency — Official ANSM documentation states 20 columns, actual observed files have 19 or 21. Resolution: derive column count from the first row of the file (header row) rather than hardcoding."

**What our plan says:**
- 01-01-PLAN.md: `BDPMFile` enum with hardcoded `field_count: usize` per file
- 01-02-PLAN.md: `RowIterator` checks `record.fields.len() == schema.field_count` — static check

**Concrete recommendation:**
Add a fallback mode where, if the actual field count differs from the hardcoded value, the parser logs a warning and accepts the actual count. This handles format evolution without breaking the import.

---

### S4: Date plausibility bounds — our plan mentions but doesn't fully spec

**What the external sources say:**
- format_doc.md Section 3.6: "Date fields use `DD/MM/YYYY` (French convention)... A drug authorized on `01/02/2024` means February 1st, 2024"

**What our plan says:**
- BRIEF.md: "Validate date range on ingest, flag outliers" (for `29/11/2924`)
- 01-04-PLAN.md: "Reject dates outside 1900–2100 range"

**Concrete recommendation:**
Document the validation thresholds explicitly: AMM dates must be between 1950-01-01 and today+30 days. Alert (don't reject) dates outside this range. This catches the `29/11/2924` typo mentioned in BRIEF.md.

---

## CONFIRMED (external analysis confirms our approach is correct)

### CONF1: Tab-separated format (vs fixed-width)

External sources confirm BDPM uses tab delimiters. Despite format_doc's outlier claim of fixed-width, etude_faisabilite (768 lines, direct file inspection) and analyse_technique (933 lines, actual parsing) both confirm tab-separated. Our tab-split approach is correct.

---

### CONF2: Date normalization to ISO-8601

All four external sources confirm:
- DD/MM/YYYY format in CIS_bdpm, CIS_CIP, CIS_InfoImportantes, CIS_CIP_Dispo
- YYYYMMDD integer format in CIS_HAS_SMR, CIS_HAS_ASMR
- All must normalize to ISO 8601 `YYYY-MM-DD` for correct SQL sorting

Our plan's date normalization approach is confirmed correct.

---

### CONF3: Price thousands-separator handling

analyse_technique.md explicitly documents the critical case: "466 rows with values >1000 use comma as thousands separator (`1,466,29` → must remove both commas, not replace)". This matches our plan's three-comma handling in `parse_price_cents`. Our approach is correct and matches the external analysis.

---

### CONF4: Soft-delete/incremental import for historical data

etude_faisabilite.md: "Historical data loss on full reload — High likelihood, High impact. Mitigation: incremental import with soft-delete from day one; archive all raw files."

format_doc.md Section 3.4: "CIS codes are retired (produits retirés du marché) but not reused. Deleted codes remain in archive files. When merging current and archive files, duplicates may appear."

Our plan's decision to use BLAKE3 hash comparison + full-table refresh (not incremental upsert) is documented as a conscious trade-off given the simplicity goal. This is defensible at the 32K-row scale. However, the historical data preservation concern from external analysis is real — document that full-table refresh means we rely on `import_log` for historical tracking, not row-level history.

---

### CONF5: BLAKE3/SHA-256 as authoritative change signal

etude_faisabilite.md Table: "SHA-256 of content — Full download + hash — 100%: any change detected — RECOMMENDED"

analyse_technique.md: "BLAKE3 hash computed during download (stream hashing) — AUTHORITATIVE SIGNAL"

format_doc.md: "The correct behavior is to use the `ETAT` field to distinguish active from withdrawn products"

Our plan uses BLAKE3 (faster than SHA-256, parallelizable) as the authoritative signal. This is confirmed correct and superior to the feasibility study's SHA-256 suggestion.

---

### CONF6: encoding_rs as the correct decoding crate

etude_faisabilite.md: "`encoding_rs` — Zero-copy UTF-8 decoding; cp1252/latin-1/UTF-8 support; encoding resolved at compile time"

analyse_technique.md: Lists `encoding_rs = "0.8"` as required crate

Our plan's use of `encoding_rs` is confirmed correct. The only fix needed is which encoding variant to use (Windows-1252, not ISO-8859-1, for 7 files).

---

### CONF7: rusqlite with bundled SQLite

etude_faisabilite.md: "`rusqlite` — De facto standard for SQLite in Rust; supports WAL, prepared statements, UDFs"

analyse_technique.md: "rusqlite with `bundled` feature: embedding SQLite in the binary is the correct approach for a portable CLI tool"

Our plan to use `rusqlite = { version = "0.31", features = ["bundled"] }` is confirmed correct.

---

### CONF8: HTML stripping from avis fields

analyse_technique.md: "4,031 rows contain HTML `<br>` tags in avis field (13% SMR, 21% ASMR). Decision: strip HTML on store, preserve text content for API output."

format_doc.md: Documents `<br>` tags in avis and recommends stripping.

Our plan's `strip_avis_html` function in 01-04 is confirmed correct and critical (13-21% of records affected).

---

## Summary Table

| ID | Category | Finding | Severity | Action |
|----|----------|---------|----------|--------|
| C1 | Encoding | ISO-8859-1 → Windows-1252 for 7 files; 0x92 mapping fix | CRITICAL | Add `Windows1252` enum variant + use `encoding_rs::WINDOWS_1252` |
| C2 | Format | format_doc fixed-width claim is outlier; tab is correct | CRITICAL | Keep tab-split; add fallback validation |
| C3 | Schema | Dispo CHECK IN (1,4) → CHECK IN (1,2,3,4) | CRITICAL | Update CHECK constraint |
| W1 | Encoding | CIS_CIP_Dispo_Spec: use latin-1 not UTF-8 | WARNING | Fix encoding assignment |
| W2 | Encoding | CIS_CIP_bdpm: UTF-8 confirmed by all sources | WARNING | None — already correct |
| W3 | Format | CRLF claim wrong for 3 files | WARNING | Update BRIEF.md framing |
| W4 | Format | Multi-line record handling missing | WARNING | Implement continuation-line algorithm |
| W5 | Schema | import_id FK pattern missing from plan | WARNING | Add `_import_id` to all data tables |
| S1 | Storage | INTEGER cents vs rust_decimal — keep cents | SUGGESTION | Document as explicit design decision |
| S2 | Schema | ATC 5-level hierarchy not fully spec'd | SUGGESTION | Add derivation logic + 9-char variant handling |
| S3 | Schema | Hardcoded field count vs dynamic | SUGGESTION | Add drift-detection fallback |
| S4 | Schema | Date plausibility bounds not fully spec'd | SUGGESTION | Document validation thresholds |
| CONF1-8 | — | 8 approaches confirmed correct | CONFIRMED | No changes needed |

**Priority order for fixes:**
1. C1 (encoding) — will corrupt data if not fixed
2. C3 (dispo codes) — will silently drop valid records
3. W1 (CIS_CIP_Dispo encoding) — small fix, high confidence
4. W4 (multi-line) — low cost, prevents parsing failure on edge cases
5. W5 (import_id) — enables auditability