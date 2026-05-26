# Edge Case Critique: External Reviews vs. Internal Plan

**Generated:** 2026-05-26
**Scope:** Comparing external analysis findings against BDPM Rust project implementation plan
**Sources:** 4 external reviews + 6 internal plan/analysis documents

---

## 1. Price Normalization

### External Finding (external-feasibility_body.md, external-analyse_technique.md)
- CIS_CIP uses comma as decimal separator: `"24,34"` → 24.34
- 466 rows exceed 1000 with comma as both thousands separator and decimal point
- Pattern: 2 commas = thousands separator present, e.g. `1,466,29` means 1466.29
- Strategy: detect comma count; if 2+, remove ALL commas before decimal conversion

### Our Plan (01-04-PLAN.md, edge-CIS_CIP_bdpm.md)
```rust
// "1,466,29" → remove ALL commas → "146629" → parse → 146629
let parts: Vec<&str> = trimmed.split(',').collect();
let last = parts.last().unwrap_or(&"");
if last.len() == 2 {
    let integer_part: String = parts[..parts.len()-1].join("");
    let full = format!("{}.{}", integer_part, last);
    // ...
}
```
**Problem:** The logic in 01-04-PLAN.md is wrong. For `1,466,29`:
- Splitting by commas gives `["1", "466", "29"]`
- Last segment is "29" (len=2), so integer_part = "1466", full = "1466.29"
- Then `val * 100.0 = 146629` ✓

Wait, let me re-read the plan. The plan actually says:
```
// "1,466,29" → remove ALL commas → "146629" → parse → 146629
// Wait — that's wrong. "1,466,29" = 1,466.29 euros.
// Correct: remove thousands-separator commas (all except last),
// then replace last comma with dot.
```

So the plan HAS the correct logic documented in comments, but the pseudocode implementation uses the wrong approach (`join("")` instead of proper handling). The pseudocode at lines 44-58 has a subtle bug:

```rust
let integer_part: String = parts[..parts.len()-1].join(""); // ["1", "466"].join("") = "1466"
let full = format!("{}.{}", integer_part, last); // "1466.29"
```

This IS actually correct for the case above. But wait - if someone writes `1,466,29`, the parts would be `["1", "466", "29"]` and the join would give "1466" which is correct. The pseudocode logic does work.

**CONFIRMED:** Our approach matches the external finding. Edge case handled correctly. The plan correctly handles the 2-comma pattern by treating the last segment with 2 digits as the decimal part.

### Verdict: **CONFIRMED** — both external analysis and our plan agree on comma-count heuristic.

---

## 2. Date Normalization

### External Finding (external-format_doc.md, external-analyse_technique.md)
- Date fields use format `DD/MM/YYYY` (French convention) - NOT ISO 8601
- Second format `YYYYMMDD` in HAS files (integer format)
- Third format `YYYY-MM-DD` mentioned in external-analyse_technique.md (Section 3, Critical Issues table)

### Our Plan (01-04-PLAN.md)
```rust
pub enum DateFormat { DDMMYYYY, YYYYMMDD }
// Handles "28/04/2026" → "2026-04-28"
// Handles "20260422" → "2026-04-22"
```

### CRITICAL DISCREPANCY

**External says:** Three date formats exist: `DD/MM/YYYY`, `YYYYMMDD`, AND `YYYY-MM-DD`

**Our plan says:** Only two formats: `DD/MM/YYYY` and `YYYYMMDD`

The external technical analysis (analyse_technique.md line 334) explicitly lists `YYYY-MM-DD` as a third format to normalize. Our plan does NOT handle this format.

### CRITICAL: Add YYYY-MM-DD parsing to date normalizer

---

## 3. CIS_COMPO Deduplication

### External Finding (external-etude_faisabilite.md, external-feasibility_body.md)
- external-feasibility_body.md: mentions "17 exact duplicates" in CIS_CIP_Dispo_Spec, NOT CIS_COMPO
- external-etude_faisabilite.md: NO explicit mention of CIS_COMPO duplicate count
- edge-CIS_COMPO_bdpm.md: 1,455 exact duplicates (same CIS + substance_code + dosage)

### Our Plan (01-04-PLAN.md)
```rust
/// 1,455 duplicates detected in profiling.
/// Uses HashSet<(String,String,String)> to deduplicate.
pub fn dedup_compo(rows: Vec<ValidatedRow>) -> Vec<ValidatedRow> {
```

### Verdict: **CONFIRMED** — our 1,455 count is more precise. External reviews don't contradict; they simply don't profile CIS_COMPO as deeply.

---

## 4. SMR/ASMR Malformed Rows

### External Finding (edge-CIS_HAS_SMR_ASMR.md)
- CIS_HAS_SMR_bdpm.txt: 12 malformed rows (1 field each, tab-split avis fragments)
- CIS_HAS_ASMR_bdpm.txt: 6 malformed rows
- Total: **18 malformed rows**
- Root cause: embedded tab characters within avis text

