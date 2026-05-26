# CI/CD Critique: BDMP_DB Project Plan

**Critique Author:** Senior Rust Developer + CI/CD Architect
**Date:** 2026-05-26
**Scope:** All plan artifacts in `.principled/plans/`

---

## Summary

Two specific changes requested:

1. **Remove ALL Docker-related content** — the project runs as a native CLI tool, not containerized
2. **GitHub Actions as first-class CI/CD target** — database build, API, and sync pipeline must work in GitHub Actions runners

Both changes require surgical edits across the artifacts, plus one architecturally new plan for GitHub Actions workflow design. Current Phase 4 has "Docker packaging" as a deliverable; this entire track must be replaced.

---

## FILE: BRIEF.md

### REMOVE

**Lines 386-387 (Phase 4 description):**

```
Phase 4: CI regression suite, Docker, operational documentation
```

REPLACE with:

```
Phase 4: CI regression suite, GitHub Actions CI/CD pipeline, operational documentation
```

**Line 387:**
Remove entirely (no Phase 4 Docker line needed).

### MODIFY

**Lines 322-364 (CI Regression Tests section):**
The tests are correctly designed but the section should clarify they run in GitHub Actions. Add this note at the top of the section:

> **CI Context:** These tests target `tests/` directory and run via `cargo test` in a GitHub Actions runner on every push. They are NOT conditional on Docker.

**Lines 373-387 (Priority Order for Rust Implementation):**
Update Phase 4 to reflect GitHub Actions pipeline:

```
Phase 1 (Foundation): [unchanged]
Phase 2: FTS5 search, drug detail API, price lookup, generic groups, availability
Phase 3: File-level polling, full-table refresh, import log viewer
Phase 3.5 (deferred): CIS_InfoImportantes with 6-hour TTL cache
Phase 4: CI regression suite, GitHub Actions workflow (sync + artifact), operational documentation
```

**Success Criteria, line 401:**
Add after existing criteria:
- `bdpm-ingest` compiles and runs on `ubuntu-latest` GitHub Actions runner (x86_64-unknown-linux-gnu)
- SQLite database file is a self-contained artifact that can be uploaded/downloaded as a workflow artifact
- Sync pipeline triggers via `workflow_dispatch` or `cron` schedule

---

## FILE: ROADMAP.md

### REMOVE

**Lines 69-73 (Phase 4 table row for Docker):**

```
| 04-02 | Docker packaging |
| 04-03 | Schema change response procedure + operational runbook |
```

### MODIFY

**Lines 5-12 (Phase structure):**
Current:

```
Phase 1: Foundation          [01-01 → 01-02 → 01-03 → 01-04 → 01-05 → 01-06 → 01-07]
Phase 2: API                 [02-01 → 02-02 → 02-03 → 02-04]
Phase 3: Sync Engine         [03-01 → 03-02 → 03-03 → 03-04]
Phase 3.5: Safety Data        [CIS_InfoImportantes — deferred]
Phase 4: Polish              [04-01 → 04-02 → 04-03]
```

Replace with:

```
Phase 1: Foundation          [01-01 → 01-02 → 01-03 → 01-04 → 01-05 → 01-06 → 01-07]
Phase 2: API                 [02-01 → 02-02 → 02-03 → 02-04]
Phase 3: Sync Engine         [03-01 → 03-02 → 03-03 → 03-04]
Phase 3.5: Safety Data       [CIS_InfoImportantes — deferred]
Phase 4: CI/CD Pipeline      [04-01 → 04-02 → 04-03 → 04-04]
```

### ADD

After the Phase 4 table, insert:

```
Phase 4: CI/CD Pipeline

| Plan | Goal |
|------|------|
| 04-01 | GitHub Actions workflow — build + test on push |
| 04-02 | Sync pipeline as workflow (workflow_dispatch + cron) |
| 04-03 | Database artifact publishing (upload/download as workflow artifact or GitHub Release) |
| 04-04 | Operational runbook + schema change response procedure |
```

### MODIFY

**Lines 67-73 (Phase 4 description):**
The "04-01 CI regression suite" is already correct (tests are always CI). 04-02 "Docker packaging" must be replaced as above. 04-03 "Operational runbook" moves to 04-04.

---

## FILE: 01-01-PLAN.md

### MODIFY

**Lines 126-133 (CLI commands, state file location):**
Update the Store state location and downloaded files location to account for GitHub Actions artifacts:

Current:
```
- Store state at `{project_dir}/data/import_state.json`
- Downloaded files at `{project_dir}/raw/{filename}`
```

