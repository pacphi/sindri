# Dead Code Audit — February 2026

**Date:** 2026-02-20
**Branch:** `feature/sindri-administrator`
**Scope:** Dead code, unused exports, unused dependencies, and suppressed warnings across all three language ecosystems in `v3/`

---

## Table of Contents

- [Executive Summary](#executive-summary)
- [Tools Used](#tools-used)
- [Rust Findings](#rust-findings)
  - [Compiler Warnings](#rust-compiler-warnings)
  - [Suppressed Dead Code (`#[allow(dead_code)]`)](#rust-suppressed-dead-code)
  - [Suppressed Unused Imports](#rust-suppressed-unused-imports)
- [TypeScript Findings](#typescript-findings)
  - [Unused Local Variables (tsc TS6133)](#ts-unused-local-variables)
  - [Missing Module References (tsc TS2307)](#ts-missing-module-references)
  - [Unused Files (knip)](#ts-unused-files)
  - [Unused Dependencies (knip)](#ts-unused-dependencies)
  - [Unused Exports (knip)](#ts-unused-exports)
  - [Unused Exported Types (knip)](#unused-exported-types-93)
- [Go Findings](#go-findings)
- [Prioritized Recommendations](#prioritized-recommendations)

---

## Executive Summary

| Language   | Tool                                        | Findings                              | Severity |
| ---------- | ------------------------------------------- | ------------------------------------- | -------- |
| Rust       | `cargo check -W dead_code,unused-*`         | **0 warnings**                        | Clean    |
| Rust       | `#[allow(dead_code)]` annotations           | **34 suppressed items**               | Review   |
| Rust       | `#[allow(unused_imports)]`                  | **1 suppressed import**               | Low      |
| TypeScript | `tsc --noUnusedLocals --noUnusedParameters` | **6 unused variables**                | Medium   |
| TypeScript | `tsc` (missing modules)                     | **2 broken imports**                  | High     |
| TypeScript | `knip` (unused files)                       | **52 unused files**                   | High     |
| TypeScript | `knip` (unused dependencies)                | **10 unused deps + 4 unused devDeps** | Medium   |
| TypeScript | `knip` (unused exports)                     | **145 unused exports**                | Medium   |
| TypeScript | `knip` (unused exported types)              | **93 unused types**                   | Low      |
| Go         | `go vet ./...`                              | **0 findings**                        | Clean    |
| Go         | `deadcode ./...`                            | **0 findings**                        | Clean    |
| Go         | `staticcheck ./...`                         | **0 findings**                        | Clean    |

**Bottom line:** Rust and Go are clean. TypeScript has significant dead code accumulation — 52 unused files, 14 unused package dependencies, and 238 unused exports/types. The TypeScript console is the only area requiring cleanup action.

---

## Tools Used

| Language   | Tool                                                             | Version         | Command                                                                                                                      |
| ---------- | ---------------------------------------------------------------- | --------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Rust       | `cargo check`                                                    | rustc stable    | `RUSTFLAGS="-W dead_code -W unused-imports -W unused-variables -W unused-mut -W unused-assignments" cargo check --workspace` |
| Rust       | Manual audit                                                     | —               | Grep for `#[allow(dead_code)]` and `#[allow(unused` annotations                                                              |
| TypeScript | `tsc`                                                            | Local workspace | `tsc --noEmit --noUnusedLocals --noUnusedParameters -p <tsconfig>`                                                           |
| TypeScript | [`ts-prune`](https://github.com/nadeesha/ts-prune)               | 0.10.3          | `npx ts-prune -p <tsconfig>`                                                                                                 |
| TypeScript | [`knip`](https://github.com/webpro/knip)                         | 5.84.1          | `npx knip --no-progress`                                                                                                     |
| Go         | `go vet`                                                         | go 1.26.0       | `go vet ./...`                                                                                                               |
| Go         | [`deadcode`](https://pkg.go.dev/golang.org/x/tools/cmd/deadcode) | latest          | `deadcode ./...`                                                                                                             |
| Go         | [`staticcheck`](https://staticcheck.dev/)                        | 0.7.0           | `staticcheck ./...`                                                                                                          |

---

## Rust Findings

### Rust Compiler Warnings

```
RUSTFLAGS="-W dead_code -W unused-imports -W unused-variables -W unused-mut -W unused-assignments" \
  cargo check --workspace --message-format=short
```

**Result: 0 warnings.** The entire Rust workspace compiles cleanly with all unused-code warnings enabled. No dead code, no unused imports, no unused variables, no unused mut bindings.

---

### Rust Suppressed Dead Code

34 instances of `#[allow(dead_code)]` exist across the workspace. These are items the compiler _would_ flag as dead code, but warnings have been explicitly suppressed. Grouped by location:

#### Production Code (4 items — review recommended)

| File                                                                                                        | Line          | Context                                                         |
| ----------------------------------------------------------------------------------------------------------- | ------------- | --------------------------------------------------------------- |
| [`crates/sindri-providers/src/northflank.rs`](../../../crates/sindri-providers/src/northflank.rs)           | 46            | Struct field in provider implementation                         |
| [`crates/sindri-providers/src/runpod.rs`](../../../crates/sindri-providers/src/runpod.rs)                   | 46            | Struct field in provider implementation                         |
| [`crates/sindri-providers/src/runpod.rs`](../../../crates/sindri-providers/src/runpod.rs)                   | 471, 473, 475 | Three struct fields                                             |
| [`crates/sindri-update/src/download.rs`](../../../crates/sindri-update/src/download.rs)                     | 132           | Field in download module                                        |
| [`crates/sindri/src/commands/project/template.rs`](../../../crates/sindri/src/commands/project/template.rs) | 23            | Comment: "Used for future dependency installation enhancements" |
| [`crates/sindri-core/src/templates/context.rs`](../../../crates/sindri-core/src/templates/context.rs)       | 18            | Field in template context                                       |

**Assessment:** The production `#[allow(dead_code)]` annotations on struct fields likely exist because the fields are deserialized from JSON/YAML but not yet used in logic. The `template.rs` annotation explicitly notes future use. These should be reviewed — if fields are genuinely unused, remove them; if they're populated for API completeness, keep the annotation.

#### Test Code (28 items — acceptable)

| File                                                                                                          | Count | Context                                |
| ------------------------------------------------------------------------------------------------------------- | ----- | -------------------------------------- |
| [`crates/sindri-packer/tests/common/mock_cloud.rs`](../../../crates/sindri-packer/tests/common/mock_cloud.rs) | 6     | Mock struct fields and builder methods |
| [`crates/sindri-packer/tests/common/assertions.rs`](../../../crates/sindri-packer/tests/common/assertions.rs) | 3     | Assertion helper functions             |
| [`crates/sindri-providers/tests/common/mod.rs`](../../../crates/sindri-providers/tests/common/mod.rs)         | 18    | Test fixtures, builders, mock helpers  |
| [`crates/sindri/tests/bom_cli_tests.rs`](../../../crates/sindri/tests/bom_cli_tests.rs)                       | 1     | Test helper                            |

**Assessment:** These are test utility code shared across multiple test files. Rust's dead code analysis for test code has a known limitation — shared test modules need `#[allow(dead_code)]` when not all helpers are used by every test binary. **No action needed.**

---

### Rust Suppressed Unused Imports

| File                                                                                          | Line | Context                                      |
| --------------------------------------------------------------------------------------------- | ---- | -------------------------------------------- |
| [`crates/sindri-secrets/src/s3/backend.rs`](../../../crates/sindri-secrets/src/s3/backend.rs) | 411  | `#[allow(unused_imports)]` — single instance |

**Assessment:** Likely a conditional import. Low priority — verify if import is actually needed.

---

## TypeScript Findings

### TS Unused Local Variables

Detected by `tsc --noEmit --noUnusedLocals --noUnusedParameters` on `apps/api/tsconfig.json`:

| File                                                                                                                                | Line | Variable           | Error                           |
| ----------------------------------------------------------------------------------------------------------------------------------- | ---- | ------------------ | ------------------------------- |
| [`console/apps/api/src/services/alerts/dispatcher.service.ts`](../../../console/apps/api/src/services/alerts/dispatcher.service.ts) | 268  | `_text`            | TS6133: declared but never read |
| [`console/apps/api/src/services/alerts/dispatcher.service.ts`](../../../console/apps/api/src/services/alerts/dispatcher.service.ts) | 292  | `payload`          | TS6133: declared but never read |
| [`console/apps/api/src/services/alerts/evaluator.service.ts`](../../../console/apps/api/src/services/alerts/evaluator.service.ts)   | 37   | `_ruleInstanceIds` | TS6133: declared but never read |
| [`console/apps/api/src/services/alerts/evaluator.service.ts`](../../../console/apps/api/src/services/alerts/evaluator.service.ts)   | 48   | `_instanceMap`     | TS6133: declared but never read |
| [`console/apps/api/src/services/costs/cost.service.ts`](../../../console/apps/api/src/services/costs/cost.service.ts)               | 50   | `_periodBounds`    | TS6133: declared but never read |
| [`console/apps/api/src/services/costs/rightsizing.service.ts`](../../../console/apps/api/src/services/costs/rightsizing.service.ts) | 161  | `_currentUsdMo`    | TS6133: declared but never read |

**Assessment:** The underscore-prefixed variables (`_text`, `_ruleInstanceIds`, etc.) suggest intentional placeholders for future implementation. `payload` at line 292 has no underscore prefix and should either be used or prefixed with `_`. These align with the stub functions identified in `todo-audit-2026-02.md` (IMPL-01/02/03).

The `apps/web`, `packages/shared`, and `packages/ui` projects passed with **0 unused variable warnings**.

---

### TS Missing Module References

Detected by `tsc --noEmit --noUnusedLocals --noUnusedParameters` on `apps/web/tsconfig.json`:

| File                                                                                                                                            | Line | Error                                          |
| ----------------------------------------------------------------------------------------------------------------------------------------------- | ---- | ---------------------------------------------- |
| [`console/apps/web/src/components/instances/InstanceDetailPage.tsx`](../../../console/apps/web/src/components/instances/InstanceDetailPage.tsx) | 8    | TS2307: Cannot find module `@/components/logs` |
| [`console/apps/web/src/pages/LogsPage.tsx`](../../../console/apps/web/src/pages/LogsPage.tsx)                                                   | 1    | TS2307: Cannot find module `@/components/logs` |

**Assessment: HIGH PRIORITY.** The `@/components/logs` directory does not exist on disk. Two files import from a non-existent module. These imports will fail at build time with type-checking enabled. Either:

1. The logs component module was deleted without updating its consumers, or
2. It was never created but imports were added in advance

**Action:** Create `apps/web/src/components/logs/` with the expected exports, or remove the broken imports from `InstanceDetailPage.tsx` and `LogsPage.tsx`.

---

### TS Unused Files

Detected by [`knip`](https://github.com/webpro/knip) — files with no inbound imports from the dependency graph:

#### API App — 14 unused files

| #   | File                                                                                                                | Category                 |
| --- | ------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| 1   | [`console/apps/api/src/routes/lifecycle.ts`](../../../console/apps/api/src/routes/lifecycle.ts)                     | Route (never registered) |
| 2   | [`console/apps/api/src/services/alerts/index.ts`](../../../console/apps/api/src/services/alerts/index.ts)           | Barrel re-export         |
| 3   | [`console/apps/api/src/services/costs/index.ts`](../../../console/apps/api/src/services/costs/index.ts)             | Barrel re-export         |
| 4   | [`console/apps/api/src/services/extensions/index.ts`](../../../console/apps/api/src/services/extensions/index.ts)   | Barrel re-export         |
| 5   | [`console/apps/api/src/services/lifecycle.ts`](../../../console/apps/api/src/services/lifecycle.ts)                 | Service module           |
| 6   | [`console/apps/api/src/websocket/handlers.ts`](../../../console/apps/api/src/websocket/handlers.ts)                 | WebSocket handlers       |
| 7   | [`console/apps/api/src/websocket/redis.ts`](../../../console/apps/api/src/websocket/redis.ts)                       | Redis pubsub helpers     |
| 8   | [`console/apps/api/src/websocket/server.ts`](../../../console/apps/api/src/websocket/server.ts)                     | WebSocket server         |
| 9   | [`console/apps/api/tests/agent-registration.test.ts`](../../../console/apps/api/tests/agent-registration.test.ts)   | Test file                |
| 10  | [`console/apps/api/tests/auth-middleware.test.ts`](../../../console/apps/api/tests/auth-middleware.test.ts)         | Test file                |
| 11  | [`console/apps/api/tests/database-operations.test.ts`](../../../console/apps/api/tests/database-operations.test.ts) | Test file                |
| 12  | [`console/apps/api/tests/heartbeat-metrics.test.ts`](../../../console/apps/api/tests/heartbeat-metrics.test.ts)     | Test file                |
| 13  | [`console/apps/api/tests/metrics.test.ts`](../../../console/apps/api/tests/metrics.test.ts)                         | Test file                |
| 14  | [`console/apps/api/tests/terminal-session.test.ts`](../../../console/apps/api/tests/terminal-session.test.ts)       | Test file                |

**Note:** Test files appearing as "unused" is expected — knip's default config may not resolve test entry points. Items 1–8 are genuine dead code (modules not imported anywhere in the application).

#### Web App — 37 unused files

| #     | File                                                                                                                                                            | Category         |
| ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- |
| 1     | [`console/apps/web/src/api/logs.ts`](../../../console/apps/web/src/api/logs.ts)                                                                                 | API client       |
| 2     | [`console/apps/web/src/components/costs/index.ts`](../../../console/apps/web/src/components/costs/index.ts)                                                     | Barrel re-export |
| 3     | [`console/apps/web/src/components/dashboard/DashboardPage.tsx`](../../../console/apps/web/src/components/dashboard/DashboardPage.tsx)                           | Page component   |
| 4     | [`console/apps/web/src/components/deployment/index.ts`](../../../console/apps/web/src/components/deployment/index.ts)                                           | Barrel re-export |
| 5     | [`console/apps/web/src/components/deployment/templates/index.ts`](../../../console/apps/web/src/components/deployment/templates/index.ts)                       | Barrel re-export |
| 6     | [`console/apps/web/src/components/deployment/templates/TemplateCard.tsx`](../../../console/apps/web/src/components/deployment/templates/TemplateCard.tsx)       | Component        |
| 7     | [`console/apps/web/src/components/deployment/templates/templateData.ts`](../../../console/apps/web/src/components/deployment/templates/templateData.ts)         | Data             |
| 8     | [`console/apps/web/src/components/deployment/templates/TemplateDetail.tsx`](../../../console/apps/web/src/components/deployment/templates/TemplateDetail.tsx)   | Component        |
| 9     | [`console/apps/web/src/components/deployment/templates/TemplateFilter.tsx`](../../../console/apps/web/src/components/deployment/templates/TemplateFilter.tsx)   | Component        |
| 10    | [`console/apps/web/src/components/deployment/templates/TemplateGallery.tsx`](../../../console/apps/web/src/components/deployment/templates/TemplateGallery.tsx) | Component        |
| 11    | [`console/apps/web/src/components/extensions/CustomExtensionUpload.tsx`](../../../console/apps/web/src/components/extensions/CustomExtensionUpload.tsx)         | Component        |
| 12    | [`console/apps/web/src/components/extensions/ExtensionDependencyGraph.tsx`](../../../console/apps/web/src/components/extensions/ExtensionDependencyGraph.tsx)   | Component        |
| 13    | [`console/apps/web/src/components/extensions/ExtensionPolicies.tsx`](../../../console/apps/web/src/components/extensions/ExtensionPolicies.tsx)                 | Component        |
| 14    | [`console/apps/web/src/components/extensions/ExtensionUsageMatrix.tsx`](../../../console/apps/web/src/components/extensions/ExtensionUsageMatrix.tsx)           | Component        |
| 15    | [`console/apps/web/src/components/fleet/index.ts`](../../../console/apps/web/src/components/fleet/index.ts)                                                     | Barrel re-export |
| 16    | [`console/apps/web/src/components/instances/index.ts`](../../../console/apps/web/src/components/instances/index.ts)                                             | Barrel re-export |
| 17    | [`console/apps/web/src/components/security/index.ts`](../../../console/apps/web/src/components/security/index.ts)                                               | Barrel re-export |
| 18    | [`console/apps/web/src/components/ui/separator.tsx`](../../../console/apps/web/src/components/ui/separator.tsx)                                                 | UI primitive     |
| 19    | [`console/apps/web/src/hooks/useLogs.ts`](../../../console/apps/web/src/hooks/useLogs.ts)                                                                       | Hook             |
| 20    | [`console/apps/web/src/hooks/useTerminalSearch.ts`](../../../console/apps/web/src/hooks/useTerminalSearch.ts)                                                   | Hook             |
| 21    | [`console/apps/web/src/pages/InstancesPage.tsx`](../../../console/apps/web/src/pages/InstancesPage.tsx)                                                         | Page             |
| 22    | [`console/apps/web/src/pages/TerminalPage.tsx`](../../../console/apps/web/src/pages/TerminalPage.tsx)                                                           | Page             |
| 23    | [`console/apps/web/src/stores/terminal.ts`](../../../console/apps/web/src/stores/terminal.ts)                                                                   | Zustand store    |
| 24    | [`console/apps/web/src/types/log.ts`](../../../console/apps/web/src/types/log.ts)                                                                               | Type definitions |
| 25–37 | `console/apps/web/tests/e2e/*.spec.ts` (13 files)                                                                                                               | E2E test files   |

**E2E test files** (items 25–37): `alerting`, `budget-alerts`, `deployment-wizard`, `drift-detection`, `extension-admin`, `fleet-dashboard`, `instance-dashboard`, `instance-lifecycle`, `log-search`, `parallel-commands`, `rbac-teams`, `scheduled-tasks`, `security-dashboard`

Plus 1 unit test: [`console/apps/web/tests/instance-realtime.test.ts`](../../../console/apps/web/tests/instance-realtime.test.ts)

**Assessment:** The unused source files (items 1–24) represent components, hooks, stores, and API clients that are defined but never imported into the application's route tree or component hierarchy. Many appear to be features built ahead of their integration point. The barrel `index.ts` files aggregate exports that are never consumed.

---

### TS Unused Dependencies

Detected by `knip` — packages listed in `package.json` but never imported in source code:

#### Unused `dependencies` (10)

| Package                         | Location                                                                     |
| ------------------------------- | ---------------------------------------------------------------------------- |
| `pino-pretty`                   | [`console/apps/api/package.json`](../../../console/apps/api/package.json):32 |
| `@radix-ui/react-avatar`        | [`console/apps/web/package.json`](../../../console/apps/web/package.json):18 |
| `@radix-ui/react-dropdown-menu` | [`console/apps/web/package.json`](../../../console/apps/web/package.json):20 |
| `@radix-ui/react-label`         | [`console/apps/web/package.json`](../../../console/apps/web/package.json):21 |
| `@radix-ui/react-scroll-area`   | [`console/apps/web/package.json`](../../../console/apps/web/package.json):22 |
| `@radix-ui/react-separator`     | [`console/apps/web/package.json`](../../../console/apps/web/package.json):24 |
| `@radix-ui/react-tabs`          | [`console/apps/web/package.json`](../../../console/apps/web/package.json):26 |
| `@radix-ui/react-toast`         | [`console/apps/web/package.json`](../../../console/apps/web/package.json):27 |
| `@radix-ui/react-tooltip`       | [`console/apps/web/package.json`](../../../console/apps/web/package.json):28 |
| `@types/recharts`               | [`console/apps/web/package.json`](../../../console/apps/web/package.json):31 |

#### Unused `devDependencies` (4)

| Package            | Location                                                                           |
| ------------------ | ---------------------------------------------------------------------------------- |
| `@types/ws`        | [`console/apps/web/package.json`](../../../console/apps/web/package.json):53       |
| `ws`               | [`console/apps/web/package.json`](../../../console/apps/web/package.json):62       |
| `@types/react`     | [`console/packages/ui/package.json`](../../../console/packages/ui/package.json):22 |
| `@types/react-dom` | [`console/packages/ui/package.json`](../../../console/packages/ui/package.json):23 |

#### Unlisted dependency (1)

| Package                  | Location                                                                                                                                            |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `@sindri-console/shared` | [`console/apps/api/tests/scheduled-tasks.test.ts`](../../../console/apps/api/tests/scheduled-tasks.test.ts):23 (imported but not in `package.json`) |

**Assessment:** The 7 unused `@radix-ui/*` packages were likely added in anticipation of UI components that haven't been built yet (or were built with different primitives). `pino-pretty` is typically a CLI dev tool — check if it's used via `pino-pretty` transport configuration rather than direct import. `@types/react` and `@types/react-dom` in `packages/ui` may be needed as peer type references — verify before removing.

---

### TS Unused Exports

Detected by `knip` — 145 exported functions/variables and 93 exported types that are never imported by any other module. Due to the volume, these are grouped by subsystem.

#### API App — Unused Function Exports (30)

| Export                                                   | File                                                                                                                         | Line |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ---- |
| `getGatewayStatus`                                       | [`agents/gateway.ts`](../../../console/apps/api/src/agents/gateway.ts)                                                       | 645  |
| `getActiveAlertCount`                                    | [`services/alerts/alert.service.ts`](../../../console/apps/api/src/services/alerts/alert.service.ts)                         | 71   |
| `flyPricing`                                             | [`services/costs/pricing.ts`](../../../console/apps/api/src/services/costs/pricing.ts)                                       | 36   |
| `awsPricing`                                             | [`services/costs/pricing.ts`](../../../console/apps/api/src/services/costs/pricing.ts)                                       | 104  |
| `gcpPricing`                                             | [`services/costs/pricing.ts`](../../../console/apps/api/src/services/costs/pricing.ts)                                       | 164  |
| `azurePricing`                                           | [`services/costs/pricing.ts`](../../../console/apps/api/src/services/costs/pricing.ts)                                       | 224  |
| `runpodPricing`                                          | [`services/costs/pricing.ts`](../../../console/apps/api/src/services/costs/pricing.ts)                                       | 284  |
| `northflankPricing`                                      | [`services/costs/pricing.ts`](../../../console/apps/api/src/services/costs/pricing.ts)                                       | 328  |
| `maxSeverity`                                            | [`services/drift/comparator.ts`](../../../console/apps/api/src/services/drift/comparator.ts)                                 | 236  |
| `runDriftDetection`                                      | [`services/drift/detector.worker.ts`](../../../console/apps/api/src/services/drift/detector.worker.ts)                       | 48   |
| `getPolicyById`                                          | [`services/extensions/policy.service.ts`](../../../console/apps/api/src/services/extensions/policy.service.ts)               | 30   |
| `getExtensionByName`                                     | [`services/extensions/registry.service.ts`](../../../console/apps/api/src/services/extensions/registry.service.ts)           | 76   |
| `ingestMetric`                                           | [`services/metrics/metric.service.ts`](../../../console/apps/api/src/services/metrics/metric.service.ts)                     | 53   |
| `subscribeToInstance`                                    | [`services/metrics/stream.ts`](../../../console/apps/api/src/services/metrics/stream.ts)                                     | 25   |
| `unsubscribeFromInstance`                                | [`services/metrics/stream.ts`](../../../console/apps/api/src/services/metrics/stream.ts)                                     | 39   |
| `unsubscribeAll`                                         | [`services/metrics/stream.ts`](../../../console/apps/api/src/services/metrics/stream.ts)                                     | 52   |
| `publishMetricUpdate`                                    | [`services/metrics/stream.ts`](../../../console/apps/api/src/services/metrics/stream.ts)                                     | 81   |
| `parseCron`                                              | [`services/scheduler/cron.service.ts`](../../../console/apps/api/src/services/scheduler/cron.service.ts)                     | 53   |
| `getNextDate`                                            | [`services/scheduler/cron.service.ts`](../../../console/apps/api/src/services/scheduler/cron.service.ts)                     | 82   |
| `refreshOverdueFlags`                                    | [`services/security/secret-rotation.service.ts`](../../../console/apps/api/src/services/security/secret-rotation.service.ts) | 119  |
| `isWeakKey`                                              | [`services/security/ssh-audit.service.ts`](../../../console/apps/api/src/services/security/ssh-audit.service.ts)             | 23   |
| `refreshExpiredKeys`                                     | [`services/security/ssh-audit.service.ts`](../../../console/apps/api/src/services/security/ssh-audit.service.ts)             | 96   |
| `AuthError`                                              | [`websocket/auth.ts`](../../../console/apps/api/src/websocket/auth.ts)                                                       | 37   |
| `hashApiKey`                                             | [`websocket/auth.ts`](../../../console/apps/api/src/websocket/auth.ts)                                                       | 51   |
| `extractRawKey`                                          | [`websocket/auth.ts`](../../../console/apps/api/src/websocket/auth.ts)                                                       | 60   |
| `extractInstanceId`                                      | [`websocket/auth.ts`](../../../console/apps/api/src/websocket/auth.ts)                                                       | 83   |
| + 6 via barrel re-exports in `services/metrics/index.ts` | —                                                                                                                            | —    |

#### Web App — Unused Component/Hook/Store Exports (115)

The full list includes exports from barrel `index.ts` files across all feature areas. Key patterns:

- **Barrel re-exports:** 8 `index.ts` files re-export components never imported elsewhere (alerts, commands, costs, drift, extensions, fleet, instances, security, tasks, terminal, deployment)
- **Unused hooks:** `useAlert`, `useUpdateChannel`, `costDateRange`, `useLatestSnapshot`, `useSecret`, `useUpdateSecret`, `useExtensionDependencies`, `useUsageMatrix`, `useExtensionPolicies`, `useCreateExtension`, `useUpdateExtension`, `useDeleteExtension`, `useSetPolicy`, `useDeletePolicy`, `useVulnerability`, `useAcknowledgeVulnerability`, `useFixVulnerability`, `useMarkFalsePositive`, `useTask`
- **Unused API clients:** `getCommandExecution`, `providersApi`, `getTerminalWebSocketUrl`
- **Unused components:** `RemediationOptions`, `MultiTerminal`, `saveSessionState`, `clearSessionState`, `fuzzyScore`, `fuzzyMatch`, and many more
- **Unused UI primitives:** `badgeVariants`, `buttonVariants`, `DialogPortal`, `DialogOverlay`, `DialogClose`, `DialogTrigger`, `SelectGroup`, `SelectLabel`, `SelectSeparator`, `SelectScrollUpButton`, `SelectScrollDownButton`

#### Unused Exported Types (93)

Spread across API services, web app types, and component props interfaces. These represent type definitions for features not yet wired into the application.

**Full list omitted for brevity** — run `npx knip --no-progress` from `v3/console/` to regenerate.

---

## Go Findings

### Module: `github.com/pacphi/sindri/v3/console/agent`

**Go 1.26.0** | Dependencies: `creack/pty`, `gorilla/websocket`, `shirou/gopsutil/v4`

| Tool                | Result                              |
| ------------------- | ----------------------------------- |
| `go vet ./...`      | **0 issues** — clean                |
| `deadcode ./...`    | **0 unreachable functions** — clean |
| `staticcheck ./...` | **0 findings** — clean              |

**Assessment:** The Go agent is a compact, well-maintained codebase with no dead code.

---

## Prioritized Recommendations

### P0 — Broken Code (fix immediately)

1. **TS-BROKEN-01:** Fix missing `@/components/logs` module. Two files ([`InstanceDetailPage.tsx:8`](../../../console/apps/web/src/components/instances/InstanceDetailPage.tsx), [`LogsPage.tsx:1`](../../../console/apps/web/src/pages/LogsPage.tsx)) import from a non-existent directory. Create the module or remove the imports.

### P1 — Unused Dependencies (clean up this sprint)

2. **TS-DEP-01:** Remove 7 unused `@radix-ui/*` packages from `apps/web/package.json` (avatar, dropdown-menu, label, scroll-area, separator, tabs, toast, tooltip). These add ~700KB to `node_modules` and create false dependency audit noise.
3. **TS-DEP-02:** Remove `@types/recharts`, `@types/ws`, and `ws` from `apps/web/package.json`.
4. **TS-DEP-03:** Verify `pino-pretty` usage in `apps/api` — if used only via pino transport config, move to `devDependencies`; if unused, remove.
5. **TS-DEP-04:** Add `@sindri-console/shared` to `apps/api/package.json` devDependencies (unlisted import in test file).

### P2 — Unused Files (clean up next sprint)

6. **TS-FILES-01:** Audit the 8 unused API source files (`routes/lifecycle.ts`, `services/*/index.ts` barrels, `websocket/handlers.ts`, `websocket/redis.ts`, `websocket/server.ts`). If these are scaffolded-but-unfinished features, track them in GitHub issues; if abandoned, delete them.
7. **TS-FILES-02:** Audit the 24 unused web source files. Many are scaffolded feature modules (costs, deployment templates, fleet, extensions) with components that were built but never integrated into routes. Decision needed: integrate or defer.

### P3 — Unused Exports (clean up incrementally)

8. **TS-EXPORTS-01:** Remove or mark as `@internal` the 30 unused API function exports. Priority targets: `getGatewayStatus`, 6 provider pricing objects, 5 metrics stream functions, and 4 websocket auth utilities.
9. **TS-EXPORTS-02:** Clean up barrel `index.ts` files in `apps/web/src/components/` — they re-export components that are never imported. Either wire the components into routes or reduce the barrel exports.

### P4 — Rust Annotations (review as time permits)

10. **RS-ALLOW-01:** Review 6 `#[allow(dead_code)]` annotations in production Rust code (`northflank.rs`, `runpod.rs`, `download.rs`, `template.rs`, `context.rs`). Remove the annotations and the dead fields if they serve no purpose.

### P5 — Structural Improvements (backlog)

11. **TS-KNIP-CONFIG:** Create `knip.json` configuration files for `apps/web` and `apps/api` workspaces to properly configure entry points and reduce false positives (especially for test files and e2e specs).
12. **TS-STRICT:** Enable `noUnusedLocals` and `noUnusedParameters` in `apps/api/tsconfig.json` to catch future unused variables at compile time.
13. **RS-UDEPS:** Install and run `cargo-udeps` with a nightly toolchain to detect unused Cargo dependencies (could not run during this audit due to missing nightly toolchain).
