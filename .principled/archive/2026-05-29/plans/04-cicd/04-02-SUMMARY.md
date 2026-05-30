# 04-02 SUMMARY — GitHub Actions Release Workflows

## What was done

### CI workflow (`ci.yml`)
- Runs on every push to main and on all PRs
- Steps: rustfmt check, cargo test --lib, cargo clippy (deny warnings)
- Uses `dtolnay/rust-toolchain@master` with stable toolchain
- Blocks merge on format/lint/test failures

### Release workflow (`release.yml`)
- Runs on every push to main + manual `workflow_dispatch`
- Optional `force_full` boolean input for forced re-import
- Builds optimized release binary: `LTO=true`, `strip=symbols`
- Downloads BDPM files, runs full ingest, publishes `bdpm.db` as release artifact

### Monthly DB rebuild (`monthly-db-release.yml`)
- Scheduled: `0 2 1 * *` — 1st of each month at 02:00 UTC
- Full pipeline: fetch + ingest + release
- Manual trigger via `workflow_dispatch`

### Weekly dispo sync (`weekly-dispo.yml`)
- Scheduled: `0 3 * * 1` — Monday at 03:00 UTC
- Syncs CIS_CIP_Dispo_Spec.txt independently (weekly cadence)
- Uses `workflow_dispatch` for manual trigger

## Verification
- All 4 workflow files present in `.github/workflows/`
- `monthly-sync.yml` marked deprecated, superseded by `monthly-db-release.yml`

## Files created
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `.github/workflows/monthly-db-release.yml`
- `.github/workflows/weekly-dispo.yml`
- `.github/workflows/monthly-sync.yml` (deprecated, reference only)
