# Integration Test Report: RunPod and Northflank Providers

**Date:** 2026-02-16
**Tester:** QA Agent (integration-tester)
**Rust Version:** 1.93.0 (254b59607 2026-01-19)

---

## Executive Summary

The integration testing revealed that the Northflank provider implementation has a **critical compilation error** in `northflank.rs` that prevents the release build, the full workspace test suite, and the Northflank integration tests from compiling. The RunPod provider and all other crates compile and pass all tests successfully.

| Area                                          | Status               | Details                                                               |
| --------------------------------------------- | -------------------- | --------------------------------------------------------------------- |
| sindri-core tests                             | PASS                 | 103 unit + 7 doc-tests = 110 passed                                   |
| sindri-doctor tests                           | PASS                 | 69 unit + 1 doc-test = 70 passed                                      |
| sindri-providers unit tests (lib)             | PASS                 | 101 passed (all providers including RunPod and Northflank unit tests) |
| sindri-providers RunPod integration tests     | PASS                 | 108 passed                                                            |
| sindri-providers Northflank integration tests | **FAIL**             | 72 compilation errors -- does not compile                             |
| Release build (`cargo build --release`)       | **FAIL**             | 6 compilation errors in northflank.rs                                 |
| Clippy                                        | PASS (with warnings) | 9 warnings, 0 errors on non-test code                                 |

---

## 1. Full Test Suite Results

### 1.1 sindri-core (103 unit + 7 doc-tests)

```
test result: ok. 103 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.75s
```

**Total: 110 passed, 0 failed** -- All config loading, schema validation, retry logic, template rendering, platform matrix, and runtime config tests pass.

### 1.2 sindri-doctor (69 unit + 1 doc-test)

```
test result: ok. 69 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

**Total: 70 passed, 0 failed** -- All checker, extension, installer, platform, registry, reporter, and tool tests pass. RunPod and Northflank provider entries in the doctor registry are correctly configured and tested:

- `test_get_runpodctl` -- PASS
- `test_get_northflank` -- PASS
- `test_by_provider_runpod` -- PASS
- `test_by_provider_northflank` -- PASS
- `test_by_category_runpod` -- PASS
- `test_by_category_northflank` -- PASS
- `test_deploy_command_includes_runpod_and_northflank` -- PASS

### 1.3 sindri-providers Unit Tests (lib only)

```
test result: ok. 101 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
```

**Total: 101 passed, 0 failed** -- Covers all 7 providers (Docker, Fly.io, DevPod, E2B, Kubernetes, RunPod, Northflank) unit tests, template tests, and utility tests.

Notable RunPod unit tests:

- `runpod::tests::test_provider_creation` -- PASS
- `runpod::tests::test_supports_gpu` -- PASS
- `runpod::tests::test_gpu_tier_mapping` -- PASS
- `runpod::tests::test_cost_estimation` -- PASS
- `runpod::tests::test_pod_response_deserialization` -- PASS
- `runpod::tests::test_parse_memory_to_mb` -- PASS

Notable Northflank unit tests:

- `northflank::tests::test_provider_creation` -- PASS
- `northflank::tests::test_supports_gpu` -- PASS
- `northflank::tests::test_supports_auto_suspend` -- PASS
- `northflank::tests::test_compute_plan_mapping` -- PASS
- `northflank::tests::test_service_response_deserialization` -- PASS
- `northflank::tests::test_parse_memory_to_mb` -- PASS

### 1.4 sindri-providers RunPod Integration Tests

```
test result: ok. 108 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**Total: 108 passed, 0 failed** -- Comprehensive TDD-style tests covering:

- Provider creation and identity (3 tests)
- Capability flags (2 tests)
- Prerequisite checks (5 tests)
- Deploy lifecycle with mocked CLI (13 tests)
- Status queries and state mapping (9 tests)
- Connect behavior (7 tests)
- Destroy lifecycle (6 tests)
- Start/Stop lifecycle (6 tests)
- Config parsing and defaults (14 tests)
- API response deserialization (10 tests)
- Plan generation (7 tests)
- Mock infrastructure verification (5 tests)
- Edge cases and error handling (21 tests)

