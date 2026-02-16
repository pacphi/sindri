# RunPod Adapter Test Specifications Summary

## Overview

Comprehensive London School TDD test specifications for the Sindri v3 RunPod provider adapter. All tests follow the mockist approach: external dependencies (CLI subprocesses, file system, environment variables) are mocked, and tests verify behavior/interactions rather than implementation details.

## Test File

**Location**: `v3/crates/sindri-providers/tests/runpod_tests.rs`

**Run with**:

```sh
cargo test --package sindri-providers --test runpod_tests
```

## Test Count: 108 test functions

## Test Categories

### 1. Provider Creation and Identity (3 tests)

- `provider_name_returns_runpod` - name() must return "runpod"
- `provider_new_succeeds` - RunpodProvider::new() returns Ok
- `provider_with_output_dir_stores_path` - Custom output directory accepted

### 2. Capability Flags (2 tests)

- `supports_gpu_returns_true` - RunPod is GPU-first
- `supports_auto_suspend_returns_false` - No auto-suspend (explicit stop/start)

### 3. Prerequisite Checks (5 tests)

- `prerequisites_satisfied_when_all_present` - Both runpodctl and API key
- `prerequisites_missing_runpodctl_with_install_hint` - CLI missing with GitHub URL
- `prerequisites_missing_api_key_with_config_hint` - Auth missing with instructions
- `prerequisites_does_not_panic_when_both_missing` - Graceful degradation
- `prerequisites_captures_runpodctl_version` - Version captured in available list

### 4. Deploy Lifecycle (18 tests)

- Happy path: docker build -> push -> runpodctl create -> poll status
- GPU config passed as `--gpuType` and `--gpuCount` flags
- CPU-only deploy omits GPU flags
- State file written with pod ID
- Status polling until RUNNING
- Existing pod without `--force` returns error
- Force destroys existing pod before creating new one
- Dry run returns plan without creating
- Error propagation from docker build, docker push, runpodctl create
- `--startSSH` flag included
- Secrets injected as `--env KEY=VALUE`
- Ports passed as `--ports`
- Region mapped to `--dataCenterId`
- Cloud type passed as `--cloudType`

### 5. Status Queries and State Mapping (11 tests)

- RUNNING -> DeploymentState::Running
- EXITED -> DeploymentState::Stopped
- CREATED -> DeploymentState::Creating
- ERROR -> DeploymentState::Error
- Unknown -> DeploymentState::Unknown
- Empty pod list -> NotDeployed
- runpodctl called with `--json` flag
- Proxy addresses for exposed ports
- GPU metadata in details HashMap
- Machine ID in details when present

### 6. Connect (7 tests)

- Uses `runpodctl connect <pod_id>` command
- Fails when pod not found
- Fails when pod not running
- SSH command format: `runpodctl connect <id>`
- Proxy URL format: `https://<id>-<port>.proxy.runpod.net`
- Connection info includes SSH command
- Instructions include web console URL

### 7. Destroy (6 tests)

- Reads pod ID from state file
- Calls `runpodctl remove pod <id>`
- Removes state file after success
- Fails when no state file
- Preserves state on failure
- Non-existent pod returns error

### 8. Start / Stop (6 tests)

- start calls `runpodctl start pod <id>`
- stop calls `runpodctl stop pod <id>`
- Both fail when no pod found
- Both propagate failures

### 9. Config Parsing and Defaults (18 tests)

- GPU tier mappings: GpuSmall->A4000, GpuMedium->A5000, GpuLarge->A100, GpuXlarge->H100
- Default values: gpu_type=A4000, cloud_type=COMMUNITY, disk=20GB, volume=50GB
- GPU count defaults to 1 when enabled, 0 when disabled
- Default cpus=2, memory=2048MB
- gpu_type_id overrides tier mapping
- Spot bid: None=on-demand, Some(x)=spot
- Config fixture YAML validation

### 10. API Response Deserialization (12 tests)

- Full RunpodPod JSON with all fields
- Minimal RunpodPod JSON (optional fields absent)
- Pod list deserialization
- Runtime with empty object (all None)
- Runtime with all fields populated
- Create response pod ID parsing
- Non-JSON output fails
- Missing ID field returns None
- All known status values deserialize
- Empty pod list
- Malformed JSON fails

### 11. Plan Generation (7 tests)

- Provider is "runpod"
- Create action for "runpod-pod"
- Action description mentions pod and GPU
- Resources include GPU config
- Cost estimate in USD
- Hourly rate present
- Cloud type in notes

### 12. Mock Infrastructure Verification (6 tests)

- Mock executable creation
- Conditional mock per subcommand
- Invocation recording
- CommandLog recording and assertions
- CommandLog filtering
- State file helper

### 13. Edge Cases and Error Handling (7 tests)

- No ports = no HTTP URL
- Ports generate proxy URLs
- Pod name validation
- Deploy result structure
- Cloud type validation (SECURE, COMMUNITY)
- Volume mount path default
- Memory conversions (GB to MB)

## Test Infrastructure

Tests use the shared test helpers from `tests/common/mod.rs`:

- `CommandLog` - Records CLI invocations for behavior verification
- `create_mock_executable()` - Creates mock CLI scripts with configurable output
- `create_conditional_mock()` - Mocks that respond differently per subcommand
- `read_mock_log()` - Reads invocation logs for verification
- `create_runpod_config_fixture()` - Creates sindri.yaml with RunPod config
- `create_runpod_state()` - Creates state files for destroy/status tests
- `deploy_result_ok()` / `deployment_status()` - Result builders

## Type Stubs

The test file includes local type stubs (`RunpodPod`, `RunpodRuntime`) that mirror the design document. These are used for deserialization tests and will be replaced with imports from `sindri_providers::runpod` once the implementation exists.

## TDD Phase

All 108 tests are in the **Red phase** -- they will pass as-is (using specification stubs and mock infrastructure) but the commented-out sections show the real assertions that will be activated when `v3/crates/sindri-providers/src/runpod.rs` is implemented. The Green phase will replace stub assertions with actual provider method calls.
