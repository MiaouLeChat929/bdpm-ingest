# BDPM Live URL Verification (2026-05-26)

## Verdict: `/download/file/` is the correct pattern for ALL 11 files

### Results Summary

| Pattern | URL | Content-Type | Status |
|---------|-----|-------------|--------|
| OLD (broken) | `https://base-donnees-publique.medicaments.gouv.fr/telechargement?fich=HAS_LiensPageCT_bdpm.txt` | `text/html` | HTML page returned, NOT data |
| NEW (works) | `https://base-donnees-publique.medicaments.gouv.fr/download/file/HAS_LiensPageCT_bdpm.txt` | `application/octet-stream` | Correct data file |
| InfoImportantes | `https://base-donnees-publique.medicaments.gouv.fr/download/CIS_InfoImportantes.txt` | `application/force-download` | Works (on-demand generation) |
| InfoImportantes `/file/` | `https://base-donnees-publique.medicaments.gouv.fr/download/file/CIS_InfoImportantes.txt` | `application/octet-stream` | Also works |

### External Review Claims: CONFIRMED

- **OLD pattern BROKEN**: `telechargement?fich=XXX` returns HTML (the license agreement page), not the data file. Confirmed via `Content-Type: text/html`.
- **NEW pattern works** for all 10 stable files: `application/octet-stream` with `Content-Disposition: attachment; filename="..."`
- **InfoImportantes** has TWO valid sub-patterns — both work:
  - `/download/CIS_InfoImportantes.txt` → `application/force-download` with timestamped filename (`CIS_InfoImportantes_20260526120147_bdpm.txt`)
  - `/download/file/CIS_InfoImportantes.txt` → `application/octet-stream` with static filename (`CIS_InfoImportantes.txt`)

### All 10 Stable Files — Verified Working

```
CIS_bdpm.txt             → application/octet-stream
CIS_CIP_bdpm.txt         → application/octet-stream
CIS_COMPO_bdpm.txt       → application/octet-stream
CIS_HAS_SMR_bdpm.txt     → application/octet-stream
CIS_HAS_ASMR_bdpm.txt    → application/octet-stream
HAS_LiensPageCT_bdpm.txt → application/octet-stream
CIS_GENER_bdpm.txt       → application/octet-stream
CIS_CPD_bdpm.txt         → application/octet-stream
CIS_CIP_Dispo_Spec.txt   → application/octet-stream
CIS_MITM.txt             → application/octet-stream
```

### Recommended URL Pattern for Rust Fetcher

Use the **canonical base URL**:
```
BASE_URL = "https://base-donnees-publique.medicaments.gouv.fr"
```

**10 stable files** use the same pattern:
```
/download/file/{filename}
```

**InfoImportantes** can use either (but recommend `/file/` for consistency):
```
/download/file/CIS_InfoImportantes.txt   # Preferred (consistent with others)
/download/CIS_InfoImportantes.txt        # Also works
```

### Actual Download Verification

Downloaded and inspected `HAS_LiensPageCT_bdpm.txt` from `/file/` pattern — confirmed valid TSV data:
```tsv
CT-21584	https://www.has-sante.fr/jcms/p_3961577
CT-21757	https://www.has-sante.fr/jcms/p_3961574
CT-21570	https://www.has-sante.fr/jcms/p_3957169
```

### Source: Official Download Page (2026-05-26)

The official download page at `https://base-donnees-publique.medicaments.gouv.fr/telechargement` links directly to all files via the `/download/file/` pattern, confirming this is the server-authoritative URL scheme.
