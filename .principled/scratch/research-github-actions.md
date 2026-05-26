# Research: GitHub Actions + Rust CLI + SQLite CI/CD

**Scope**: Best practices for running Rust CLI tools with SQLite in GitHub Actions (2025-2026).
**Sources**: GitHub Actions docs, Swatinem/rust-cache, mozilla-actions/sccache, Depot blog, Cloudflare D1, Shuttle.dev, Fly.io, GitHub community discussions.

---

## 1. GitHub Actions + Rust Best Practices

### 1.1 Caching Cargo Build Artifacts

**Recommendation: Use `Swatinem/rust-cache@v2` (not raw `actions/cache`)**

The official example using raw `actions/cache` for Cargo is suboptimal. The `rust-cache` action (1.8K stars, actively maintained) provides smart Rust-specific caching that automatically handles:
- `~/.cargo` (registry cache, git deps)
- `./target` (dependency artifacts only)
- Hash of `Cargo.lock`, `rust-toolchain.toml`, `.cargo/config.toml` as cache keys
- Automatic cleanup of non-dependency artifacts
- Sets `CARGO_INCREMENTAL=0` to avoid caching non-dependency workspace crates

**Why raw `actions/cache` fails**: It caches the coarse `target/` directory as a blob. Downloads the entire blob even when you only need a subset. The 10GB GitHub cache limit fills quickly, and network transfer is slow.

```yaml
- uses: actions/checkout@v4
- run: rustup toolchain install stable --profile minimal
- uses: Swatinem/rust-cache@v2
- run: cargo build --release
```

**Key `rust-cache` options**:
- `prefix-key`: Change to force a new cache
- `shared-key`: Stable cross-job key
- `cache-targets: false`: Only cache deps, not workspace crates
- `save-if: ${{ github.ref == 'refs/heads/main' }}`: Only save on main branch

**Source**: https://github.com/marketplace/actions/rust-cache

### 1.2 Alternative: sccache for Even Faster Builds

For large workspaces, `sccache` outperforms `rust-cache`. It wraps `rustc` and uses content-addressable storage (CAS) rather than caching the whole `target/` dir. It starts building immediately and fetches only what's needed concurrently.

```yaml
- uses: mozilla-actions/sccache-action@v0.0.7
- name: Compile project
  env:
    RUSTC_WRAPPER: "sccache"
    SCCACHE_GHA_ENABLED: "true"
  run: cargo build --release
```

**Trade-off**: For your BDMP project (likely 50-100MB database, modest code), `rust-cache` is sufficient. sccache shines on monorepos with many crates.

**Source**: https://depot.dev/blog/sccache-in-github-actions

### 1.3 `cargo build --release` in CI

Standard pattern. Note: `ubuntu-latest` is now **Ubuntu 24.04** (migrated Dec 2024 - Jan 2025). This is important for SQLite native bindings.

```yaml
- run: rustup toolchain install stable --profile minimal
- uses: Swatinem/rust-cache@v2
- run: cargo build --release
- name: Strip debug symbols (optional, reduces binary size ~30%)
  if: matrix.os == 'linux'
  run: strip target/release/bdmp-cli
```

### 1.4 `rust-toolchain.toml` vs `dtolnay/rust-toolchain`

**Use `rust-toolchain.toml`** for reproducible CI:

```toml
# rust-toolchain.toml
[toolchain]
channel = "stable"
profile = "minimal"
components = ["llvm-tools-preview"]
```

Commit this file. `rust-cache` automatically hashes it for the cache key. This is cleaner than `dtolnay/rust-toolchain` action calls.

### 1.5 Static Binary (musl Target) for Linux

For a truly portable single-file binary with no glibc dependency:

```yaml
- run: rustup target add x86_64-unknown-linux-musl
- run: cargo build --release --target x86_64-unknown-linux-musl
```

**Limitations**:
- Requires crates to be compatible with musl (no glibc-only FFI)
- If you use `rusqlite` with bundled SQLite (default), this works fine
- If you use OpenSSL, use `openssl-sys` with `vendored` feature
- Binary size is larger but truly portable

**Docker alternative for complex deps**: `messense/rust-musl-cross` or `clux/muslrust` Docker images.

