# Implementer Dry Run — BDPM Critical Fixes

**Date:** 2026-05-26
**Purpose:** Mentally implement the 5 CRITICAL fixes to identify plan gaps before writing code

---

## FIX 1: Add Windows-1252 Encoding

### Plan location
- BRIEF.md: "8 files ISO-8859-1" (needs correction to Windows-1252)
- 01-01-PLAN.md Task 2: `Encoding { Latin1, Utf8 }` enum (line ~100)
- 01-02-PLAN.md Task 1: "Decode using `encoding_rs::ISO_8859_1` or `UTF_8`" (line ~21)

### Mental implementation

**Step 1: Add Windows1252 to Encoding enum**

Current plan (01-01, ~line 100):
```rust
pub enum Encoding { Latin1, Utf8 }
```

New:
```rust
pub enum Encoding { Windows1252, Latin1, Utf8 }
```

**Step 2: Assign Windows1252 to 7 files**

From 01-01 Task 2, change these lines:
- CIS_bdpm: 12 fields, **Windows1252**, F0=CIS (PK), F7=DD/MM/YYYY
- CIS_CIP: 13 fields, UTF-8, F1=CIP (PK) ← stays UTF-8
- CIS_COMPO: 8 fields, **Windows1252**, F0=CIS
- CIS_HAS_SMR: 6 fields, **Windows1252**, F3=YYYYMMDD, F5=avis
- CIS_HAS_ASMR: 6 fields, **Windows1252**, F3=YYYYMMDD, F5=avis
- CIS_GENER: 5 fields, **Windows1252**, F0=group_id, F3=type(0/1/2/4)
- CIS_CPD: 2 fields, **Windows1252**
- CIS_CIP_Dispo: 8 fields, **Latin1** ← changed from UTF-8 per W1
- CIS_MITM: 4 fields, **Windows1252**, F1=ATC
- HAS_Liens: 2 fields, **Windows1252**

**Step 3: Update TabParser decode function**

Current plan (01-02 Task 1, ~line 21):
```rust
fn from_file(path: &Path, encoding: Encoding) -> Result<Self> {
    // Decode using encoding_rs::ISO_8859_1 or UTF_8
}
```

New:
```rust
fn from_file(path: &Path, encoding: Encoding) -> Result<Self> {
    let decoder = match encoding {
        Encoding::Windows1252 => encoding_rs::WINDOWS_1252.new_decoder(),
        Encoding::Latin1 => encoding_rs::ISO_8859_1.new_decoder(),
        Encoding::Utf8 => encoding_rs::UTF_8.new_decoder(),
    };
    // ... decode bytes through decoder
}
```

### Where plan is CONCRETE
- Enum variant name: `Windows1252` (confirmed from consolidated-audit.md)
- Files to assign: 7 files listed explicitly
- Decoder usage: `encoding_rs::WINDOWS_1252.new_decoder()`

### Where plan is AMBIGUOUS
- **Dependency version:** Does `encoding_rs = "0.25"` in Cargo.toml support `WINDOWS_1252` constant? The plan uses `0.25` but 01-01 doesn't specify. Need to verify `encoding_rs::WINDOWS_1252` exists in that version. The plan doesn't specify which encoding_rs API to use.
- **Decoder API:** The plan doesn't specify the exact API for `encoding_rs::Decoder`. Is it `new_decoder()` or `Decoder::new()`? Need to verify.

### Where plan is MISSING
- **Cargo.toml:** No explicit update specified. Needs: add `encoding_rs = "0.25"` (currently planned in 01-01 dependencies).
- **Fix the false claim:** 01-02 line ~22 says "handles Windows-1252 residuals natively: `\x92 → U+2019`" — this is factually wrong. ISO-8859-1 does NOT do this. The plan needs this statement removed or corrected.

### Specific changes needed
1. 01-01-PLAN.md: Add `Windows1252` to Encoding enum, update file assignments, add `encoding_rs` to Cargo.toml
2. 01-02-PLAN.md: Update decode function, REMOVE false claim about ISO-8859-1 handling 0x92
3. BRIEF.md: Update "8 files ISO-8859-1" → "7 files Windows-1252, 1 file Latin-1, 2 files UTF-8"

---

## FIX 2: Smart Quote Normalization (U+2019 → U+0027)

### Plan location
- 01-04-PLAN.md: normalize/fields.rs for field normalization
- NOT mentioned anywhere in current plan

### Mental implementation