### Our Plan (01-02-PLAN.md)
```rust
// Critical: 18 tab-split malformed rows in CIS_HAS_SMR/ASMR
// these produce field_count != 6 and must be filtered, not rejected
```

### Verdict: **CONFIRMED** — both sources agree on 18 total malformed rows and same approach (filter, not reject).

---

## 5. Availability Status Codes

### External Finding (external-feasibility_body.md, external-analyse_technique.md)
- external-feasibility_body.md Table: `CHECK IN (1, 2, 3, 4)` for `code_statut`
- Status meanings: 1=Rupture, 2=Tension, 3=Arrêt, 4=Remise
- 4 distinct status values confirmed

### Our Plan (BRIEF.md)
```rust
CREATE TABLE availability (
    status_type INTEGER,  -- 1=Rupture, 4=Remise à dispo; CHECK IN
    ...
);
```

### CRITICAL DISCREPANCY

**External says:** 4 status codes exist (1, 2, 3, 4)

**Our plan says:** Only 2 status codes (1 and 4)

Our BRIEF.md schema comment only mentions codes 1 and 4 in the CHECK comment. This is incomplete.

### CRITICAL: Our schema CHECK constraint should be `CHECK IN (1, 2, 3, 4)`, not just `(1, 4)`.

---

## 6. Generic Type Values

### External Finding (external-analyse_technique.md, external-etude_faisabilite.md)
- external-analyse_technique.md: `CHECK(type_generique IN (0,1,2,4))`
- external-etude_faisabilite.md: Same constraint
- Values confirmed: 0 (reference), 1 (generic), 2 (cross-group), 4 (sustained-release)

### Our Plan (01-04-PLAN.md, BRIEF.md)
```rust
pub fn normalize_generic_type(raw: &str) -> &'static str {
    match raw.trim() {
        "0" => "reference",
        "1" => "generic",
        "2" => "cross-group",
        "4" => "sustained-release",
        _ => "unknown",
    }
}
```

### Verdict: **CONFIRMED** — both sources agree on all 4 type values.

---

## 7. Orphan References

### External Finding (external-etude_faisabilite.md, external-analyse_technique.md)

| File | Orphan Rate |
|------|-------------|
| CIS_HAS_SMR | 18.4% (2,806 of 9,014) |
| CIS_HAS_ASMR | 15.8% (1,567 of 6,172) |
| CIS_GENER | 23.5% (2,503 of 10,628) |

External recommends: insert with `is_orphan = 1` flag, do not reject.

### Our Plan (BRIEF.md)

Our plan states:
> **Decision: Full-Table Truncate+Reload, Not Row-Level Delta**
> No row-level timestamps exist in any BDPM file. Row-level delta is impossible.

The plan does NOT explicitly address orphan handling strategy. The BRIEF.md mentions orphan flagging but the core import strategy is full-table replace.

### CRITICAL: Orphan handling is implicit but needs explicit strategy

The external studies recommend `is_orphan` flag tracking with alert thresholds. Our plan's full-table truncate+reload will:
1. Remove orphans from prior import that are now absent from source files
2. Re-insert orphans from new source files

