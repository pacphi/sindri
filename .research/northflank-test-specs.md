# Northflank Adapter Test Specifications

## Overview

Comprehensive Rust test specifications for the Northflank provider adapter, following **London School TDD** (mock-first, behavior-driven approach).

**File**: `v3/crates/sindri-providers/tests/northflank_tests.rs`
**Run**: `cargo test --package sindri-providers --test northflank_tests`
**Total tests**: 42

## Test Categories

| #   | Category                     | Count | Style | Key Assertions                                                                         |
| --- | ---------------------------- | ----- | ----- | -------------------------------------------------------------------------------------- |
| 1   | Provider Creation            | 3     | sync  | `new()`, `with_output_dir()`, `name()` return correct values                           |
| 2   | Capability Flags             | 2     | sync  | `supports_gpu()` and `supports_auto_suspend()` return `true`                           |
| 3   | Prerequisite Checks          | 4     | sync  | CLI detection, auth detection, install hints                                           |
| 4   | API Response Deserialization | 6     | sync  | `NorthflankService`, `NorthflankServicePort`, `NorthflankMetrics`, `NorthflankProject` |
| 5   | Status Mapping               | 1     | sync  | All 8 status strings map to correct `DeploymentState`                                  |
| 6   | Compute Plan Mapping         | 6     | sync  | CPU/memory -> nf-compute-{10,20,50,100,200}                                            |
| 7   | GPU Tier Mapping             | 5     | sync  | gpu-small/medium -> nvidia-a10g, gpu-large/xlarge -> nvidia-a100                       |
| 8   | Deploy Lifecycle             | 4     | async | dry_run, existing service error, --force, project creation                             |
| 9   | Status Queries               | 3     | async | running, paused, not deployed                                                          |
| 10  | Connect                      | 3     | async | exec call, auto-resume paused, error on missing                                        |
| 11  | Destroy                      | 3     | async | delete service, preserve project, error on missing                                     |
| 12  | Start/Stop                   | 3     | async | resume, pause, error on missing                                                        |
| 13  | Plan (Dry-Run)               | 2     | async | project+service actions, volume action                                                 |
| 14  | Service Definition Builder   | 3     | sync  | basic JSON, health check, auto-scaling                                                 |
| 15  | Secret Groups                | 1     | async | CLI called with `create secret`                                                        |
| 16  | Config/State Fixtures        | 2     | sync  | valid YAML/JSON output                                                                 |
| 17  | Factory Integration          | 1     | sync  | `create_provider(Northflank)` returns correct provider                                 |

## Mocking Strategy

### CLI Mocking (Process-Level)

Tests use shell script mocks from `tests/common/mod.rs`:

- **`create_mock_executable()`**: Simple mock that logs invocations and returns fixed output
- **`create_conditional_mock()`**: Returns different output based on argument patterns
- **`read_mock_log()`**: Reads the invocation log for assertion

Mock scripts are placed in a temp directory prepended to `$PATH`, and `NORTHFLANK_API_TOKEN` is set to bypass auth checks.

### Environment Management

Helper functions `setup_mock_env()` / `restore_env()` handle PATH and token manipulation:

```rust
fn setup_mock_env(tmp: &Path) -> String {
    let original = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:{}", tmp.display(), original));
    std::env::set_var("NORTHFLANK_API_TOKEN", "test-token");
    original
}
```

### What Gets Mocked

| Dependency          | Mock Mechanism                        |
| ------------------- | ------------------------------------- |
| `northflank` CLI    | Shell script on PATH returning JSON   |
| Authentication      | `NORTHFLANK_API_TOKEN` env var        |
| Filesystem (config) | `tempfile::tempdir()` + YAML fixtures |
| Filesystem (state)  | `create_northflank_state()` helper    |

## Public API Surface Required

The test file imports these symbols from `sindri_providers::northflank`:

### Structs (must be `pub`)

- `NorthflankProvider` -- main provider struct
- `NorthflankService` -- API response deserialization
- `NorthflankServicePort` -- port in service response
- `NorthflankMetrics` -- metrics in service response
- `NorthflankProject` -- project response deserialization
- `NorthflankDeployConfig` -- internal deploy configuration
- `NorthflankPort` -- port configuration
- `NorthflankHealthCheck` -- health check configuration
- `NorthflankAutoScaling` -- auto-scaling configuration

### Functions (must be `pub`)

- `compute_plan_from_resources(cpus: u32, memory_mb: u32) -> String`
- `northflank_gpu_from_tier(tier: Option<&str>) -> String`
- `map_service_status(status: &str) -> DeploymentState`

### Methods on `NorthflankProvider` (must be `pub`)

- `new() -> Result<Self>`
- `with_output_dir(PathBuf) -> Result<Self>`
- `build_service_definition(&self, &NorthflankDeployConfig) -> Result<String>`
- `create_secret_group(&self, project: &str, service: &str, secrets: &HashMap<String, String>) -> Result<()>` (async)

### Provider Trait Methods (via `impl Provider`)

- `name()`, `check_prerequisites()`, `supports_gpu()`, `supports_auto_suspend()`
- `deploy()`, `connect()`, `status()`, `destroy()`, `plan()`, `start()`, `stop()`

## Red Phase Status

All 42 tests are in **Red phase** -- they will fail until:

1. `v3/crates/sindri-providers/src/northflank.rs` is created with the full implementation
2. `v3/crates/sindri-providers/src/lib.rs` adds `pub mod northflank;` and the match arm
3. `v3/crates/sindri-core/src/types/config_types.rs` adds `Northflank` to the `Provider` enum

## Key Design Decisions

1. **No TemplateRegistry needed** -- Northflank service definitions are built as inline JSON, not via Tera templates
2. **stop() -> pause, start() -> resume** -- maps Sindri's generic start/stop to Northflank's pause/resume
3. **Destroy preserves project** -- only deletes the service, since projects are organizational containers
4. **Connect auto-resumes** -- if service is paused, connect() calls resume first then exec
5. **Secret groups** -- secrets are created as Northflank secret groups with type "environment"