**Step 1: Decide where normalization happens**

Two options:
- **Option A:** In `normalize_rows()` in import orchestrator (01-05): Apply to all fields before insert
- **Option B:** In each per-file normalizer (01-04): Only normalize when reading HAS files

Current plan has no `normalize_rows()` function. The plan has individual normalizers (price, date, fields) but no centralized pipeline.

**Step 2: Implement normalization**

```rust
/// Normalize curly apostrophe (U+2019) to straight apostrophe (U+0027).
/// Happens AFTER Windows-1252 decode converts 0x92 → U+2019.
pub fn normalize_apostrophe(raw: &str) -> String {
    raw.replace('\u{2019}', '\u{0027}')
       .replace('\u{2018}', '\u{0027}')  // also normalize left curly
}
```

### Where plan is CONCRETE
- Target: U+2019 → U+0027 (confirmed from consolidated-audit.md E2)
- Files affected: CIS_HAS_SMR_bdpm.txt, CIS_HAS_ASMR_bdpm.txt (52K instances)

### Where plan is AMBIGUOUS
- **Pipeline location:** Plan doesn't specify WHERE this normalization happens. Is it:
  - In TabParser (post-decode)? Would apply to ALL files, not just HAS
  - In `normalize_rows()` (new function in 01-04 or 01-05)?
  - In each per-file normalizer?
  - In SQL before insert (e.g., `REPLACE(col, '\u{2019}', '\u{0027}')`)?
- **Scope:** Should this normalization apply to ALL files or only HAS files? U+2019 could appear in any Windows-1252 file. Plan doesn't specify.

### Where plan is MISSING
- **No normalization step exists** in 01-04. The plan has `strip_field()`, `normalize_spaces()`, `normalize_apostrophe()` but NO mention of curly quote handling.
- **No test defined:** No test case like `normalize_apostrophe("drogue's") → "drogue's"` in the verification section.

### Specific changes needed
1. 01-04-PLAN.md: Add `normalize_apostrophe()` to `src/normalize/fields.rs` (or create `src/normalize/quotes.rs`)
2. 01-04-PLAN.md: Add `normalize_rows()` pipeline that chains all normalizers (decide location)
3. 01-04-PLAN.md: Add test case for curly quote normalization
4. BRIEF.md: Add "Smart quote normalization (U+2019 → U+0027)" to data characteristics

---

## FIX 3: Trailing Tab Stripping for CIS_CIP_Dispo

### Plan location
- 01-02-PLAN.md Task 1: TabParser strips trailing `\r`, skips empty lines
- 01-02-PLAN.md Task 2: Field-count guard: `record.fields.len() == schema.field_count`
- BRIEF.md: "CIS_CIP_Dispo_Spec.txt — 8 fields"

### Mental implementation

**The problem:** 96% of CIS_CIP_Dispo lines have a phantom 9th field (empty string from trailing tab). Our field-count guard would reject 96% of rows.

**Step 1: Understand the issue**

From raw file analysis (not in plan, but known from audit):
- CIS_CIP_Dispo has 766 lines
- Most lines look like: `field1\tfield2\t...\tfield8\t` (8 real fields + trailing empty)
- TabParser splits on `\t`, producing 9 elements (last is empty string)
- Field-count guard checks `len == 8`, fails on `len == 9`

**Step 2: Fix approaches**

Option A: Strip trailing empty field in TabParser:
```rust
// In TabParser::next_record
let fields: Vec<String> = line.split('\t').map(|s| s.to_string()).collect();
// Strip trailing empty field (phantom tab from Windows-1252 files)
if fields.last().map(|s| s.is_empty()).unwrap_or(false) {
    fields.pop();
}
```

Option B: Accept 8 or 9 fields in field-count guard:
```rust
// In RowIterator::next
let len = record.fields.len();
if len != schema.field_count && len != schema.field_count + 1 {
    // log mismatch, skip row
}
```

Option C: Strip trailing tab in raw line before split:
```rust
// In TabParser::next_record
let line = line.trim_end_matches('\t');  // strip trailing tabs
let fields: Vec<String> = line.split('\t').map(|s| s.to_string()).collect();
```

### Where plan is CONCRETE
- File: CIS_CIP_Dispo_Spec.txt (8 fields per BRIEF.md)
- Root cause: trailing tab creates phantom 9th field
- Solution direction: strip before or after split

