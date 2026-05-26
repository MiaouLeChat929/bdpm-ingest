# GitHub Actions CI/CD — Research Findings 2026-05-26

## Recommendations

### Cache
- **Use `Swatinem/rust-cache@v2`** — better hit rates than manual `actions/cache`, handles `target/` correctly
- Skip `cargo-chef` — no Docker layer caching needed
- Do NOT cache `~/.cargo/bin/`

### MSRV
- **1.80 (pinned via `rust-toolchain.toml`)** — axum 0.8 pins MSRV at 1.80
- Test on `stable` + `1.80` in matrix

### Clippy
- Gate with `-D warnings` in CI
- `unsafe_code = "forbid"` — zero unsafe allowed
- No need for `cargo-deny` (personal project, no org license policy)
- Use `cargo-audit` for security (checks Cargo.lock vs RustSec advisory DB)

### Build Artifacts
- Release binary only, no `.db` files in repo
- Deterministic builds: `LTO=true`, `codegen-units=1`, `strip=symbols`
- Cross-platform: `cross` for non-Linux targets
- Database: `gh release create` + release asset, NOT git-committed (LFS or just release attachment)

### Complete CI workflow (ci.yml) and release workflow (release.yml) — see agent output
