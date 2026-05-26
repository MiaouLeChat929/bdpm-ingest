# Consolidated External Review Audit — BDPM Project

**Date:** 2026-05-26
**Sources:** 6 external review files (format_doc.pdf, BDPM_Analyse_Technique_Final.pdf, bdpm_feasibility_body.pdf, BDPM_Etude_Faisabilite.pdf, review1.txt, review2.txt)
**Compared against:** BRIEF.md, ROADMAP.md, 01-01 through 01-05 PLAN files

---

## VERIFIED AGAINST RAW DATA

These findings have been confirmed by running commands against the actual files in `/raw/`:

### V1: 0x92 curly apostrophe bytes (CRITICAL)
- CIS_HAS_SMR_bdpm.txt: **22,253** bytes with value 0x92 ✓ confirmed
- CIS_HAS_ASMR_bdpm.txt: **29,704** bytes with value 0x92 ✓ confirmed
- Our plan uses ISO-8859-1 → maps 0x92 to U+0092 (control char, WRONG)
- Fix: Use Windows-1252 decoding → maps 0x92 to U+2019 (right single quote)

### V2: Dispo status codes (CRITICAL)
- Code 1 (Rupture): 66 rows
- Code 2 (Tension): 421 rows ← NOT in our plan
- Code 3 (Arrêt): 15 rows ← NOT in our plan
- Code 4 (Remise): 264 rows
- Our CHECK IN (1,4) would **drop 56.9% of availability records** (436 of 766)
- Fix: CHECK IN (1, 2, 3, 4)

### V3: Orphan CIS references (CRITICAL)
- SMR: 2,806 orphans (18.4%) ✓ confirmed
- ASMR: 1,567 orphans (15.8%) ✓ confirmed
- GENER: 2,503 orphans (23.5%) ✓ confirmed
- COMPO: 0 orphans ✓ confirmed
- MITM: 0 orphans ✓ confirmed
- Our plan has strict FK constraints → orphan inserts would FAIL
- Fix: PRAGMA foreign_keys=OFF during import, or nullable FK

### V4: CIS_CIP_Dispo_Spec.txt encoding (WARNING)
- UTF-8 decode fails on byte 0xe0 at position 207 ✓ confirmed
- Latin-1 decode succeeds ✓ confirmed
- Our plan says UTF-8 → WRONG
- Fix: Change to Latin-1

### V5: Line endings (WARNING)
- CIS_CPD_bdpm.txt: 6 × `\r\r\n` sequences ✓ confirmed (mixed)
- CIS_CIP_bdpm.txt: Pure LF, zero CRLF ✓ confirmed
- Our plan says "CRLF everywhere" → WRONG for these files
- Fix: Update BRIEF.md, keep existing strip logic (already handles both)

### V6: TSV format confirmed
- All 10 files contain tab characters ✓ confirmed
- format_doc's "fixed-width" claim is WRONG
- Our tab-split approach is correct

---

## VERIFIED FROM EXTERNAL REVIEWS (not yet raw-verified)

### E1: URL pattern change (CRITICAL)
- External claim: `/telechargement?fich=XXX` no longer works; use `/download/file/XXX`
- Impact: Fetcher would download HTML pages instead of data files
- Status: **Needs verification against live server** (cannot verify from raw files alone)

### E2: Smart quote normalization (CRITICAL)
- External claim: 51,957 total curly apostrophes (0x92) across SMR+ASMR files
- Fix: After Windows-1252 decode, normalize U+2019 → U+0027
- Our plan doesn't mention this normalization step
- Status: Byte counts verified, fix needed in 01-04 normalization pipeline

### E3: Three encoding groups, not two (CRITICAL)
- Group 1: Windows-1252 (7 files: CIS_bdpm, CIS_COMPO, CIS_CPD, CIS_GENER, CIS_HAS_SMR, CIS_HAS_ASMR, CIS_MITM)
- Group 2: Latin-1 (1 file: CIS_CIP_Dispo_Spec)
- Group 3: UTF-8 (2 files: CIS_CIP_bdpm, HAS_LiensPageCT)
- Our plan has two groups (Latin-1 + UTF-8) → misses cp1252 vs latin-1 distinction
- Fix: Add Windows1252 variant to Encoding enum

