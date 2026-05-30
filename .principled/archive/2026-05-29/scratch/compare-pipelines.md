# Pipeline Strategy Comparison: External Analyses vs Our Plan

## Sources Analyzed

| Source | Document | Key Pages |
|--------|----------|-----------|
| External #1 | `bdpm_feasibility_body.txt` | lines 356-400 |
| External #2 | `BDPM_Analyse_Technique_Final.txt` | lines 778-810 |
| Our Plan | `01-02-PLAN.md`, `01-04-PLAN.md`, `01-05-PLAN.md` | full |

---

## 1. Pipeline Stages

### External #1 (5-stage pipeline)
Named stages with deterministic ordering:

| Stage | Name | Activities |
|-------|------|------------|
| 1 | Download + integrity | SHA-256 hash, skip if unchanged, archive with date stamp |
| 2 | Decode + line normalize | Encoding decode (cp1252/latin-1/utf-8), strip `\r`, filter `\r\r\n` empties, NFC Unicode |
| 3 | Tab split + struct validation | Split on `\t`, check column count, log anomalies without aborting |
| 4 | Per-field normalize | Dates→ISO-8601, comma decimal→dot, smart quotes→straight apostrophe, HTML extract, trim whitespace |
| 5 | Semantic validation + insert | Enum checks, orphan flagging, per-file transaction INSERT |

### External #2 (5-stage pipeline)
Different stage boundaries, merged encoding detection with parsing:

| Stage | Name | Activities |
|-------|------|------------|
| 1 | Fetch | reqwest, SHA-256 computed during download, 2s interval between requests |
| 2 | Encoding detection | Inspect bytes → Windows-1252 / UTF-8 / ASCII, decode to UTF-8, 0x92→apostrophe |
| 3 | TSV parse | Split tabs, filter empty lines, strip spaces, normalize percentages, dates→ISO-8601 |
| 4 | SQLite import | Transaction, PRAGMA foreign_keys=ON, INSERT OR IGNORE for duplicates |
| 5 | Validation | Row count check, NULL absence check, FK coherence |

### Our Plan (3-phase, 3-plan decomposition)
Clean separation of concerns across three plans:

| Phase | Plan | Activities |
|-------|------|------------|
| 1 | `01-02` Tab Parser | Streaming tab parser, per-file encoding via `encoding_rs`, CRLF strip, field-count guard, emit warnings |
| 2 | `01-04` Normalization | Price cents, date ISO-8601, whitespace/CIP/encoding, HTML strip, dedup helper |
| 3 | `01-05` Import | DB init + migrations, per-file import functions, master orchestrator, report + log |

**Assessment**: All three approaches cover the same ground. External #1 stages match our decomposition most closely (encoding/CRLF, split+validate, normalize, insert). External #2 merges encoding detection into the parse stage, which is pragmatic but less testable. Our 3-phase split maximizes test isolation per `01-02/01-04/01-05` plan boundaries.

---

## 2. Error Handling

| Source | Malformed Row Behavior |
|--------|----------------------|
| External #1 | "journaliser les lignes anormales sans interrompre l'import" — log and continue |
| External #2 | INSERT OR IGNORE for duplicates; no explicit malformed-row strategy stated |
| Our plan | **Field-count guard**: log to `import_log` with `line_number`, emit `None`, increment `skipped_rows` counter. 18 known malformed rows in CIS_HAS_SMR/ASMR filtered silently. |

**Key gap**: External #1 and #2 do not address field-count mismatch scenarios. Our `01-02-PLAN.md` Task 2 explicitly handles the 18 CIS_HAS_SMR malformed rows — this is an undocumented edge case the external analyses missed.

**Gap in our plan**: No explicit recovery mode strategy if encoding produces replacement characters (U+FFFD). External #1's risk section (section 9.1) flags this as a mitigation: "implementer un detecteur d'encodage de secours". Our plan has no fallback encoding path.

---

## 3. Transaction Strategy

| Source | Strategy |
|--------|----------|
| External #1 | "transaction par fichier (autocommit desactive pour la performance)" — per-file, autocommit OFF |
| External #2 | "transaction SQLite" — single transaction wrapping insert phase, no per-file scope stated |
| Our plan | "BEGIN IMMEDIATE / COMMIT" per import function; on failure: ROLLBACK, preserve previous state. Partial import acceptable: continue with other files on one file's failure. |

**Assessment**: Our plan is the most robust. Per-file transactions with rollback preserve previous state on failure. External #1 says the same but is vague on partial-import behavior. External #2 doesn't clarify transaction scope or failure handling.

**Notable**: Our `01-05-PLAN.md` explicitly allows partial imports ("Continue with other files on failure") — this is a deliberate tradeoff favoring availability over atomicity. External analyses don't discuss this.

---

## 4. Batch Size

| Source | Recommendation |
|--------|---------------|
| External #1 | Not specified |
| External #2 | "batch size de 1000 lignes" for SQLite WAL; 50,000+ rows/second achievable |
| Our plan | Not specified |

**Assessment**: External #2 provides the only concrete batch recommendation (1000 rows). Our plan has no explicit batch size, relying on transactional bulk inserts. 1000 rows per batch is a reasonable default that balances memory and SQLite write performance — our plan should consider adding this.

---

## 5. Dedup Strategy

| Source | CIS_COMPO Duplicate Handling |
|--------|---------------------------|
| External #1 | Not explicitly addressed; generic "journaliser les valeurs inattendues" |
| External #2 | INSERT OR IGNORE — duplicates silently skipped at DB level |
| Our plan | HashSet dedup of `(CIS, substance_code, dosage)` tuples before insert; 1,455 duplicates removed (32,389 → 30,934 rows). Implemented in `01-04-PLAN.md` Task 5 (`dedup_compo` function) |

