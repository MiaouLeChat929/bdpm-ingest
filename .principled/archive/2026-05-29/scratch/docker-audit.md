# Docker Audit — BDMP Project Plans

## Summary

Found **2 Docker references** in the project plan files, both in Phase 4 (Polish) section.

---

## Docker References

### 1. ROADMAP.md — Line 72

**File:** `/home/devadmin/Desktop/BDMP_DB/.principled/plans/ROADMAP.md`
**Line:** 72
**Content:**
```markdown
| 04-02 | Docker packaging |
```

**Action:** Replace with: `| 04-02 | GitHub Actions release workflow |`

---

### 2. BRIEF.md — Line 387

**File:** `/home/devadmin/Desktop/BDMP_DB/.principled/plans/BRIEF.md`
**Line:** 387
**Content:**
```markdown
**Phase 4**: CI regression suite, Docker, operational documentation
```

**Action:** Replace with: `**Phase 4**: CI regression suite, GitHub Actions release workflow, operational runbook`

---

## Phase 4 Context

Phase 4 is currently documented with these deliverables:
- 04-01: CI regression suite
- **04-02: Docker packaging** ← remove
- 04-03: Schema change response procedure + operational runbook

---

## No Other References Found

Searched for and found no occurrences of:
- `dockerfile` / `.dockerfile`
- `docker-compose`
- `container` / `containerized` / `containerise`
- `podman` / `containerd` / `kubernetes` / `k8s`
- `cron` / `systemd` / local scheduling
- `deploy` / `deployment` (in Docker context)

---

## Recommended Replacement for 04-02

Based on the project's stated goal of GitHub Actions for everything, replace "Docker packaging" with:

```
04-02: GitHub Actions release workflow
  - macOS/Windows/Linux binary builds via cross-compilation
  - Version tagging from Cargo.toml
  - Release asset upload
  - Homebrew tap formula generation (for macOS users)
```

---

## Phase 4 Full Rewrite

```markdown
## Phase 4: Polish

| Plan | Goal |
|------|------|
| 04-01 | CI regression suite (row counts, field counts, referential integrity, normalization) |
| 04-02 | GitHub Actions release workflow (cross-platform binaries, Homebrew tap) |
| 04-03 | Schema change response procedure + operational runbook |
```

```markdown
**Phase 4**: CI regression suite, GitHub Actions release workflow, operational documentation
```