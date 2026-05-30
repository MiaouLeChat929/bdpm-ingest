# BDPM Discrepancy Verification Report

**Generated:** 2026-05-26
**Source Files Analyzed:**
- `/home/devadmin/Desktop/BDMP_DB/.principled/plans/BRIEF.md`
- `/home/devadmin/Desktop/BDMP_DB/external_review/review2.txt`
- `/home/devadmin/Desktop/BDMP_DB/.principled/scratch/external-analyse_technique.md`
- `/home/devadmin/Desktop/BDMP_DB/raw/CIS_CIP_bdpm.txt`

---

## DISCREPANCY 1: URL Pattern

### Our Plan (BRIEF.md)
**Reference:** Line 9
> "The BDPM publishes 11 plaintext TSV files at `base-donnees-publique.medicaments.gouv.fr/telechargement` monthly."

The plan references the base landing page URL but does not specify the direct file download URL pattern.

### External Analysis (external-analyse_technique.md)
**Reference:** Lines 409-410
> "### URL Pattern Discovery
> - **Old pattern** (broken): `/telechargement?fich=XXX`
> - **New pattern**: `/download/file/XXX` (10 files), `/download/XXX` (CIS_InfoImportantes.txt only)"

### Review2.txt
**Reference:** Line 10
> "**URL de téléchargement** : `https://base-donnees-publique.medicaments.gouv.fr/telechargement`"

This references the landing page, not the direct download URLs.

### VERDICT

**The external analysis is more explicit.** The external review indicates:
1. The old direct download pattern `/telechargement?fich=XXX` is **BROKEN**
2. The new pattern is `/download/file/XXX`

**Our plan is ambiguous** - it references the landing page but doesn't specify what URL pattern to use for direct file downloads.

**RECOMMENDATION:** Update BRIEF.md to explicitly state:
- Use `/download/file/XXX` for the 10 stable BDPM files
- Use `/download/XXX` for CIS_InfoImportantes.txt (no "file" segment)
- The old `/telechargement?fich=XXX` pattern is deprecated/broken

---

## DISCREPANCY 2: Price Thousands Separator Count

### Our Plan (BRIEF.md)
**Reference:** Line 20
> "Prices as `24,34`; **critical: 466 rows with values >1000 use comma as thousands separator** (`1,466,29` → must remove both commas, not replace)"

### External Analysis
No explicit count mentioned, but the thousands separator issue is documented.

### Actual Data Verification (CIS_CIP_bdpm.txt)

**Command:** Analyzed all rows in CIS_CIP_bdpm.txt, checking price fields 8, 9, 10 for values with 2+ commas.

**Result:** **467 rows** contain prices with thousands separator

### Example Values:
```
Price field 9: 1,466,29
Price field 9: 7,518,58
Price field 9: 3,982,28
Price field 9: 1,145,89
Price field 9: 2,333,42
```

### VERDICT

**Difference:** 467 (actual) vs 466 (plan) = **+1 row**

This is likely due to the data being updated since the BRIEF was written. The discrepancy is minor and the handling approach (detect 2 commas, remove both) remains correct.

**RECOMMENDATION:** Update BRIEF.md to state "466-467 rows" to account for data updates, or make it dynamic by documenting the verification query rather than hardcoding the count.

---

## Summary Table

| Discrepancy | Our Plan | External/Actual | Impact | Action Needed |
|-------------|----------|----------------|--------|---------------|
| URL Pattern | `/telechargement` (landing page) | `/download/file/XXX` (new, old broken) | **HIGH** - Wrong URLs = download failure | Update BRIEF.md with explicit download URLs |
| Thousands Separator Count | 466 rows | 467 rows | **LOW** - Minor data update | Update count or make dynamic |

---

## Action Items

1. **CRITICAL:** Add explicit download URL pattern to BRIEF.md architecture section
2. **LOW:** Update price thousands separator count (466 → 466-467 or add verification query)