### Where plan is AMBIGUOUS
- **Which approach:** Plan doesn't specify HOW to handle the phantom field. The field-count guard is a good idea but doesn't account for this specific file.
- **Scope:** Should the trailing-tab stripping be file-specific or general? Are other files affected?
- **Log vs. silent:** If we strip silently, should we emit a warning?

### Where plan is MISSING
- **The phantom field issue is not mentioned in ANY plan file.** BRIEF.md, 01-02, 01-04, 01-05 all assume field count is exact.
- **No test defined:** No test case for CIS_CIP_Dispo field count in the verification section.

### Specific changes needed
1. 01-02-PLAN.md: Add note about trailing tab in CIS_CIP_Dispo to Task 1
2. 01-02-PLAN.md: Choose and specify ONE approach for phantom field stripping
3. 01-02-PLAN.md: Add verification step: "CIS_CIP_Dispo parses to exactly 766 rows"

---

## FIX 4: Dispo CHECK Constraint (1,2,3,4)

### Plan location
- BRIEF.md availability table: `status_type INTEGER — CHECK IN (1, 4)` (line ~255)
- 01-05-PLAN.md Task 2: "availability: INSERT with parsed DD/MM/YYYY dates" — no CHECK mentioned
- 01-05-PLAN.md Task 1: migrations/001_initial.sql reference only

### Mental implementation

**Step 1: Update SQL schema**

Current plan (BRIEF.md ~line 255):
```sql
CREATE TABLE availability (
    ...
    status_type            INTEGER,  -- 1=Rupture, 4=Remise à dispo; CHECK IN (1, 4)
    ...
);
```

New:
```sql
CREATE TABLE availability (
    ...
    status_type            INTEGER,  -- 1=Rupture, 2=Tension, 3=Arrêt, 4=Remise à dispo; CHECK IN (1,2,3,4)
    ...
);
```

**Step 2: Update import function**

Current plan (01-05 Task 2) doesn't validate status_type before insert:
```rust
// availability.rs
pub fn import_availability(conn: &Connection, rows: &[ValidatedRow]) -> Result<ImportStats> {
    let mut stmt = conn.prepare_cached(
        "INSERT INTO availability (cis, cip, status_type, status, date_start, ...) VALUES (?,?,?,?,?,...)"
    )?;
    for row in rows {
        let status_type: i32 = row.fields[6].parse()?;
        stmt.execute(params![row.fields[0], row.fields[1], status_type, ...])?;
    }
    Ok(stats)
}
```

New: Add validation:
```rust
let status_type: i32 = row.fields[6].trim().parse().map_err(|_| {
    ImportError::InvalidField("status_type", row.fields[6].clone())
})?;
if !matches!(status_type, 1 | 2 | 3 | 4) {
    return Err(ImportError::InvalidStatusType(status_type, row.line_number));
}
```

### Where plan is CONCRETE
- SQL syntax: `CHECK IN (1,2,3,4)` — this is SQLite-compatible
- Validation: must validate before insert (not rely on CHECK constraint alone)
- Status codes: 1, 2, 3, 4 with meanings confirmed

### Where plan is AMBIGUOUS
- **FK validation:** The orphan FK issue (V3 from audit: 436 Dispo rows reference non-existent CIS) — does the import function handle this? The plan doesn't mention.
- **Import order:** Does availability import AFTER drugs? 01-05 Task 3 import order puts CIS_CIP_Dispo last — correct, but plan doesn't explain WHY (depends on drugs FK).

### Where plan is MISSING
- **Migration location:** The SQL is referenced in 01-05 Task 1 as `migrations/001_initial.sql` but the file doesn't exist. Need to create it.
- **Validation in import:** 01-05 doesn't specify that import functions must validate enum values before insert.
- **Error handling:** What happens on invalid status_type? Skip row? Abort import? Plan doesn't specify.

### Specific changes needed
1. BRIEF.md: Update availability table schema `CHECK IN (1,2,3,4)`
2. 01-05-PLAN.md Task 1: Create `migrations/001_initial.sql` with correct CHECK
3. 01-05-PLAN.md Task 2: Add validation to `import_availability()`
4. 01-05-PLAN.md Task 2: Add error handling for invalid status_type

---

## FIX 5: URL Pattern Fix (/download/file/XXX)

### Plan location
- BRIEF.md: base-donnees-publique.medicaments.gouv.fr/telechargement
- 01-01-PLAN.md Task 3: Fetcher downloads files, URL construction not specified
- E1 from consolidated-audit.md: "/telechargement?fich=XXX no longer works; use /download/file/XXX"

