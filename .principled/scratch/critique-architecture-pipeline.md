# Architecture & Pipeline Critique — External vs. Internal Plan

**Comparing:** External reviews (feasibility study, technical analysis, format doc, Python script) against internal plan (BRIEF, ROADMAP, 01-01, 01-03, 01-05)

**Scope:** Architecture, sync strategy, change detection, FTS5, config, CI timing, CIS_InfoImportantes

---

## CRITICAL (must fix before execution)

### C1. Sync Strategy: Full-table truncate destroys historical HAS references

**What external says:**
- Feasibility study: incremental import model with soft-delete (`is_active = 0`) is **essential** to preserve historical references to withdrawn drugs
- "Historical data loss on full reload: **HIGH** likelihood, **HIGH** impact"
- CIS_bdpm.txt retains only drugs marketed or discontinued within the last 2 years
- 18.4% of CIS_HAS_SMR, 15.8% of CIS_HAS_ASMR, 23.5% of CIS_GENER reference withdrawn drugs absent from CIS_bdpm
- Feasibility body recommends: "Insert phase → Update phase → Delete phase (soft-delete records present in DB but absent from new file)"

**What our plan says:**
- BRIEF.md: "Full-Table Truncate+Reload, Not Row-Level Delta"
- "No row-level timestamps exist in any BDPM file. Row-level delta is impossible."
- "For 32K-row tables this completes in seconds monthly."

**Analysis:**
The external concern is valid. If we truncate `drugs` table on each monthly sync, withdrawn drugs disappear, breaking FK references from SMR/ASMR tables. Even with `PRAGMA foreign_keys=OFF`, orphaned references create query failures and data integrity issues.

**However:** The external reviews treat this as catastrophic, but our architecture has a mitigating factor that the external analysis does not fully account for:

1. SMR/ASMR are stored in **separate tables** with their own rowids
2. Truncating `drugs` does NOT delete rows in `smr`/`asmr` tables — SQLite truncate is table-level, not cascade
3. The issue is referential integrity (FK checks fail on queries), not data loss

**Recommendation:** Reject full truncate+reload for `drugs` table. Use upsert (INSERT OR REPLACE) for `drugs`, which preserves rows whose CIS unchanged and updates only changed/new records. This is the actual "incremental" behavior we need without row-level timestamps.

For `presentations`, `compositions`, `generic_groups` — truncate+reload is acceptable because:
- These are all derived from current CIS set
- Missing presentation for withdrawn drug is correct behavior
- No historical references to these tables from other tables

**Action required:** Modify 01-05 plan to use upsert for `drugs` table instead of DELETE+INSERT. Keep truncate+reload for dependent tables.

---

### C2. FTS5 standalone vs content= (already fixed in Python version)

**What external says:**
- Feasibility body: "The index FTS5 with `content=` and `content_rowid=` provoked a corruption SQLite (`database disk image is malformed`) during synchronization"
- Solution: "Table FTS5 standalone without `content=` — manual synchronization via `sync_fts5()`"

**What our plan says:**
- 01-05 plan: Does not yet specify FTS5 implementation
- BRIEF.md schema: No FTS5 definition in schema section
- ROADMAP Phase 2: "02-01 FTS5 drug name search"

**Analysis:**
The external Python implementation already hit this corruption bug and fixed it. Our plan defers FTS5 to Phase 2, which is fine, but the implementation must follow the standalone pattern — NOT `content=` tables.

**Recommendation:** In Phase 2, use standalone FTS5 virtual tables with explicit `INSERT INTO fts_table SELECT ...` synchronization after each import. Document the `content=` anti-pattern explicitly.

---

## WARNING (should fix, non-blocking)

### W1. Change detection: BLAKE3 sufficient, but data.gouv.fr API missing

**What external says:**
- Technical analysis: "three-layer monitoring (SHA-256 hash primary, data.gouv.fr API secondary, HTML scraping fallback)"
- "No ETag/Last-Modified — conditional HTTP requests impossible"
- HTML scraping recommended as a secondary detection layer

**What our plan says:**
- 01-01: BLAKE3 hash is authoritative signal
- BRIEF.md: "BLAKE3 hash is the only authoritative change signal — computed post-download"
- Content-Length is optimization only