**Note**: Rust 1.93 (Jan 2026) updated musl to 1.2.5, improving compatibility.

---

## 2. SQLite as a GitHub Actions Artifact

### 2.1 Upload as Workflow Artifact

**Yes, works fine.** Use `actions/upload-artifact@v4`:

```yaml
- uses: actions/upload-artifact@v4
  with:
    name: bdmp-database
    path: data/bdmp.db
    retention-days: 30
```

**Artifact size limit**: Individual artifact **2GB max** (GitHub Releases assets: 2GB; Workflow artifacts: 500MB per file, but up to 10GB total per repo).

### 2.2 Size Limits for 50-100MB DB

Your 50-100MB SQLite file is well within limits:
- Workflow artifacts: 500MB/file, shared 10GB pool per repo
- **Caveat**: The 10GB is shared with cache storage. If your `rust-cache` is also consuming space, you may hit quota.

**Mitigation**:
```yaml
# Use short retention for DB artifacts
- uses: actions/upload-artifact@v4
  with:
    name: bdmp-database-${{ github.run_number }}
    path: data/bdmp.db
    retention-days: 7  # Keep only 7 days, not the default 90
```

**Source**: https://docs.github.com/en/actions/managing-workflow-runs-and-deployments/managing-workflow-runs/downloading-or-storing-artifacts

### 2.3 Publish as GitHub Release Asset

**This is the recommended approach for distribution.** Much better than workflow artifacts:

```yaml
- name: Create Release
  uses: softprops/action-gh-release@v2
  with:
    files: |
      target/release/bdmp-cli
      data/bdmp.db
    body: "BDMP Database v${{ github.run_number }}"
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

**Release asset limits**: 2GB per file, unlimited total. **SHA256 digests are now automatically computed and displayed** (GitHub Changelog, Jun 2025).

### 2.4 WAL Mode + Cross-Platform Portability

**Critical**: Before uploading a SQLite file, always checkpoint the WAL:

```bash
sqlite3 data/bdmp.db "PRAGMA wal_checkpoint(TRUNCATE);"
# OR
sqlite3 data/bdmp.db ".backup 'bdmp.db'"  # Creates a clean copy
```

**Why**: WAL mode leaves a `-wal` and `-shm` file alongside the `.db` file. Upload all three, or checkpoint first. A `.backup` command produces a single portable file.

**Cross-platform**: SQLite is endian-aware but byte-order-consistent on x86_64. The same `.db` file works on Linux, macOS, Windows.

---

## 3. Scheduled Workflows in GitHub Actions

### 3.1 Monthly Cron (BDPM Sync)

```yaml
name: Monthly BDPM Sync

on:
  schedule:
    # First day of every month at 2 AM UTC
    - cron: '0 2 1 * *'
  workflow_dispatch:
    inputs:
      full:
        description: 'Full sync (drop and rebuild)'
        required: false
        default: 'false'
        type: boolean
```

### 3.2 Weekly Cron (Dispo File)

```yaml
name: Weekly Dispo Update

on:
  schedule:
    # Every Sunday at 3 AM UTC
    - cron: '0 3 * * 0'
  workflow_dispatch:
```

### 3.3 `workflow_dispatch` for Manual Trigger

Always include `workflow_dispatch` alongside `schedule`. This allows manual testing without waiting for cron:

```yaml
on:
  schedule:
    - cron: '0 2 1 * *'
  workflow_dispatch:
    inputs:
      full:
        description: 'Run full sync'
        type: boolean
        default: false
```

```bash
# Access the input in steps:
- name: Build DB
  run: |
    FULL_FLAG=""
    if [ "${{ github.event.inputs.full }}" == "true" ]; then
      FULL_FLAG="--full"
    fi
    cargo run --release -- $FULL_FLAG
```

### 3.4 Pass DB to Deployment Step

Use `actions/upload-artifact@v4` + `actions/download-artifact@v4` between jobs:

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo build --release
      - run: cargo run --release -- build-db
      - uses: actions/upload-artifact@v4
        with:
          name: bdmp-artifacts
          path: |
            data/bdmp.db
            target/release/bdmp-cli
          retention-days: 7

  publish:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          name: bdmp-artifacts
          path: artifacts/
      - name: Publish to Release
        uses: softprops/action-gh-release@v2
        with:
          files: |
            artifacts/bdmp.db
            artifacts/bdmp-cli
```