Replace with:
```
- State stored in GitHub Actions workflow: `${{ github.workspace }}/data/import_state.json`
- Downloaded files at `${{ github.workspace }}/raw/{filename}` (workspace-relative, ephemeral per run)
- For local use: state lives in the project data/ directory, persisted via workflow artifact or persistent storage between runs
- **CROSS-RUN STATE PERSISTENCE IN CI:** The state store (`import_state.json`) must be preserved across workflow runs to avoid re-downloading unchanged files:
  - On workflow start: download `import_state.json` as a workflow artifact if it exists
  - After fetch: re-upload the updated `import_state.json` as a workflow artifact
  - In Docker-free CI: this is the mechanism for detecting file changes without re-downloading every run
```

**Lines 19-37 (Task 1: dependencies — verify rust platform support):**
Add to verification step:

```
Verify: `cargo build` succeeds with `cross` or on `ubuntu-latest` natively (no Docker-based cross-compilation needed for x86_64 target)
Action: Ensure all dependencies compile on x86_64-unknown-linux-gnu. At minimum: rusqlite (bundled), reqwest, tokio, sha2, csv, clap, serde, serde_json, thiserror, tracing. Confirm `cargo build --target x86_64-unknown-linux-gnu` works (even if cross-compiling from a non-Linux host).
```

---

## FILE: 01-02-PLAN.md

### NO CHANGES NEEDED

The tab parser (01-02) has no Docker references. No modifications needed.

---

## FILE: 01-03-PLAN.md

### MODIFY

**Lines 42-68 (Task 2: Integration tests):**
Clarify that tests run in GitHub Actions:

Add at top of Task 2:

> **CI Note:** These tests are designed to run via `cargo test` in GitHub Actions. The test files read from `raw/` directory which must be present in the runner. Two options:
> 1. Include the raw BDPM files in the repo (acceptable for ~5MB total)
> 2. Fetch raw files as part of the CI workflow before running tests
>
> Option 1 is recommended for Phase 1-3 simplicity. If files are large (>20MB), use a setup step in the workflow to download them.

**Lines 44-67 (Test file paths):**
Current test references:

```rust
let path = Path::new("raw/CIS_bdpm.txt");
```

This works if `raw/` is committed to the repo or fetched in CI. Add a NOTE that these paths are workspace-relative:

> **Path Note:** In GitHub Actions, `${{ github.workspace }}` is the runner root. Tests running from `.github/workflows/` working directory will need `CARGO_MANIFEST_DIR` or workspace-relative paths. Use `env!("CARGO_MANIFEST_DIR")` or set `BDPM_RAW_DIR` environment variable to allow CI to specify the raw file location.

**Lines 86-87 (Field-count drift simulation test):**
Add note:

> This test is the **CI gate for schema drift**. It must pass on every push. The manual simulation (temporary column addition) is only done during development to verify the test works correctly.

---

## FILE: 01-04-PLAN.md

### NO CHANGES NEEDED

The normalization pipeline (01-04) has no Docker references. No modifications needed.

---

## FILE: 01-05-PLAN.md

### MODIFY

**Lines 14-23 (Task 1: Database initialization):**
Clarify that the SQLite file must be portable:

Current:
```rust
let migrations = Migrations::new(vec![
    M::up(include_str!("migrations/001_initial.sql")),
]);
```

Replace/add:

> The SQLite database file generated by `init_db` must be:
> 1. **Self-contained**: single `.db` file, no external dependencies
> 2. **Portable**: portable across Linux x86_64 runners (no OS-specific features)
> 3. **Artifact-friendly**: can be archived/uploaded as a GitHub Actions artifact
>
> `rusqlite` with `features = ["bundled"]` meets all these requirements (bundles libsqlite3).

**Lines 52-101 (Task 3: Master import orchestrator):**
Clarify import state persistence in CI context:

Current `state.mark_updated()` and `state.save()` logic:

Add a dedicated section on **Cross-Run State in CI**:

```
## Cross-Run State in GitHub Actions

The `bdpm-ingest import` command relies on `import_state.json` to skip re-downloading unchanged files. In GitHub Actions:

1. Workflow run starts
2. Download previous `import_state.json` from workflow artifact (if exists)
3. Run `bdpm-ingest import` — only downloads files with changed SHA256
4. Upload updated `import_state.json` as workflow artifact for next run
5. Optionally upload the built `.db` file as a workflow artifact

**Important for scheduled/cron runs:** The state file IS the change-detection mechanism. Without it, every cron run would re-download all files even if unchanged.

State file should be treated as a workflow artifact with retention matching the cron interval (30 days minimum for monthly cadence).
```

**Lines 96-99 (CLI command descriptions — update):**
Current:
- `--full`: re-import all files regardless of state
- `--file=NAME`: import only one file
- Default: import only files whose SHA256

Add:
- `--state-file=PATH`: path to `import_state.json` (for CI cross-run state persistence, defaults to `{data_dir}/import_state.json`)
- `--output-db=PATH`: path to output SQLite file (defaults to `{data_dir}/bdpm.db`; for CI: set to workspace-relative path for artifact upload)

