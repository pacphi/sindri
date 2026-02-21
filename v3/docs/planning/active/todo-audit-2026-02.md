# TODO Audit — February 2026

**Date:** 2026-02-20
**Branch:** `feature/sindri-administrator`
**Scope:** Full repository audit of implementation, tests, documentation, and CI/CD
**Last updated:** 2026-02-20 — Resolution status added for completed D and CI items

---

## Table of Contents

- [Executive Summary](#executive-summary)
- [Part 1: Implementation & Tests](#part-1-implementation--tests)
  - [Critical — Silently Broken Functionality](#critical--silently-broken-functionality)
  - [Important — Missing Features / Broken UX](#important--missing-features--broken-ux)
  - [Low — Polish & Scaling](#low--polish--scaling)
  - [Tests — Ignored & Skipped](#tests--ignored--skipped)
- [Part 2: Documentation](#part-2-documentation)
  - [High Priority](#documentation-high-priority)
  - [Medium Priority](#documentation-medium-priority)
  - [Low Priority](#documentation-low-priority)
- [Part 3: GitHub Actions & Workflows](#part-3-github-actions--workflows)
  - [High Priority](#ci-high-priority)
  - [Medium Priority](#ci-medium-priority)
  - [Low Priority](#ci-low-priority)
- [Prioritized Recommendations](#prioritized-recommendations)

---

## Executive Summary

| Area                 | Critical | Important | Medium | Low    | Total  |
| -------------------- | -------- | --------- | ------ | ------ | ------ |
| Implementation stubs | 3        | 3         | 0      | 3      | 9      |
| Documentation        | 0        | 0         | 8      | 4      | 12     |
| CI/CD                | 0        | 0         | 9      | 7      | 16     |
| **Totals**           | **3**    | **3**     | **17** | **14** | **37** |

The Rust crates (`v3/crates/`) are clean — zero TODOs, zero stubs, zero clippy warnings. All findings are in the console application (`v3/console/`), documentation (`v3/docs/`), and GitHub Actions (`.github/`).

---

## Part 1: Implementation & Tests

### Critical — Silently Broken Functionality

#### IMPL-01: Security Alert Evaluation Always Returns Unfired

- **File:** [`v3/console/apps/api/src/services/alerts/evaluator.service.ts`](../../../console/apps/api/src/services/alerts/evaluator.service.ts) (lines 343–348)
- **Severity:** Critical
- **Description:** The `evaluateSecurity()` function is a stub that always returns `{ fired: false }`. All alert rules of type `security` (CVE checks, secret exposure, compliance violations) silently never fire. Users who configure security-based alert rules receive no notifications regardless of actual security events.

```typescript
function evaluateSecurity(cond: SecurityCondition, _ctx: EvaluationContext): EvaluationResult {
  // Security checks require external integrations (CVE feed, secret manager)
  logger.debug({ check: cond.check }, "Security check evaluation (stub)");
  return { fired: false, title: "", message: "" };
}
```

- **Impact:** An entire alert category is non-functional. No user-facing indication that security alerts are disabled.
- **Action:** Integrate CVE feed or secret manager data source, or add explicit UI warning that security alerts are not yet operational.

---

#### IMPL-02: Cost Alert Evaluation Always Returns Unfired

- **File:** [`v3/console/apps/api/src/services/alerts/evaluator.service.ts`](../../../console/apps/api/src/services/alerts/evaluator.service.ts) (lines 350–354)
- **Severity:** Critical
- **Description:** The `evaluateCost()` function is a stub that always returns `{ fired: false }`. All alert rules of type `cost` (budget overrun, spending anomaly) silently never fire.

```typescript
function evaluateCost(cond: CostCondition, _ctx: EvaluationContext): EvaluationResult {
  // Cost checks require billing data integration
  logger.debug({ period: cond.period, budget: cond.budget_usd }, "Cost check evaluation (stub)");
  return { fired: false, title: "", message: "" };
}
```

- **Impact:** Budget alerts configured via the UI will never trigger. Silent data loss for cost monitoring.
- **Action:** Integrate billing data source, or add explicit UI warning that cost alerts are not yet operational.

---

#### IMPL-03: Email Notifications Are Dropped

- **File:** [`v3/console/apps/api/src/services/alerts/dispatcher.service.ts`](../../../console/apps/api/src/services/alerts/dispatcher.service.ts) (line 281)
- **Severity:** Critical
- **Description:** The `sendEmail()` function logs "Email notification (stub)" and returns immediately. The `mailer.send()` call is commented out. Any email notification channel configured in production silently drops all alerts.

```typescript
// Email integration stub — in production wire up to Resend, SendGrid, or Nodemailer
// TODO: integrate SMTP/email service
// await mailer.send({ to: config.recipients, subject, text });
```

- **Impact:** Users configuring email as an alert channel receive nothing. No error or warning is surfaced.
- **Action:** Wire up SMTP provider (Resend, SendGrid, or Nodemailer) or remove email from the UI as a channel option until implemented.

---

### Important — Missing Features / Broken UX

#### IMPL-04: Instance Navigation Not Wired

- **File:** [`v3/console/apps/web/src/pages/InstancesPage.tsx`](../../../console/apps/web/src/pages/InstancesPage.tsx) (lines 6–8)
- **Severity:** Important
- **Description:** The `InstancesPage` renders an `InstanceList` but clicking on an instance does nothing. The `handleSelectInstance` handler body is empty.

```tsx
function handleSelectInstance(_instance: Instance) {
  // TODO: Implement navigation to /instances/${instance.id}
}
```

- **Impact:** Broken primary UX path — users cannot navigate from fleet list to instance detail view.
- **Action:** Wire up TanStack Router navigation to `/instances/${instance.id}`.

---

#### IMPL-05: Fleet Scan Button Non-Functional

- **File:** [`v3/console/apps/web/src/components/security/SecurityDashboard.tsx`](../../../console/apps/web/src/components/security/SecurityDashboard.tsx) (line 178)
- **Severity:** Important
- **Description:** The "Run Scan" button in the Security Dashboard has an empty `onClick` handler. No API call is triggered, no instance is selected. The `scanning` state never becomes true from user interaction.

```tsx
<Button
  onClick={() => {
    /* TODO: pick instance for fleet scan */
  }}
  disabled={scanning}
>
  {scanning ? "Scanning..." : "Run Scan"}
</Button>
```

- **Impact:** Security scanning feature is rendered but completely non-functional.
- **Action:** Implement instance picker and trigger fleet scan API call.

---

#### IMPL-06: API Key Management Endpoint Missing

- **File:** [`v3/console/apps/api/tests/auth-middleware.test.ts`](../../../console/apps/api/tests/auth-middleware.test.ts) (line 80)
- **Severity:** Important
- **Description:** The `/api/v1/api-keys` route is referenced in integration tests but never registered in `app.ts`. No corresponding route file exists. The test silently passes by skipping when the endpoint returns a non-201 response.

```typescript
if (createRes.status !== 201) return; // Skip if endpoint not implemented
```

- **Impact:** API key lifecycle management (create, list, revoke) is a security feature that is untested and unimplemented. The test gives a false green.
- **Action:** Implement the `/api/v1/api-keys` route or remove the skip guard so the test correctly fails.

---

### Low — Polish & Scaling

#### IMPL-07: Silent Error on Terminal Tab Creation

- **File:** [`v3/console/apps/web/src/components/terminal/MultiTerminal.tsx`](../../../console/apps/web/src/components/terminal/MultiTerminal.tsx) (line 148)
- **Severity:** Low
- **Description:** When terminal tab creation fails, the error is silently swallowed and `null` is returned. No user feedback.

```tsx
} catch {
    // TODO: Show error notification to user
    return null;
}
```

- **Action:** Add a toast or notification on failure.

---

#### IMPL-08: Shared UI Package Is Phase-1 Stub

- **File:** [`v3/console/packages/ui/src/index.ts`](../../../console/packages/ui/src/index.ts) (line 1)
- **Severity:** Low
- **Description:** The `@sindri-console/ui` package exports only `StatusBadge`. The comment indicates this is an intentional Phase 1 state.
- **Action:** Promote shared components from `apps/web` as they become reusable. No urgency.

---

#### IMPL-09: SSE Fan-Out Scaling Concern

- **File:** [`v3/console/apps/api/src/routes/logs.ts`](../../../console/apps/api/src/routes/logs.ts) (lines 168–170)
- **Severity:** Low
- **Description:** Each SSE subscriber creates its own Redis connection via `redisSub.duplicate()`. Works at low scale but will not scale for high fan-out.

```typescript
// NOTE (Phase 4 scaling): redisSub.duplicate() creates one ioredis connection per SSE
// subscriber. For high fan-out, replace with a single shared subscriber.
```

- **Action:** Refactor to shared `psubscribe` when scaling becomes necessary. Not a current issue.

---

### Tests — Ignored & Skipped

#### Rust `#[ignore]` Tests (5 total) — No Action Needed

All are correctly infrastructure-gated. Run via `cargo test -- --ignored` when infra is available.

| File                                                                                                               | Line | Test                               | Gating Requirement |
| ------------------------------------------------------------------------------------------------------------------ | ---- | ---------------------------------- | ------------------ |
| [`v3/crates/sindri-clusters/tests/k3d_integration.rs`](../../../crates/sindri-clusters/tests/k3d_integration.rs)   | 17   | `test_k3d_cluster_lifecycle`       | Docker + k3d       |
| [`v3/crates/sindri-clusters/tests/k3d_integration.rs`](../../../crates/sindri-clusters/tests/k3d_integration.rs)   | 79   | `test_k3d_cluster_with_registry`   | Docker + k3d       |
| [`v3/crates/sindri-clusters/tests/kind_integration.rs`](../../../crates/sindri-clusters/tests/kind_integration.rs) | 15   | `test_kind_cluster_lifecycle`      | Docker + kind      |
| [`v3/crates/sindri-clusters/tests/kind_integration.rs`](../../../crates/sindri-clusters/tests/kind_integration.rs) | 77   | `test_kind_cluster_already_exists` | Docker + kind      |
| [`v3/crates/sindri-packer/src/tests/aws_tests.rs`](../../../crates/sindri-packer/src/tests/aws_tests.rs)           | 76   | `test_aws_list_images`             | AWS credentials    |

#### Playwright E2E Conditional Skips (23 total) — No Action Needed

All use the pattern `if (count === 0) { test.skip(); return; }` — they skip on absent seed data, not incomplete code.

| File                                                                                                                         | Skip Count |
| ---------------------------------------------------------------------------------------------------------------------------- | ---------- |
| [`v3/console/apps/web/tests/e2e/security-dashboard.spec.ts`](../../../console/apps/web/tests/e2e/security-dashboard.spec.ts) | 7          |
| [`v3/console/apps/web/tests/e2e/drift-detection.spec.ts`](../../../console/apps/web/tests/e2e/drift-detection.spec.ts)       | 6          |
| [`v3/console/apps/web/tests/e2e/rbac-teams.spec.ts`](../../../console/apps/web/tests/e2e/rbac-teams.spec.ts)                 | 4          |
| [`v3/console/apps/web/tests/e2e/extension-admin.spec.ts`](../../../console/apps/web/tests/e2e/extension-admin.spec.ts)       | 4          |
| [`v3/console/apps/web/tests/e2e/alerting.spec.ts`](../../../console/apps/web/tests/e2e/alerting.spec.ts)                     | 1          |
| [`v3/console/apps/web/tests/e2e/instance-dashboard.spec.ts`](../../../console/apps/web/tests/e2e/instance-dashboard.spec.ts) | 1          |

**Minor gap:** `instance-dashboard.spec.ts:180` depends on `TEST_HIGH_CPU_INSTANCE_ID` env var — likely never set in CI, so the CPU threshold alert test case never runs.

---

## Part 2: Documentation

### Documentation High Priority

#### D-09: ADR README Index Is Severely Stale — ✅ RESOLVED

- **File:** [`v3/docs/architecture/adr/README.md`](../../architecture/adr/README.md) (lines 110, 157)
- **Category:** Documentation-Stale
- **Description:** The index states "Total ADRs: 37" and "next: 038" but the filesystem contains 45 ADR files (001–037, 038, 040, 041, 042, two competing 043 files, 044, and 045). ADRs 038–045 are absent from the index table and By-Phase sections. ADR-039 is skipped in the numbering sequence.
- **Action:** Recount, re-index, and add all missing ADR entries to the README.
- **Resolution:** Updated total to 45, next to 047, added all missing ADRs (038, 040–046) to Quick Reference table and By-Phase sections. Added new "Extension Operations" phase section.

---

#### D-10: Duplicate ADR-043 Numbering — ✅ RESOLVED

- **Files:**
  - [`v3/docs/architecture/adr/046-event-driven-status.md`](../../architecture/adr/046-event-driven-status.md) _(renumbered from 043)_
  - [`v3/docs/architecture/adr/043-shell-startup-optimization.md`](../../architecture/adr/043-shell-startup-optimization.md)
- **Category:** Documentation-Stale
- **Description:** Two files share the number 043, creating an ambiguous reference. One must be renumbered (likely to 046).
- **Action:** Renumber the newer ADR and update any cross-references.
- **Resolution:** Renamed `043-event-driven-status.md` → `046-event-driven-status.md` (via `git mv`). Updated title inside file and all 6 cross-references in CHANGELOG, CLI.md, ADR-044, ADR-045, and planning docs.

---

#### D-13: WORKFLOW_ARCHITECTURE.md Missing 20+ Workflows — ✅ RESOLVED

- **File:** [`.github/WORKFLOW_ARCHITECTURE.md`](../../../../.github/WORKFLOW_ARCHITECTURE.md) (lines 26–42)
- **Category:** Documentation-Stale
- **Description:** The workflow architecture document lists only a subset of existing workflows. The following are completely absent:
  - `console-agent-ci.yml`, `console-agent-release.yml`, `console-agent-test.yml`
  - `console-makefile-ci.yml`
  - `integration-test-providers.yml`
  - `v3-discover-extensions.yml`, `v3-matrix-generator.yml`
  - `v3-packer-build.yml`, `v3-packer-test.yml`
  - `v3-pre-release-test.yml`
  - `v3-provider-devpod.yml`, `v3-provider-docker.yml`, `v3-provider-fly.yml`
  - `v3-provider-k3d.yml`, `v3-provider-northflank.yml`, `v3-provider-packer.yml`
  - `v3-provider-runpod.yml`
  - `v3-test-profiles.yml`
  - `build-base-image.yml`, `cleanup-container-images.yml`
- **Action:** Update the architecture doc to include all production workflows with descriptions and trigger conditions.
- **Resolution:** Rewrote WORKFLOW_ARCHITECTURE.md with all 36 workflows. Added dedicated sections for Console, Infrastructure, Extension Helpers, Packer, and Provider workflows. Added complete inventory table.

---

### Documentation Medium Priority

#### D-01: BACKUP_RESTORE.md WIP Features — ✅ RESOLVED

- **File:** [`v3/docs/BACKUP_RESTORE.md`](../../BACKUP_RESTORE.md) (lines 594–595)
- **Category:** Documentation-Stale
- **Description:** Two features marked `WIP` in the "Current Limitations" table — S3 Backup Destination and HTTPS Download Source — are not implemented. The `todo-tracker.md` confirms 8 low-priority TODOs remain open for the backup/restore system.
- **Action:** Add GitHub issue links for tracking, or update the table to reflect actual status.
- **Resolution:** Updated both entries from "WIP" to "Planned — tracked in backlog".

---

#### D-04: Active Planning Doc Has Unresolved TODO — ✅ RESOLVED

- **File:** [`v3/docs/planning/active/event-driven-status-architecture.md`](event-driven-status-architecture.md) (line 745)
- **Category:** Documentation-TODO
- **Description:** Contains an unresolved `// TODO: Collect from executor` in the architectural reference implementation code snippet.
- **Action:** Complete the code snippet or annotate it as intentionally illustrative.
- **Resolution:** Replaced `// TODO: Collect from executor` with `// Collect execution metrics from the executor`.

---

#### D-05: Testing Strategy Documents Missing Test Files — ✅ RESOLVED

- **File:** [`v3/docs/planning/active/overarching-testing-strategy.md`](overarching-testing-strategy.md) (lines 67, 77–78, 199)
- **Category:** Documentation-Gap
- **Description:** Multiple outstanding gaps:
  - Line 67: "Provider-based tests: Partial — Docker tested, others TBD"
  - Lines 77–78: `removal_lifecycle_tests.rs` and `upgrade_lifecycle_tests.rs` are called out as "not yet created"
  - Line 199: "NO security scan integration — OpenSCAP planned but not implemented"
- **Action:** Create the missing test files or update the strategy doc to reflect actual scope.
- **Resolution:** Updated all three gaps with "tracked for future implementation" annotations to clarify backlog status.

---

#### D-06: Incomplete Document in `complete/` Folder — ✅ RESOLVED

- **File:** [`v3/docs/planning/active/v3-dockerfile-validation-checklist.md`](v3-dockerfile-validation-checklist.md) (lines 422, 424–425, 529–531)
- **Category:** Documentation-Stale
- **Description:** This document is in the `complete/` folder but contains multiple unfilled `[ ] TBD` cells in performance comparison tables and empty sign-off fields (Reviewer, QA Lead, Release Manager).
- **Action:** Move back to `active/` or fill in the remaining fields.
- **Resolution:** Moved from `complete/` to `active/` via `git mv`.

---

#### D-11: ADR Index Status Mismatches — ✅ RESOLVED

- **File:** [`v3/docs/architecture/adr/README.md`](../../architecture/adr/README.md) (lines 23, 24, 28)
- **Category:** Documentation-Stale
- **Description:** ADR-015, ADR-016, and ADR-020 are listed in the index with status `Proposed`, but the actual ADR files have `**Status**: Accepted`.
- **Action:** Update the index to reflect the correct status.
- **Resolution:** Updated all three entries in the Quick Reference table from `Proposed` to `Accepted`.

---

#### D-12: 10 of 12 Rust Crates Lack README Files — ✅ RESOLVED

- **Category:** Documentation-Gap
- **Description:** Only `sindri-projects` and `sindri-update` have README files. The following 10 crates have none:
  - [`v3/crates/sindri/`](../../../crates/sindri/)
  - [`v3/crates/sindri-backup/`](../../../crates/sindri-backup/)
  - [`v3/crates/sindri-clusters/`](../../../crates/sindri-clusters/)
  - [`v3/crates/sindri-core/`](../../../crates/sindri-core/)
  - [`v3/crates/sindri-doctor/`](../../../crates/sindri-doctor/)
  - [`v3/crates/sindri-extensions/`](../../../crates/sindri-extensions/)
  - [`v3/crates/sindri-image/`](../../../crates/sindri-image/)
  - [`v3/crates/sindri-packer/`](../../../crates/sindri-packer/)
  - [`v3/crates/sindri-providers/`](../../../crates/sindri-providers/)
  - [`v3/crates/sindri-secrets/`](../../../crates/sindri-secrets/)
- **Action:** Add minimal README files with crate purpose, public API overview, and usage examples.
- **Resolution:** Created README.md for all 10 crates with Features, Modules, and Usage sections based on source analysis. All 12 crates now have READMEs.

---

#### D-14: Distribution Plans Stalled — ✅ RESOLVED

- **Files:**
  - [`v3/docs/planning/active/homebrew-distribution.md`](homebrew-distribution.md) — Status: "Planned — pending release pipeline integration"
  - [`v3/docs/planning/active/mise-distribution.md`](mise-distribution.md) — Status: "Planned — pending release pipeline integration"
- **Category:** Documentation-Gap
- **Description:** Both plans are marked ready but no corresponding workflow or CI job exists in `release-v3.yml`.
- **Action:** Either begin implementation or update plan status to reflect actual timeline.
- **Resolution:** Updated status in both plan documents from "Ready for Implementation" to "Planned — pending release pipeline integration".

---

### Documentation Low Priority

#### D-02: Commented-Out Link to Non-Existent Document

- **File:** [`v3/docs/MAINTAINER_GUIDE.md`](../../MAINTAINER_GUIDE.md) (line 831)
- **Category:** Documentation-TODO
- **Description:** `<!-- [Architecture: Docker Builds](architecture/docker-build-architecture.md) (TODO: Create this document) -->`
- **Action:** Create the document or remove the commented-out link.

---

#### D-03: TBD Decision in ADR-034

- **File:** [`v3/docs/architecture/adr/034-image-handling-consistency-framework.md`](../../architecture/adr/034-image-handling-consistency-framework.md) (line 107)
- **Category:** Documentation-TODO
- **Description:** E2B provider row in the decision table is marked `TBD (research)`.
- **Action:** Complete the research and fill in the decision.

---

#### D-07: Open TODOs in "Complete" Tracker

- **File:** [`v3/docs/planning/complete/todo-tracker.md`](../complete/todo-tracker.md) (lines 461–477)
- **Category:** Documentation-Stale
- **Description:** 8 low-priority `[ ]` TODOs remain open for backup/restore in a document filed under `complete/`.
- **Action:** Move open items to an active tracker or close them.

---

#### D-08: Open Items in "Complete" Migration Plan

- **File:** [`v3/docs/planning/complete/rust-cli-migration-v3.md`](../complete/rust-cli-migration-v3.md) (lines 726, 2395)
- **Category:** Documentation-Stale
- **Description:** Two unresolved items: `// TODO: parse actual start time` in a code snippet, and "Code Signing: TBD" which is tracked separately in `planning/active/macos-code-signing.md`.
- **Action:** Clean up the code snippet; add a cross-reference to the active signing plan.

---

## Part 3: GitHub Actions & Workflows

### CI High Priority

#### CI-01: Artifact Action Version Mismatch — ✅ RESOLVED

- **Category:** CI-Stale
- **Description:** `upload-artifact@v6` is used for uploads, but downloads are split between `@v4` and `@v7`. Artifacts uploaded with v6 may not be downloadable with v4 due to backend changes.

**`download-artifact@v4` instances (should be `@v7`):**

| File                                                                                                       | Lines         |
| ---------------------------------------------------------------------------------------------------------- | ------------- |
| [`.github/workflows/v3-provider-k3d.yml`](../../../../.github/workflows/v3-provider-k3d.yml)               | 118, 268, 292 |
| [`.github/workflows/v3-provider-docker.yml`](../../../../.github/workflows/v3-provider-docker.yml)         | 83, 210       |
| [`.github/workflows/v3-provider-northflank.yml`](../../../../.github/workflows/v3-provider-northflank.yml) | 277           |
| [`.github/workflows/v3-packer-test.yml`](../../../../.github/workflows/v3-packer-test.yml)                 | 65, 359       |
| [`.github/workflows/v3-provider-runpod.yml`](../../../../.github/workflows/v3-provider-runpod.yml)         | 252           |
| [`.github/workflows/v3-packer-build.yml`](../../../../.github/workflows/v3-packer-build.yml)               | 397           |
| [`.github/workflows/v3-provider-devpod.yml`](../../../../.github/workflows/v3-provider-devpod.yml)         | 248           |
| [`.github/workflows/v3-provider-fly.yml`](../../../../.github/workflows/v3-provider-fly.yml)               | 218           |
| [`.github/workflows/v3-provider-packer.yml`](../../../../.github/workflows/v3-provider-packer.yml)         | 461           |

**Mixed version in composite action:**
| File | Line | Version |
|------|------|---------|
| [`.github/actions/shared/deploy-provider/action.yml`](../../../../.github/actions/shared/deploy-provider/action.yml) | 124 | `download-artifact@v6` |

- **Action:** Standardize all to `upload-artifact@v6` / `download-artifact@v7`.
- **Resolution:** Updated all 14 instances of `download-artifact@v4` across 9 workflow files to `@v7`. Updated the composite action `deploy-provider/action.yml` from `@v6` to `@v7`.

---

#### CI-02: Unpinned `@main` Action References — ✅ RESOLVED

- **Category:** CI-Deprecated
- **Description:** Three actions reference `@main` — a mutable, unpinned branch. Supply chain risk if upstream pushes breaking or malicious commits.

| File                                                                                               | Lines                       | Action                           |
| -------------------------------------------------------------------------------------------------- | --------------------------- | -------------------------------- |
| [`.github/workflows/ci-v3.yml`](../../../../.github/workflows/ci-v3.yml)                           | 196, 492                    | `cargo-bins/cargo-binstall@main` |
| [`.github/workflows/v3-packer-build.yml`](../../../../.github/workflows/v3-packer-build.yml)       | 59, 125, 179, 237, 306, 366 | `hashicorp/setup-packer@main`    |
| [`.github/workflows/v3-provider-packer.yml`](../../../../.github/workflows/v3-provider-packer.yml) | 88                          | `hashicorp/setup-packer@main`    |

- **Action:** Pin to specific version tags or commit SHAs.
- **Resolution:** Pinned `cargo-bins/cargo-binstall@main` → `@v1.12.3` (2 occurrences in `ci-v3.yml`). Pinned `hashicorp/setup-packer@main` → `@v3.1.0` (6 occurrences in `v3-packer-build.yml`, 1 in `v3-provider-packer.yml`).

---

### CI Medium Priority

#### CI-04: Outdated SBOM Action — ✅ RESOLVED (release-v3.yml only)

- **Files:**
  - [`.github/workflows/release-v2.yml`](../../../../.github/workflows/release-v2.yml) (line 420) — left as-is (intentional)
  - [`.github/workflows/release-v3.yml`](../../../../.github/workflows/release-v3.yml) (line 476)
- **Category:** CI-Deprecated
- **Description:** `anchore/sbom-action@v0` — very early version tag. Current releases are v0.18+. The `@v0` floating tag may not resolve correctly.
- **Action:** Pin to the latest specific release (e.g., `@v0.18.3`).
- **Resolution:** Pinned `release-v3.yml` to `anchore/sbom-action@v0.22.2`. `release-v2.yml` left unchanged (out of scope).

---

#### CI-05: Inconsistent Cosign Installer Versions — ⏭️ SKIPPED (intentional)

- **Files:**
  - [`.github/workflows/release-v2.yml`](../../../../.github/workflows/release-v2.yml) (lines 357, 417) — uses `@v3`
  - [`.github/workflows/ci-v3.yml`](../../../../.github/workflows/ci-v3.yml) (line 361) — uses `@v4.0.0`
  - [`.github/workflows/release-v3.yml`](../../../../.github/workflows/release-v3.yml) (lines 358, 462) — uses `@v4.0.0`
- **Category:** CI-Stale
- **Description:** `sigstore/cosign-installer` is at two different versions across release workflows.
- **Action:** Harmonize to `@v4.0.0` (or latest) in `release-v2.yml`.
- **Note:** Version difference is intentional — `release-v2.yml` deliberately kept at `@v3`. No action taken.

---

#### CI-06: `actions/cache` Version Split — ✅ RESOLVED

- **`@v4` (should be `@v5`):**
  - [`.github/workflows/console-makefile-ci.yml`](../../../../.github/workflows/console-makefile-ci.yml) (line 150)
  - [`.github/actions/shared/build-image/action.yml`](../../../../.github/actions/shared/build-image/action.yml) (line 56)
- **`@v5` (current):**
  - [`.github/workflows/check-links.yml`](../../../../.github/workflows/check-links.yml) (lines 31, 74)
  - [`.github/workflows/release-v3.yml`](../../../../.github/workflows/release-v3.yml) (line 124)
  - [`.github/workflows/v3-test-profiles.yml`](../../../../.github/workflows/v3-test-profiles.yml) (line 76)
  - [`.github/actions/v3/setup-rust/action.yml`](../../../../.github/actions/v3/setup-rust/action.yml) (line 54)
- **Action:** Upgrade v4 instances to v5.
- **Resolution:** Upgraded `console-makefile-ci.yml` and `build-image/action.yml` from `@v4` to `@v5`.

---

#### CI-07: `actions/checkout` Version Split — ✅ RESOLVED

- **`@v4` (should be `@v6`):**
  - [`.github/workflows/console-makefile-ci.yml`](../../../../.github/workflows/console-makefile-ci.yml) (lines 34, 67, 107, 133, 182)
- **`@v6`:** Used in all other workflow files.
- **Action:** Upgrade to `@v6`.
- **Resolution:** All 5 occurrences in `console-makefile-ci.yml` updated from `@v4` to `@v6`.

---

#### CI-08: `actions/setup-node` Version Split — ✅ RESOLVED

- **`@v4` (should be `@v6`):**
  - [`.github/workflows/console-makefile-ci.yml`](../../../../.github/workflows/console-makefile-ci.yml) (lines 43, 136)
- **`@v6`:**
  - [`.github/workflows/validate-markdown.yml`](../../../../.github/workflows/validate-markdown.yml) (lines 28, 46, 69)
  - [`.github/workflows/validate-yaml.yml`](../../../../.github/workflows/validate-yaml.yml) (lines 94, 125)
- **Action:** Upgrade to `@v6`.
- **Resolution:** Both occurrences in `console-makefile-ci.yml` updated from `@v4` to `@v6`.

---

#### CI-14: Raw TODO in Generated Migration Guide — ✅ RESOLVED

- **File:** [`.github/scripts/generate-migration-guide.sh`](../../../../.github/scripts/generate-migration-guide.sh) (line 131)
- **Category:** CI-TODO
- **Description:** The script emits `**TODO**: Add migration instructions, code examples (before/after), and user impact.` into every generated migration guide's "Breaking Changes" section. No automated gate catches unfilled TODOs.
- **Action:** Add a CI step that fails if a generated guide still contains raw TODO strings, or change the template to use a more descriptive placeholder.
- **Resolution:** Replaced `**TODO**: Add migration instructions...` with `> **Action Required**: Fill in migration instructions, code examples (before/after), and user impact for each breaking change listed above.`

---

#### CI-15: Missing Distribution Jobs in Release Pipeline — ✅ RESOLVED (docs updated)

- **File:** [`.github/workflows/release-v3.yml`](../../../../.github/workflows/release-v3.yml)
- **Category:** CI-Gap
- **Description:** No Homebrew formula update job or Mise registry PR job exists, despite both distribution plans being marked "Ready for Implementation" in active planning docs.
- **Action:** Implement distribution jobs or update planning docs to reflect actual timeline.
- **Resolution:** Planning docs updated to reflect actual status (see D-14). CI jobs remain unimplemented — tracked in planning docs as backlog items.

---

#### CI-16: No macOS Code Signing — ⏭️ SKIPPED (excluded from scope)

- **File:** [`.github/workflows/release-v3.yml`](../../../../.github/workflows/release-v3.yml)
- **Category:** CI-Gap
- **Description:** No macOS code-signing or notarization step exists. Unsigned macOS binaries are shipped with no user warning in release notes. Blocked by Apple Developer Account (see `planning/active/macos-code-signing.md`).
- **Action:** Add a release note warning about unsigned binaries. Implement signing when account is available.

---

#### CI-18: `console-makefile-ci.yml` Entirely Behind — ✅ RESOLVED

- **File:** [`.github/workflows/console-makefile-ci.yml`](../../../../.github/workflows/console-makefile-ci.yml)
- **Category:** CI-Stale
- **Description:** This is the only workflow file using `actions/checkout@v4`, `actions/setup-node@v4`, and `actions/cache@v4`. Every other file in the repo has moved to v5/v6. This file was created separately and never harmonized.
- **Action:** Update all action references to match the rest of the repository (CI-06, CI-07, CI-08 combined).
- **Resolution:** All action versions in `console-makefile-ci.yml` harmonized: `checkout@v4`→`@v6` (5×), `setup-node@v4`→`@v6` (2×), `cache@v4`→`@v5` (1×). Also fixed `build-image/action.yml` `cache@v4`→`@v5`.

---

### CI Low Priority

#### CI-03: `dtolnay/rust-toolchain@stable` — Unpinned but Conventional

- **Files:** 22 locations across 11 files including [`ci-v3.yml`](../../../../.github/workflows/ci-v3.yml), [`release-v3.yml`](../../../../.github/workflows/release-v3.yml), [`v3-packer-build.yml`](../../../../.github/workflows/v3-packer-build.yml), provider workflows, and [`actions/v3/setup-rust/action.yml`](../../../../.github/actions/v3/setup-rust/action.yml).
- **Description:** `@stable` is the Rust community convention for this action (resolves to latest stable toolchain). Acceptable but noted for awareness.
- **Action:** No immediate action. Consider pinning if reproducibility becomes a concern.

---

#### CI-09: sccache-action Pinned to v0.0.9

- **File:** [`.github/workflows/ci-v3.yml`](../../../../.github/workflows/ci-v3.yml) (line 139)
- **Description:** `mozilla-actions/sccache-action@v0.0.9` — explicitly pinned, not floating. Check periodically for newer releases.

---

#### CI-10: `helm/kind-action@v1` — Floating Major Version

- **File:** [`.github/workflows/ci-v3.yml`](../../../../.github/workflows/ci-v3.yml) (line 596)
- **Action:** Pin to specific minor version for reproducibility.

---

#### CI-11: `AbsaOSS/k3d-action@v2` — Floating Major Version

- **File:** [`.github/workflows/ci-v3.yml`](../../../../.github/workflows/ci-v3.yml) (line 641)
- **Action:** Pin to specific minor version.

---

#### CI-12: `ruby/setup-ruby@v1` — Floating Major Version

- **File:** [`.github/workflows/v3-packer-test.yml`](../../../../.github/workflows/v3-packer-test.yml) (lines 54, 132, 193, 256, 311)
- **Action:** Pin to specific minor version.

---

#### CI-13: `actions/attest-build-provenance@v3` — Verify Validity

- **Files:**
  - [`.github/workflows/ci-v3.yml`](../../../../.github/workflows/ci-v3.yml) (line 372)
  - [`.github/workflows/release-v3.yml`](../../../../.github/workflows/release-v3.yml) (line 439)
- **Description:** The stable series is v2.x. Confirm that `@v3` is a valid published release and not a pre-release or nonexistent tag.

---

#### CI-17: Missing Dependabot Groups

- **File:** [`.github/dependabot.yml`](../../../../.github/dependabot.yml)
- **Description:** No Cargo groups for `aws-sdk-*` crates or Docker-related dependencies. Adding groups would reduce PR noise.

---

#### CI-19: VERSION_PLACEHOLDER / REPO_PLACEHOLDER in Heredocs

- **Files:**
  - [`.github/workflows/release-v2.yml`](../../../../.github/workflows/release-v2.yml) (lines 473–557)
  - [`.github/workflows/release-v3.yml`](../../../../.github/workflows/release-v3.yml) (lines 695–845)
- **Description:** These are intentional runtime-replaced template tokens (`sed`-substituted during execution). They appear as false positives in codebase searches. Consider adding a comment explaining the pattern.

---

## Prioritized Recommendations

### Immediate (this week)

1. ~~**CI-01**: Standardize artifact actions — `upload-artifact@v6` / `download-artifact@v7`. The v4 downloads in 9 provider workflows risk silent failures.~~ ✅ DONE
2. ~~**CI-02**: Pin `cargo-binstall` and `setup-packer` to specific tags. `@main` is an unacceptable supply chain risk.~~ ✅ DONE
3. ~~**D-09 + D-10**: Fix ADR README index (recount to 45, add missing entries, fix statuses) and renumber duplicate ADR-043.~~ ✅ DONE

### Next Sprint

4. ~~**CI-18 (CI-06 + CI-07 + CI-08)**: Harmonize `console-makefile-ci.yml` to match repo-wide action versions.~~ ✅ DONE
5. ~~**D-13**: Update WORKFLOW_ARCHITECTURE.md — 20+ undocumented workflows is a serious onboarding gap.~~ ✅ DONE
6. **IMPL-01/02/03**: Triage the 3 critical console stubs (evaluateSecurity, evaluateCost, sendEmail). Decide: implement, add UI warnings, or track as Phase 2 scope with GitHub issues.
7. **CI-05**: Harmonize cosign-installer to `@v4.0.0` in release-v2.yml. _(intentionally skipped — version difference is deliberate)_
8. ~~**D-11**: Correct ADR index statuses for ADR-015, 016, 020.~~ ✅ DONE

### Backlog

9. ~~**D-12**: Add minimal README files to the 10 crates lacking them.~~ ✅ DONE
10. **CI-10/11/12**: Pin floating major version actions for kind, k3d, and ruby.
11. ~~**D-06**: Move `v3-dockerfile-validation-checklist.md` back to `active/` or complete it.~~ ✅ DONE
12. ~~**D-14 / CI-15**: Decide on Homebrew/Mise distribution — start or update timeline.~~ ✅ DONE (docs updated to reflect backlog status)
13. ~~**CI-04**: Upgrade `anchore/sbom-action@v0` to latest pinned release.~~ ✅ DONE (release-v3.yml pinned to v0.22.2)
14. **CI-13**: Verify `attest-build-provenance@v3` is a valid release.
15. **IMPL-04/05/06**: Wire up instance navigation, fleet scan button, and API key endpoint.
