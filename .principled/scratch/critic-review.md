# Architecture Critique — BDPM Rust Project

## Preamble

These artifacts show competent preparation. The feasibility study is thorough — encoding split, Windows-1252 residuals, European decimal quirks, multi-row edge cases are all documented. The BRIEF.md makes good decisions and owns its assumptions explicitly. However, three fundamental problems run through the work: (1) the update detection strategy conflates two separate signals and builds a fragile dependency chain around it, (2) the phased structure hides critical cross-file validation dependencies inside individual phases rather than surfacing them as explicit gates, and (3) the schema design has five fields where the proposed type is wrong or the null semantics are ambiguous under load.

---

## Classification: CRITICAL

### C1: Content-Length is not a change signal — it is a hint, and treating it as one is fine; treating it as two is not

The plan says: "compare byte sizes before processing. If unchanged, skip re-import with a hash check."

The phrase "Content-Length comparison → if changed: ... → SHA256 → compare to stored hash" appears in both BRIEF.md and the Update Pipeline section.

This conflates two logically distinct operations:

- Content-Length is consulted first as a cheap pre-filter before downloading.
- SHA256 is computed only after download, as the authoritative change detector.

The problem is that **Content-Length is not a change signal**. The BDPM server returns no ETag, no Last-Modified. Content-Length is a response header that tells you how big the response body is. A file whose content has changed can have the same byte size. Conversely, a file padded with whitespace or whose encoding has shifted slightly could change size without meaningful content change. Using Content-Length as a gate before download is reasonable as an optimization (don't download what hasn't changed in size). But it must never be the authoritative comparison — SHA256 is that authority, and SHA256 must be computed after download regardless.

The plan says: "If hash matches previous, log as 'unchanged' and exit." That's correct for the post-download path. But the pre-download Content-Length gate needs to be described as what it is: an optimization, not a correctness mechanism. If the plan is implemented literally with Content-Length as the gating condition, the code will silently skip downloads for files whose content changed without a size change.

**More critically**: The manifest in 01-01-Task 2 defines `BDPMFile` but does not include a `byte_size` field. The change detection logic (Task 3) compares hash + size, but the manifest doesn't store the expected or previous size. This means the size comparison is floating — where does the stored size come from? The plan says it comes from `import_state.json` but the first run has no stored size. Does `needs_update` return true on first run or false? The logic needs to be explicit.

**Fix required**: State explicitly that Content-Length is an optimization only. The first run must always download (no stored state). After the first download, SHA256 is the only authoritative signal. Content-Length may be used as a cheap pre-filter (skip the full HTTP GET if HEAD response size matches stored size), but downloading must proceed if SHA256 differs or if this is a first run. Document this in both the BRIEF.md Update Detection Strategy and in the 01-01 plan's Task 3.

---

### C2: Incremental sync means nothing without per-source publication timestamps

The plan explicitly states: "No incremental sync on first pass. Full initial import is the only viable path." This is correct. But the BRIEF.md then goes on to describe Phase 3 as "Monthly cron integration → change diffing → delta updates."

**Change diffing of what?** There are no row-level timestamps in any of the 10 files. The plan has no answer for this — it only describes the infrastructure to detect *file-level* changes, not row-level changes. The delta update pipeline is therefore: download changed file → re-import entire changed file → diff against previous import → apply row-level changes.

This is not incremental. This is full-file refresh with row-level diffing. The plan needs to say this explicitly and reframe Phase 3 accordingly, because the word "incremental" sets the wrong expectation. The actual operation is: **file-level change detection + full-table refresh with row-level diff for audit**.