### 3.5 Running `sqlite3` Commands on ubuntu-latest

**Ubuntu 24.04** (the current `ubuntu-latest`) has SQLite 3 preinstalled:

```bash
sqlite3 --version
# 3.45.1 or similar
```

No install step needed. Just run `sqlite3 data/bdmp.db "PRAGMA wal_checkpoint(TRUNCATE);"`.

**Note**: There was a reported issue with SQLite3 on ubuntu-24.04 runners (GitHub issue #11450) regarding spatialite, but plain SQLite3 CLI works fine.

---

## 4. Publishing the Database as an API

### 4.1 GitHub Pages for Static JSON API

**Option A: Pre-generate JSON files from SQLite**

In your workflow, dump SQLite to JSON:

```bash
# Using sqlite-utils (install in CI):
pip install sqlite-utils
sqlite-utils rows data/bdmp.db "SELECT * FROM drugs" --json > drugs.json
```

Then serve via GitHub Pages. Limit: No server-side logic. Static JSON only.

**Option B: sql.js-httpvfs for SQLite-over-HTTP on GitHub Pages**

This is interesting: Use sql.js (SQLite compiled to WebAssembly) + httpvfs to query SQLite files over HTTP Range Requests. The browser loads only needed chunks, not the whole DB.

```html
<script type="module">
import { openKv } from 'https://esm.sh/sql.js-httpvfs';
const db = await openKv({
  url: 'https://raw.githubusercontent.com/user/repo/main/data/bdmp.db',
  config: { /* table schemas */ }
});
```

**Limitation**: Complex queries slow; good for simple lookups. Requires defining table schemas upfront.

**Reference**: https://recca0120.github.io/en/2026/03/07/sql-js-httpvfs-static-hosting/

### 4.2 GitHub Releases as Binary Distribution

The simplest API: ship the CLI binary + SQLite file as release assets. Users download and run locally:

```yaml
- uses: softprops/action-gh-release@v2
  with:
    files: |
      target/release/bdmp-cli
      data/bdmp.db
    tag_name: v${{ github.run_number }}
```

Users: `curl -L $(gh release latest -R user/repo --json assets --jq '.assets[] | select(.name=="bdmp-cli") | .browser_download_url') | tar -xz && ./bdmp-cli query ...`

### 4.3 No "SQLite-over-HTTP from GitHub Actions Artifacts" Projects

There are no established projects that serve a SQLite file directly from GitHub Actions artifacts over HTTP. The standard pattern is:
1. Build in CI
2. Publish to Release (GitHub-hosted, slow CDN)
3. Or: Publish to R2/S3/Cloudflare R2 + Cloudflare Workers as front-end

### 4.4 Cloudflare Workers + R2 as Database API

For a proper HTTP API serving SQLite data:

**Architecture**:
- R2 (S3-compatible blob storage): Store the `.db` file
- Cloudflare Workers: Serve as API layer, query R2 via Workers KV or D1

**D1 approach** (Cloudflare's SQLite-at-edge product):
- D1 is SQLite with global replication
- You can create a D1 database, then use `wrangler d1 execute` in CI to load your data
- Free tier: 5GB storage, 100k reads/day, 50k writes/day
- Workers can query D1 directly

```yaml
# In GitHub Actions:
- name: Deploy to Cloudflare D1
  run: |
    npx wrangler d1 execute BDMP --file=./data/bdmp.db
  env:
    CLOUDFLARE_API_TOKEN: ${{ secrets.CLOUDFLARE_API_TOKEN }}
```

**Limitation**: D1 imports from `.sql` dumps, not raw `.db` files. You'd need to export from SQLite first.

**Reference**: https://developers.cloudflare.com/d1/

### 4.5 Cloudflare Workers + R2 (Bring Your Own SQLite)

If you want to serve your existing `.db` file:

```typescript
// Cloudflare Worker
export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    // R2 object: "bdmp.db"
    const dbFile = await env.BDMP_BUCKET.get('bdmp.db');
    // Serve the file with appropriate headers
    return new Response(dbFile.body, {
      headers: { 'Content-Type': 'application/x-sqlite3' }
    });
  }
};
```

Users download the `.db` file. For query API: use `@libsql/client` or `better-sqlite3` (via WASM) in the Worker.

**Reference**: https://blog.cloudflare.com/d1-ga

---

## 5. Recommended Workflow Structure

### 5.1 Monthly Workflow: BDPM Full Sync

```yaml
# .github/workflows/monthly-sync.yml
name: Monthly BDPM Sync

on:
  schedule:
    - cron: '0 2 1 * *'        # 1st of month, 2 AM UTC
  workflow_dispatch:
    inputs:
      full:
        description: 'Full rebuild (drops DB)'
        type: boolean
        default: false

jobs:
  sync:
    runs-on: ubuntu-latest
    timeout-minutes: 60
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        run: rustup toolchain install stable --profile minimal

      - name: Cache Rust artifacts
        uses: Swatinem/rust-cache@v2

      - name: Build
        run: cargo build --release

      - name: Fetch BDPM files
        run: |
          # Download BDPM data files (implement per your source)
          cargo run --release -- fetch-bdpm

      - name: Build SQLite DB
        run: |
          FULL_FLAG=""
          if [ "${{ github.event.inputs.full }}" == "true" ]; then
            FULL_FLAG="--full"
          fi
          cargo run --release -- build-db $FULL_FLAG

      - name: Checkpoint WAL
        run: sqlite3 data/bdmp.db "PRAGMA wal_checkpoint(TRUNCATE);"

      - name: Create Release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: bdmp-$(date +'%Y-%m')
          name: BDMP Database $(date +'%B %Y')
          files: data/bdmp.db
          generate_release_notes: true
          draft: false
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

      - name: Notify (optional)
        if: always()
        run: |
          echo "Sync completed at $(date)"
          ls -lh data/bdmp.db
```

### 5.2 Weekly Workflow: Dispo Update

```yaml
# .github/workflows/weekly-update.yml
name: Weekly Dispo Update

on:
  schedule:
    - cron: '0 3 * * 0'        # Sunday 3 AM UTC
  workflow_dispatch:

jobs:
  update:
    runs-on: ubuntu-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --release

      - name: Download latest release DB
        run: |
          gh release download latest --pattern '*.db' --dir data/ || echo "No existing DB, will create new"
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

      - name: Update Dispo
        run: cargo run --release -- update-dispo

      - name: Checkpoint WAL
        run: sqlite3 data/bdmp.db "PRAGMA wal_checkpoint(TRUNCATE);"

      - name: Update Release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: bdmp-latest
          name: BDMP Latest (Weekly)
          files: data/bdmp.db
          prerelease: true
          draft: true
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### 5.3 Manual Trigger with `--full` Flag

The `workflow_dispatch` with boolean input handles this in the monthly workflow above.

```yaml
# Trigger with:
# gh workflow run monthly-sync.yml --field full=true
# OR manually via GitHub UI
```

---

## 6. Alternative API Deployment (No Docker)

### 6.1 Can a Rust API Binary Run as a GitHub Actions Service?

**No.** GitHub Actions is a CI/CD platform, not a hosting platform. Workflows run ephemerally (minutes to hours). There's no concept of a long-running service in Actions.

### 6.2 Fly.io (Recommended for Rust APIs)

**Best fit for Rust**: Fly.io has native Rust deployment support with minimal cold starts.

- **Free tier**: 3 shared VMs, 160GB outbound bandwidth/month
- Deploy with: `fly launch && fly deploy`
- Attach volumes for SQLite persistence (or use LiteFS)
- Managed Postgres available (but you want SQLite)

```toml
# fly.toml
app = "bdmp-api"
primary_region = "iad"

[build]

[env]
  PORT = "8080"

[[services]]
  internal_port = 8080
  protocol = "tcp"

  [[services.ports]]
    handlers = ["http"]
    port = 80

  [[services.ports]]
    handlers = ["tls", "http"]
    port = 443

[[statics]]
  guest_path = "/app/bdmp.db"
  url_prefix = "/db"
```

**Reference**: https://fly.io/rust

### 6.3 Shuttle.rs (Rust-Native Deployment)

**Best DX for Rust**: Shuttle analyzes your code, provisions infrastructure from annotations, and deploys. No Docker files, no infrastructure config.

- **Free tier**: Shared resources, 3 deployments
- Supports Axum, Actix-web, Rocket, Poem, Tide
- Has a SQLx resource annotation for databases (but uses Postgres/MySQL under the hood, not SQLite)

**For SQLite specifically**: Shuttle can run any Rust binary, but doesn't natively provision SQLite. You'd ship the `.db` file as a static asset or mount a volume.

```rust
#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    // Your API code
    // SQLite file served as static asset or read at startup
}
```

```bash
cargo shuttle deploy
```

**Reference**: https://www.shuttle.dev

### 6.4 Railway (Simple, Flexible)

- **Free tier**: $5 credit/month (1GB RAM, shared CPU)
- No Docker required: detects Rust, builds with Cargo
- SQLite works fine: mount a volume or include in deployment

```bash
railway login
railway init
railway up
```

**Reference**: https://railway.com

### 6.5 Render (Managed, PostgreSQL-Focused)

- **Free tier**: Services sleep after 15 min inactivity
- No native SQLite support: you'd use a volume mount
- Not ideal for SQLite since cold starts mean reading DB each time

### 6.6 Vercel (Edge Functions)

- **Not suitable**: Vercel Functions are stateless, no persistent filesystem
- For serving SQLite data, you'd need to bundle the DB in the function (read-only) and use D1 instead

### 6.7 "SQLite as a Service" Options

| Platform | Type | SQLite Support | Free Tier |
|----------|------|--------------|-----------|
| **Turso/libSQL** | SQLite fork | Native | 9GB storage, 500 reqs/sec |
| **Cloudflare D1** | SQLite-at-edge | Native | 5GB, 100k reads/day |
| **PlanetScale** | MySQL | No | 1 DB, 1GB storage |
| **Neon** | Postgres | No | 3 branches, 0.5GB |
| **Supabase** | Postgres | No | 500MB database |

**Turso** is the closest to SQLite-as-a-service:
- libSQL is an open-source fork of SQLite
- Has HTTP-based replication
- Edge replication to 35+ regions
- Free tier generous for small projects

**Reference**: https://turso.tech

---

## Summary of Recommendations

| Need | Recommended Solution |
|------|----------------------|
| Rust caching in CI | `Swatinem/rust-cache@v2` |
| Static Linux binary | `cargo build --release --target x86_64-unknown-linux-musl` |
| SQLite artifact distribution | GitHub Release assets (2GB limit, no quota impact) |
| DB file preparation | `PRAGMA wal_checkpoint(TRUNCATE)` before upload |
| Scheduled builds | `schedule: cron:` + `workflow_dispatch:` |
| sqlite3 in CI | Pre-installed on ubuntu-latest (Ubuntu 24.04) |
| Static JSON API | Pre-generate JSON, serve via GitHub Pages |
| HTTP API | Fly.io or Shuttle.rs with SQLite volume |
| SQLite-as-a-service | Turso (libSQL) or Cloudflare D1 |

---

## References

- Swatinem/rust-cache: https://github.com/marketplace/actions/rust-cache
- sccache in CI: https://depot.dev/blog/sccache-in-github-actions
- GitHub Actions artifact docs: https://docs.github.com/en/actions/managing-workflow-runs/downloading-storing-artifacts
- GitHub Release assets (2GB limit, SHA digests): https://github.com/blog/changelog/2025-06-03
- Cloudflare D1: https://developers.cloudflare.com/d1/
- Fly.io Rust: https://fly.io/rust
- Shuttle.rs: https://www.shuttle.dev
- Turso/libSQL: https://turso.tech
- sql.js-httpvfs for GitHub Pages: https://recca0120.github.io/en/2026/03/07/sql-js-httpvfs-static-hosting/
- Scheduled workflows: https://oneuptime.com/blog/post/2025-12-20-scheduled-workflows-cron-github-actions/view
- musl static linking: https://blog.rust-lang.org/2025/12/05/Updating-musl-1.2.5/
