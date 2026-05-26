# Self-Review: Top 5 Vulnerabilities + Assumptions Risk Assessment

**Date:** 2026-05-26
**Scope:** BRIEF.md, 01-01 through 01-05 PLANs, consolidated-audit.md, critique files, 08_risques_validation.md
**Goal:** Find where the plan will break in production — the places where data corrupts, imports fail silently, or users get wrong results.

---

## TOP 5 VULNERABILITIES

### V1: Encoding Bug — ISO-8859-1 Decodes 0x92 as Control Character, Not Apostrophe

**Failure mode:** 52,957 bytes with value 0x92 across SMR/ASMR files decode to U+0092 (private control character) instead of U+2019 (right single quotation mark). Every avis field containing a curly apostrophe becomes an invisible garbage byte. Search breaks. Display shows garbage. Users trust nothing.

**What it looks like:**
```
# User searches for "doliprane"
SELECT * FROM smr WHERE avis LIKE '%doliprane%';
# Returns nothing — apostrophe in source ("doliprane's") decoded as control char
# User reports "database search is broken"
```

**Is it addressed in our plan?** No. BRIEF.md says "8 files ISO-8859-1". 01-02 says "Decode using `encoding_rs::ISO_8859_1`". The plan explicitly calls out the wrong encoding for 7 files. The claim that "encoding_rs::ISO_8859_1 handles Windows-1252 residuals natively: \x92 → U+2019" is factually incorrect — ISO-8859-1 maps 0x92 to U+0092, not U+2019.

**Fix:** Add `Windows1252` to the Encoding enum. Use `encoding_rs::WINDOWS_1252` for CIS_bdpm, CIS_COMPO, CIS_CPD, CIS_GENER, CIS_HAS_SMR, CIS_HAS_ASMR, CIS_MITM. After decoding, add a normalization pass: `U+2019 → U+0027` (smart quote → straight apostrophe).

**How hard to fix now vs later:** NOW: trivial — one enum variant + 7 file remappings. LATER: when Phase 2 search is built on corrupted data, you need a migration script to find and fix every U+0092 in the database, plus re-import. Rewriting production data is much harder than doing it right on first import.

---

### V2: Dispo CHECK Constraint — 56.9% of Availability Records Would Be Dropped

**Failure mode:** Our plan says `CHECK(identifiant_disponibilite IN (1, 4))` for the availability table. Raw data analysis shows: 66 rows code 1, 421 rows code 2, 15 rows code 3, 264 rows code 4. Codes 2 and 3 are valid but not in our CHECK. The import silently drops or rejects 436 of 766 availability records (57% loss).

**What it looks like:**
```
# Monthly import runs
# 436 availability records silently rejected
# "Stock status" API returns incomplete data
# User: "Why is this drug showing as available when it's in shortage?"
# Answer: we dropped the "tension" (code 2) status because our CHECK was wrong
```

**Is it addressed in our plan?** No. BRIEF.md says `CHECK IN (1, 4)`. 01-05 doesn't specify the CHECK constraint at all in the import function. The consolidated-audit raw-data verification confirms 2 and 3 are real, common codes.