**Analysis:**
BLAKE3 vs SHA-256: Both are cryptographic hashes; BLAKE3 is faster (4-10x) but functionally equivalent for content identity. Our choice is sound.

The "data.gouv.fr API" mentioned in external reviews refers to France's open data portal API — a separate endpoint from the BDPM site. This could provide independent confirmation of updates. However:
- Adds external dependency (data.gouv.fr availability)
- Would require additional parsing of API responses
- BLAKE3 on downloaded content is already definitive

**Recommendation:** Reject three-layer monitoring as over-engineering for solo dev. BLAKE3 on download is sufficient and authoritative. If server returns different content, BLAKE3 catches it. Consider data.gouv.fr API only if monitoring shows frequent false positives (downloaded but unchanged).

---

### W2. CIS_InfoImportantes deferred to Phase 3.5 — clarify scope

**What external says:**
- Feasibility body: "Dynamically generated with timestamp in filename (e.g., CIS_InfoImportantes_20260526111334_bdpm.txt)"
- "Generated on-demand, 6-hour TTL"
- Feasibility study: "Requires dedicated scraping with TTL cache"

**What our plan says:**
- ROADMAP: Phase 3.5 Safety Data deferred
- BRIEF.md: "excluded from v1 — safety-critical, requires dedicated scraping with TTL cache"

**Analysis:**
The deferral is correct. CIS_InfoImportantes is special-cased (dynamic filename, on-demand generation, TTL semantics) and requires different infrastructure than the other 10 files.

**However:** The "6-hour TTL cache" needs explicit implementation. What does this mean?
1. Scrape on first access, cache result
2. After 6 hours, re-scrape on next access
3. Never proactively poll (unlike other 10 files)

**Recommendation:** Define Phase 3.5 scope as:
- Scrape CIS_InfoImportantes on-demand (never scheduled)
- Store with timestamp
- Return cached result if age < 6 hours
- Re-scrape if age >= 6 hours
- Flag staleness in API response

---

### W3. CI regression tests should start earlier than Phase 4

**What external says:**
- Feasibility study: "CI must catch field-count changes before production re-import"
- Technical analysis: "When field count changes, CI fails before production re-import"

**What our plan says:**
- 01-01: No CI tests
- 01-03: "field-count regression tests pass on current files; would fail on schema drift"
- ROADMAP: Phase 4 = "CI regression suite"

**Analysis:**
Our plan already has CI-quality tests embedded in 01-03 (field-count regression, row-count assertions). The ROADMAP naming "Phase 4 CI" refers to GitHub Actions workflow setup, not test authoring. The tests themselves are defined in 01-03.

**Recommendation:** Clarify in ROADMAP that "Phase 4 CI" = GitHub Actions workflow setup. Test authoring is already in Phase 1. This is a documentation clarity issue, not a structural problem.

---

## SUGGESTION (nice-to-have improvement)

### S1. Config file vs CLI flags

**What external says:**
- Feasibility body: "bdpm.toml configuration file" with sections for source, fetch, parse, database, validation
- etude_faisabilite: recommends `bdpm.toml` with user_agent, request_timeout_sec, retry_max, batch_size

**What our plan says:**
- 01-01: CLI commands accept `--data-dir <PATH>`
- No config file; all parameters via CLI flags

