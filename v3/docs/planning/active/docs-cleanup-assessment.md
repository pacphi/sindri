# Documentation Cleanup Assessment

**Version:** 1.0.0
**Created:** 2026-01-26
**Status:** Active Planning
**Priority:** P0 - Critical

---

## Executive Summary

A brutally honest assessment of Sindri documentation reveals **fragmentation and inconsistency** that confuses users choosing between V2 and V3. The documentation grew organically without a coherent versioning strategy, resulting in broken links, misleading content, and significant gaps.

**Overall Grade: C- (Functional but Chaotic)**

---

## Critical Findings

### 1. Extensions Docs Are NOT Bifurcated

**Severity:** CRITICAL

**Current State:**

- Extension documentation lives in shared `/docs/extensions/` (47 files)
- These docs describe V2 implementation (`v2/docker/lib/extensions/`)
- V3 has its own extensions at `v3/extensions/` with **NO corresponding documentation**

**Evidence:**

- `docs/EXTENSIONS.md` line 331 references: `/v2/docker/lib/extensions/`
- `docs/EXTENSION_AUTHORING.md` line 10: `mkdir -p v2/docker/lib/extensions/myext/{templates,scripts}`

**Impact:**

- V3 users reading extension docs get V2-only information
- V3 extensions are undocumented
- Users cannot determine which docs apply to their version

**Required Actions:**

1. Move `docs/extensions/` to `v2/docs/extensions/`
2. Create `v3/docs/extensions/` with V3-specific documentation
3. Update `docs/EXTENSIONS.md` → `v2/docs/EXTENSIONS.md`
4. Create `v3/docs/EXTENSIONS.md` for V3 extensions

---

### 2. Slides Are NOT Bifurcated

**Severity:** HIGH

**Current State:**

- Slides exist ONLY at `/v2/docs/slides/` (3 HTML files):
  - `getting-started.html`
  - `extensions.html`
  - `workspace-and-projects.html`
- V3 has NO slides directory

**Impact:** Presentation materials are V2-specific but not labeled as such.

**Required Actions:**

1. Add version indicator to V2 slides
2. Decide: Create V3 slides OR declare "slides not available for V3"

---

### 3. FAQ JSON Data Is NOT Bifurcated

**Severity:** CRITICAL

**Location:** `/docs/faq/src/faq-data.json` (670 lines, 99 Q&A entries)

**Broken References Found:**

| FAQ Line | Reference                      | Actual Location                   | Status     |
| -------- | ------------------------------ | --------------------------------- | ---------- |
| 59       | `docs/ARCHITECTURE.md`         | `v2/docs/ARCHITECTURE.md`         | BROKEN     |
| 83       | `docs/DEPLOYMENT.md`           | `v2/docs/DEPLOYMENT.md`           | BROKEN     |
| 91       | `docs/providers/`              | `v2/docs/providers/`              | BROKEN     |
| 115      | `docs/SCHEMA.md`               | `v2/docs/SCHEMA.md`               | BROKEN     |
| 499      | `docs/ARCHITECTURE.md`         | `v2/docs/ARCHITECTURE.md`         | BROKEN     |
| 515      | `docker/scripts/entrypoint.sh` | `v2/docker/scripts/entrypoint.sh` | BROKEN     |
| 531      | `deploy/adapters/`             | `v2/deploy/adapters/`             | BROKEN     |
| 627      | `docs/GPU.md`                  | `v2/docs/GPU.md` (V2-ONLY)        | MISLEADING |

**Required Actions:**

1. Bifurcate: Create `v2-faq-data.json` and `v3-faq-data.json`
2. Fix all path references to include version prefix
3. Add version selector to FAQ web interface

---

### 4. File Naming Inconsistencies

**Severity:** MEDIUM

**V2 Standard:** `UPPER_CASE_UNDERSCORE.md` (consistent)

**V3 Violations:**