**Fix:** Change the CHECK constraint to `CHECK IN (1, 2, 3, 4)`. Update the availability table schema. Update the import logic to accept all four codes (no filtering needed — SQLite will store any integer, but without CHECK it won't reject malformed data on insert).

**How hard to fix now vs later:** NOW: one line in the CREATE TABLE statement. LATER: you have a database full of records that passed through the broken filter, and you need to re-import the Dispo file to get the missing 436 rows. Re-import is easy but it means you had a window of missing data.

---

### V3: Orphan CIS References — FK Constraints Would Reject 15-23% of SMR/ASMR/GENER Records

**Failure mode:** SMR has 2,806 orphan CIS (18.4%), ASMR has 1,567 orphan CIS (15.8%), GENER has 2,503 orphan CIS (23.5%). These CIS exist in SMR/ASMR/GENER but are absent from the current CIS_bdpm.txt (withdrawn drugs). Our plan uses `REFERENCES drugs(cis)` on secondary tables. With FK enforcement ON, every orphan insert fails the transaction — entire monthly import aborts.

**What it looks like:**
```
BEGIN IMMEDIATE;
INSERT INTO smr (cis, ct_id, decision_type, decision_date, level, avis) VALUES (...);
-- ERROR: FOREIGN KEY constraint failed
-- ROLLBACK
-- 0 records imported
-- Monthly sync failed entirely
-- User: "Why didn't the database update this month?"
```

**Is it addressed in our plan?** No. BRIEF.md schema shows FK references from smr/asmr/generic_groups to drugs(cis). 01-05 import functions don't mention orphan handling. The orphan percentages are confirmed by raw data analysis.

**Fix:** Two options:
- **Option A:** `PRAGMA foreign_keys=OFF` during import, orphan inserts work, you get the data. Risk: no referential integrity enforcement.
- **Option B:** Insert orphan CIS into a `drugs_withdrawn` table (or `drugs` with `withdrawn_at` timestamp), then FK references work. More work but maintains integrity.
- **Option C (recommended):** Use `PRAGMA foreign_keys=OFF` for Phase 1. Add a post-import cleanup: scan for orphan CIS, log them, optionally insert into a `withdrawn_drugs` shadow table. This gets the data in without blocking the import.

**How hard to fix now vs later:** NOW: one `PRAGMA foreign_keys=OFF` statement before the import loop. LATER: the orphan problem exists whether or not you fix it — it will always be there because the BDPM intentionally keeps HAS evaluations for withdrawn drugs. Fix it now to avoid every monthly import failing.

---

### V4: Sync Strategy — Full Truncate+Reload Destroys Historical HAS References

**Failure mode:** BRIEF.md commits to "Full-Table Truncate+Reload, Not Row-Level Delta" for all tables. The external audit confirms: "CIS_bdpm.txt retains only drugs marketed or discontinued within the last 2 years. 18.4% of CIS_HAS_SMR references withdrawn drugs absent from CIS_bdpm." Truncating the drugs table removes those withdrawn CIS. SMR/ASMR rows become orphan references. Even with FK OFF, you lose the drug metadata (name, lab, authorization date) for historical evaluations.

**What it looks like:**
```
# Month 3: CIS 12345678 (withdrawn 18 months ago) is no longer in CIS_bdpm.txt
# Truncate drugs table
# DELETE FROM drugs WHERE cis = '12345678'
# SMR rows for 12345678 remain but drug info is gone

# User queries:
SELECT d.name, s.avis FROM smr s JOIN drugs d ON s.cis = d.cis;
# 2,806 rows return NULL for d.name — drug metadata is gone
# User: "What drug is this evaluation for?" → "We don't know"
```

**Is it addressed in our plan?** No. The critique-architecture-pipeline.md correctly identifies this as C1 (critical). Our plan explicitly rejects incremental and commits to truncate. The justification "For 32K-row tables this completes in seconds monthly" is correct about performance but wrong about correctness.

**Fix:** Replace `DELETE FROM drugs; INSERT INTO drugs...` with `INSERT OR REPLACE INTO drugs...` for the drugs table. Truncate is fine for `presentations`, `compositions`, `generic_groups` (all derived from current CIS). Only drugs needs upsert to preserve historical CIS that appear in SMR/ASMR/GENER.

**How hard to fix now vs later:** NOW: change 3 lines in the import orchestrator (delete truncate, add upsert for drugs). LATER: you have months of truncated history. You can recover from archived raw files if you kept them (external recommendation: always archive), but it's additional work. The fix is easier now.

---

### V5: URL Pattern — Download Path May Have Changed Since Initial Analysis

**Failure mode:** External review claims the download URL pattern changed from `/telechargement?fich=XXX` to `/download/file/XXX`. If the fetcher uses the old URL pattern, every download returns an HTML page instead of a TSV file. The parser tries to tab-split HTML. Row count assertions fail. No data imported. CI fails.

**What it looks like:**
```
# Fetch CIS_bdpm.txt from https://base-donnees-publique.medicaments.gouv.fr/telechargement?fich=CIS_bdpm
# Returns: "<html><body>Page not found</body></html>" (404 or redirect page)
# BLAKE3 hash computed on HTML
# Hash differs from stored hash (or hash computed on empty/corrupt data)
# Import begins...
# Tab parser: 0 tab characters found
# Field count: 1
# Row count assertion fails: expected 15848, got 1
# CI: FAILED
```

**Is it addressed in our plan?** No. 01-01 has hardcoded base URL but no verification that the pattern works. The external review says this needs live server verification. We cannot verify from raw files alone.

**Fix:** Before writing the fetcher code, add a verification step: try fetching a known file with the expected URL pattern, verify it returns tab-delimited content with correct field count. If the pattern fails, update the URL. Keep the base URL configurable (not hardcoded) so a pattern change is a config update, not a code change.

**How hard to fix now vs later:** NOW: if the URL is wrong, you catch it in the first run and fix it. LATER: if you write the full pipeline assuming the URL works, you have a broken tool the first time you run it in production. Add the verification check as part of Task 3 in 01-01.

---

## ASSUMPTIONS MOST LIKELY TO BECOME WRONG

### A1: Data Characteristics Will Drift Between Now and Phase 1 Execution

**Assumption:** All 11 files have stable, known characteristics (field count, encoding, price formats, date formats, edge cases).

**What will change:**
- The BDPM could publish a new file version with different field count, encoding, or new edge cases
- New price formats could appear (e.g., prices over 999,999 with 3 commas)
- New generic type values could appear (we know types 0/1/2/4 but 3/5/6 are possible)
- avis field could grow beyond 2048 chars
- EAN normalization (strip 34009 prefix) works on current data but could fail on non-French codes

**Mitigation already in plan:** Field-count guard (01-02) fires on schema drift. Row-count assertions (01-03) catch structural changes. The plan does NOT have: a schema version check, notification when known edge cases change, or a way to surface "new anomaly detected" to the operator.

**Recommendation:** Add a `schema_version` field to `import_log`. If the hardcoded `field_count` for a file differs from actual, log a CRITICAL warning with the actual count. Don't silently accept drift — alert the operator.

---

### A2: Monthly Cadence Means Rare Edge Cases Won't Be Caught During Development

**Assumption:** 120 file downloads per year (10 monthly + 2 weekly). We'll see all edge cases during development from the current files.

**What will actually happen:** The current files are a snapshot. If a specific edge case only appears once every 2 years (e.g., a drug with 3 prices, a CIP with non-numeric characters, a date with a 4-digit year), we won't encounter it during the 2-month Phase 1 build. We only find it when it appears in production and the import fails.

**Risk:** The plan embeds specific edge cases (466 thousands-separator prices, 2,254 populated F8 fields in CIS_bdpm, 18 tab-split malformed rows) as known. But there may be edge cases we haven't discovered.

**Recommendation:** Archive every downloaded version (external review explicitly recommends this: "archiver les fichiers bruts, ne jamais supprimer"). On each run, compare current file to previous version — structural changes surface automatically. The cost of archiving is low (compressed TSV is small); the value of version history is high.

---

### A3: BLAKE3 Hash + Size is Sufficient Change Detection

**Assumption:** BLAKE3 hash computed on downloaded content is definitive. If content changed, hash differs. If hash matches, content is identical.

**What could go wrong:**
- Server could serve compressed content (gzip) — BLAKE3 computed on compressed bytes is stable, but if the server starts serving uncompressed (or vice versa), hash changes even though data is identical. The `Content-Encoding` header isn't part of our hash but it's part of the download.
- File could be regenerated with identical content but different whitespace, comments, or metadata (newline at end of file added/removed)
- Encoding changes after download don't affect hash (we hash bytes, not decoded text), but could produce different decoded data

**Risk:** We might re-import files that are semantically unchanged but byte-different. The re-import is fast (seconds for 32K rows) so it's not a correctness problem, but it generates spurious import_log entries and runs the full normalization pipeline for no reason.

**Recommendation:** Accept this. The cost of a spurious re-import (seconds) is far less than the cost of a missed change (stale data). BLAKE3 is the right signal.

---

### A4: 11 Tables, All Well-Defined, No Missing Relationships

**Assumption:** The 11 BDPM files cover all the drug information we need. There are no cross-file relationships we haven't discovered.

**What the external reviews say:** The betagouv/infomedicament project references a MySQL dump with additional data (codes ATC, images, RCP texts) not in the TSV files. We don't know what's in that dump or if we need it.

**Assumption failure:** If we build the full Phase 2 API and users ask "where is the drug image?" or "where is the RCP text?", we have no data for it. The 11 files might be a subset of the full BDPM dataset.

**Recommendation:** Acknowledge in BRIEF.md that the 11-file scope is an operational assumption. If future API requirements need additional data, a separate research task (accessing the MySQL dump, scraping ANSM website) would be needed. Don't over-engineer now.

---

### A5: CI Will Be Available and Reliable (GitHub Actions)

**Assumption:** GitHub Actions runners are always available, artifacts persist, workflows trigger on schedule.

**What could go wrong:**
- GitHub Actions has had outages (December 2022, August 2023, May 2024)
- Runner minutes might be exhausted on free tier
- Artifacts expire after 90 days (free tier) or 500 days (paid)
- Workflow triggers might miss a scheduled run

**Recommendation:** The plan mentions committing state to a `state/` branch as an alternative to artifacts. Implement this early — the state branch is more durable than workflow artifacts and survives GitHub outages. Add a manual `workflow_dispatch` trigger as a fallback.

---

## SUMMARY: CRITICAL PATH ITEMS BEFORE PHASE 1 BEGINS

| Priority | Item | What breaks if skipped |
|----------|------|----------------------|
| 1 | Fix encoding: Windows-1252 for 7 files + smart quote normalization | 52K apostrophe chars corrupted in SMR/ASMR |
| 2 | Fix Dispo CHECK: IN (1,2,3,4) not (1,4) | 57% of availability records dropped |
| 3 | Fix orphan handling: PRAGMA foreign_keys=OFF during import | Monthly import fails entirely |
| 4 | Fix sync strategy: upsert drugs, truncate dependents only | Historical drug metadata for withdrawn drugs lost |
| 5 | Verify download URL pattern against live server | Fetcher downloads HTML, not data |

These are not refinements — they are correctness issues. Skipping any of them means Phase 1 produces a broken database. All five are low-cost to fix now (a few lines each). All five are high-cost to fix later (data migration, re-import, debugging production failures).

**Assumptions to lock down:**
- Archive every raw file version (enables structural drift detection)
- Add schema_version + structural drift alert to import_log
- Use state branch (not artifacts) for cross-run state persistence