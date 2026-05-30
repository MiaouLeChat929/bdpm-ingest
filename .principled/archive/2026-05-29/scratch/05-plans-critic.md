## 05-01 Critic Findings

### HIGH

- **EAN-13 validation location is misattributed**: The plan says to add the validation call in `normalize_cis_cip()` and places the helper in `src/normalize/fields.rs`. But `normalize_cis_cip()` lives in `src/normalize/mod.rs` (internal function, not exported). The `validate_ean13()` helper can go in `fields.rs` (correct), but the call site injection must be into `normalize_cis_cip()` in mod.rs — which the plan never explicitly names. The "Files: src/normalize/fields.rs, src/import/mod.rs" entry under Task 4 omits mod.rs entirely. An executor would add the call to the wrong location or not know where to add it.

### MEDIUM

- **ImportStats counter unspecified**: Task 4 says "add a counter to `ImportStats` for `invalid_ean13`" but provides no details on field type, field name, or how the counter gets incremented. `ImportStats` in `src/import/mod.rs` (line 822) only has `rows_imported: usize` and `bad_rows: usize`. Adding a new field requires defining the field, initializing it, incrementing it from the validation call, and reporting it. The plan skips all of this.

- **`server_integration_test()` block reference is stale**: Task 1 says to also update "the `server_integration_test()` block (around line 160)" with the parent hierarchy UPDATE. No such function appears in import/mod.rs. Either the function was removed or the plan copied a reference from a different era of the codebase. Executor would search and not find it.

- **`gtin-validate = "1.3"` is a non-issue but inconsistent with existing Cargo.toml style**: The existing deps in Cargo.toml use bare version strings (no caret, no patch pinning). Specifying `"1.3"` is technically a caret range. Minor inconsistency, but easy to harmonize.

## 05-02 Critic Findings

### HIGH

- **`phf_map!` does not support OR patterns**: The plan shows entries like:
  ```rust
  "comprime" | "comprime pellicule" | "comprime enrobe" | "comprime enrobe gastro" => "CPR",
  ```
  This is **not valid syntax** for `phf::phf_map!`. The macro accepts `key => value` pairs only. OR patterns require separate entries per key. The plan's `FORM_CANONICAL` would fail to compile. Each variant must be its own key mapping to the same value.

### MEDIUM

- **`generic_groups.type = 'reference'` uses wrong value**: Task 2's threshold check query uses `WHERE type = 'reference'`. Looking at schema.sql line 91: `CHECK (type IN ('reference', 'generic', 'cross-group', 'sustained-release'))`. So `'reference'` is correct for the constraint. However, the plan's reference text says "Princeps coverage" with a `type = 'reference'` filter — this is correct against the schema, but the plan never verifies this against the actual schema CHECK constraint. If the constraint ever changes, the plan's query silently returns wrong results.

- **Missing schema for `prescription_flags` table location**: Task 4 says to add a new table to `src/db/schema.sql`, but schema.sql is a consolidated initialization file — the plan must specify exactly where in the file (before/after which table) and whether the table should be added before running ingest or as a separate migration step. The `CREATE TABLE` block is provided but the placement instruction is absent.

- **`CpdFlags::from_rule` — pattern matching by string comparison is fragile**: The implementation iterates over a `HashMap<&str, Regex>` and matches flag names as strings in a match statement. Adding a new flag requires updating both the `HashMap` entries and the match arms — two places that must stay in sync. If a developer adds an entry to the HashMap but forgets the match arm, the flag silently does nothing. This is a code smell, not a plan bug per se, but the plan should note the maintenance hazard.

- **Duplicate `"comprime pellicule"` entry in FORM_CANONICAL**: The plan shows `"comprime pellicule"` listed twice (once standalone, once as the start of a longer chain). Triggers: the first OR-pattern chain starts with `"comprime" | "comprime pellicule"`, which the chain itself is invalid (see above), and then the standalone `"comprime pellicule" => "CPR"` appears again. Copy-paste artifact.

## 05-03 Critic Findings

### HIGH

- **Parallel normalization conflicts with existing `dedup_compo` ordering**: The existing pipeline in import/mod.rs (lines 58-72) does:
  1. `filter_map(normalize_row)` → produces Vec<NormalizedRow>
  2. `dedup_compo()` → sequential dedup
  3. `import_file()`

  The plan's proposed parallel approach (Task 4) introduces rayon in the normalization step, but `dedup_compo()` is a sequential HashSet-based dedup that runs after normalization in the existing code. The plan says to parallelize "CIS_COMPO normalization" but provides no implementation for where dedup fits in the new parallelized pipeline. If dedup runs after parallel normalization, the parallelism is preserved — but the plan doesn't verify this ordering or explain it. An executor implementing `normalize_compo_parallel()` as a drop-in replacement for the current normalization block could accidentally remove or misplace `dedup_compo()`.

- **`substance_name_clean` column may not exist at plan-writing time**: The plan's threshold query references `substance_name_clean` from compositions. Looking at schema.sql lines 64-77, `substance_name_clean TEXT` is defined. However, the plan references it as a query column without confirming it was created in an earlier phase. If 05-01 or 05-02 adds this column, fine. If not, the query fails. The plan should either confirm the column exists or specify its creation.

### MEDIUM