| Current               | Should Be             |
| --------------------- | --------------------- |
| `getting-started.md`  | `GETTING_STARTED.md`  |
| `image-management.md` | `IMAGE_MANAGEMENT.md` |

**Pattern Standards:**

- Core docs: `UPPER_CASE_UNDERSCORE.md`
- Extensions: `UPPER-CASE-HYPHEN.md`
- ADRs: `NNN-kebab-case-description.md`

**Required Actions:**

1. Rename V3 inconsistent files
2. Document naming convention in CONTRIBUTING.md
3. Add naming linter to CI

---

### 5. False Claims / Misleading Documentation

**Severity:** HIGH

#### 5a. GPU Documentation Claims

FAQ line 627 references `docs/GPU.md` - only exists at `v2/docs/GPU.md`. V3 has no GPU documentation.

#### 5b. Extension Authoring Guide

`docs/EXTENSION_AUTHORING.md` is V2-only but presented as universal. V3 users following this guide create files in wrong location.

#### 5c. Profile Count Inconsistencies

- `docs/EXTENSIONS.md`: 12 profiles
- V2/V3 comparison guide: 8 profiles
- Various FAQ entries: inconsistent numbers

**Required Actions:**

1. Add version badges to all version-specific docs
2. Create V3 extension authoring guide
3. Audit and reconcile profile counts

---

### 6. Documentation Gaps

| Document                  | V2  | V3  | Action Required                                |
| ------------------------- | :-: | :-: | ---------------------------------------------- |
| `ARCHITECTURE.md`         | ✅  | ❌  | Create V3 overview (ADRs exist but no summary) |
| `GPU.md`                  | ✅  | ❌  | Create V3 GPU docs or document "not supported" |
| `TESTING.md`              | ✅  | ❌  | Create V3 testing docs                         |
| `TROUBLESHOOTING.md`      | ✅  | ❌  | Create V3 troubleshooting                      |
| `MANIFEST.md`             | ✅  | ❌  | Create V3 manifest docs                        |
| `BOM.md`                  | ✅  | ❌  | Create V3 BOM docs                             |
| `DEPLOYMENT.md`           | ✅  | ❌  | Create V3 deployment overview                  |
| `SECURITY.md`             | ✅  | ❌  | Create V3 security overview                    |
| `CI_WORKFLOW_IN_DEPTH.md` | ✅  | ❌  | Create V3 CI docs                              |
| `providers/` subdirectory | ✅  | ❌  | Create V3 provider docs                        |
| Extensions documentation  | ✅  | ❌  | Create V3 extension docs                       |
| `DOCTOR.md`               | ❌  | ✅  | V3-only (OK)                                   |
| `image-management.md`     | ❌  | ✅  | V3-only (OK)                                   |

---

### 7. Organization Inconsistencies

#### V2 Structure (Clean)

```
v2/docs/
├── *.md (16 files - uppercase, consistent)
├── providers/ (5 files)
├── security/ (3 files)
├── planning/ (1 file)
├── architecture/adr/ (1 file)
└── slides/ (3 HTML + guide)
```

#### V3 Structure (Issues)

```
v3/docs/
├── *.md (10 files - MIXED CASE)
├── .archived/ (11 files - should be hidden or removed)
├── planning/
│   ├── active/ (4 files)
│   └── complete/ (6 files)
└── architecture/adr/ (31 files + README)
```

**Issues:**

1. `.archived/` exposes internal implementation details
2. No `providers/` subdirectory
3. No `security/` subdirectory
4. No `extensions/` subdirectory

---

### 8. Root /docs Directory Confusion

**Current State:**

```
docs/
├── EXTENSIONS.md              ← V2-specific (misleading location)
├── EXTENSION_AUTHORING.md     ← V2-specific (misleading location)
├── extensions/                ← V2 extension docs (misleading location)
├── faq/                       ← Broken links, not bifurcated
├── ides/                      ← Version-agnostic (OK)
├── v2-v3-comparison-guide.md  ← Good
└── v2-v3-migration-guide.md   ← Good
```