Furthermore, the plan does not address what "apply delta" means in practice when the data is in SQLite. Options are:
1. Truncate + reload the affected table (simple, no delta logic needed, valid for tables under 30k rows)
2. Full row-level diff against previous import (complex, needed only if you need to track when a drug's price changed)
3. A hybrid: store previous import in a shadow table, diff, then merge

For a dataset this size (max 32k rows per file), option 1 is almost certainly the right choice. Truncating and reloading `compositions`, `smr`, `asmr` on a monthly cadence is fast and avoids months of accumulated complexity. The plan should make this decision explicit and explain why.

**Fix required**: Rename Phase 3 from "Incremental Sync" to "File-Level Change Detection + Refresh". Document that row-level timestamps do not exist, so delta updates are full-table refreshes with optional row-level diff logging. State the decision to truncate + reload rather than row-level merge and explain the size threshold reasoning.

---

### C3: Schema — five fields typed wrong or ambiguous

**C3a: `compositions.candidates` table name is a typo**

Line 126 of BRIEF.md: `CREATE TABLE candidates (` — this is obviously meant to be `compositions`. The table name `candidates` makes no semantic sense. It's also missing the closing parenthesis on the CREATE statement (the snippet cuts off mid-line). This is a trivial fix but indicates the schema was written hastily.

**C3b: `drugs.is_patent` as TEXT "Oui"/"Non"**

The BRIEF says this field is TEXT with values "Oui"/"Non". This is a boolean stored as a string. The schema also stores `generic_type` as TEXT with values 'reference'/'generic'/NULL. These two fields should be INTEGER/BOOLEAN, not TEXT. A boolean field stored as TEXT with French strings will cause bugs in every comparison — `WHERE is_patent = "Oui"` works but `WHERE is_patent = 1` does not, and the type system won't catch the mismatch.

**C3c: `presentations.comm_status` (presentation commercial status) vs `drugs.comm_status` (drug commercial status)**

Both tables have a `comm_status` field. They have different semantics: drug-level "Déclaration de commercialisation" vs presentation-level "Déclaration de commercialisation". These are not the same thing — a drug can be declared commercial while a specific CIP presentation is not yet on market. The fields share a name but not semantics. Rename one or both for clarity: `drugs.drug_commercial_status` and `presentations.presentation_commercial_status`.

**C3d: `smr.decision_date` and `asmr.decision_date` stored as TEXT in YYYYMMDD format**

This is documented as "YYYYMMDD integer format" but stored as TEXT. That's correct — TEXT is the right type for an unparsed date string. However, the plan does not address the conversion path. Should the normalized schema store YYYYMMDD TEXT or convert to ISO date? For SQL date arithmetic (sorting, filtering by year), ISO date is strictly better. But the BRIEF recommends storing the raw CSV format for "auditability." The plan needs to decide: is the normalized schema an audit log (store raw), a queryable DB (convert to ISO), or both (store both)? This decision cascades to every date field in every table.

**Recommendation**: Store dates as TEXT in ISO-8601 format (YYYY-MM-DD) in the normalized tables, and keep the raw CSV value in the staging table. This is a one-time conversion on ingest and makes all downstream queries correct by default.

**C3e: `availability.status_type` as INTEGER with values 1, 4**

The plan says `status_type INTEGER (1=Rupture, 4=Remise à dispo)`. Storing these as INTEGER is correct, but the plan doesn't address how the API will map these magic numbers to human-readable strings. Either document the enum mapping or define it as a check constraint in SQLite: `CHECK (status_type IN (1, 4))`. Without the check constraint, the normalized schema allows any INTEGER, and the magic numbers are only documented outside the schema.

**C3f: `generic_groups.group_id` — TEXT but used as a grouping key, not a FK**

The BRIEF correctly stores this as TEXT. However, `drugs.generic_group_id` is also TEXT, and the plan stores the group_id in `drugs` directly. This means `drugs.generic_group_id` can reference a `generic_groups.group_id` that doesn't exist. The schema has no FK constraint enforcing this. Either add the FK (allowing NULL for drugs with no generic group) or document that the relationship is informational-only and not enforced.

---

## Classification: WARNING

### W1: CIS_InfoImportantes exclusion is a major blind spot, not a minor flag

The BRIEF says: "Skip CIS_InfoImportantes for v1. On-demand generation means no local cache strategy." The risk register marks this as "Low" severity.

This is wrong. **CIS_InfoImportantes contains safety alerts and contraindications** — the most safety-critical data in the entire database. The reason it's "on-demand generation" is not an implementation quirk; it's by design because the data changes unpredictably (new safety alerts, new contraindications, updated warnings). Excluding it from v1 means the API cannot serve one of the most important data points a drug database can provide.

The framing "polling it generates load on their server" is backwards — this is an argument *for* local caching, not against. The solution is to cache aggressively with a short TTL (hours, not months), not to exclude entirely.

**The plan should have a Phase 1.5 or at minimum a Phase 3.5: CIS_InfoImportantes integration** with:
- A dedicated scraping strategy that respects rate limits (they generate on-demand, so it can't be polled frequently)
- A TTL cache (e.g., 6-hour refresh) for safety-critical data
- A fallback: if the scraping fails, serve stale data with a freshness indicator
- An explicit decision that this file is treated differently from all others

Marking this as "Low" in the risk register understates the business impact. A drug database that doesn't include safety alerts is significantly diminished.

**Fix required**: Reclassify CIS_InfoImportantes from "exclude v1" to "phase 3.5, deferred but tracked." Document the cache TTL strategy. This is a Phase 3 deliverable, not a Phase 4 polish item.

---

### W2: Schema migrations are underspecified

The plan mentions "schema migrations — rusqlite migrations.rs pattern, versioning" as a Phase 1 outcome but provides no details. This is the most critical operational concern in the entire project.

Key questions unanswered:

1. **What triggers a schema migration?** When the BDPM publishes a file with a different field count. The plan mentions this in the risk register ("Quirk changes silently in future file versions") but provides no automated response. The migration logic must detect field count changes in the manifest and fail with a clear error rather than silently mis-parsing.

2. **How is migration tested?** The plan has no test strategy for schema changes. When CIS_bdpm.txt gains a 13th column next month, the CI needs to catch this before production. The plan must specify: per-file row-count regression tests AND per-file field-count tests in CI.

3. **What is the rollback strategy?** If the migration logic has a bug and imports corrupted data, what's the recovery? The plan stores raw CSV lines in `staging` tables — this is good for recovery. But the plan never says "on migration failure, the normalized tables remain untouched and the previous state is preserved."

4. **How many migrations are expected?** The plan says "migrations.rs-based schema versioning" but doesn't define the migration table or how migrations are applied in order. This needs a concrete pattern.

**Fix required**: Add an explicit "Schema Migration Strategy" section to the BRIEF.md. Define: how schema changes are detected (field count check), how they're handled (fail-fast with clear error + CI regression), and the rollback mechanism (staging preserved, normalized untouched on failure).

---

### W3: No integration test strategy for data correctness

The plan mentions "CI regression tests" in Phase 4 but never specifies what they test. The success criteria say "All 10 stable files parseable with 0 silent data loss" — but how is data loss measured? There are no assertions on:
- Row counts against known totals (e.g., CIS_bdpm.txt must have exactly 15,848 rows)
- Field-level validation (e.g., CIS must be 8 digits, price must be numeric after normalization)
- Cross-file referential integrity (every CIS in CIS_CIP_bdpm.txt must exist in CIS_bdpm.txt)
- Price normalization correctness (e.g., "24,34" becomes 2434 cents)
- Date format conversion (e.g., "28/04/2026" becomes "2026-04-28")

Without these assertions, "0 silent data loss" is unverifiable. CI will pass after any change that doesn't cause a panic.

**Fix required**: Define a test suite in Phase 1 (not Phase 4):
- **Row count tests**: one test per file asserting exact known row count (current counts are documented in BRIEF.md)
- **Field count tests**: one test per file asserting exact field count
- **Referential integrity tests**: CIS cross-file membership checks
- **Parse correctness tests**: sample assertions on known values (prices, dates, encodings)
- **Schema migration regression tests**: simulate a field count change, verify it fails clearly

---

### W4: Phase ordering — staging → normalization is backward as written

The plan structures Phase 1 as: scaffold (01-01) → parser (01-02) → staging layer (01-03) → normalized migration (01-04). This implies: first build the parser, then build a staging layer, then build the normalization. But the staging and normalization logic cannot be designed without knowing exactly what the parser will produce and what the normalized schema expects.

More critically: the staging → normalized migration is described as a single step (01-04: "Normalized migration + price normalization"). But normalization includes:
- Price conversion (European comma → cents)
- Date conversion (DD/MM/YYYY and YYYYMMDD → ISO)
- CIP-7 code stripping (34009 prefix → canonical)
- Windows-1252 byte normalization
- Encoding normalization (Latin-1 raw storage vs Unicode API output)

Each of these is non-trivial. Price normalization alone is a full transformation pass. Date format handling has two formats across different files. The plan treats this as one task; it should be three:
- 01-04a: Price normalization (all files, European decimal → cents)
- 01-04b: Date normalization (per-file format handling)
- 01-04c: ID/code normalization (CIP stripping, encoding fixes)

**Fix required**: Split 01-04 into explicit sub-tasks. The staging → normalized step is the most complex part of the entire ingest pipeline and deserves its own micro-phase.

---

### W5: CIS_HAS_ASMR and CIS_HAS_SMR avis field is not characterized

The BRIEF notes that ASMR has a "free-text avis field" and SMR has one too. Both are marked as TEXT and stored in the normalized schema. But the plan does not address:
- What does this text look like? (e.g., 50 words or 500?)
- Does it contain newlines, tabs, HTML fragments?
- Is it useful for the API as raw text, or does it need NLP processing?
- What is the typical length distribution?

This matters for the API design: if avis is 500+ words, returning it as a raw TEXT field in a JSON response is awkward. A search API would need to handle full-text search on avis. A drug detail API would need to decide whether to embed the full avis or truncate it.

The feasibility study examined the file line counts (9,906 and 15,257 rows) but not the avis field content. Without characterizations of the avis field length and structure, the API design decisions are made on incomplete information.

**Fix required**: Add a characterization pass on CIS_HAS_SMR/ASMR avis fields before Phase 2 API design. Sample 100 avis entries, measure average length, check for HTML, newlines, special characters. Document findings.

---

## Classification: SUGGESTION

### S1: SQLite is the right choice for this project — confirmed, with one caveat

The plan recommends SQLite and this is correct for the stated use case: single-writer local database, no concurrency requirement, sub-50ms reads. rusqlite is battle-tested. The three-tier staging/normalization/views pattern is sound.

**The caveat**: WAL mode must be explicitly set. SQLite's default journal mode (DELETE) has a single-writer bottleneck. For a monthly re-import that truncates and reloads a table, WAL mode is essential. The plan mentions "rusqlite connection pool, WAL mode" in the architecture diagram but doesn't mandate it in the schema design. WAL mode must be enabled explicitly on every connection.

**Suggested addition**: A `db.rs` that sets `PRAGMA journal_mode = WAL` and `PRAGMA synchronous = NORMAL` on every new connection. Document why these pragmas are chosen.

---

### S2: File-level vs. table-level import granularity

The plan's sync pipeline imports at the file level: download changed file → re-import entire file → update normalized tables. But `generic_groups` and `availability` have different update cadences than the core drug data. The plan acknowledges this for `availability` (weekly polling) but doesn't for `generic_groups`. If generic groups rarely change, why re-import all 10,704 rows monthly?

More fundamentally: the plan doesn't distinguish between files that are truly static (HAS_LiensPageCT, CIS_MITM) and files that change monthly (CIS_bdpm, CIS_CIP_bdpm). Treating all 10 files with the same monthly cadence is wasteful.

**Suggested addition**: Categorize files by expected change frequency:
- **Monthly**: CIS_bdpm, CIS_CIP_bdpm, CIS_COMPO_bdpm, CIS_HAS_SMR, CIS_HAS_ASMR, CIS_GENER, CIS_CPD
- **Weekly**: CIS_CIP_Dispo_Spec (availability)
- **Static (re-import only on hash change)**: CIS_MITM, HAS_LiensPageCT
- **Excluded from polling**: CIS_InfoImportantes

This categorization should drive the cron schedule and the per-file polling logic, not a single monthly trigger for all files.

---

### S3: The `BDPMFile` enum should carry `expected_field_count` but also `required_fields` and `nullable_fields`

The plan defines `expected_field_count: usize` per `BDPMFile`. This is good. But the schema validation strategy (mentioned as "Per-file schema validation, row count checks" in the architecture) needs more than field count.

For example:
- CIS_CPD has exactly 2 fields, both nullable? Or is field 1 required?
- CIS_CIP_Dispo_Spec: field 7 (CIP) can be empty string. Is empty string treated as NULL?
- CIS_GENER: field 3 is the actual CIS code but stored as text — is it always 8 digits?

**Suggested addition**: Define a `FileSchema` struct per `BDPMFile` with:
```rust
struct FileSchema {
    field_count: usize,
    required_fields: BitSet,     // which fields cannot be empty
    nullable_fields: BitSet,     // which fields may be empty
    date_fields: HashMap<usize, DateFormat>,  // which field indices have dates, and their format
    numeric_fields: BitSet,      // which fields should be numeric after normalization
}
```

This makes schema validation explicit and testable rather than embedded in parser logic.

---

### S4: Consider a data profiling phase before Phase 1.5

The BRIEF.md is labeled "feasibility study" but it's more of a structural survey. A true data profiling pass would add:

- **Value distribution** for enum-like fields (e.g., `drugs.status` — how many distinct values? Are there values not documented?)
- **Null frequency** per field (not just "35% of presentations have no city price" — what about other NULL rates?)
- **String length distributions** for TEXT fields (max, average, p95)
- **Cross-field dependencies** (e.g., if `comm_status = "Non commercialisée"` then `prix_ville` is always NULL — is this invariant true?)
- **Outlier detection** in numeric fields (price = 0? price = absurdly high?)

This profiling data is essential for writing meaningful integration tests (W3 above). Without it, the test suite will be built on assumptions that may not hold.

**Suggested addition**: Insert a Phase 1.5 — "Data Profiling + Schema Validation" between scaffold/parser (01-02) and staging (01-03). This phase: (1) runs full characterization of each file's fields, (2) generates a `profiling_report.md` documenting distributions and invariants, (3) defines the `FileSchema` structs (S3 above), (4) seeds the CI test suite with invariant assertions derived from profiling.

---

### S5: Missing coverage of EAN stripping edge cases

The BRIEF mentions "CIP-7 codes: 7-digit codes prefixed with 34009 in CIS_CIP_bdpm.txt. Strip prefix for canonical CIP." This is straightforward. But the plan does not address:

- Are there CIP codes in the data that don't have the 34009 prefix?
- Does CIS_CIP_Dispo_Spec use the same CIP format as CIS_CIP_bdpm?
- Should the stripped CIP be stored alongside the raw CIP, or only the stripped version?

The `presentations.cip_ean` field stores the 13-digit EAN with 34009 prefix. The plan doesn't say whether this should also be stripped, or whether 34009 is a genuine prefix to keep.

**Suggested addition**: In the data profiling phase (S4), characterize all CIP variations in all files. Define a canonical CIP format and document the stripping rule precisely. Store both raw and canonical forms if there is any ambiguity.

---

## CONFIRMED GOOD

- **Encoding documentation is excellent**: The Latin-1/UTF-8 split, Windows-1252 residual bytes, and CRLF handling are all correctly identified and the normalization strategy (store raw, normalize on read) is sound.
- **Price normalization to cents** is the right decision. Storing "24,34" as 2434 cents avoids floating-point errors entirely.
- **Three-tier staging/normalization/views architecture** is correct for this use case. Staging preserves raw CSV for recovery, normalized tables are for queries, views provide the API surface.
- **Rate limiting at 1 req/5s** is appropriate. Government servers deserve politeness, and 10 files at 5s intervals = 50s total download window, which is acceptable.
- **Separate availability polling (weekly)** correctly identifies that CIS_CIP_Dispo_Spec.txt has an independent update cadence from the monthly files.
- **Raw imports table for auditability**: Storing raw CSV lines in a `raw_imports` table is good practice for data that changes format over time.
- **User-Agent header with contact info**: Proper identification for a polite crawler. Government servers often block unidentified user-agents.

---

## REVISED PHASE STRUCTURE

The current structure is: Foundation → API → Sync → Polish.

Problems with this structure:
1. Polish (CI, OpenAPI, Docker) is Phase 4, but integration tests (CI regression) should be Phase 1 — they're how you know the ingest pipeline works correctly.
2. "Sync" is ambiguous — it mixes file-level polling logic with API integration, which are separate concerns.
3. Data profiling is missing entirely.
4. Schema migration strategy is buried inside 01-03.

**Proposed revised structure with explicit dependencies**:

```
Phase 0: Data Profiling (NEW — runs once, before any implementation)
├─ Dependencies: None
├─ Purpose: Characterize field distributions, null rates, string lengths,
│           cross-field invariants, avis field content analysis
├─ Deliverables: profiling_report.md, FileSchema definitions, seeded
│                integration test assertions, CIP/EAN variation inventory
└─ Why: Informs all downstream design decisions. Cannot design the API or
        write meaningful tests without knowing what the data actually looks like.

Phase 1: Foundation
├─ 01-01: Project scaffold + fetcher + state store
│   └─ Depends on: Phase 0 (knows field counts from profiling)
├─ 01-02: Tab parser with per-file encoding handling
│   └─ Depends on: 01-01
├─ 01-03: Data profiling integration + FileSchema validation
│   └─ Depends on: Phase 0 + 01-02 (can now run the parser against live data)
├─ 01-04: Staging layer + raw imports table
│   └─ Depends on: 01-03 (schema validation logic is the gate)
├─ 01-05: Price normalization (European decimal → cents)
│   └─ Depends on: 01-04
├─ 01-06: Date normalization (DD/MM/YYYY + YYYYMMDD → ISO-8601)
│   └─ Depends on: 01-05 (but can run in parallel with it)
└─ 01-07: Normalized migration + FK enforcement
    └─ Depends on: 01-05 + 01-06

Phase 2: API (depends on: Phase 1 complete)
├─ 02-01: FTS5 drug name search
├─ 02-02: Drug detail + price lookup
├─ 02-03: Generic groups + availability status
└─ 02-04: OpenAPI spec + API documentation

Phase 3: Change Detection + Refresh (depends on: Phase 1 complete)
├─ 03-01: File-level polling + SHA256 change detection
│   └─ Note: Content-Length is documented as optimization, not signal
├─ 03-02: Full-table refresh with row-diff logging
│   └─ Note: Explicitly NOT incremental — truncate + reload is the chosen strategy
├─ 03-03: Availability file (weekly cadence, independent)
└─ 03-04: Import log viewer + health dashboard

Phase 3.5: Safety Data (NEW — depends on: Phase 3.01 complete)
├─ CIS_InfoImportantes integration with 6-hour TTL cache
└─ Safety alert API endpoint

Phase 4: Polish (depends on: Phases 2 + 3 complete)
├─ 04-01: CI regression tests (file count, field count, referential integrity,
│         price normalization, date conversion, schema migration regression)
├─ 04-02: Docker packaging
└─ 04-03: Operational documentation (runbook for monthly sync, schema change
           response procedure)
```

**Key dependency changes from the original plan**:
1. Phase 0 added explicitly. Without profiling, Phase 1's field counts, integration tests, and API design decisions are all based on incomplete information.
2. Phase 1's staging (01-04) now depends on profiling + schema validation (01-03), not the other way around. You can't build a staging layer without knowing what you're staging.
3. Integration tests moved to Phase 4.01 — they run as gatekeepers throughout, but the test suite definition is a Phase 4 deliverable so it can be informed by what Phase 1 and Phase 0 discovered.
4. Phase 3.5 added for CIS_InfoImportantes — currently listed as "exclude v1" but it is safety-critical data that should not be indefinitely deferred.
5. Operational documentation added as Phase 4.03 — the schema change response procedure (how to handle a field count change) is an operational need, not a polish item.

---

## SUMMARY OF REQUIRED ACTIONS

| ID | Priority | Action |
|----|----------|--------|
| C1 | CRITICAL | Clarify Content-Length as optimization only, SHA256 as authority. Fix manifest to carry byte_size field. Document first-run behavior. |
| C2 | CRITICAL | Reframe Phase 3 as "file-level refresh" not "incremental sync." Explicitly choose truncate+reload over row-level merge. |
| C3 | CRITICAL | Fix `compositions.candidates` typo. Change `is_patent` to BOOLEAN. Rename duplicate `comm_status` fields. Decide on date storage format (raw vs ISO). Add CHECK constraint on `availability.status_type`. Add FK between `drugs.generic_group_id` and `generic_groups.group_id`. |
| W1 | WARNING | Reclassify CIS_InfoImportantes from "exclude v1" to "Phase 3.5 with 6h TTL cache." |
| W2 | WARNING | Add explicit schema migration strategy: detection, fail-fast behavior, rollback, CI regression. |
| W3 | WARNING | Define integration test suite in Phase 1 (not Phase 4): row counts, field counts, referential integrity, parse correctness. |
| W4 | WARNING | Split 01-04 into 01-04a/01-04b/01-04c for price/date/ID normalization. Add profiling dependency between 01-02 and 01-04. |
| W5 | WARNING | Characterize CIS_HAS_SMR/ASMR avis field content (length distribution, HTML presence) before API design. |
| S1 | SUGGESTION | Explicitly enable WAL mode + NORMAL synchronous pragma in db.rs. |
| S2 | SUGGESTION | Categorize files by change frequency. Adjust cron schedule per category. |
| S3 | SUGGESTION | Define `FileSchema` struct per BDPMFile with required/nullable/numeric/date field metadata. |
| S4 | SUGGESTION | Insert Phase 1.5: Data Profiling + Schema Validation. Generates profiling_report.md and seeds test assertions. |
| S5 | SUGGESTION | Inventory all CIP/EAN format variations across all files. Document stripping rule precisely. |