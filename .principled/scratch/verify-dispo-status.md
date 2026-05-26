# Dispo Status Code Verification

## Source
`/home/devadmin/Desktop/BDMP_DB/raw/CIS_CIP_Dispo_Spec.txt`

## All Distinct Status Codes Found

| Code | Label                      | Row Count |
|------|----------------------------|----------:|
| 1    | Rupture de stock           |        66 |
| 2    | Tension d'approvisionnement|       421 |
| 3    | Arrêt de commercialisation |        15 |
| 4    | Remise à disposition       |       264 |

**Total valid data rows: 766**

Note: One row (code 4) uses lowercase "remise à disposition" — a minor casing anomaly, functionally equivalent.

## Plan's CHECK IN (1, 4) Would Reject Valid Data

Yes. A CHECK constraint on `dispo_status_code CHECK IN (1, 4)` would **silently reject**:
- 421 rows with code 2 (tension d'approvisionnement) — 55.0%
- 15 rows with code 3 (arrêt de commercialisation) — 2.0%
- **Total: 436 rows lost (~56.9% of all availability records)**

This would silently discard the majority of supply tension and discontinuation records without any error or warning.

## Recommendation

Change the availability table schema from:
```sql
dispo_status_code SMALLINT NOT NULL CHECK (dispo_status_code IN (1, 4))
```

to:
```sql
dispo_status_code SMALLINT NOT NULL CHECK (dispo_status_code IN (1, 2, 3, 4))
```

Optionally add a lookup table or CHECK with explicit labeling for documentation purposes:
```sql
-- 1 = Rupture de stock
-- 2 = Tension d'approvisionnement
-- 3 = Arrêt de commercialisation
-- 4 = Remise à disposition
```

The external review description (1, 2, 3, 4) is accurate. The plan's CHECK IN (1, 4) was an erroneous truncation — likely an oversight when referencing the specification.