**Assessment**: Our plan is the most explicit. We identify the exact dedup key (`CIS + substance_code + dosage`), the exact duplicate count (1,455), and the resulting row count (30,934). External #2's INSERT OR IGNORE approach is simpler but less visible — dedup happens silently at the DB layer without counts being surfaced. External #1 has no dedup strategy documented.

---

## 6. Quality Checks

| Source | Post-Import Validation |
|--------|----------------------|
| External #1 | 5 SQL check categories: (1) completeness: row count vs source file, 0% tolerance; (2) referential coherence: orphan count, alert if >5% increase; (3) enum validity: zero out-of-domain values; (4) temporal coherence: AMM dates 1950→today; (5) regression detection: row count drop >2% vs previous import |
| External #2 | "Row count per table, absence of unexpected NULLs, FK coherence" |
| Our plan | `cargo test` on normalization functions; row count verification against known values (e.g., `drugs = 15,848`, `compositions = 30,934`); referential integrity tests; sample value checks (price, date, lab name formats) |

**Assessment**: External #1 has the most sophisticated QC framework. Our plan lacks: (1) orphan percentage delta alert, (2) regression detection with a >2% threshold, (3) temporal plausibility checks. Our `01-03-PLAN.md` tests cover normalization correctness, but the post-import QC layer is shallower than what External #1 proposes.

**Missing in all**: None of the sources address data freshness checks (date of last import vs file update date displayed on BDPM website).

---

## 7. CLI Subcommand Design

| Subcommand | External #1 | External #2 | Our Plan |
|------------|-------------|-------------|----------|
| `fetch` | ✓ (download all or changed) | ✓ (downloads all BDPM files) | Proposed via `bdpm-ingest fetch` (implied by `01-05`) |
| `import` | Not named | ✓ (parse + import) | ✓ `bdpm-ingest import [--full] [--file=NAME]` |
| `check` | ✓ (verify DB integrity) | ✓ (FK, duplicates, anomalies) | Not named; QC via tests |
| `status` | ✓ (display DB state, dates, counts) | Not named | Not named; `import_log` table exists |
| `update` | Not named | ✓ (fetch + import in one) | Not named |
| `monitor` | ✓ (daemon with adaptive polling) | Not named | Not named |
| `search` | ✓ (FTS5 full-text) | Not named | Not named |

**Assessment**: External #1 and #2 converge on a minimal CLI (`fetch`, `import`, `check`, `status`). Our plan has the import machinery but no explicit CLI design document. The `check` subcommand (run integrity QC without re-importing) and `status` subcommand (read `import_log` table) are logical extensions not yet planned. FTS5 `search` is future work.

---

## 8. Normalization Order

| Source | When Normalization Occurs |
|--------|--------------------------|
| External #1 | During parse (Stage 4: per-field normalize embedded in parsing pipeline) |
| External #2 | During parse (Stage 3: dates, percentages normalized inline during TSV split) |
| Our plan | **Separate pass**: parse (`01-02`) → normalize (`01-04`) → import (`01-05`) |

**Assessment**: This is the most significant architectural difference. External #1 and #2 embed normalization into the parse stage. Our plan separates normalization as a distinct phase with pure functions and independent tests.

**Tradeoff**:
- Embedded normalization (external): fewer data structure traversals, but harder to test in isolation
- Separate pass (ours): more passes over data, but each normalizer is independently testable and composable

Our approach aligns with the "pure functions with tests" goal in `01-04-PLAN.md`. However, it means the import orchestrator in `01-05` must handle the parse→normalize→import flow explicitly (which it does, in lines 75-81 of `01-05-PLAN.md`).

---

## Summary Matrix

| Aspect | External #1 | External #2 | Our Plan |
|--------|-------------|-------------|----------|
| Stage count | 5 | 5 | 3 phases |
| Encoding strategy | Hardcoded per file | Hardcoded per file | `encoding_rs` per file |
| Malformed row handling | Log + continue | Not specified | Field-count guard + counter |
| Transaction scope | Per file | Not specified | Per file + partial on failure |
| Batch size | Not specified | 1000 rows | Not specified |
| Dedup strategy | Not specified | INSERT OR IGNORE | HashSet dedup pre-insert |
| QC framework | 5 SQL categories | 3 categories | Unit tests + row counts |
| CLI coverage | 6 subcommands | 4 subcommands | 1 subcommand (import) |
| Normalization order | Embedded in parse | Embedded in parse | Separate pass |
| Orphan strategy | `is_orphan` flag | Not specified | `is_orphan` flag |
| Hash algorithm | SHA-256 | SHA-256 (computed during fetch) | BLAKE3 |

---

## Recommendations

1. **Adopt External #1's QC framework** — the 5 SQL check categories (completeness, orphans, enums, temporal, regression) are well-designed. Our plan should include a `bdpm check` subcommand that runs these queries.

2. **Add batch size to our import** — External #2's 1000-row batch recommendation is backed by a performance claim (50k+ rows/sec). Our `01-05-PLAN.md` should specify this.

3. **Add encoding fallback** — our plan has no strategy if primary encoding produces U+FFFD replacement characters. External #1 flags this risk. Consider a chardet/encoding_rs detective fallback.

4. **Add `status` and `check` CLI subcommands** — these are implied by the import_log table and QC requirements, but not planned explicitly.

5. **Document the normalization separation rationale** — our embedded-vs-separate tradeoff should be stated explicitly in `01-04-PLAN.md` rationale, as it's the main architectural departure from both external proposals.
