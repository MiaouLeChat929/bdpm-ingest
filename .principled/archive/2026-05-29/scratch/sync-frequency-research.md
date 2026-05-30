# BDPM Sync Frequency — Research Findings 2026-05-26

## Key Facts

| Fact | Evidence |
|------|----------|
| Official cadence | Monthly (observed ~28th of each month) |
| Third-party reference | medicaments-api.giygas.dev — full-table reload only |
| Intra-month | No delta, no partial — all 10 files fully replaced |
| Change detection | No RSS, no webhook. Use Last-Modified on HTML listing page as pre-check |
| Community poll interval | 2x daily (06h/18h) — validated by reference implementation |
| Legal obligation | ANSM license Article L. 161-40-1 — intra-month safety withdrawals possible |
| Zero-byte anomalies | CIS_CIP_bdpm + Ruptures_stocks can return 0-byte responses (01/01/1970 timestamps) |
| CIS_MITM | MITM = Autorisation de Mise sur le Marché (Market Authorization). Changes infrequently — market authorizations are rare. Accept monthly sync. |

## BDPM Server Behavior (measured 2026-05-26)

### TXT Files — No HTTP Caching Primitives
- **No ETag** — standard HTTP conditional GET impossible
- **No Last-Modified** — If-Modified-Since impossible
- **No Content-Length** — chunked transfer encoding only
- **No rate-limit headers** observed — server doesn't indicate limits
- **Cache-Control: private, must-revalidate** — says don't cache, but no enforcement

### HTML Listing Page — Embedded Dates
The listing page (`/telechargement/`) contains per-file update dates embedded in HTML text:
- CIS_CIP_Dispo_Spec.txt → **19/05/2026** (NEWEST of all files!)
- CIS_CIP_bdpm.txt → 25/05/2026
- CIS_bdpm.txt → 28/04/2026
- CIS_MITM.txt → 09/03/2026

**Key insight**: Parsing the HTML listing page gives file-level update dates without downloading the files. Page is ~5-10 Ko. Extract date with regex or HTML parse.

### medicaments-api.giygas.dev Reference
- Updates 2x/day (06h/18h UTC) — faster than BDPM's "monthly" claim
- Has proper ETag/Last-Modified on their API endpoints
- Rate limit: 1000 tokens/IP at 3/sec
- `data_age_hours: 10` in health response
- Can be used as intermediary for change detection (ultra-light)

- **Primary**: Monthly cron (1st of month OR ~28th) — catches official batch
- **Secondary**: Twice-daily (06h/18h) poll — satisfies ANSM intra-month obligation
- **Both**: Full-table reload, not delta — no other mechanism exists

## Implementation Constraints

1. Full-table reload always — delta is impossible, this is by design
2. Handle zero-byte downloads gracefully (null/empty checks)
3. Use `Last-Modified` header on HTML listing page as lightweight pre-check
4. `CIS_InfoImportantes` is live query — separate from batch scheduler
5. Windows-1252 encoding — confirmed by third-party integrators
6. Retry with exponential backoff on HTTP 5xx
7. HTTPS required per ANSM license

## Sources

- ANSM download page (https://base-donnees-publique.medicaments.gouv.fr)
- ANSM license PDF (Article L. 161-40-1)
- medicaments-api.giygas.dev reference Go implementation
- BDPM data format specification PDF