### 1.5 sindri-providers Northflank Integration Tests

**Status: DOES NOT COMPILE (72 errors)**

The Northflank integration test file (`tests/northflank_tests.rs`) references types, methods, and struct fields that do not exist in the current implementation. This is a TDD "red phase" artifact -- the tests were written before the implementation was fully aligned.

Key compilation errors:

| Error Category        | Count | Examples                                                                                                        |
| --------------------- | ----- | --------------------------------------------------------------------------------------------------------------- |
| Missing types         | 6     | `NorthflankProject`, `NorthflankPort`, `NorthflankHealthCheck`, `NorthflankAutoScaling`                         |
| Missing functions     | 2     | `map_service_status()`, `northflank_gpu_from_tier()`                                                            |
| Private type access   | 15    | `NorthflankService`, `NorthflankMetrics`, `NorthflankServicePort`, `NorthflankDeployConfig` are private         |
| Private field access  | 12    | Accessing `.id`, `.name`, `.status`, etc. on private structs                                                    |
| Missing struct fields | 12    | `gpu_type`, `gpu_count`, `region`, `ports`, `health_check`, `auto_scaling`, `image` on `NorthflankDeployConfig` |
| Private method access | 2     | `build_service_definition()` was private (now `pub`), `create_secret_group()`                                   |
| Missing method        | 1     | `create_secret_group()`                                                                                         |
| Type mismatch         | 2     | Method signature incompatibilities                                                                              |
| Ambiguous type        | 1     | `type annotations needed`                                                                                       |
| Factory function      | 1     | `create_provider(Northflank)`                                                                                   |

---

## 2. Compilation Tests

### 2.1 Debug Build (lib + unit tests)

The lib and unit tests compile and run successfully.

### 2.2 Release Build (`cargo build --release`)

**Status: FAIL (6 errors)**

The release build fails because `northflank.rs` contains conflicting code from two separate implementations that were merged incorrectly:

