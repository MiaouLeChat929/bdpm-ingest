# ATC Data Source Research

**Date:** 2026-05-26
**Table:** `atc_codes` in BDMP schema
**Goal:** Populate from authoritative WHO source

---

## 1. Authoritative Source: WHO Collaborating Centre for Drug Statistics Methodology

**Primary URL:** https://atcddd.fhi.no/atc_ddd_index/

### Commercial Option
The WHOCC sells electronic files (.xlsx/.xlm) for **EUR 200**. This includes the full ATC-DDD index with alterations tracked via an API. Access to the ATC alteration API is granted upon purchase until the next annual update.

### Free Option (Web Scraping)
The same data is published freely on the WHOCC website. Web scraping is permissible per the copyright disclaimer (see below).

### License / Copyright
From https://atcddd.fhi.no/copyright_disclaimer/:
- Reference to WHOCC required when using material
- Commercial distribution NOT allowed
- Changing/manipulating material NOT allowed
- The WHOCC is funded by the Norwegian government; sales income funds ATC/DDD system maintenance

**Conclusion:** Non-commercial scraping + bundling as static data in a personal hobby project is acceptable under this license.

---

## 2. Best Available CSV: fabkury/atcd GitHub Repository

**URL:** https://github.com/fabkury/atcd

This repository scrapes the WHOCC website and provides a ready-to-use CSV with snapshot files.

### Key Facts (as of 2026-04-25):
- **6,996 unique ATC codes** across all levels
- CSV columns: `atc_code,atc_name,ddd,uom,adm_r,note`
- No parent columns — hierarchy must be derived from code structure

### ATC Level Breakdown
| Level | Length | Count | Example |
|-------|--------|------:|---------|
| 1 | 1 char | 14 | `N` |
| 2 | 3 chars | 94 | `N02` |
| 3 | 4 chars | 271 | `N02A` |
| 4 | 5 chars | 939 | `N02AX` |
| 5 | 7 chars | 5,678 | `N02AX02` |

### Download Link
```
https://github.com/fabkury/atcd/raw/master/WHO%20ATC-DDD%202026-04-25.csv
```
File is approximately 310 KB.

### Combinations File (244 rows)
Separate file for combination products (antimicrobials, etc.) that lack DDD on the main index:
```
WHO ATC-DDD-combinations 2026-04-25.csv
```
Contains: `atc_code,brand_name,dosage_form,ingredients,ddd_comb`

---

## 3. Parent Hierarchy Derivation

The CSV has no explicit parent columns. However, ATC codes are structurally hierarchical:
- **parent_5_char**: first 5 chars of 7-char code (e.g., `N02AX` from `N02AX02`)
- **parent_3_char**: first 3 chars (e.g., `N02` from `N02AX02`)
- **parent_1_char**: first 1 char (e.g., `N` from `N02AX02`)

For codes shorter than 7 chars (levels 1-4), parent references still apply but may not exist in the table since parents could be level-1 through level-4 themselves.

### Algorithm
```rust
fn derive_parents(code: &str) -> (Option<&str>, Option<&str>, Option<&str>) {
    match code.len() {
        7 => (Some(&code[0..5]), Some(&code[0..3]), Some(&code[0..1])),
        5 => (None, Some(&code[0..3]), Some(&code[0..1])),
        4 => (None, None, Some(&code[0..1])),
        3 => (None, None, Some(&code[0..1])),
        1 => (None, None, None),
        _ => (None, None, None),
    }
}
```

### Handling Missing Parents
The CSV includes codes at all levels. For level-1 (1-char), level-2 (3-char), and level-3 (4-char) codes, `parent_5_char` and `parent_3_char` will be NULL — which matches your schema where those columns are nullable.

---

## 4. No Rust Crates Available

Searched crates.io for "atc" and "who atc" — no relevant drug classification crates exist. The ecosystem has general-purpose routing crates (matchers, matchit) and flight ATC crates, but nothing for pharmaceutical ATC classification.

---

## 5. Wikidata Alternative (not recommended)

Wikidata contains ATC classification via property P372 (therapeutic chemical identification code per ATC). However, coverage is incomplete (~4,446 statements), and extracting the full hierarchy requires complex SPARQL queries. Not a viable primary source.

---

## 6. Import Strategy Recommendation

For a **solo hobby project**, the pragmatic approach is:

### Option A: Bundle Static CSV (Recommended)
1. Download `WHO ATC-DDD 2026-04-25.csv` from the atcd repo
2. Pre-process with a script that adds parent columns (`parent_5_char`, `parent_3_char`, `parent_1_char`)
3. Store as `atc_codes.csv` in the project (e.g., under `data/` or `assets/`)
4. At import time, load from the bundled CSV
5. On future updates, re-download and regenerate the CSV

**Pros:** Zero runtime dependency, reproducible, fast (6,996 rows)
**Cons:** Requires re-download when WHO updates (1-2x/year)

### Option B: Build-Time Download (Alternative)
Add a `build.rs` or Makefile target that downloads the latest CSV and pre-processes it into a generated module. Keeps data fresh but adds complexity.

### Option C: Runtime Fetch (Not Recommended)
Fetching from the WHOCC website at runtime adds latency, failure modes, and rate-limiting concerns. Not suitable for a local CLI tool.

---

## 7. Implementation Notes

### Pre-processing Script (Rust or Python)
```python
import csv

def add_parents(input_file, output_file):
    with open(input_file, 'r') as f:
        reader = csv.DictReader(f)
        rows = list(reader)

    with open(output_file, 'w', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(['atc_code', 'parent_5_char', 'parent_3_char', 'parent_1_char'])
        for row in rows:
            code = row['atc_code']
            length = len(code)
            parent_5 = code[:5] if length >= 5 else None
            parent_3 = code[:3] if length >= 3 else None
            parent_1 = code[:1] if length >= 1 else None
            writer.writerow([code, parent_5, parent_3, parent_1])
```

### Insert Strategy
6,996 rows with simple `INSERT OR REPLACE` statements. Could be done as a single transaction for speed (~10ms total).

### Update Cadence
The atcd repo is updated periodically. Check the repo releases or re-scrape annually. WHOCC makes alterations available for free via their website; the annual purchased index includes API access for alterations.

---

## 8. Summary

| Aspect | Value |
|--------|-------|
| Authoritative source | https://atcddd.fhi.no/atc_ddd_index/ |
| Best free CSV | fabkury/atcd GitHub (6,996 codes, 2026-04-25 snapshot) |
| Download URL | https://github.com/fabkury/atcd/raw/master/WHO%20ATC-DDD%202026-04-25.csv |
| Row count | 6,996 (all levels) |
| License | WHOCC copyright — non-commercial OK, commercial requires purchase |
| Parent derivation | String slice from code (first 5/3/1 chars) |
| Rust crates | None found |
| Recommended approach | Bundle static CSV + pre-process to add parent columns |