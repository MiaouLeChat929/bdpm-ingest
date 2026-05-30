# Orphan CIS Verification

**Date:** 2026-05-26
**Source:** Raw BDPM files vs. CIS_bdpm.txt master
**Master CIS count:** 15,848

## Results

| File | External Claim | Actual Count | Match |
|------|---------------|--------------|-------|
| CIS_HAS_SMR | 2,806 | 2,806 | YES |
| CIS_HAS_ASMR | 1,567 | 1,567 | YES |
| CIS_GENER | 2,503 | 2,503 | YES |
| CIS_COMPO | 0 | 0 | YES |
| CIS_MITM | 0 | 0 | YES |

**All external review claims VERIFIED against raw data.**

## Interpretation

- SMR orphans (18.4%) — About 1 in 5 CIS codes lack SMR ratings. These drugs have no reimbursement evaluation on record.
- ASMR orphans (15.8%) — Slightly fewer, but still significant. Drugs missing ASMR cannot be graded for medical benefit added.
- GENER orphans (23.5%) — Highest orphan rate. Drugs without generic groupings cannot be linked for substitution/equivalence analysis.
- COMPO/MITM — Clean join. Every CIS in those files exists in master.

## Impact on Plan

1. **FK constraints:** Enforce FK only on COMPO and MITM. SMR/ASMR/GENER must be optional many-to-one (drop NOT NULL, use LEFT JOIN).
2. **Import ordering:** Load CIS_GENER before CIS_HAS_SMR/CIS_HAS_ASMR (or accept NULLs). GENER orphans require same accommodation.
3. **Downstream queries:** Any SMR/ASMR/GENER LEFT JOIN will return 15-23% NULL-sided rows — expected behavior, not a bug.
4. **Schema design:** `is_orphan_smr`, `is_orphan_asmr`, `is_orphan_gener` computed columns or views surface orphan rates per CIS.

## Recommendation

- **Accept the orphans.** They reflect real BDPM data incompleteness, not import errors.
- **Enforce FK only on COMPO and MITM** — these files are complete.
- **Document orphan thresholds** in schema comments so consumers understand NULL semantics.
- **Optional:** Build `v_drug_with_grades` view that LEFT JOINs SMR/ASMR to CIS and labels orphans, so analysts avoid accidental INNER JOIN exclusion.
