# Self-Critique: Challenging Our Own Proposed Fixes

**Date:** 2026-05-26
**Scope:** 5 of the 7 CRITICAL proposed fixes
**Method:** Each fix challenged on correctness, sufficiency, and proportionality before accepting

---

## Challenge 1: "Add Windows1252 encoding"

**Proposed fix:** Use `encoding_rs::WINDOWS_1252` for 7 files instead of ISO-8859-1, specifically because 0x92 bytes in SMR/ASMR files produce U+2019 (curly apostrophe) under cp1252 vs U+0092 (invisible control char) under latin-1.

### The challenge

If Fix 2 normalizes U+2019 → U+0027 anyway, does it matter whether the decode produced U+2019 or U+0092? We strip both to straight apostrophe in the next pipeline step. Adding a third encoding variant increases normalization surface area for a character we're discarding.

### Is this a valid concern? **PARTIALLY**

The concern is valid but the conclusion is wrong.

The proposed challenge overestimates the cost. The encoding difference matters even when we subsequently normalize, for three reasons:

1. **Raw text inspection / logging**: When we dump a parsed record for debugging, U+0092 prints as an invisible blank while U+2019 prints as ' — this matters during development when tracing why search is broken.

2. **Subsequent string operations**: A U+0092 control character embedded in a drug name could cause silent failures in regex patterns, Unicode normalization (NFC/NFD), or SQL LIKE clauses. Stripping it is not the same as never encountering it.

3. **Code clarity**: Using ISO-8859-1 for files that contain Windows-1252 codepoints is semantically incorrect even if the final output is identical after normalization. The encoding choice documents intent.

**However**, the framing in the challenge is important — adding `Windows1252` as a third variant to the `Encoding` enum is disproportionate if the only downstream effect is that U+2019 gets normalized anyway. The real question is whether the normalization step (Fix 2) should happen before or after storage.

### Verdict

**The proposed fix is still correct**, but for the right reason: honest encoding. `encoding_rs::WINDOWS_1252` is the correct decoder for these files because that's what they actually contain, regardless of what happens downstream. The normalization step does not retroactively fix the decoding step.

**Better framing for the fix:** "Files contain Windows-1252 bytes. Decode as Windows-1252. Normalization is a separate, optional step for display/search purposes." This keeps the two concerns separate.

---

## Challenge 2: "Normalize U+2019 → U+0027"

**Proposed fix:** After Windows-1252 decoding, replace all U+2019 (right single quotation mark) with U+0027 (straight apostrophe) before storage.

### The challenge

This changes data. What if downstream consumers (the API, a frontend, a PDF generator) need the original curly apostrophe for display? Are we losing typographic fidelity for all use cases?

### Is this a valid concern? **YES**

The concern is valid, but the fix is still correct as a default with a caveat.

**Why the concern is valid:**
- U+2019 → U+0027 is not lossless. It's intentional data transformation.
- Downstream display systems (web pages, PDFs, mobile UIs) may render U+0027 identically to U+2019, but semantically the distinction is lost.
- If HAS ever changes their documentation styles (e.g., starts releasing data with curly quotes for typographic reasons in ALL fields), our normalization would systematically alter their data.

**Why the fix is still correct as DefaultStorageBehavior:**
This is a pharmaceutical ingestion pipeline, not a typography preservation system. The primary use case is search, analysis, and lookup. For these, apostrophe equivalence is correct:

- `Doliprane®` is semantically the same whether stored as `Doliprane\u{2019}s` or `Doliprane\u{0027}s`
- A search for `Doliprane's` should match both
- Database storage with straight apostrophes avoids cross-system Unicode normalization issues

**The right design:** Store with normalization (straight apostrophe) as the default, document it explicitly, and provide a raw-field accessor for cases needing the original byte sequence.

**Better fix:** Not a schema change, but a policy decision. The implementation should expose this normalization transparently — e.g., "this field has been normalized to Unicode NFC with straight apostrophes" — so consumers who care can opt into raw access.

### Verdict

**Fix approved**, but add explicit documentation. The normalization is correct for the use case. The concern about fidelity is valid but doesn't apply to this pipeline's primary consumers (search and analysis).

---

## Challenge 3: "CHECK IN (1,2,3,4)"

**Proposed fix:** Change the CHECK constraint on `dispo_status_code` from `CHECK IN (1, 4)` to `CHECK IN (1, 2, 3, 4)`.

### The challenge

Is CHECK actually enforced in SQLite? Does violating the CHECK cause an error, or does it silently allow the insert and move on? If CHECK doesn't actually prevent invalid rows, the fix is largely cosmetic — the real validation must happen in application code.

### Is this a valid concern? **YES, AND IT REVEALS A DEEPER ISSUE**

The concern is valid and the proposed fix is insufficient without a companion fix.