### Mental implementation

**Step 1: Define base URLs per file**

Current plan: No URL structure defined. The Fetcher just takes a URL.

New: BDPMFile manifest carries URL:
```rust
pub enum BDPMFile {
    CIS_bdpm = BDPMFile::new("CIS_bdpm.txt", "/download/file/CIS_bdpm.txt", ...),
    CIS_CIP_bdpm = BDPMFile::new("CIS_CIP_bdpm.txt", "/download/file/CIS_CIP_bdpm.txt", ...),
    // ...
}
```

Full URL would be:
```
https://base-donnees-publique.medicames.medicaments.gouv.fr/telechargement/download/file/CIS_bdpm.txt
```

**Step 2: Fetcher builds URL from manifest**

```rust
impl Fetcher {
    pub fn fetch(&self, file: &BDPMFile, dest_dir: &Path) -> Result<(Vec<u8>, u64, String)> {
        let url = format!("{}{}", BASE_URL, file.download_path());
        // GET request...
    }
}
```

### Where plan is CONCRETE
- New URL pattern: `/download/file/{filename}` (confirmed from audit E1)
- Fetcher already exists (01-01 Task 3)
- Base domain: `base-donnees-publique.medicaments.gouv.fr`

### Where plan is AMBIGUOUS
- **Live verification:** The audit says "/telechargement?fich=XXX no longer works" but this is from external review, NOT verified against live server. The plan needs a verification step before implementation.
- **URL construction:** How are file-specific paths constructed? Is it `/download/file/{filename}` for ALL files? What about CIS_InfoImportantes which has dynamic filename?
- **Fallback:** If `/download/file/` doesn't work, what's the backup URL?

### Where plan is MISSING
- **The URL pattern is not specified in ANY plan file.** 01-01 mentions Fetcher but doesn't define the URL structure.
- **Live server verification:** No plan step says "verify URL pattern against live server before implementing Fetcher."
- **Base URL constant:** No `BASE_URL` defined anywhere.

### Specific changes needed
1. 01-01-PLAN.md Task 3: Add URL construction details to Fetcher
2. 01-01-PLAN.md: Add `BASE_URL` constant and per-file path
3. Add verification step: "Test fetch CIS_bdpm.txt from live server before finalizing"
4. BRIEF.md: Update "URL pattern" section with `/download/file/` path

---

## Cross-Cutting Issues Found

### Issue A: No migrations/001_initial.sql file exists
- Referenced in 01-05 Task 1
- Contains ALL CREATE TABLE statements from BRIEF.md
- Does not exist in the plan directory
- **Action needed:** Create this file in the plan

### Issue B: BDPMFile manifest is underspecified
- 01-01 Task 2 defines the struct but doesn't show the full enum with all 10 files
- Missing: download_path, full schema per file
- **Action needed:** Add complete BDPMFile enum definition to 01-01

### Issue C: Import order dependency on FK
- 01-05 Task 3 specifies import order but doesn't explain WHY
- drugs must be first (FK dependency for all others)
- **Action needed:** Add explanation of FK dependency chain

### Issue D: Orphan FK handling (V3 from audit)
- 18.4% of SMR, 15.8% of ASMR, 23.5% of GENER rows reference non-existent CIS
- Current plan: `PRAGMA foreign_keys=ON` + strict FK
- This would cause 56% of availability and ~19% of SMR/ASMR inserts to FAIL
- **Plan says:** "On any INSERT failure: ROLLBACK, return error" — but this would cause FULL import failure on partial orphan data
- **Action needed:** Decide: disable FK during import (01-05 Task 1 says `PRAGMA foreign_keys=ON`) or use `INSERT OR IGNORE` for orphan-referencing tables

---

## Summary: Plan Completeness by Fix

| Fix | Concrete | Ambiguous | Missing |
|-----|----------|-----------|---------|
| Windows-1252 | 60% | 20% (version, API) | 20% (Cargo, claim fix) |
| Smart quotes | 30% | 40% (location, scope) | 30% (function, tests) |
| Trailing tab | 40% | 30% (approach) | 30% (awareness, tests) |
| Dispo CHECK | 70% | 20% (orphan FK) | 10% (migration file) |
| URL pattern | 30% | 50% (live verification) | 20% (URL definition) |

**Overall:** The plan is ~50% complete for critical fixes. The encoding fix is clearest; URL pattern is least defined.