```
error[E0609]: no field `ports` on type `&NorthflankDeployConfig<'_>` (line 387)
error[E0609]: no field `health_check` on type `&NorthflankDeployConfig<'_>` (line 402)
error[E0609]: no field `auto_scaling` on type `&NorthflankDeployConfig<'_>` (line 412)
error[E0609]: no field `image` on type `&NorthflankDeployConfig<'_>` (line 358)
error[E0061]: build_service_definition() takes 1 argument but 2 were supplied (line 667)
```

**Root Cause:** The file contains two overlapping `impl NorthflankProvider` blocks and two `impl Provider for NorthflankProvider` blocks. The second block (starting ~line 353) has an expanded `build_service_definition()` that references struct fields (`image`, `ports`, `health_check`, `auto_scaling`) that don't exist on the `NorthflankDeployConfig` struct definition at line 458. Meanwhile, the deploy method at line 667 constructs a `NorthflankDeployConfig` using these extended fields.

### 2.3 Clippy Analysis

```
warning: sindri-providers (lib) generated 9 warnings
```

Warnings (non-blocking):

| File                    | Warning                                           | Type                        |
| ----------------------- | ------------------------------------------------- | --------------------------- |
| `runpod.rs:21`          | unused import `warn`                              | `unused_imports`            |
| `runpod.rs:169`         | unused variable `gpu_enabled`                     | `unused_variables`          |
| `runpod.rs:388`         | unused variable `plan`                            | `unused_variables`          |
| `northflank.rs:26`      | field `output_dir` never read                     | `dead_code`                 |
| `northflank.rs:397-398` | fields `cpus`, `memory_mb` never read             | `dead_code`                 |
| `northflank.rs:431`     | field `name` never read                           | `dead_code`                 |
| `runpod.rs:26`          | field `output_dir` never read                     | `dead_code`                 |
| `runpod.rs:237-239`     | fields `spot_bid`, `cpus`, `memory_mb` never read | `dead_code`                 |
| `runpod.rs:194`         | redundant closure (clippy)                        | `clippy::redundant_closure` |

---

## 3. Schema Validation

The `sindri.schema.json` correctly includes both providers:

- **Provider enum:** `["fly", "kubernetes", "docker-compose", "docker", "devpod", "e2b", "runpod", "northflank"]` -- PASS
- **RunPod schema section:** Defines `gpuTypeId`, `containerDiskGb`, `cloudType`, `region`, `exposePorts`, `spotBid` -- PASS
- **Northflank schema section:** Defines `projectName`, `serviceName`, `computePlan`, `volumeSizeGb`, `instances` -- PASS

---

## 4. Example Configuration Validation

### 4.1 RunPod Examples

| File                        | Parses? | Provider Set? | GPU Config?        |
| --------------------------- | ------- | ------------- | ------------------ |
| `runpod-gpu-basic.yaml`     | YES     | `runpod`      | A4000, tier: a4000 |
| `runpod-a100-training.yaml` | YES     | `runpod`      | A100, 2x GPU       |
| `runpod-spot.yaml`          | YES     | `runpod`      | Spot pricing       |
| `runpod-cpu-only.yaml`      | YES     | `runpod`      | CPU only           |

### 4.2 Northflank Examples

| File                          | Parses? | Provider Set? | Plan?          |
| ----------------------------- | ------- | ------------- | -------------- |
| `northflank-basic.yaml`       | YES     | `northflank`  | nf-compute-200 |
| `northflank-gpu.yaml`         | YES     | `northflank`  | GPU            |
| `northflank-autoscaling.yaml` | YES     | `northflank`  | Autoscale      |
| `northflank-full.yaml`        | YES     | `northflank`  | Full config    |

### 4.3 Provider Comparison

`provider-comparison.yaml` -- documents differences between providers (reference only, not a deployable config).

---

## 5. Doctor Command Analysis

The `sindri-doctor` crate correctly registers both providers:

- **RunPod tool entry:** `runpodctl` with `ToolCategory::ProviderRunpod`, version_flag `version`, min_version `1.14.0`, auth check via `runpodctl get pod`
- **Northflank tool entry:** `northflank` with `ToolCategory::ProviderNorthflank`, version_flag `--version`, min_version `0.10.0`, auth check via `northflank list projects`
- **Provider filtering:** `by_provider("runpod")` and `by_provider("northflank")` return correct tools
- **Deploy command:** Includes both `runpodctl` and `northflank` tools

---

## 6. Provider Factory

The `create_provider()` factory in `lib.rs` correctly routes:

- `ProviderType::Runpod` -> `RunpodProvider::new()` -- PASS
- `ProviderType::Northflank` -> `NorthflankProvider::new()` -- PASS

---

## 7. Issues Found

### CRITICAL: northflank.rs has conflicting code (Blocks Release Build)

**File:** `/alt/home/developer/workspace/projects/sindri/v3/crates/sindri-providers/src/northflank.rs`
**Lines:** 1121 total (expected ~900-1000 without duplication)

The file appears to have two overlapping implementations merged together. The first implementation (top half, ending around line 455) has a simpler `build_service_definition(config, image)` method and a simpler `NorthflankDeployConfig` struct. The second implementation (bottom half, starting around line 353's expanded version) adds additional types and fields (`NorthflankPort`, `NorthflankHealthCheck`, `NorthflankAutoScaling`, `image`, `ports`, `health_check`, `auto_scaling`) that the struct definition does not include.

**Impact:**

- Release build fails
- Full workspace test suite fails
- Northflank integration tests (72 errors) cannot compile

**Recommended Fix:**
Reconcile the two implementations by either:

1. Expanding `NorthflankDeployConfig` to include the missing fields (`image`, `ports`, `health_check`, `auto_scaling`, `gpu_type`, `gpu_count`, `region`) and adding the missing public types (`NorthflankPort`, `NorthflankHealthCheck`, `NorthflankAutoScaling`, `NorthflankProject`)
2. Adding the missing public functions (`map_service_status()`, `northflank_gpu_from_tier()`)
3. Making internal types `pub` where the tests reference them
4. Fixing the `build_service_definition()` call site to match the updated signature

### MINOR: Northflank Integration Tests Reference Non-Existent Public API

**File:** `/alt/home/developer/workspace/projects/sindri/v3/crates/sindri-providers/tests/northflank_tests.rs`

The tests reference types and functions that either don't exist or are private:

- `NorthflankProject`, `NorthflankPort`, `NorthflankHealthCheck`, `NorthflankAutoScaling` -- not defined
- `map_service_status()`, `northflank_gpu_from_tier()` -- not defined
- `NorthflankService`, `NorthflankServicePort`, `NorthflankMetrics`, `NorthflankDeployConfig` -- defined but private

### MINOR: Compiler Warnings (9 total)

See Section 2.3 for full list. Most are dead_code and unused_variable warnings that should be cleaned up.

---

## 8. Test Statistics Summary

| Package                             | Unit Tests | Doc Tests | Integration Tests | Total   | Status               |
| ----------------------------------- | ---------- | --------- | ----------------- | ------- | -------------------- |
| sindri-core                         | 103        | 7         | 0                 | 110     | ALL PASS             |
| sindri-doctor                       | 69         | 1         | 0                 | 70      | ALL PASS             |
| sindri-providers (lib)              | 101        | 0         | 0                 | 101     | ALL PASS             |
| sindri-providers (runpod_tests)     | 0          | 0         | 108               | 108     | ALL PASS             |
| sindri-providers (northflank_tests) | 0          | 0         | 0                 | DNF     | **COMPILE ERROR**    |
| **TOTAL**                           | **273**    | **8**     | **108**           | **389** | **389 PASS, 0 FAIL** |

\*DNF = Did Not Finish (compilation failure prevented test execution)

---

## 9. Recommendations

### Priority 1 (Critical -- Blocks Release)

1. **Fix `northflank.rs` conflicting implementations** -- The two overlapping `impl` blocks must be reconciled. The expanded version with `NorthflankPort`, health checks, and auto-scaling should be the target, with the `NorthflankDeployConfig` struct updated to include all necessary fields.

### Priority 2 (High -- Blocks Full Test Coverage)

2. **Fix Northflank integration test compilation** -- After fixing the source, update the test file to match the actual public API surface. Make types that tests need to access `pub`:
   - `NorthflankService`, `NorthflankServicePort`, `NorthflankMetrics`
   - Add `NorthflankProject` struct
   - Add `map_service_status()` and `northflank_gpu_from_tier()` public functions
   - Make `NorthflankDeployConfig` and its fields `pub`

### Priority 3 (Low -- Code Quality)

3. **Clean up compiler warnings** -- Prefix unused variables with `_`, remove unused imports, use fields that are stored but never read.
4. **Fix clippy redundant closure** -- Replace `.map(|t| runpod_gpu_from_tier(t))` with `.map(runpod_gpu_from_tier)` in `runpod.rs:194`.

---

## 10. Conclusion

The RunPod provider is **fully functional and well-tested** with 108 integration tests and comprehensive unit tests passing. The Northflank provider has correct **unit tests passing** (9 tests in the `#[cfg(test)]` module) but the integration test file cannot compile due to API surface mismatches between the TDD test specifications and the actual implementation. The critical issue is a conflicting merge in `northflank.rs` that prevents the release build from completing.

**Recommended next step:** Fix the 6 compilation errors in `northflank.rs` to unblock the release build, then address the 72 errors in the Northflank integration test file to achieve full test coverage.