The illusion of "shared docs" is misleading - most content is V2-specific.

---

## Recommended Target Structure

```
docs/
├── README.md                        ← Version selector landing page
├── shared/
│   ├── ides/                        ← IDE integration (version-agnostic)
│   └── migration/
│       ├── v2-v3-comparison.md
│       └── v2-v3-migration.md
├── v2/
│   ├── QUICKSTART.md
│   ├── ARCHITECTURE.md
│   ├── CLI.md
│   ├── CONFIGURATION.md
│   ├── DEPLOYMENT.md
│   ├── EXTENSIONS.md
│   ├── EXTENSION_AUTHORING.md
│   ├── extensions/                  ← V2 extension docs (47 files)
│   ├── providers/
│   ├── security/
│   ├── GPU.md
│   ├── TROUBLESHOOTING.md
│   └── slides/
├── v3/
│   ├── QUICKSTART.md
│   ├── CLI.md
│   ├── CONFIGURATION.md
│   ├── EXTENSIONS.md                ← NEW: V3 extensions overview
│   ├── EXTENSION_AUTHORING.md       ← NEW: V3 extension guide
│   ├── extensions/                  ← NEW: V3 extension docs
│   ├── PROJECTS.md
│   ├── IMAGE_MANAGEMENT.md
│   ├── DOCTOR.md
│   ├── TROUBLESHOOTING.md           ← NEW
│   ├── architecture/adr/
│   └── providers/                   ← NEW
└── faq/
    ├── v2-faq-data.json            ← Bifurcated
    └── v3-faq-data.json            ← Bifurcated
```

---

## Actionable Remediation Plan

### Phase 1: Critical Fixes (Week 1)

| ID  | Task                                                            | Effort | Impact | Owner |
| --- | --------------------------------------------------------------- | ------ | ------ | ----- |
| 1.1 | Fix FAQ broken links - update all `docs/X.md` to `v2/docs/X.md` | 2h     | HIGH   |       |
| 1.2 | Add version badges to EXTENSIONS.md and EXTENSION_AUTHORING.md  | 1h     | HIGH   |       |
| 1.3 | Rename V3 files: `getting-started.md` → `GETTING_STARTED.md`    | 30m    | MEDIUM |       |
| 1.4 | Rename V3 files: `image-management.md` → `IMAGE_MANAGEMENT.md`  | 30m    | MEDIUM |       |
| 1.5 | Create `docs/README.md` as version selector landing page        | 2h     | HIGH   |       |

### Phase 2: Structural Refactoring (Week 2)

| ID  | Task                                                                   | Effort | Impact | Owner |
| --- | ---------------------------------------------------------------------- | ------ | ------ | ----- |
| 2.1 | Move `docs/extensions/` to `v2/docs/extensions/`                       | 2h     | HIGH   |       |
| 2.2 | Move `docs/EXTENSIONS.md` to `v2/docs/EXTENSIONS.md`                   | 1h     | HIGH   |       |
| 2.3 | Move `docs/EXTENSION_AUTHORING.md` to `v2/docs/EXTENSION_AUTHORING.md` | 1h     | HIGH   |       |
| 2.4 | Create `v3/docs/EXTENSIONS.md` for V3 extensions                       | 4h     | HIGH   |       |
| 2.5 | Bifurcate FAQ: create `v2-faq-data.json` and `v3-faq-data.json`        | 4h     | HIGH   |       |
| 2.6 | Update all cross-references after moves                                | 3h     | HIGH   |       |

### Phase 3: V3 Extension Documentation (Week 3)

| ID  | Task                                             | Effort | Impact | Owner |
| --- | ------------------------------------------------ | ------ | ------ | ----- |
| 3.1 | Create `v3/docs/EXTENSION_AUTHORING.md`          | 4h     | HIGH   |       |
| 3.2 | Create `v3/docs/extensions/` directory structure | 1h     | MEDIUM |       |
| 3.3 | Document V3 extensions (estimate 40+ extensions) | 20h    | HIGH   |       |
| 3.4 | Create extension comparison table (V2 vs V3)     | 2h     | MEDIUM |       |

