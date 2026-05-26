# Monitoring & Synchronization Strategy Comparison

**Date:** 2026-05-26
**Source:** External review analysis (BDPM_Analyse_Technique_Final.txt, bdpm_feasibility_body.txt, BDPM_Etude_Faisabilite.txt, review1.txt, review2.txt) vs BDMP_DB internal plans (BRIEF.md, 01-01-PLAN.md, ROADMAP.md)

---

## 1. Change Detection

### Internal Plan (ROADMAP Phase 3 / BRIEF)
- **Primary signal:** BLAKE3 hash computed post-download
- **Optimization:** Content-Length from HTTP HEAD as cheap pre-filter (skip GET if size matches stored)
- Content-Length is explicitly documented as optimization only, not authoritative
- First run: no state → always downloads
- Subsequent runs: compare BLAKE3 hash

### External Reviews
- **Primary signal:** SHA-256 ( unanimously recommended across all 3 reviews)
- **Secondary signal (BDPM_Analyse_Technique_Final):** data.gouv.fr API `last_update` field and per-resource checksums
- **Tertiary/Fallback:** HTML scraping of download page for modification dates
- **Rejection of alternatives:** ETag, Last-Modified, HEAD + Content-Length all unavailable

### Assessment
**BLAKE3 vs SHA-256:** For content identity detection (not cryptographic verification), BLAKE3 is 4-10x faster and perfectly adequate. Our plan wins on performance. SHA-256 from externals is conservative but correct.

**data.gouv.fr API:** BDPM_Analyse_Technique_Final recommends this as a cheap secondary check (~few KB JSON vs 26 MB download). BDPM_Etude_Faisabilite.txt (section 9) explicitly **rejects** API data.gouv.fr for medical use due to reliability concerns. Our plan does not mention this. Worth evaluating if HEAD/GET cost becomes problematic at scale.

**HTML scraping (3rd layer):** Supported only in BDPM_Analyse_Technique_Final as fragile fallback. Our plan has no equivalent. This is low-value given that BLAKE3 is definitive after download.

**Verdict:** Our BLAKE3 approach is sound. The external three-layer approach adds complexity we don't need since BLAKE3 after download is definitive. data.gouv.fr API is worth a one-time investigation but should not become a dependency.

---

## 2. Polling Frequency

### Internal Plan
- **Monthly:** All 10 stable files (full sync)
- **Weekly:** CIS_CIP_Dispo_Spec.txt (independent cadence)
- **No adaptive scheduling:** Static intervals, GitHub Actions cron

### External Reviews
**BDPM_Analyse_Technique_Final (adaptive polling table):**

| Period | Interval | Rationale |
|--------|----------|-----------|
| First week of month | Every 6 hours | Typical mid-month update |
| Second week of month | Every 12 hours | Update still possible |
| Rest of month | Every 24 hours | Unlikely to change |
| Post-update detection | Every 2 hours for 48h | Capture subsequent corrections |
| InfoImportantes (dynamic) | Every 6 hours | Real-time generated file |

**bdpm_feasibility_body.txt:**
- Standard regime: weekly for 10 static files (e.g., Monday 3 AM)
- Dynamic regime: daily for CIS_InfoImportantes.txt (e.g., 6h and 18h)
- Limit CIS_InfoImportantes.txt re-download to one per day, only if size/hash differs

**BDPM_Etude_Faisabilite.txt:**
- `check_interval_hours = 24` in config.toml example

### Assessment
**Frequency mismatch:** Our monthly cadence aligns with BDPM's published monthly schedule. External adaptive proposals are more aggressive (6h in first week) but are unnecessary overhead for a CLI tool. The weekly check proposed in bdpm_feasibility_body.txt is more conservative than our monthly plan.

**Key nuance:** externals correctly observe that CIS_CIP_Dispo_Spec.txt updates weekly (our plan matches this). CIS_InfoImportantes.txt is the outlier requiring more frequent polling.

**Verdict:** Our static monthly + weekly cadence is appropriate for a self-hosted CLI. The adaptive polling complexity adds no value unless we operate as a real-time monitoring service. CIS_InfoImportantes handling should use a 6-hour TTL as noted in our deferral (Phase 3.5).

---

## 3. Rate Limiting

### Internal Plan (01-01-PLAN.md)
- **5 seconds** between requests
- 3 retries with exponential backoff: 5s, 10s, 30s
- Synchronous ureq client (no concurrent connections)

### External Reviews
**BDPM_Analyse_Technique_Final:**
- **2 seconds** minimum between consecutive requests
- Single simultaneous connection to BDPM server (serialized requests only)
- Exponential backoff on error: starting at 30s, max 1 hour
- Preference: 02:00-06:00 CET (off-peak hours)