### E4: CIS_InfoImportantes.txt is dynamically generated (WARNING)
- Content-Disposition header includes timestamp in filename
- MIME type is `application/force-download` not `application/octet-stream`
- Can change at any time, not just monthly
- Our plan defers to Phase 3.5 — correct decision
- Status: Noted for future implementation

### E5: Sync strategy — upsert vs truncate (CRITICAL)
- External recommends incremental with soft-delete
- Our plan uses full truncate+reload
- Truncating `drugs` would lose withdrawn drugs that SMR/ASMR reference
- Fix: Use INSERT OR REPLACE (upsert) for drugs table, truncate OK for dependents

### E6: FTS5 corruption with content= (CRITICAL for Phase 2)
- External Python implementation hit `database disk image is malformed`
- Caused by FTS5 `content=` + `content_rowid=` pattern
- Fix: Use standalone FTS5 tables with manual sync
- Our plan defers FTS5 to Phase 2 — add anti-pattern warning

### E7: import_id traceability (SUGGESTION)
- External adds `_import_id INTEGER REFERENCES import_log(id)` to every table
- Our plan only has `import_log` without cross-referencing
- Enables audit queries: "which import brought this record?"
- Fix: Add _import_id column to all data tables

### E8: Price thousands separator (CONFIRMED)
- External: 466 rows with values >999 use comma thousands separator
- Our plan handles this: detect comma count, treat all commas as thousands separators except last
- Approach confirmed correct by all external reviews

### E9: HTML in avis fields (CONFIRMED)
- External: 4,031 rows with `<br>`/`<p>`/`<b>` tags in SMR/ASMR
- Our plan: strip HTML on store, preserve text content
- Approach confirmed correct

### E10: Date normalization (CONFIRMED)
- DD/MM/YYYY in CIS_bdpm, CIS_CIP, CIS_CIP_Dispo, CIS_InfoImportantes
- YYYYMMDD in CIS_HAS_SMR, CIS_HAS_ASMR
- External mentions YYYY-MM-DD as third format (InfoImportantes only)
- Our plan normalizes both to ISO-8601 — correct

---

## PRIORITY FIX LIST

### Before execution begins (CRITICAL):

| # | Fix | File | Impact if skipped |
|---|-----|------|-------------------|
| 1 | Add Windows1252 encoding variant; use for 7 files | 01-01, 01-02, BRIEF | 52K apostrophe chars corrupted |
| 2 | Add smart quote normalization (U+2019→U+0027) | 01-04 | Search broken on HAS data |
| 3 | Change Dispo CHECK to (1,2,3,4) | BRIEF, 01-05 | 57% of availability records dropped |
| 4 | Change CIS_CIP_Dispo encoding to Latin-1 | BRIEF, 01-01 | Decode error on 0xe0 |
| 5 | Relax FK for orphan imports | 01-05 | SMR/ASMR/GENER import fails |
| 6 | Upsert drugs table (not truncate) | 01-05 | Historical HAS references lost |
| 7 | Verify URL pattern against live server | 01-01 | Fetcher downloads HTML not data |

### Before execution begins (WARNING):

| # | Fix | File | Impact if skipped |
|---|-----|------|-------------------|
| 8 | Fix CRLF "everywhere" claim in BRIEF | BRIEF | Misleading docs |
| 9 | Document FTS5 content= anti-pattern | ROADMAP, Phase 2 | DB corruption risk |
| 10 | Add _import_id to data tables | 01-05 | No import traceability |
| 11 | Add multi-line record handling note | 01-02 | Parser may split records |

### Nice-to-have (SUGGESTION):

| # | Fix | File | Impact if skipped |
|---|-----|------|-------------------|
| 12 | Config file (bdpm.toml) | Phase 2+ | Minor convenience |
| 13 | data.gouv.fr API as secondary source | Phase 3+ | Redundant detection |
| 14 | 5-crate workspace | — | Rejected: single crate for solo dev |
| 15 | rust_decimal instead of INTEGER cents | — | Rejected: cents is correct |

---

## CONFIRMED CORRECT IN OUR PLAN

- Tab-separated format (not fixed-width)
- BLAKE3 for change detection (faster than SHA-256)
- INTEGER cents for prices (not float)
- ISO-8601 date normalization
- encoding_rs crate choice
- rusqlite with bundled SQLite
- HTML stripping from avis fields
- Synchronous stack (ureq, not reqwest+tokio)
- Single crate architecture
- CIS as TEXT primary key