### Phase 4: Gap Filling (Week 4)

| ID  | Task                                                    | Effort | Impact | Owner |
| --- | ------------------------------------------------------- | ------ | ------ | ----- |
| 4.1 | Create `v3/docs/TROUBLESHOOTING.md`                     | 4h     | HIGH   |       |
| 4.2 | Create `v3/docs/providers/` with provider-specific docs | 6h     | HIGH   |       |
| 4.3 | Create `v3/docs/DEPLOYMENT.md` overview                 | 4h     | HIGH   |       |
| 4.4 | Create `v3/docs/ARCHITECTURE.md` as ADR summary         | 3h     | MEDIUM |       |
| 4.5 | Hide or remove `.archived/` directory                   | 30m    | LOW    |       |
| 4.6 | Create V3 slides OR add deprecation notice              | 8h     | MEDIUM |       |

### Phase 5: Quality Assurance (Ongoing)

| ID  | Task                                         | Effort | Impact | Owner |
| --- | -------------------------------------------- | ------ | ------ | ----- |
| 5.1 | Verify link checker CI coverage              | 2h     | HIGH   |       |
| 5.2 | Add naming convention linter to CI           | 4h     | MEDIUM |       |
| 5.3 | Update CONTRIBUTING.md with naming standards | 2h     | MEDIUM |       |
| 5.4 | Create quarterly doc review checklist        | 2h     | LOW    |       |

---

## Priority Summary

### P0 - Do Immediately (This Week)

1. **Fix FAQ broken links** - Users hit dead ends constantly
2. **Add version badge to EXTENSION_AUTHORING.md header** - Stop V3 users from following V2 guide
3. **Create docs/README.md** - Version selector landing page

### P1 - Do Next Sprint

4. **Move extension docs under v2 namespace**
5. **Bifurcate FAQ data** - Two JSON files with version-specific paths
6. **Create V3 EXTENSIONS.md** - Document V3 extension system
7. **Rename V3 inconsistent files**

### P2 - Plan for Next Month

8. **Document all V3 extensions** (major effort - 40+ extensions)
9. **Fill V3 documentation gaps** (TROUBLESHOOTING, providers, DEPLOYMENT)
10. **Create V3 slides or deprecation notice**
11. **Implement naming convention linter**

---

## Success Metrics

| Metric                     | Current | Target |
| -------------------------- | ------- | ------ |
| Broken FAQ links           | ~15     | 0      |
| V3 extension docs coverage | 0%      | 100%   |
| File naming compliance     | ~80%    | 100%   |
| Version-labeled docs       | ~20%    | 100%   |
| User confusion reports     | Unknown | -50%   |

---

## Risk Assessment

| Risk                                       | Likelihood | Impact | Mitigation                                  |
| ------------------------------------------ | ---------- | ------ | ------------------------------------------- |
| Breaking existing links during restructure | HIGH       | HIGH   | Implement redirects, update in phases       |
| V3 extension docs become stale             | MEDIUM     | HIGH   | Automate doc generation from extension.yaml |
| Team bandwidth for 40+ extension docs      | HIGH       | MEDIUM | Prioritize most-used extensions first       |
| FAQ bifurcation introduces new bugs        | MEDIUM     | MEDIUM | Comprehensive link testing after split      |

---

## Notes

- V3 extensions exist at `v3/extensions/` (verified)
- V2 extensions exist at `v2/docker/lib/extensions/`
- Current `docs/extensions/` documents V2 implementation only
- This is a documentation-as-code problem requiring systematic approach

---

_Assessment generated: 2026-01-26_
_Review mode: Ramsay (Standards) + Bach (BS Detection)_