**bdpm_feasibility_body.txt:**
- "1 request every 2-5 seconds"

**BDPM_Etude_Faisabilite.txt:**
- "Retry with backoff: 3 attempts max, exponential backoff" (no specific intervals)

### Assessment
**Gap: 5s vs 2s.** Our plan is 2.5x more conservative. The BDPM server has no documented rate limiting, so no explicit protection needed from our side. Our 5s interval is safer and still completes all 10 files in ~55 seconds total — completely acceptable for monthly runs.

**Off-peak window:** Our plan does not implement 02:00-06:00 preference. GitHub Actions scheduled workflows can be set to run during this window (e.g., `schedule: '0 2 1 * *'` for monthly). This is sufficient — the cron trigger IS the scheduling mechanism, no need for runtime sleep.

**Verdict:** Keep 5s rate limit — it's more conservative and completes in under 60 seconds. No need to reduce to 2s unless we observe actual server delays.

---

## 4. data.gouv.fr API

### What It Provides
- Endpoint: `/api/1/datasets/base-de-donnees-publique-des-medicaments-base-officielle/`
- Returns JSON with `last_update` timestamp and per-resource modification dates + checksums
- Response size: few KB (vs 26 MB full download)

### External Opinions
- **BDPM_Analyse_Technique_Final:** recommends as Layer 2 (secondary after SHA-256)
- **BDPM_Etude_Faisabilite.txt:** explicitly rejects: "D. API data.gouv.fr: Not reliable for medical use" (section 9 comparison table)

### Assessment
**Reliability concern is valid.** data.gouv.fr is a separate system — it can be down or lagging while BDPM source is current. Relying on it as a gate for medical drug data is inappropriate.

