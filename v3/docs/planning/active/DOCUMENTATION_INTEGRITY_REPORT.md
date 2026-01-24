# Sindri Documentation Integrity Report

**Report Date:** 2026-01-24
**Analysis Method:** Multi-Agent Swarm (6 specialized agents)
**Scope:** v2/, v3/, .github/, docs/ directories
**Version:** 1.0.0

---

## Executive Summary

### Overall Documentation Health Score: C+ (72/100)

| Category | Score | Status |
|----------|-------|--------|
| V2 Documentation | 78/100 | Good - minor path issues |
| V3 Documentation | 68/100 | Needs Work - major gaps |
| GitHub Documentation | 78/100 | Good - phantom directories |
| Root Documentation | 60/100 | Critical - CLAUDE.md misaligned |

### Critical Metrics

| Metric | Value |
|--------|-------|
| **Total Gaps Identified** | 42 |
| **Critical Issues** | 8 |
| **Phantom Directories** | 4 |
| **Dead Links** | 5+ |
| **Inaccurate Claims** | 12+ |
| **V3 Doc Coverage vs V2** | ~45% |

### Top 5 Critical Issues (Fix This Week)

1. **CLAUDE.md Identity Crisis** - Contains Claude Flow V3 orchestration docs, not Sindri guidance
2. **V3 Profile Descriptions Are WRONG** - `getting-started.md` profiles don't match `profiles.yaml`
3. **ADR Status Lies** - ADR-015, 016, 020 marked "Proposed" but fully implemented
4. **Dead CI Badge** - README.md links to non-existent `ci.yml`
5. **V3 CLI Reference Missing** - V2 has 1089-line CLI.md, V3 has none

### Estimated Effort to Fix

| Priority | Hours | Weeks |
|----------|-------|-------|
| Critical | 8-12 | 1 |
| High | 20-30 | 2 |
| Medium | 25-35 | 3 |
| Low | 15-20 | 4 |
| **Total** | **80-100** | **4** |

---

## Section 1: Critical Issues (Fix Immediately)

### 1.1 CLAUDE.md Scope Misalignment

**File:** `/CLAUDE.md`
**Severity:** CRITICAL
**Impact:** Claude Code follows wrong guidance for Sindri tasks

**Problem:** The root CLAUDE.md contains extensive documentation for "Claude Flow V3" orchestration system, MCP tools, and swarm configurations that are unrelated to the Sindri development environment tool.

**Current Content (excerpt):**
```markdown
# Claude Code Configuration - Claude Flow V3
## AUTOMATIC SWARM ORCHESTRATION
...
```

**Recommended Fix:** Replace with Sindri-specific guidance:
```markdown
# Claude Code Configuration - Sindri

## Project Overview
Sindri is a declarative, provider-agnostic cloud development environment system.

## Versions
- **v2**: Bash/Docker implementation (stable) - `/v2/`
- **v3**: Rust CLI implementation (active development) - `/v3/`

## Development Commands
pnpm v2:validate   # Validate v2 code
pnpm v3:test       # Run v3 tests (cargo test)

## Key Documentation
- CLI Reference: /v2/docs/CLI.md
- Architecture: /v2/docs/ARCHITECTURE.md
- ADRs: /v3/docs/architecture/adr/
```

---

### 1.2 V3 Profile Descriptions Are Inaccurate

**File:** `/v3/docs/getting-started.md` (lines 200-246)
**Severity:** CRITICAL
**Impact:** Users get wrong expectations about profile contents

| Profile | Documented | Actual (profiles.yaml) |
|---------|-----------|------------------------|
| `minimal` | "git, vim, basic shell tools" | nodejs, python |
| `mobile` | "Android SDK, iOS tools, Flutter" | nodejs, linear-mcp, supabase-cli |
| `fullstack` | "Node.js, Python, databases, Docker-in-Docker" | nodejs, python, docker, nodejs-devtools |
| `ai-dev` | "Python, Jupyter, TensorFlow, PyTorch" | nodejs, python, golang, spec-kit, ollama, ai-toolkit |

**Action:** Update profile descriptions to match actual `v2/docker/lib/profiles.yaml` contents.

---

### 1.3 ADR Status Inconsistencies