**SQLite CHECK constraint reality:** In SQLite, a CHECK constraint produces an error **at INSERT/UPDATE time when violated**. The database will reject the row. This is different from NOT NULL or UNIQUE in some edge cases.

From the SQLite documentation: `CHECK` constraints are enforced at row居室-time. `INSERT OR REPLACE` that violates a CHECK fires an error, not a silent replacement. So our import code, if it uses standard SQL inserts, will fail with an error for status codes 2 and 3 — not silently drop them.

**This is actually worse than "silently drops 57%"**: it would hard-fail the entire import run when it encounters the first status code 2 or 3 row. Our code would have to catch that error, roll back, and skip those rows — which is functionally equivalent to silently dropping them, but with more ceremony.

**The deeper issue:** SQLite CHECK constraints produce `SQLITE_CONSTRAINT_CHECK` errors. Our rusqlite error handling must explicitly catch these. If we use `PRAGMA foreign_keys=OFF` during import (Fix 4), we also need to ensure the CHECK constraint is either enforced correctly or deferred appropriately.

**The real fix needs two parts:**
1. Change CHECK constraint to `CHECK IN (1, 2, 3, 4)` (as proposed)
2. Ensure rusqlite error handling routes CHECK constraint violations gracefully — either by skipping invalid rows or by logging and continuing

### Verdict

**The proposed fix is correct but incomplete.** The CHECK change is necessary, but the rusqlite error handling for CHECK violations must also be specified in the import pipeline. Without both parts, the import will hard-fail on valid data.

### Better fix

Add both:
```rust
// Schema
dispo_status_code SMALLINT NOT NULL CHECK (dispo_status_code IN (1, 2, 3, 4))

// Import error handling
match db.execute(&sql, params) {
    Ok(_) => {},
    Err(rusqlite::Error::SqliteFailure(code, msg))
        if msg.contains("CHECK constraint failed") => {
            // Log and skip — valid data from source but outside our domain
            ewarn!("Skipping row with unrecognized status code");
        }
    Err(e) => return Err(e.into()),
}
```

---

## Challenge 4: "PRAGMA foreign_keys=OFF during import"

**Proposed fix:** Disable SQLite foreign key enforcement during the import phase to allow insertion of SMR/ASMR/GENER rows that reference CIS codes not present in `drugs`.

### The challenge

If we disable FK checks, what prevents us from inserting rows with invalid CIS references? Are we trading database-enforced referential integrity for simple accommodation of known-bad data? What is the compensating control?

### Is this a valid concern? **YES, BUT THE TRADE-OFF IS JUSTIFIED**

The concern is valid — disabling FK enforcement is a real safety reduction. But the trade-off is justified for a specific reason the audit implicitly acknowledges: these are **not invalid references**, they are **valid historical references** to withdrawn drugs. The FK constraint is the wrong tool for this job.

**Why the trade-off is justified:**
1. SMR/ASMR/GENER evaluations are historical records attached to a CIS at the time of evaluation. If the drug has since been withdrawn from the market, the evaluation is still valid and still needs to be queryable.
2. The orphan status is not a bug in the data — it's an expected consequence of the BDPM data lifecycle (withdrawn drugs are not re-exported in CIS_bdpm.txt).
3. FK constraints enforce referential integrity for operational data. Historical analytics data with known orphan patterns does not fit the FK model.

**Why this doesn't trade safety for accommodation:**
- We are not ignoring invalid references. We are explicitly handling valid historical references.
- The import pipeline should explicitly track orphan insertions (e.g., log when we insert a row for a CIS not in `drugs`, or mark it with an `is_orphan=1` flag).
- The alternative (nullable FKs) has its own problems: nullable FKs prevent SQLite from enforcing constraints for LEFT JOIN patterns, and they shift the burden of checking to every query.

**However, one real risk remains:** `PRAGMA foreign_keys=OFF` disables ALL FK enforcement during the import transaction. This means if our import code produces a genuinely bad reference (wrong digit count, typo), SQLite won't catch it. The application-level compensating control must be explicit validation that the CIS exists in `drugs` at the time of query, not at the time of insert.

### Verdict

**Fix approved with caveat.** The orphan case is real and well-justified. The risk (missed validation) is manageable because orphaned CIS refs are the expected case, not an error case. Document that FK is disabled intentionally because these are historical references, not invalid data.

### Add to the fix

After import completion, validate consistency via:
```sql
-- Post-import consistency check (informational, not enforced)
SELECT COUNT(*) FROM smr WHERE cis NOT IN (SELECT cis FROM drugs) AS orphaned_rows;
```
Log the orphan count. If it jumps unexpectedly, investigate — but don't hard-fail, since orphan patterns are normal for this dataset.

---

## Challenge 5: "Upsert drugs instead of truncate"

**Proposed fix:** Change `DELETE FROM drugs; INSERT INTO drugs SELECT ...` (truncate+reload) to `INSERT OR REPLACE INTO drugs SELECT ...` (upsert) on the `drugs` table. Keeps existing CIS rows intact when the CIS has not changed, only updates changed/new records, preserves FK references from SMR/ASMR.