---

## ARCHITECTURAL CHANGES (Cross-Cutting)

### A1. New GitHub Actions Workflow Plan (Missing — Must Be Added)

**No existing plan covers this.** A new Phase 4 plan is required. Proposed: `04-01-PLAN.md`

Core design:

```yaml
# .github/workflows/ci.yml
name: BDPM CI

on:
  push:
    branches: [main]
  pull_request:
  workflow_dispatch:  # manual trigger for sync

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Cache cargo registry
        uses: actions/cache@v4
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
      - name: Cache cargo index
        uses: actions/cache@v4
        with:
          path: ~/.cargo/git
          key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}
      - name: Cargo build
        run: cargo build --release --target x86_64-unknown-linux-gnu
      - name: Run tests
        run: cargo test
        env:
          # For schema drift detection tests
          BDPM_RAW_DIR: ${{ github.workspace }}/raw
          # For cross-run state persistence
          BDPM_STATE_FILE: ${{ github.workspace }}/data/import_state.json
          BDPM_DATA_DIR: ${{ github.workspace }}/data
          BDPM_DB_PATH: ${{ github.workspace }}/data/bdpm.db

  sync-and-publish:
    runs-on: ubuntu-latest
    needs: test
    if: github.event_name == 'workflow_dispatch' || github.event_name == 'schedule'
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Restore state from artifact
        if: startsWith(github.event_name, 'schedule')
        uses: actions/download-artifact@v4
        with:
          name: bdpm-state
          path: data/
          continue-on-error: true
      - name: Build release binary
        run: cargo build --release
      - name: Run full sync
        run: |
          mkdir -p data/
          ./target/release/bdpm-ingest fetch --all
          ./target/release/bdpm-ingest import --full --output-db=data/bdpm.db --state-file=data/import_state.json
      - name: Upload state artifact
        uses: actions/upload-artifact@v4
        with:
          name: bdpm-state
          path: data/import_state.json
          retention-days: 35  # covers monthly cadence + buffer
      - name: Upload database artifact
        uses: actions/upload-artifact@v4
        with:
          name: bdpm-database
          path: data/bdpm.db
          retention-days: 90

  # Scheduled monthly sync (every 1st of month at 03:00 UTC)
  monthly-sync:
    runs-on: ubuntu-latest
    if: false  # enable via cron when ready
    # cron: '0 3 1 * *'  # uncomment when Phase 3 complete
```

Key design decisions:
- **Dual workflow strategy**: `ci.yml` runs on every push (test-only). A separate workflow or conditional job handles the scheduled sync and artifact publishing
- **State as artifact**: `import_state.json` is the key to avoiding re-downloads. Uploading it at end of each run and downloading at start is the cross-run persistence mechanism
- **No Docker**: `bdpm-ingest` is a native binary built in the `ubuntu-latest` runner
- **`workflow_dispatch`**: Allows manual triggering of a full sync (e.g., for immediate refresh instead of waiting for cron)
- **Artifact naming and retention**: State needs 35-day retention (monthly cadence). Database needs 90-day retention (for reproducibility)
- **Rust toolchain**: `dtolnay/rust-toolchain` action is stable, well-maintained, no Docker

For the weekly sync (CIS_CIP_Dispo_Spec.txt):
- Separate workflow or separate cron schedule: `0 3 * * 1` (Monday 03:00 UTC)
- Can share the state file with monthly sync (independent tracking per file)

### A2. SQLite as Workflow Artifact

Current plan focuses on local file storage. Modifications needed:

| Component | Change |
|-----------|--------|
| `init_db()` | Default path should be overridable via `--output-db` CLI flag |
| Import orchestrator | Accept `--output-db` parameter |
| CI workflow | Specify `BDPM_DB_PATH` to control output location |
| State store | `--state-file` flag for cross-run persistence in CI |
| Artifact upload step | Upload `bdpm.db` to GitHub Actions artifact store |

### A3. No Local Scheduler — All Scheduling in GitHub Actions

Current plan mentions "monthly cron" and "weekly cron" but doesn't specify where these timers live. Add explicit architectural decision:

> **Scheduling Decision:** All scheduling is handled by GitHub Actions `cron` syntax inside workflow files. No local `cron` entries, no `systemd` timers, no external schedulers. A GitHub Actions workflow is the single source of truth for scheduling.

Two separate schedules:
1. **Monthly sync**: All 10 stable files. Cron: `0 3 1 * *` (first day of month, 03:00 UTC — reasonable for low-traffic window)
2. **Weekly sync**: CIS_CIP_Dispo_Spec.txt only. Cron: `0 3 * * 1` (Monday 03:00 UTC)

### A4. GitHub Release vs. Pages-Hosted API