**Files:** `/v3/docs/architecture/adr/`
**Severity:** CRITICAL
**Impact:** Contributors may waste time implementing features that already exist

| ADR | Documented Status | Implementation Status |
|-----|-------------------|----------------------|
| ADR-015 (Secrets Resolver) | Proposed | **IMPLEMENTED** (`sindri-secrets/src/resolver.rs`) |
| ADR-016 (Vault Integration) | Proposed | **IMPLEMENTED** (VaultSource exists) |
| ADR-020 (S3 Encrypted Storage) | Proposed | **IMPLEMENTED** (`sindri-secrets/src/s3/`) |

**Action:** Update status from "Proposed" to "Accepted" in these 3 ADR files.

---

### 1.4 Dead CI Badge Link

**File:** `/README.md` (line 4)
**Severity:** CRITICAL
**Impact:** Broken badge on primary landing page

**Problem:**
```markdown
[![CI](https://github.com/pacphi/sindri/actions/workflows/ci.yml/badge.svg)]
```

**Reality:** `ci.yml` doesn't exist. Project uses `ci-v2.yml` and `ci-v3.yml` (per ADR-021).

**Fix:**
```markdown
[![CI V2](https://github.com/pacphi/sindri/actions/workflows/ci-v2.yml/badge.svg)]
[![CI V3](https://github.com/pacphi/sindri/actions/workflows/ci-v3.yml/badge.svg)]
```

---

### 1.5 Phantom Directory References

**Severity:** HIGH
**Impact:** Contributors look for non-existent directories, copy commands that fail

| Claimed Directory | Claimed In | Reality |
|-------------------|-----------|---------|
| `.github/actions/core/` | `.github/README.md`, `WORKFLOW_ARCHITECTURE.md` | Does NOT exist |
| `test/` (at root) | `WORKFLOW_ARCHITECTURE.md` | Actual: `v2/test/` |
| `.github/scripts/lib/` | `WORKFLOW_ARCHITECTURE.md` | Does NOT exist |
| `.github/scripts/calculate-profile-resources.sh` | `WORKFLOW_ARCHITECTURE.md` | Does NOT exist |

**Action:** Update all documentation to reflect actual directory structure.

---

## Section 2: V3 Documentation Gaps

### 2.1 Critical Gaps (Risk 0.9-1.0)

| Gap | Risk Score | Effort | Impact |
|-----|------------|--------|--------|
| **v3 CLI Reference** (v2 has 1089-line CLI.md) | 0.98 | 6-8h | Users can't discover 45+ commands |
| **v3 Configuration Reference** | 0.95 | 4-6h | No sindri.yaml v3 schema docs |
| **v3 Secrets Management Guide** | 0.95 | 3-4h | Fully implemented, zero user docs |
| **v3 Backup/Restore User Guide** | 0.92 | 2-3h | Commands exist, no guides |
| **v3 Project Command Documentation** | 0.92 | 3-4h | `sindri project new/clone` undocumented |
| **v3 Quickstart Guide** | 0.90 | 2-3h | No onboarding path |
| **v3 Doctor Command Documentation** | 0.90 | 1-2h | Diagnostics undocumented |
| **v3 K8s Cluster Management Guide** | 0.90 | 2-3h | `sindri k8s` commands undocumented |

### 2.2 High Priority Gaps (Risk 0.7-0.89)