- **`strip_salt` prefix matching is fragile with mixed-case input**: The SALT_PREFIXES list has entries like `"chlorhydrate de"`, `"chlorhydrate d'"`. The strip logic upper-cases both the input and the prefix:
  ```rust
  let prefix_upper = prefix.to_uppercase();
  let result_upper = result.to_uppercase();
  if result_upper.starts_with(&prefix_upper) {
  ```
  This handles mixed-case correctly. But SALT_SUFFIXES includes `"chlorhydrate monohydrate"` (with space, lowercase). The suffix check does the same upper-casing. So `"PARACETAMOL CHLORHYDRATE MONOHYDRATE"` would correctly strip `"CHLORHYDRATE MONOHYDRATE"`. This works, but the plan doesn't explain why the suffix list has `"chlorhydrate monohydrate"` (no leading space) and `"chlorhydrate"` separately. An executor adding entries needs to know whether leading spaces matter in the suffix list.

- **`strip_parens` pattern has a subtle bug**: The regex `r"\s*\([^)]*\b(chlorhydrate|sulfate|sel|base|hydrate)\b[^)]*\)"` uses `[^)]*` (any character except `)`) which is non-greedy in Rust's `regex_lite`. The `\b` word boundaries around "hydrate" work for `"hydrate"` but the outer `[^(]` doesn't exclude parentheses — it excludes right-paren only. A string like `"Paracetamol (chlorhydrate (monohydrate))"` would fail to match past the inner `(`. This is an edge case that won't appear in BDPM data, but the regex pattern is slightly wrong. The plan should either fix the pattern to `r"\s*\([^)]*\)"` or add a comment noting the limitation.

- **Threshold query for `substance_name_clean` coverage uses a column that might be null-heavy**: The query:
  ```sql
  SELECT COUNT(DISTINCT substance_name_clean) * 1.0 /
   NULLIF(COUNT(DISTINCT substance_code), 0) FROM compositions
   WHERE substance_name_clean IS NOT NULL
  ```
  If `substance_name_clean` is newly added by this phase and populated via `normalize_compo()` (which stores it in `values[8]`), then the coverage could be very low until the column is backfilled. The plan doesn't address this — it treats the threshold as a given, but the initial run after Phase 05 would likely trigger a false-positive warning because the column is new and populated from the current normalization only (no historical backfill).

- **FTS normalization (`fts_normalize`) requires updating `src/db/fts.rs` — file not referenced**: Task 3 says to "Update `src/db/fts.rs` to use `fts_normalize()` when inserting into FTS columns." But fts.rs is not listed in the Context section, not checked against the current code, and not named in the Action step's Files list. An executor would have to find fts.rs and figure out which insert paths need updating.

## Cross-Plan Issues

- **05-02 Task 3 (lab_name_canonical) modifies the same INSERT path that 05-03 Task 1 (salt stripping) touches for compositions**: Both plans modify normalization output. 05-02 adds a new column to `drugs`, 05-03 adds salt stripping to `substance_name` in compositions. These don't conflict but they touch adjacent INSERT paths in the same import pipeline. No cross-plan sequencing is specified — an executor doing 05-03 before 05-02 would implement salt stripping on a schema that doesn't yet have the canonical lab column, and vice versa. Safe to execute in either order but the plans should note independence.

- **`substance_name_clean` column status is ambiguous across plans**: 05-03 queries this column (threshold check) and populates it via `normalize_compo()` (values[8]). 05-02 doesn't mention it. The column exists in schema.sql, so this is fine — but the plans never reference the schema directly for this, relying on implicit knowledge.

- **No unified verification: each plan specifies its own test suite count**: 05-01 expects 155+ tests after completion, 05-02 expects pass, 05-03 expects pass. No plan references the others' test counts. If 05-02 breaks a test that 05-01 added, there's no mechanism to catch it until the final integration run.

## Verdict

Executing these plans right now: **PARTIAL FAILURE** expected.

**05-01** would likely succeed with minor corrections (correct the mod.rs call site for EAN-13, define ImportStats field). The ATC parent hierarchy task is well-specified. The homeopathy expansion is detailed and matches current code. Only the EAN-13 task has the misattribution issue.

**05-02** would fail at compile time on the phf_map! syntax in Task 1. The OR patterns in the FORM_CANONICAL and ROUTE_CANONICAL maps are syntactically invalid. Fixing this requires expanding ~50 entries into separate key-value pairs per variant — a significant rewrite of the plan's examples.

**05-03** would likely succeed for Tasks 1-3. Task 4 (parallelization) requires clarification of where dedup fits in the new parallelized flow. The current code sequence is normalize → dedup → import. The plan's parallel approach is plausible but the dedup ordering is not confirmed, and the plan doesn't provide the actual code to integrate rayon into the existing normalization loop in import/mod.rs.

**What must change first:**
1. 05-02: Rewrite FORM_CANONICAL and ROUTE_CANONICAL with separate entries per key (no OR patterns)
2. 05-01: Specify the `validate_ean13()` call site as `src/normalize/mod.rs` normalize_cis_cip() — not fields.rs. Define the ImportStats field increment.
3. 05-03: Confirm `substance_name_clean` column exists and is populated before threshold queries run. Clarify dedup ordering relative to parallel normalization.