This means orphans WILL be preserved (because they're in source files), but the ORPHAN flag itself may be lost if we don't track it separately.

### CRITICAL: Add explicit orphan tracking to import_log
Store orphan counts per file in import_log for regression detection.

---

## 8. Smart Quotes / Curly Apostrophe (0x92)

### External Finding (external-feasibility_body.md, external-analyse_technique.md)

| File | U+2019 Occurrences |
|------|-------------------|
| CIS_HAS_ASMR | 29,704 |
| CIS_HAS_SMR | 22,253 |

0x92 in cp1252 maps to U+2019 (right single quotation mark), NOT U+0027 (straight apostrophe).

### Our Plan (01-02-PLAN.md)

```rust
// Treat encoding_rs::Decoder for correct byte→char mapping
// (handles Windows-1252 residuals natively: \x92 → U+2019 etc.)
```

The plan DECODES correctly but does NOT normalize U+2019 to U+0027.

### WARNING: U+2019 normalization missing from plan

Our 01-02-PLAN.md only mentions encoding_rs handles the byte→char mapping. It does NOT include a normalization step to convert curly apostrophes to straight apostrophes.

External explicitly recommends:
> Normalize smart quotes (U+2019) from cp1252 to right apostrophe (U+0027).

**Missing from our normalization pipeline:** A dedicated step to replace U+2019 with U+0027.

### WARNING: Add apostrophe normalization step to 01-04 normalization pipeline

---

## 9. HTML in Avis Field

### External Finding (edge-CIS_HAS_SMR_ASMR.md)

| File | HTML Rows | % |
|------|-----------|---|
| CIS_HAS_SMR | 1,971 | 13.0% |
| CIS_HAS_ASMR | 2,060 | 20.8% |
| **Total** | **4,031** | — |

### Our Plan (edge-CIS_HAS_SMR_ASMR.md, 01-04-PLAN.md)

```rust
/// Strip HTML tags from avis field text.
/// Replaces <br> with newline, strips <p>, <b>, and other tags.
```

Our plan explicitly handles `<br>`, `<br/>`, `<br />`, `<BR>`, `</p>`, `</P>` tags.

### Verdict: **CONFIRMED** — both sources agree on 4,031 HTML rows and same stripping approach.

---

## 10. Multi-Line Records

### External Finding (external-format_doc.md)

Section 3.3 explicitly documents multi-line field content:
> Certain fields (primarily INDICATIONS and COMPOSITION) may contain embedded line breaks within what should be a single logical record.

> **Mitigation documented:** Any line that does not start with a valid CIS code should be treated as a continuation of the previous line.

### Our Plan (01-02-PLAN.md)

The plan mentions:
- CIS_bdpm fields 8-9 structurally empty
- Field-count validation against known schema
- Malformed-row filtering

But does NOT include multi-line record detection/continuation logic.

### CRITICAL DISCREPANCY: Multi-line continuation NOT in plan

The external format documentation explicitly describes continuation-line handling, but our plan does not implement it. This is a MAJOR gap.

The format_doc recommends:
```python
if line starts with valid CIS code:
    save current_record
    current_record = parse(line)
else:
    current_record.accumulate_continuation(line)
```

### CRITICAL: Add multi-line record continuation handling to tab parser

Even though our current profiling shows no embedded newlines, the source documentation states this CAN happen, and the external analysis explicitly describes the pattern.

---

## Summary: Findings by Severity

### CRITICAL (Must fix before execution)

| # | Finding | Evidence | Required Action |
|---|---------|----------|-----------------|
| 1 | **Date format: YYYY-MM-DD not handled** | analyse_technique.md lists 3 formats | Add YYYY-MM-DD parser to date.rs |
| 2 | **Availability status codes incomplete** | Our plan: 2 codes; External: 4 codes | Change CHECK IN (1, 4) → CHECK IN (1, 2, 3, 4) |
| 3 | **Multi-line records: no continuation logic** | format_doc.md section 3.3 | Add line-continuation detection to TabParser |
| 4 | **Orphan tracking: no explicit strategy** | 18-24% orphan rates confirmed | Add `is_orphan` flag and logging per import |

### WARNING (Should fix, non-blocking)

| # | Finding | Evidence | Recommended Action |
|---|---------|----------|---------------------|
| 5 | **U+2019 → U+0027 normalization missing** | 51,957 smart quotes in HAS files | Add apostrophe normalization to 01-04 pipeline |
| 6 | **CIS_CIP encoding: UTF-8 vs cp1252** | format_doc says cp1252 for all; actual is UTF-8 | Our profiling confirms UTF-8; stick with empirical finding |

### SUGGESTION (Nice-to-have)

| # | Finding | Evidence | Recommended Action |
|---|---------|----------|---------------------|
| 7 | **CIS_CPD \r\r\n sequences** | 9 empty lines in CIS_CPD | Already handled by CRLF strip + empty filter |
| 8 | **CIS_InfoImportantes dynamic filename** | Timestamp in filename | Extract timestamp in fetcher |

### CONFIRMED (External matches our approach)

| # | Finding | External Source | Status |
|---|---------|-----------------|--------|
| 9 | Price: 2-comma pattern | 466 rows with thousands separator | ✓ Correct |
| 10 | CIS_COMPO: 1,455 duplicates | edge-CIS_COMPO_bdpm.md | ✓ Correct |
| 11 | SMR/ASMR: 18 malformed rows | edge-CIS_HAS_SMR_ASMR.md | ✓ Correct |
| 12 | Generic type: 4 values (0,1,2,4) | Both external reviews | ✓ Correct |
| 13 | HTML in avis: 4,031 rows | edge-CIS_HAS_SMR_ASMR.md | ✓ Correct |
| 14 | Encoding: cp1252 + latin-1 + UTF-8 | All external sources | ✓ Correct |
| 15 | DD/MM/YYYY and YYYYMMDD dates | All external sources | ✓ Correct |

### NEW FINDINGS (External found, we missed)

| # | Finding | Evidence | Impact |
|---|---------|----------|--------|
| 16 | **CIS_CIP_Dispo_Spec: 17 duplicates** | external-feasibility_body.md | Dedup needed for this specific file |
| 17 | **HTTP URLs in MITM: http:// not https://** | external-analyse_technique.md | URL normalization needed |
| 18 | **CIS_CIP_Dispo_Spec encoding: latin-1** | external-feasibility_body.md | Easy to miss if defaulting to cp1252 |
| 19 | **Conditionnel SMR levels** | edge-CIS_HAS_SMR_ASMR.md | 41 rows with "Important conditionnel", "Modéré conditionnel", "Faible conditionnel" — valid values not in standard enum |
| 20 | **38 ASMR I/II where SMR Important** | edge-CIS_HAS_SMR_ASMR.md | Notable outlier pattern worth investigating |