| Gap | Risk Score | Effort |
|-----|------------|--------|
| v3 Provider-Specific Deployment Guides | 0.88 | 8-10h |
| Missing Root README for v3 directory | 0.85 | 1-2h |
| v3 Extension Authoring Guide | 0.85 | 4-5h |
| v3 Troubleshooting Guide | 0.82 | 3-4h |
| v3 Architecture Overview | 0.80 | 3-4h |
| v2-to-v3 Migration Guide | 0.80 | 4-5h |
| CI/CD Workflow Documentation for v3 | 0.78 | 2-3h |
| v3 Security Guide | 0.75 | 2-3h |
| v3 Testing Guide | 0.75 | 2-3h |
| v3 GPU Support Documentation | 0.72 | 2-3h |
| ADR-007 Missing (file doesn't exist) | 0.70 | 1-2h |

---

## Section 3: Implementation Knowledge Graph

### V2 vs V3 Comparison

| Aspect | V2 | V3 |
|--------|----|----|
| **Language** | Bash | Rust (11 crates) |
| **Extensions** | 77 | 44 |
| **CLI Commands** | ~20 | ~45+ (with subcommands) |
| **Providers** | 5 | 5 |
| **ADRs** | 1 | 30 |
| **Documentation Files** | 27+ | 52+ (mostly ADRs) |
| **Lines of Code** | ~10K (bash) | ~15K+ (rust) |

### V3 Crate Structure

```
sindri (main CLI)
├── sindri-core        # Types, config, error handling
├── sindri-providers   # Docker, Fly, DevPod, E2B, K8s
├── sindri-extensions  # Registry, dependency resolver
├── sindri-secrets     # Resolver, Vault, S3 encrypted
├── sindri-backup      # Archive builder, profiles
├── sindri-projects    # Git operations, templates
├── sindri-doctor      # Diagnostics, tool checker
├── sindri-clusters    # Kind, K3d providers
├── sindri-image       # Registry client, verifier
└── sindri-update      # Self-update system
```

### ADR Implementation Status

| Phase | ADRs | Status |
|-------|------|--------|
| Phase 1: Workspace Architecture | 001 | ✅ IMPLEMENTED |
| Phase 2-3: Providers & Templates | 002-007 | ✅ IMPLEMENTED |
| Phase 4: Extension System | 008-014, 026 | ✅ IMPLEMENTED |
| Phase 5: Secrets & Backup | 015-020 | ⚠️ PARTIAL (status lies) |
| Phase 6: CI/CD & Update | 021-022 | ✅ IMPLEMENTED |
| Phase 7-8: Projects & Doctor | 023-030 | ✅ IMPLEMENTED |

---

## Section 4: Code-Documentation Alignment Analysis

### 4.1 Schema Mismatches

| Schema Doc (SCHEMA.md) | Actual Schema | Difference |
|------------------------|---------------|------------|
| `categories.schema.json` | Not documented | Schema exists but missing from SCHEMA.md |
| `vm-sizes.schema.json` | Not documented | Entire schema undocumented |
| Extension categories | 8 documented | 11 in schema (`agile`, `database`, `mobile` missing) |
| Install methods | 6 documented | 7 in schema (`hybrid` method undocumented) |
| `capabilities.features` | Not documented | V3 feature flags undocumented |

### 4.2 Undocumented Environment Variables

| Variable | Crate | Purpose |
|----------|-------|---------|
| `EXTENSION_CONFLICT_STRATEGY` | capability-manager.sh | Conflict handling mode |
| `EXTENSION_CONFLICT_PROMPT` | capability-manager.sh | User prompt control |
| `VAULT_ADDR`, `VAULT_TOKEN` | sindri-secrets | Vault authentication |
| `GITHUB_TOKEN` | sindri-image, sindri-update | GitHub API auth |
| `E2B_API_KEY` | sindri-providers | E2B authentication |
| `DOCKER_USERNAME`, `DOCKER_PASSWORD` | sindri-providers | Docker registry auth |

### 4.3 Extension Documentation Status

| Category | Has README | Has SKILL.md | API Documented |
|----------|------------|--------------|----------------|
| Core (nodejs, python, etc.) | 0% | 0% | Yes (in /docs) |
| AI Extensions | 50% | 0% | Yes (in /docs) |
| MCP Servers | 0% | 75% | Yes (in /docs) |
| VisionFlow (vf-*) | Partial | Yes (most) | Partial |

---

## Section 5: Accuracy Problems (BS Detection)

### 5.1 Example Count Discrepancies

**File:** `/examples/README.md`

| Claim | Actual |
|-------|--------|
| "61 examples" | 67 `.sindri.yaml` files |
| "50 total in matrix" | Doesn't match |
| "55 individual examples" | Doesn't match |
| "17 Fly.io examples" | 10 actually exist |
| "11 Docker examples" | 15 actually exist |
| "19 DevPod examples" | 18 actually exist |

**Missing Fly.io examples claimed in docs:**
- `ai-dev.sindri.yaml` - NOT FOUND
- `devops.sindri.yaml` - NOT FOUND
- `enterprise.sindri.yaml` - NOT FOUND

### 5.2 SLSA Level 3 Claim

**File:** `/README.md` (line 11-12)
```markdown
**Secure Supply Chain:** All release images are signed with Cosign,
include SBOM, and have SLSA Level 3 provenance attestations.
```

**Verification Needed:** Standard `docker/build-push-action` provides basic provenance but Level 3 requires additional tooling. Claim needs verification.

### 5.3 CI Documentation Staleness

**File:** `/v2/docs/CI_WORKFLOW_IN_DEPTH.md`

Contains 20+ references to old `ci.yml` which was bifurcated into `ci-v2.yml` and `ci-v3.yml` per ADR-021. Documentation not updated.

---

## Section 6: Missing Infrastructure

### 6.1 No CODEOWNERS File

**Expected Location:** `.github/CODEOWNERS`
**Impact:** No defined ownership for PR reviews

**Recommended Content:**
```
# Code Owners
/v2/                @pacphi
/v3/                @pacphi
/.github/workflows/ @pacphi
/docs/              @pacphi
```

### 6.2 No GitHub Issue Templates

**Expected Location:** `.github/ISSUE_TEMPLATE/`

**Recommended Templates:**
- `bug_report.md`
- `feature_request.md`
- `extension_request.md`
- `documentation.md`

### 6.3 Missing Root CONTRIBUTING.md

**References point to:** `../CONTRIBUTING.md`
**Actual location:** `docs/CONTRIBUTING.md`

Fix all relative path references from `.github/` and `v2/`.

---

## Section 7: Documentation Sprint Plan

### Week 1: Critical Fixes (12-16 hours)

| Day | Task | Hours | Files |
|-----|------|-------|-------|
| 1 | Refactor CLAUDE.md for Sindri | 3h | `/CLAUDE.md` |
| 2 | Fix v3 profile descriptions | 2h | `/v3/docs/getting-started.md` |
| 3 | Update ADR statuses (015, 016, 020) | 1h | `/v3/docs/architecture/adr/` |
| 4 | Fix CI badge and phantom directories | 2h | `/README.md`, `/.github/*.md` |
| 5 | Fix v2 QUICKSTART paths | 1h | `/v2/docs/QUICKSTART.md` |
| 5 | Audit and fix example counts | 3h | `/examples/README.md` |

### Week 2: V3 User Documentation (16-20 hours)

| Task | Hours | Output |
|------|-------|--------|
| v3 CLI Reference | 6-8h | `/v3/docs/CLI.md` |
| v3 Configuration Reference | 4-6h | `/v3/docs/CONFIGURATION.md` |
| v3 Quickstart Guide | 2-3h | `/v3/docs/QUICKSTART.md` |
| v3 README.md | 1-2h | `/v3/README.md` |

### Week 3: Feature Documentation (16-20 hours)

| Task | Hours | Output |
|------|-------|--------|
| v3 Secrets Management Guide | 3-4h | `/v3/docs/SECRETS_MANAGEMENT.md` |
| v3 Backup/Restore Guide | 2-3h | `/v3/docs/BACKUP_RESTORE.md` |
| v3 Project Commands Guide | 3-4h | `/v3/docs/PROJECTS.md` |
| v3 K8s/Doctor Guide | 3-4h | `/v3/docs/K8S.md`, `/v3/docs/DOCTOR.md` |
| v3 Provider Guides | 4-5h | `/v3/docs/providers/` |

### Week 4: Migration & Polish (12-16 hours)

| Task | Hours | Output |
|------|-------|--------|
| v2-to-v3 Migration Guide | 4-5h | `/docs/MIGRATION_V2_V3.md` |
| v3 Extension Authoring | 4-5h | `/v3/docs/EXTENSION_AUTHORING.md` |
| Add CODEOWNERS | 0.5h | `/.github/CODEOWNERS` |
| Add Issue Templates | 1h | `/.github/ISSUE_TEMPLATE/` |
| Archive stale docs | 2h | Various |

---

## Section 8: Maintenance Recommendations

### Review Cadence

| Area | Frequency | Trigger |
|------|-----------|---------|
| ADRs | On implementation completion | Code merge |
| CLI Reference | With each release | Version bump |
| Extension Catalog | Monthly | Registry changes |
| CHANGELOG | Each release (automated) | Tag push |
| CLAUDE.md | Quarterly | Manual review |

### Automation Opportunities

| Task | Tool | Status |
|------|------|--------|
| Link checking | `check-links.yml` | ✅ Implemented |
| Markdown linting | `validate-markdown.yml` | ✅ Implemented |
| YAML validation | `validate-yaml.yml` | ✅ Implemented |
| Dead link detection | Consider `lychee` | 📋 Opportunity |
| Doc generation from Rust | `cargo doc` | 📋 Opportunity |
| Extension catalog auto-gen | Parse `registry.yaml` | 📋 Opportunity |

### Process Improvements

1. **ADR Lifecycle Gate:** Require status update when implementation merges
2. **Extension Docs CI:** Auto-validate extension count claims
3. **Version Labels:** Add v2-only/v3-only badges to docs
4. **Example Validation:** CI job to verify example counts match reality

---

## Section 9: Files Requiring Changes

### Priority 1: Critical (This Week)

| File | Action | Lines |
|------|--------|-------|
| `/CLAUDE.md` | Replace content | All |
| `/v3/docs/getting-started.md` | Update profile descriptions | 200-246 |
| `/v3/docs/architecture/adr/015-*.md` | Change status to Accepted | Status field |
| `/v3/docs/architecture/adr/016-*.md` | Change status to Accepted | Status field |
| `/v3/docs/architecture/adr/020-*.md` | Change status to Accepted | Status field |
| `/README.md` | Fix CI badge | Line 4 |
| `/.github/README.md` | Remove phantom directory references | 38-46 |
| `/.github/WORKFLOW_ARCHITECTURE.md` | Fix directory structure | 43-80 |

### Priority 2: High (This Sprint)

| File | Action |
|------|--------|
| `/v2/docs/QUICKSTART.md` | Fix CLI path (cli/ → v2/cli/) |
| `/examples/README.md` | Audit and fix all counts |
| `/docs/EXTENSIONS.md` | Add version-specific counts |
| `/v2/docs/CI_WORKFLOW_IN_DEPTH.md` | Update ci.yml → ci-v2.yml/ci-v3.yml |

### Priority 3: Create New

| File | Priority |
|------|----------|
| `/v3/docs/CLI.md` | Critical |
| `/v3/docs/CONFIGURATION.md` | Critical |
| `/v3/docs/QUICKSTART.md` | High |
| `/v3/README.md` | High |
| `/.github/CODEOWNERS` | Medium |
| `/.github/ISSUE_TEMPLATE/` | Medium |
| `/docs/MIGRATION_V2_V3.md` | High |

---

## Appendix A: Analysis Agents

This report was generated by a 6-agent swarm:

| Agent | Role | Key Contribution |
|-------|------|------------------|
| Researcher | Deep docs vs implementation | Found profile mismatches, ADR status lies |
| Code Intelligence | Implementation knowledge graph | Mapped 11 crates, 45+ commands, 30 ADRs |
| Reviewer (Bach Mode) | BS detection | Found phantom directories, dead links, count lies |
| Code Analyzer | Code structure analysis | Found schema mismatches, env var gaps |
| Gap Detector | Documentation coverage gaps | Identified 42 gaps with risk scores |
| System Architect | Final synthesis | Compiled actionable sprint plan |

---

## Appendix B: Summary Statistics

| Metric | Count |
|--------|-------|
| Total Documentation Files Analyzed | ~150 |
| Critical Issues Found | 8 |
| High Priority Issues | 14 |
| Stale Documentation Items | 6 |
| Missing Documentation Items | 12 |
| Phantom Directories | 4 |
| V2 Extensions | 77 |
| V3 Extensions | 44 |
| ADRs Total | 30 |
| ADRs with Status Mismatch | 3 |
| GitHub Workflows | 16 |
| Estimated Fix Effort | 80-100 hours |

---

*Report generated by Documentation Integrity Swarm Analysis*
*Swarm ID: swarm-1769212979965*
