# bdpm-ingest Runbook

Operational guide for the BDPM drug database ingest pipeline and API server.

## Quick Commands

```bash
# Health check
curl http://localhost:8080/health | jq

# Search drugs
curl "http://localhost:8080/drugs?q=paracetamol&limit=5" | jq

# Drug detail
curl http://localhost:8080/drugs/69094765 | jq

# Check row counts
./target/release/bdpm-ingest stats --data-dir data

# Check last import
./target/release/bdpm-ingest logs --data-dir data --limit 3
```

## Monitoring

### Import Failure Alerts

Watch for these indicators:

```bash
# Check import log for failures
./target/release/bdpm-ingest logs --data-dir data | grep -E "failed|error|partial"

# Watch for empty row counts
./target/release/bdpm-ingest stats --data-dir data | awk '$2 == 0 && NR > 1 {print $1" has 0 rows!"}'

# Alert threshold: any table with 0 rows after import
```

### Row Count Deviation Detection

Compare current counts against baseline:

```bash
# Save baseline
./target/release/bdpm-ingest stats --data-dir data > /tmp/baseline.txt

# Compare
./target/release/bdpm-ingest stats --data-dir data | diff - /tmp/baseline.txt

# Critical deviations (>10% change):
# - drugs table: expected ~21000 drugs
# - presentations table: expected ~38000 presentations
# - compositions table: expected ~65000 rows
```

### Schema Drift Detection

```bash
# Generate and compare OpenAPI specs
./target/release/bdpm-ingest dump-open-api > /tmp/openapi_new.yaml
diff openapi.yaml /tmp/openapi_new.yaml

# Check DB schema
sqlite3 data/bdpm.db ".schema" | head -50
```

### API Server Monitoring

```bash
# Verify endpoints respond
curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/health

# Check API logs for 5xx errors
journalctl -u bdpm-ingest | grep -E "500|502|503|ERROR"

# Monitor response times
time curl "http://localhost:8080/drugs?q=aspirine" > /dev/null
```

## Manual Operations

### Force Reimport

```bash
# Delete state file to force full reimport
rm data/import_state.json
./target/release/bdpm-ingest import --data-dir data --full

# Reimport single file
./target/release/bdpm-ingest import --data-dir data --file=CIS_CIP_bdpm.txt
```

### Single-File Sync

```bash
# Check which files have changed
./target/release/bdpm-ingest check --data-dir data

# Sync only weekly files (faster, availability-focused)
./target/release/bdpm-ingest dispo --data-dir data

# Full sync with changed files only
./target/release/bdpm-ingest import --data-dir data
```

### Health Check Script

```bash
#!/bin/bash
# health_check.sh
set -e

echo "=== BDPM Ingest Health Check ==="
echo ""

# 1. Check API health
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/health)
if [ "$HTTP_CODE" = "200" ]; then
    echo "API: OK (HTTP $HTTP_CODE)"
else
    echo "API: FAILED (HTTP $HTTP_CODE)"
    exit 1
fi

# 2. Check data freshness
LAST_IMPORT=$(curl -s http://localhost:8080/health | jq -r '.last_import')
if [ "$LAST_IMPORT" = "null" ]; then
    echo "WARNING: No successful import found"
else
    echo "Last import: $LAST_IMPORT"
fi

# 3. Check row counts
DRUG_COUNT=$(curl -s http://localhost:8080/health | jq -r '.drug_count')
if [ "$DRUG_COUNT" -lt 20000 ]; then
    echo "WARNING: Drug count ($DRUG_COUNT) below expected (~21000)"
fi

# 4. Verify search works
curl -s "http://localhost:8080/drugs?q=paracetamol&limit=1" | jq -e '. | length > 0' > /dev/null && echo "Search: OK" || echo "Search: FAILED"

echo ""
echo "=== Check Complete ==="
```

## Schema Change Response

When BDPM releases new data format:

1. **Detect**: `./target/release/bdpm-ingest poll --data-dir data`
2. **Download sample**: Fetch the changed file manually for inspection
3. **Parse test**: Run `./target/release/bdpm-ingest import --file=<file>` and watch for parse errors
4. **Update parser**: Modify `src/parse/` or `src/normalize/` as needed
5. **Add validation**: Check for new fields in `src/parse/mod.rs`
6. **Update migrations**: Add to `src/db/migrations/` if schema changes
7. **Verify**: Run tests and full import: `./target/release/bdpm-ingest import --data-dir data --full`

## Common Issues

### Database Locked

```
Error: database is locked
```
**Fix**: Close any other connections (DB Browser, other processes) and retry.

### HTTP 500 on Drug Detail

```bash
# Check if drug exists
curl "http://localhost:8080/drugs?q=CIS_CODE"

# Check DB integrity
sqlite3 data/bdpm.db "PRAGMA integrity_check;"
```

### Import Hangs

```bash
# Check for stuck process
ps aux | grep bdpm-ingest

# Kill and retry
pkill -f bdpm-ingest
./target/release/bdpm-ingest import --data-dir data
```

## File Reference

| File | Update Frequency | Primary Use |
|------|-------------------|-------------|
| CIS_CIP_Dispo_Spec | Weekly | Availability, stockouts |
| CIS_bdpm | Monthly | Core drug data |
| CIS_COMPO_bdpm | Monthly | Drug compositions |
| CIS_HAS_SMR_bdpm | Monthly | SMR ratings |
| CIS_HAS_ASMR_bdpm | Monthly | ASMR ratings |
| CIS_GENER_bdpm | Monthly | Generic groups |

## Environment

- Database: `data/bdpm.db` (SQLite with WAL mode)
- Raw files: `data/raw/`
- State: `data/import_state.json`
- API: `127.0.0.1:8080` (configurable)