**Analysis:**
For a solo developer, CLI flags are sufficient and simpler. However, a minimal config file would help for:
- Persisting user_agent across runs (don't type each time)
- Non-standard polling intervals per file
- Database path (no --data-dir on every command)

**Recommendation:** Not required for v1. Add `bdpm.toml` optional override in Phase 2 or 3 when the tool matures. Start with CLI flags, add config as additive.

---

### S2. Architecture: single crate vs 5-crate workspace

**What external says:**
- Feasibility study: 5-crate workspace (bdpm-core, bdpm-fetch, bdpm-parse, bdpm-validate, bdpm-db)
- Technical analysis: Similar module structure
- etude_faisabilite: "crates/bdpm-core, bdpm-fetch, bdpm-parse, bdpm-validate, bdpm-db, bdpm-cli"

**What our plan says:**
- Single Rust crate `bdpm-ingest`
- "Stack: ureq (sync HTTP) + rusqlite (sync SQLite) + rouille (sync HTTP API in Phase 2)"
- No tokio, no async runtime

**Analysis:**
The external reviews propose a 5-crate workspace, which is appropriate for a multi-team or production project with clear interface boundaries. For a solo developer:
- 5-crate adds compilation overhead and complexity
- Internal module organization achieves similar separation
- `src/fetch/`, `src/parse/`, `src/db/`, `src/import/` already provide logical separation

**Recommendation:** Stay with single crate. If project grows to warrant multi-crate (e.g., library crate published separately), extract later. Premature extraction is worse than delayed extraction.

---

## CONFIRMED (external analysis confirms our approach is correct)

### CO1. Encoding handling: encoding_rs, Windows-1252 first

**What external says:**
- Feasibility study: "Encoding is declared in static configuration per file (hardcoded, not dynamically detected). encoding_rs provides zero-copy decoders."
- Technical analysis: "9 files Latin-1/Windows-1252, 2 files UTF-8. Always attempt Windows-1252 first; superset includes Windows-specific characters."
- Format doc: "Use `windows-1252` (CP1252) explicitly. Do not autodetect."

**What our plan says:**
- 01-01 dependencies: `encoding_rs = "0.25"`
- 01-01: "BDPM files arrive as raw bytes in two encodings. `std::str::from_utf8` fails on Latin-1. This was missing from the original plan."

**Verification:** External confirms our encoding handling approach is correct. encoding_rs with hardcoded per-file encoding is the right strategy.

---

### CO2. Synchronous stack: ureq over reqwest+async

**What external says:**
- Feasibility study: "async for fetch, sync for parsing"
- etude_faisabilite: "sync pour le parsing, async pour le fetch"
- Technical analysis: "reqwest with tokio runtime"

**What our plan says:**
- 01-01: "No tokio: SQLite is synchronous. rusqlite is synchronous. The CLI downloads 10 files monthly. Async adds ~60s compile time + ~4MB binary size for zero benefit."
- "ureq over reqwest: ureq is synchronous. 1/5 the compile time of reqwest, no async runtime."

**Analysis:**
The external reviews recommend async for HTTP, but our plan correctly identifies that sync is sufficient for this use case:
- 10 files monthly = 120 downloads/year
- 5s rate limit between requests
- No concurrent streaming needed
- ureq is 1/5 the compile time of reqwest

**Verification:** Our synchronous approach is correct. Async would be over-engineering.

---

### CO3. Price normalization: integer cents over float

**What external says:**
- Technical analysis: "prices as `24,34`; critical: 466 rows with values >1000 use comma as thousands separator (`1,466,29` → must remove both commas, not replace)"
- Feasibility body: "Convert to REAL (f64)" but notes precision concerns

**What our plan says:**
- BRIEF.md: "Decision: Price → Integer Cents, Not Float Euros"
- "24,34` → 2434 (cents). Integer arithmetic avoids floating-point errors."
- 01-05: `prix_ht_cents INTEGER`

**Verification:** Our approach (integer cents) is superior to float. Floating-point precision errors in financial calculations are real. Integer cents is the correct choice.

---

### CO4. HTML stripping for SMR/ASMR avis

**What external says:**
- Format doc: "Fields INDICATIONS and COMPOSITION may contain embedded line breaks"
- Technical analysis: "4,031 rows contain HTML `<br>` tags in avis field (13% SMR, 21% ASMR)"
- Feasibility body: "Strip HTML on store, preserve text content for API output"

**What our plan says:**
- BRIEF.md: "Decision: strip HTML on store, preserve text content for API output"
- 01-03: "HTML stripping tests" defined

**Verification:** HTML stripping is correctly identified as necessary and our plan includes it.

---

### CO5. Smart quote normalization (U+2019 → U+0027)

**What external says:**
- Feasibility study: "0x92 (right single quotation mark, U+2019) appears massivement in HAS files: 29,704 occurrences in CIS_HAS_ASMR and 22,253 in CIS_HAS_SMR. Normalize to straight apostrophe."
- Technical analysis: "Critical: Replace `\u{2019}` (right single quotation mark) with standard apostrophe `'` after decoding Windows-1252 files"

**What our plan says:**
- 01-04: Normalization pipeline (deferred)
- BRIEF.md edge cases: Not explicitly listed in edge cases section

**Verification:** Smart quote normalization is confirmed as required. The external analysis discovered this is the single most impactful encoding issue.

---

### CO6. Date format normalization (DD/MM/YYYY + YYYYMMDD → ISO-8601)

**What external says:**
- Format doc: "Date fields use DD/MM/YYYY (French convention), not ISO 8601"
- Feasibility study: "Convert DD/MM/YYYY and YYYYMMDD to YYYY-MM-DD (ISO 8601)"
- Technical analysis: "Three date formats: DD/MM/YYYY, YYYYMMDD, YYYY-MM-DD — normalize all to ISO 8601"

**What our plan says:**
- BRIEF.md: "Decision: Dates Converted to ISO-8601 on Ingest"
- 01-05: Date normalization to ISO-8601

**Verification:** ISO-8601 normalization is correctly planned.

---

### CO7. Orphan handling: insert with flag, don't reject

**What external says:**
- Feasibility study: "Orphan CIS references: ~18% SMR, ~16% ASMR, ~23% GENER. Handle via nullable FKs or orphan flag — do not reject."
- Feasibility body: "Insert orphans with is_orphan=1 flag rather than rejecting"

**What our plan says:**
- BRIEF.md: No explicit orphan handling strategy
- Schema: FK references from secondary tables to drugs(cis)

**Verification:** Our current plan doesn't address orphans explicitly. SMR/ASMR/GENER tables will have FK constraints that fail on orphan CIS. We need to either:
1. Disable FK for orphan inserts
2. Insert orphans into drugs table with a `withdrawn` flag
3. Use `INSERT OR IGNORE` with `PRAGMA foreign_keys=OFF`

**Recommendation:** Use `PRAGMA foreign_keys=OFF` during import OR use nullable FK references. Do not reject orphan CIS — historical evaluations are valid data.

---

## Summary Table

| Finding | Category | External Position | Internal Position | Resolution |
|---------|----------|-------------------|-------------------|------------|
| Sync strategy | CRITICAL | Incremental with soft-delete | Full truncate+reload | Use upsert for `drugs`, keep truncate for dependents |
| FTS5 corruption | CRITICAL | Standalone FTS5 | Not yet designed | Deferred to Phase 2, follow external fix |
| Change detection | WARNING | BLAKE3 sufficient | BLAKE3+Content-Length | Confirmed correct; reject three-layer monitoring |
| InfoImportantes | WARNING | Defer but clarify TTL | Phase 3.5 deferred | Define on-demand + 6-hour TTL semantics |
| CI timing | WARNING | Earlier than Phase 4 | Phase 4 = workflow | Clarify: tests in Phase 1, workflow in Phase 4 |
| Config file | SUGGESTION | bdpm.toml recommended | CLI flags only | Nice-to-have, add later |
| 5-crate vs single | SUGGESTION | 5-crate recommended | Single crate | Confirmed: single crate for solo dev |
| Encoding | CONFIRMED | encoding_rs + hardcoded | Same | Confirmed correct |
| Sync stack | CONFIRMED | Mixed (async fetch) | Pure sync | Confirmed: sync sufficient |
| Price cents | CONFIRMED | Float is risky | Integer cents | Confirmed correct |
| HTML stripping | CONFIRMED | Required | Included | Confirmed correct |
| Smart quotes | CONFIRMED | 0x92 → apostrophe | Not explicit | Confirmed required, add to normalization |
| Date ISO-8601 | CONFIRMED | Normalize both formats | Included | Confirmed correct |
| Orphan handling | CONFIRMED | Insert with flag | Not addressed | Add `PRAGMA foreign_keys=OFF` during import |

---

**Files referenced in this critique:**
- External: `/home/devadmin/Desktop/BDMP_DB/.principled/scratch/external-format_doc.md`, `external-analyse_technique.md`, `external-feasibility_body.md`, `external-etude_faisabilite.md`
- Internal: `/home/devadmin/Desktop/BDMP_DB/.principled/plans/BRIEF.md`, `ROADMAP.md`, `phases/01-foundation/01-01-PLAN.md`, `01-03-PLAN.md`, `01-05-PLAN.md`