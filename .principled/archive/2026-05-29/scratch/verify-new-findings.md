# External Review Findings Verification

## Finding 1: Trailing Tabs in CIS_CIP_bdpm.txt

**Status: VERIFIED**

- **96.1%** of lines (20,089 / 20,904) have trailing tabs
- Creates phantom empty field 14 (data has 13 fields, trailing tab adds field 14)
- Example: `6/03/2011\t3400949497294\toui\t100%\t24,34\t25,36\t1,02\t`

**Impact:** Field indexing must account for phantom field. When splitting by tab and filtering empty trailing fields, the real data is fields 0-12 (13 fields).

---

## Finding 2: CIS_bdpm.txt Retention Policy

**Status: EXTERNAL CLAIM IS WRONG**

- External claim: "medications marketed or retired <5 years"
- Actual: No time-based retention policy exists

**Data range:**
- Oldest AMM date: 1974-03-11 (CIS 62691806: ALCAPHOR)
- Newest AMM date: 2025-12-19
- 15,462 entries (97.6%) have AMM > 2 years old
- 14,179 entries (89.5%) have AMM > 5 years old

**Conclusion:** The database retains all medications regardless of authorization date. The "2 years" mentioned in the plan refers to the commercialized/withdrawn status tracking period, not a retention cutoff.

---

## Finding 3: ¿ Character (U+00BF) Encoding Issue

**Status: VERIFIED**

- 159 occurrences in CIS_CIP_bdpm.txt
- Located in field 12 (HAS indications field)
- Example: `"Ce médicament peut être pris en charge ou remboursé par l'Assurance Maladie dans¿"`

**Cause:** The raw file uses Latin-1 encoding. The ¿ character is Latin-1 U+00BF. When read as UTF-8, it appears as replacement character unless properly decoded as Latin-1.

**Root cause:** The original BDPM files are encoded in Latin-1 (Windows Western), not UTF-8. The encoding issue causes double-encoding artifacts when the file was saved as UTF-8.

**Fix:** Read the file with `encoding='latin-1'` or detect encoding before processing.

---

## Summary

| Finding | Status | Implication |
|---------|--------|-------------|
| Trailing tabs | Verified | Parse with awareness of phantom field 14 |
| Date retention | External claim wrong | No cutoff - full history retained |
| ¿ encoding | Verified | Use latin-1 decoding, not UTF-8 |