The brief mentions "The built database might be published as a GitHub Release artifact or Pages-hosted API." This is Phase 2+ territory. Architectural positioning:

- **Phase 4 deliverable**: Build the `.db` file and upload as a GitHub Actions artifact
- **Phase 2+ decision**: Whether to serve via GitHub Pages (static hosting with a thin HTTP wrapper) or GitHub Releases (downloadable file) is an API design decision
- **Recommendation for Phase 4 placeholder**: Generate a `metadata.json` alongside the `.db` file that includes: `generated_at`, `source_urls`, `source_sha256s`, `row_counts`, `schema_version`. This makes the artifact self-describing for any future Pages or Release publishing.

---

## REVISED PHASE STRUCTURE

### Current Phase 4 (Problematic)

```
Phase 4: Polish
  04-01: CI regression suite    ← GOOD (keep)
  04-02: Docker packaging       ← REMOVE
  04-03: Operational runbook   ← GOOD (keep, rename 04-04)
```

### Proposed Revision

```
Phase 4: CI/CD Pipeline
  04-01: CI regression suite    ← already correct (tests run via cargo test)
  04-02: GitHub Actions CI workflow (build + test on push)
  04-03: Sync pipeline workflow (workflow_dispatch + cron + artifact publishing)
  04-04: Operational runbook + schema change response procedure
```

**Rationale:**
- 04-01 (CI tests) already exists and is correct
- 04-02 is **missing from all current plans** — this is the most critical gap. GitHub Actions workflow design needs it's own plan
- 04-03 replaces "Docker packaging" and adds sync workflow + artifact publishing, which is more valuable for the CI/CD target
- 04-04 restructures "Operational runbook" as the final polish step

**New files needed:**

1. `phases/01-foundation/01-06-PLAN.md` — ID/code normalization (currently missing from structure but referenced in ROADMAP Line 61 as 01-06)
2. `phases/01-foundation/01-07-PLAN.md` — Database init (currently missing — also referenced in ROADMAP)
3. `phases/04-cicd/04-02-PLAN.md` — GitHub Actions CI workflow
4. `phases/04-cicd/04-03-PLAN.md` — Sync pipeline workflow + artifact publishing

Note: Check if 01-06 and 01-07 exist in the directory — the ROADMAP references them but they may not have plan files yet.

### Directory structure for new Phase 4 plans:

```
.principled/plans/
  phases/
    04-cicd/
      04-01-PLAN.md  (rename from 04-01, keep tests plan as-is)
      04-02-PLAN.md  (NEW: GitHub Actions CI workflow)
      04-03-PLAN.md  (NEW: Sync pipeline + artifact publishing)
      04-04-PLAN.md  (NEW: Operational runbook, moved from old 04-03)
```

---

## SUMMARY OF DISCREPANCIES

| Location | Issue | Action |
|----------|-------|--------|
| BRIEF.md L386 | "Docker" in Phase 4 | REPLACE with "GitHub Actions CI/CD pipeline" |
| BRIEF.md Success Criteria | No CI runner criteria | ADD criteria for x86_64-unknown-linux-gnu |
| ROADMAP.md L69-73 | Phase 4 table has Docker | REPLACE rows 04-02/04-03 |
| 01-01-PLAN.md L133 | Data paths for local-only | ADD CI workspace-relative paths + cross-run state |
| 01-03-PLAN.md L44-67 | Test paths not CI-aware | ADD BDPM_RAW_DIR env var + CI path note |
| 01-05-PLAN.md L14-23 | DB portability not stated | ADD self-contained + artifact-friendly requirements |
| 01-05-PLAN.md L96-99 | CLI flags inadequate for CI | ADD --state-file and --output-db flags |
| No plan files for 04-02, 04-03 | GitHub Actions workflows not designed | CREATE new plans |
| No plan for 01-06, 01-07 | Referenced in ROADMAP, may not exist | CHECK if files exist in directory |
| Cross-run state mechanism | Dependency but no plan | CREATE 04-03-PLAN.md with artifact strategy |
| Scheduled cron placement | No explicit cron definition | ADD to 04-03-PLAN.md |

---

## PRIORITY ORDER FOR CHANGES

1. **Remove Docker from BRIEF.md + ROADMAP.md** — immediate (2 files, small changes)
2. **Create 04-02-PLAN.md** — highest-impact new deliverable (GitHub Actions CI workflow)
3. **Create 04-03-PLAN.md** — second-highest impact (sync pipeline + artifact)
4. **Update 01-01-PLAN.md** — adds cross-run state architecture
5. **Update 01-05-PLAN.md** — adds --state-file and --output-db CLI flags
6. **Update 01-03-PLAN.md** — CI path awareness for tests
7. **Update BRIEF.md Success Criteria** — add GitHub Actions criteria