**Use case for our project:** Could serve as a "pre-check" before initiating full BLAKE3 verification (i.e., skip HEAD/GET if data.gouv.fr hasn't updated AND our local state is current). But this adds fragility for minimal gain.

**Verdict:** Do not integrate as a gating mechanism. The HEAD+GET+BLAKE3 path is reliable enough. data.gouv.fr API is a nice-to-have monitoring signal in logs but not a dependency.

---

## 5. HTML Scraping

### External Recommendation (BDPM_Analyse_Technique_Final, Layer 3)
- Parse the BDPM download page HTML to extract per-file modification dates
- Use `scraper` crate in Rust
- Fragile (HTML format can change without notice) but cheap
- Provides indication of update without download

### Internal Plan
- No HTML scraping layer
- Pure BLAKE3 post-download detection

### Assessment
**Not needed.** Our BLAKE3 approach after download is definitive. HTML scraping adds maintenance burden (HTML structure changes break the scraper) for a cheap pre-filter we don't need — Content-Length from HEAD gives us the same optimization without fragility.

**Exception:** CIS_InfoImportantes.txt has a dynamic filename containing a timestamp. We need to parse the download page HTML to find the current URL with the timestamp. This is the one legitimate use case for HTML parsing.

**Verdict:** No HTML scraping for change detection. HTML parsing only for CIS_InfoImportantes.txt dynamic filename resolution (Phase 3.5 deferred).

---

## 6. Import Traceability

### Internal Plan (import_log table)
```sql
CREATE TABLE import_log (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    file_name              TEXT NOT NULL,
    file_hash              TEXT NOT NULL,           -- BLAKE3
    file_size              INTEGER NOT NULL,
    row_count              INTEGER NOT NULL,
    status                 TEXT NOT NULL,           -- success/partial/failed
    bad_rows               INTEGER DEFAULT 0,
    skipped_rows           INTEGER DEFAULT 0,
    imported_at            DATETIME DEFAULT CURRENT_TIMESTAMP,
    duration_ms            INTEGER
);
CREATE INDEX idx_import_log_file ON import_log(file_name, imported_at DESC);
```

### External Plans
**BDPM_Analyse_Technique_Final:**
- `update_history` table: track change events per file
- `import_metadata` table: hash, row counts, encoding, timestamps
- Recommendation: separate tables for audit trail + update tracking

**BDPM_Etude_Faisabilite.txt:**
- `import_log`: import_date, source_date, file_name, file_hash (SHA-256), row_count, reject_count, encoding_detected
- `import_id` column in every data table (FK to import_log)

**bdpm_feasibility_body.txt:**
- `manifest.json` with one entry per run: collect date, source date, per-file hash, row counts, parser version

### Assessment
**Our plan is sufficient.** We track every import with file_hash (BLAKE3), file_size, row_count, status, bad_rows, skipped_rows, timestamp, duration. This covers reproducibility and diagnostics fully.

**External enhancements we lack:**
- `source_date` (the date BDPM published the update) — useful for audit but requires HTML scraping or data.gouv.fr lookup
- `parser_version` — could add `version TEXT` field
- Per-row `import_id` FK in data tables — our `imported_at` timestamp on each table is coarse but functional

**Verdict:** Our import_log design is pragmatically complete. The per-row import_id adds overhead (every INSERT needs the FK). Consider adding `source_date TEXT` to import_log when available from the download page, and `parser_version TEXT` for CI regression.

---

## 7. GitHub Actions Scheduling

### Internal Plan (ROADMAP.md)
- **Monthly:** `schedule: '0 2 1 * *'` — full BDPM sync, rebuild SQLite, publish as GitHub Release asset
- **Weekly:** `schedule: '0 3 * * 0'` — CIS_CIP_Dispo_Spec.txt update only
- **Manual:** `workflow_dispatch` with `--full` flag for forced rebuild

### External
**BDPM_Analyse_Technique_Final:**
- Adaptive polling within CI not relevant — this is for client-side use
- No specific GitHub Actions scheduling recommendation

**BDPM_Etude_Faisabilite.txt:**
- Mentions "Docker + cron mensuel" as optional, explicitly notes Docker overhead is unnecessary for a CLI tool

### Assessment
**Monthly vs "first week every 6h" gap:** Our 1st-of-month 02:00 UTC (3:00 CET) scheduled run should catch the BDPM monthly update. If BDPM publishes mid-week, our next scheduled run will catch it within days. No adaptive polling needed for CI.

**Weekday choice:** Our Sunday weekly run (`* 0` in cron) may conflict with BDPM's mid-week publication pattern. Consider Thursday (`0 3 * * 4`) as weekly for Dispo — it's closer to mid-week and gives a fresh run before weekend healthcare activity.

**Verdict:** Keep monthly + weekly. Consider shifting weekly from Sunday to Thursday for Dispo file. No adaptive polling in CI — use static schedules.

---

## 8. InfoImportantes Special Handling

### Internal Plan (ROADMAP.md Phase 3.5 — deferred)
- **6-hour TTL cache**
- **Dynamic filename pattern:** `CIS_InfoImportantes_YYYYMMDDhhmmss_bdpm.txt`
- Fallback to stale data with freshness indicator
- Safety alert API endpoint

### External Reviews
**BDPM_Analyse_Technique_Final table 7:**
- "InfoImportantes (dynamique): Toutes les 6 heures" — same 6-hour interval
- Dynamic filename: yes, requires scraping or filename parsing
- Background: file is generated in real-time (safety alerts)

**BDPM_Etude_Faisabilite.txt (section 1.1):**
- "Informations importantes: generees en direct (nom dynamique avec timestamp)"

**bdpm_feasibility_body.txt:**
- "CIS_InfoImportantes.txt: verification quotidienne (ou deux fois, par exemple a 6h et 18h)"
- "Limit CIS_InfoImportantes.txt re-download to first daily verification and only if size/hash differs"

### Assessment
**All sources agree:** 6-hour interval and dynamic filename are the core challenges. Our deferral to Phase 3.5 is correct — this requires dedicated HTML scraping or filename pattern matching and deserves its own phase.

**Implementation approach:** Parse the BDPM download page (or use data.gouv.fr API endpoint) to get the current dynamic URL. Store last known URL and only rebuild when filename changes.

**Verdict:** Keep as Phase 3.5 deferred. Our 6-hour TTL matches external consensus. Consider reducing to 4-hour if safety alerts need higher freshness.

---

## 9. Soft-Delete vs Truncate+Reload

### Internal Plan (BRIEF.md)
- **Full-table truncate + reload** — definitively chosen
- Reasoning: no row-level timestamps exist in BDPM files; row-level delta is impossible; for 32K-row tables this completes in seconds monthly
- Named correctly: "file-level change detection + full-table refresh"

### External
**BDPM_Analyse_Technique_Final:**
- No explicit soft-delete vs truncate discussion
- Full-table approach implicit in 3-layer detection strategy (hash change → full file download)

**BDPM_Etude_Faisabilite.txt:**
- Implicit full-table approach via manifest + hash tracking

**bdpm_feasibility_body.txt:**
- "Ne traiter que les fichiers dont le hash a change depuis le dernier import"
- Full-table reload for changed files

### Assessment
**Full consensus across all sources.** No one proposes soft-delete or row-level delta. The rationale is consistent: no timestamps in source files, small dataset, monthly cadence.

**Soft-delete scenarios to consider:**
- Availability tracking (CIS_CIP_Dispo_Spec.txt): Stock status changes over time — a backfill with history would be valuable. But our plan tracks this as weekly snapshots, not incremental events.
- CIS_InfoImportantes.txt: Safety alerts have lifecycle (appear → resolve). Our Phase 3.5 deferral handles this.

**Verdict:** Truncate+reload is correct. For availability specifically, we may want a date_range-based retention strategy in the future (keep history), but our Phase 3 plan is weekly snapshots, not event-level tracking.

---

## 10. Politeness Rules

### Internal Plan (01-01-PLAN.md)
- User-Agent: `bdpm-ingest/0.1 (contact@example.com)`
- 5s rate limit between requests
- 3 retries with exponential backoff (5s, 10s, 30s)
- No mention of off-peak hours (handled by cron scheduling)

### External Reviews
**BDPM_Analyse_Technique_Final (section 7.5):**
- 2s minimum between requests
- User-Agent with contact info
- Single simultaneous connection (serialized only)
- Preference for 02:00-06:00 CET
- Exponential backoff starting at 30s, max 1 hour
- Aggressive caching: only re-download files whose hash actually changed

**BDPM_Etude_Faisabilite.txt (section 3.2):**
- User-Agent: `BDPM-Pipeline/1.0 (contact@example.com)`
- 2-5 seconds between requests
- Retry with backoff (no specific intervals)
- 60s timeout for large files (>4 MB)

### Assessment
**Convergence is high.** All sources agree on: User-Agent with contact info, rate limiting, retry with backoff, single connection.

**Gaps in our plan vs external:**
- 60s timeout for large files — we need this (files up to 26 MB)
- Max backoff cap (1 hour) — we have 30s max but no explicit cap
- Aggressive content caching (only re-download changed files) — our BLAKE3 approach handles this

**Verdict:** Add `timeout = Duration::from_secs(60)` to our fetcher. Add explicit max backoff of 1 hour. User-Agent format is fine as-is.

---

## Summary Matrix

| Aspect | Our Plan | External Consensus | Gap/Delta |
|--------|----------|-----------------|-----------|
| Hash algorithm | BLAKE3 | SHA-256 | Performance win (BLAKE3 4-10x faster) |
| Change detection authority | BLAKE3 only | SHA-256 primary | Equivalent signal, different algorithm |
| Rate limit interval | 5s | 2s | Keep 5s (more conservative) |
| Backoff max | 30s | 1 hour | **Add 1h cap** |
| HTML scraping | None | Layer 3 fallback | **Only for InfoImportantes filename** |
| data.gouv.fr API | Not integrated | Layer 2 secondary | **Do not add as dependency** |
| Import log fields | hash, size, rows, status, bad_rows | + source_date, parser_version | **Add source_date, parser_version** |
| Scheduling | Monthly + weekly | Agree with monthly/weekly | **Shift weekly to Thursday** |
| InfoImportantes | 6h TTL deferred | 6h interval agree | Match |
| Truncate+reload | Full-table refresh | All agree | Aligned |
| Politeness | User-Agent + rate limit | User-Agent + rate limit + timeout | **Add 60s timeout** |

---

## Recommended Adjustments to Our Plan

### 1. Fetcher backoff cap (01-01-PLAN.md Task 3)
Add explicit max backoff:
```rust
// Current: retry with 5s, 10s, 30s backoff
// Fix: cap at 1 hour (matches external consensus)
let backoff = min(30 * 2_u64.pow(retry), 3600);
```

### 2. Request timeout (01-01-PLAN.md Task 3)
```rust
// Add to fetcher: 60s timeout for large files
// ureq: set_timeout(Duration::from_secs(60))
// Files up to 26 MB need generous timeout
```

### 3. Weekly cron adjustment (ROADMAP.md Phase 4)
Change Dispo weekly from Sunday to Thursday:
```yaml
# Current:
schedule: '0 3 * * 0'  # Sunday 03:00
# Recommended:
schedule: '0 3 * * 4'  # Thursday 03:00 (closer to mid-week)
```

### 4. Extended import_log schema (brief.md or 01-01-PLAN.md)
```sql
ALTER TABLE import_log ADD COLUMN source_date TEXT;   -- BDPM's stated update date
ALTER TABLE import_log ADD COLUMN parser_version TEXT; -- CI: commit hash matched
```

### 5. InfoImportantes Phase 3.5 scope clarification
Include HTML parsing for dynamic filename resolution:
```
CIS_InfoImportantes.txt → Parse download page for current timestamped filename
                        → Compare against stored filename via BLAKE3
                        → 6-hour TTL cache
                        → Safety alert API endpoint
```
