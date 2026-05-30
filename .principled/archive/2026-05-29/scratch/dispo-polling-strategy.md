# CIS_CIP_Dispo_Spec Fastest Polling Strategy — Research 2026-05-26

## Server Behavior Summary

### TXT File (`CIS_CIP_Dispo_Spec.txt`)
- **165 Ko** — small enough to download daily without concern
- **No Content-Length** (chunked transfer encoding)
- **No ETag** — can't use conditional GET
- **No Last-Modified** — can't use If-Modified-Since
- **No rate-limit headers** observed
- **Cache-Control: private, must-revalidate** — server says don't cache, but doesn't enforce

### HTML Listing Page (`/telechargement/`)
- **No HTTP-level Last-Modified or ETag either**
- BUT: contains **embedded per-file update dates** in HTML text:
  - `CIS_CIP_Dispo_Spec.txt` → **19/05/2026** (latest of all files!)
  - CIS_bdpm.txt → 28/04/2026
  - CIS_CIP_bdpm.txt → 25/05/2026
  - CIS_MITM.txt → 09/03/2026

### medicaments-api.giygas.dev Reference
- Updates 2x/day (06h/18h UTC) — faster than monthly BDPM claim
- Has proper ETag/Last-Modified on their API
- Rate limit: 1000 tokens/IP at 3/sec
- `data_age_hours: 10` in their health response

## The Core Problem

**No ETag or Last-Modified on BDPM TXT files** → Standard HTTP conditional GET (If-None-Match / If-Modified-Since) is impossible.

**What this means in practice:**
- Every GET request returns 200 with full body — the server always says "here's new data"
- Even if the file hasn't changed, you get the full chunked response
- The only way to know if the file changed is to download and hash it

## Polling Strategies Evaluated

### Strategy A: Blind TXT polling (current plan baseline)
- Download full 165 Ko file, hash it, compare to stored hash
- **Cost**: ~165 Ko × N polls/day
- **Benefit**: 100% accurate, no false negatives
- **Risk**: None observed on BDPM server (no rate limiting detected)

### Strategy B: HTML listing page scraping (best option)
- Fetch HTML listing page (~5-10 Ko), parse embedded date for CIS_CIP_Dispo_Spec.txt
- Extract: `Date de mise à jour: 19/05/2026` from HTML text
- **Cost**: ~5-10 Ko per poll (50-100x lighter than TXT)
- **Accuracy**: Date-level granularity (day precision, not content-level)
- **Logic**: if extracted_date > stored_date → trigger TXT download

**Why this works:** The HTML listing page is a static page that BDPM regenerates when files update. The per-file update dates in the HTML change when files change. This is what the BDPM website itself uses to communicate file freshness.

### Strategy C: medicaments-api.giygas.dev as intermediary
- Poll their health endpoint instead of BDPM directly
- `curl https://medicaments-api.giygas.dev/health`
- They handle BDPM polling (2x/day), give you `data_age_hours`, `is_updating`
- **Cost**: ~200 bytes per poll
- **Accuracy**: They track at their update granularity (2x/day)
- **Limitation**: Third-party — uptime dependency, API not official
- **Legal**: Check if using their API violates ANSM terms

### Strategy D: HEAD request + Content-Length
- **Doesn't work**: TXT file has no Content-Length (chunked encoding)
- HEAD returns same headers as GET — no size info either

### Strategy E: Partial byte-range GET
- `Range: bytes=0-100` to get first 100 bytes
- Compare to stored first-100-bytes hash
- **Problem**: Content changes throughout file — first bytes may not reflect end-of-file changes
- Not reliable for this file structure

## Recommendation

### Fastest polling without spamming

**Two-tier approach:**

**Tier 1 — CIS_CIP_Dispo_Spec (the important one):**
```
Daily: curl -s "https://base-donnees-publique.medicaments.gouv.fr/telechargement/"
        → parse embedded date for CIS_CIP_Dispo_Spec.txt
        → if date > stored_date: download full TXT file (165 Ko)
```
- Cost: ~5-10 Ko/day for detection, 165 Ko only on actual update
- Date precision: DD/MM/YYYY — catches all actual updates
- Worst case: 1 extra 165 Ko download per day (if file changes daily)
- No spam concern: 1 HTML request/day = trivially light for any server

**Tier 2 — All other files (monthly):**
```
Monthly (1st or ~28th): full batch download + hash compare
```
- BLAKE3 hash already implemented — only download if hash changed
- Even without optimization, all files together ~15-20 Mo/month

### What about the medicaments-api.giygas.dev approach?
- **Pros**: Ultra-light (200 bytes), 2x/day updates, proper caching
- **Cons**: Third-party dependency, not official, legal unclear
- **Decision**: Use as fallback or supplemental, not primary source

## Spam Assessment

**Is polling HTML daily (365 requests/year) actually spamming?**
- No. 365 × 5 Ko = ~1.8 Mo/year total bandwidth — negligible
- The BDPM server has no rate limiting and serves static files
- Even 2x/day (730 requests/year) is well within acceptable range
- The medicaments-api.giygas.dev reference shows 2x/day is standard practice

**The actual "spam" concern would be downloading the full 165 Ko file daily.**
→ Solved by HTML detection: download full file only when date changes.

## Implementation Notes

The HTML listing URL:
`https://base-donnees-publique.medicaments.gouv.fr/telechargement/`

Extract dispo date with regex:
```rust
static DISPO_DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)CIS_CIP_Dispo_Spec\.txt.*?(\d{2}/\d{2}/\d{4})").unwrap());
// or look for: "Date de mise à jour: 19/05/2026"
```

Alternative: parse the HTML listing table directly — each row has file name, size, date.