### The challenge

`INSERT OR REPLACE` in SQLite is syntactic sugar for `DELETE` followed by `INSERT` — the DELETE runs first and fires any DELETE triggers and FK cascades. If `drugs` has a surrogate auto-increment primary key, this would actually delete the old row (triggering cascades from dependent tables) and insert a new row with a new key, breaking FK references.

### Is this a valid concern? **YES, AND THIS EXPOSES A SCHEMA-SENSITIVE ISSUE**

The concern is valid and the proposed fix is **conditionally correct** — correct only if the schema uses `CIS` (the natural key from the source file) as the primary key. The fix's correctness depends on design decisions not yet verified.

**Scenario A — `drugs(cis TEXT PRIMARY KEY)` (natural key):**
`INSERT OR REPLACE INTO drugs VALUES (...)` preserves the existing row — no DELETE fires, the ON CONFLICT update path is taken, and `cis` remains the same. No FKs cascade. The fix is correct and achieves the goal.

**Scenario B — `drugs(id INTEGER PRIMARY KEY AUTOINCREMENT, cis TEXT UNIQUE)` (surrogate key):**
`INSERT OR REPLACE INTO drugs VALUES (:id, ...)` with `ON CONFLICT(cis)` would:
1. DELETE the existing row (getting the old `id`)
2. Potentially fire FK cascades referencing (or just relying on) the old `drugs.id`
3. INSERT a new row with a new `id`

This actually breaks SMR/ASMR references if they store `drugs.id` as a FK.

**Scenario C — No change at all:**
If SMR/ASMR/GENER tables don't have FK constraints (we disabled them for orphans), then `INSERT OR REPLACE` is safe regardless of key strategy because no constraint enforcement happens at insert time.

**The critical question NOT answered in the audit:** Does the `drugs` table have a surrogate auto-increment key, or does it use `cis` as the primary key? If `cis` is already the primary key, `INSERT OR REPLACE` is safe. If a surrogate key exists, the surrogate ID is what SMR/ASMR/GENER FKs reference by design, and in that case `INSERT OR REPLACE` is NOT safe.

### What the fix actually needs to specify

The fix as written in the audit is conceptually right but incomplete. It needs to specify:

1. The `drugs` table must use `cis TEXT PRIMARY KEY` — no surrogate auto-increment key
2. If using `INSERT OR REPLACE`, verify `ON CONFLICT` resolves to update path (not delete+insert)
3. Alternatively, use a true upsert that explicitly does UPDATE first, INSERT only when needed:

```sql
-- Option 1: True upsert (safer, if surrogate key exists)
UPDATE drugs SET col1 = :col1, col2 = :col2 WHERE cis = :cis;
INSERT INTO drugs SELECT :cis, :col1, :col2
  WHERE NOT EXISTS (SELECT 1 FROM drugs WHERE cis = :cis);
```

### Verdict

**Fix is conceptually correct but requires schema commitment.** It is only safe if `drugs` uses `cis` as primary key, no surrogate auto-increment. If a surrogate key exists, `INSERT OR REPLACE` actually causes the very FK cascade it was supposed to prevent. Add explicit schema design to this fix.

---

## Summary: Fix Assessment

| Challenge | Valid Concern? | Fix Still Correct? | What Needs Adding |
|-----------|---------------|-------------------|-------------------|
| Fix 1: Windows-1252 encoding | Partially | Yes (for honest decoding) | Update framing — encoding correctness is independent of normalization |
| Fix 2: U+2019 → U+0027 | Yes | Yes (for this pipeline) | Explicit policy documentation — normalization is intentional, not accidental |
| Fix 3: CHECK IN (1,2,3,4) | Yes — reveals error handling gap | Correct but incomplete | rusqlite error handling for CHECK violations must be specified |
| Fix 4: PRAGMA foreign_keys=OFF | Yes | Yes (justified trade-off) | Post-import consistency check; explicit orphan logging |
| Fix 5: Upsert vs truncate | Yes — schema-sensitive | Conditionally yes | Must verify `drugs` uses `cis` as primary key (no surrogate autoincrement); provide true upsert SQL as failsafe |

---

## Overarching Pattern

Four of the five fixes are conceptually correct but underspecified — they describe the intended behavior without fully constraining the implementation details that determine whether the fix actually works. The self-review process revealed:

1. **Fix 3** exposes a missing rusqlite error-handling specification
2. **Fix 5** exposes an unverified schema assumption

Both of these would cause the implementation to silently fail or produce wrong results — not at the planning level but in the actual code generation step. They need to be refined before the plans are considered ready for implementation.

The non-technical fix (Fix 1 reframing) is less critical but still worth separating encoding correctness from downstream normalization, so future changes to the normalization step don't create pressure to loosen the encoding specification.
