# TDD Framework Setup Summary

## Overview

Two testing layers were established for the RunPod and Northflank provider adapters:

1. **Rust integration tests** (primary) -- London School TDD tests using `cargo test`
2. **BATS bash tests** (supplementary) -- Shell-level specification tests

## Rust Test Framework (Primary)

### Setup

Added to `v3/crates/sindri-providers/Cargo.toml` dev-dependencies:

- `mockall = "0.13"` -- For future trait-level mocking
- `tokio` with `test-util` and `macros` features -- For async test support
- `tempfile` -- For isolated test directories
- `serde_yaml_ng`, `serde_json` -- For config fixture parsing
- `camino` -- For Utf8Path config loading

### Test Structure

```
v3/crates/sindri-providers/tests/
├── common/
│   └── mod.rs                    # Shared test helpers and mock infrastructure
├── runpod_tests.rs               # 28 tests for RunPod provider
└── northflank_tests.rs           # 27 tests for Northflank provider
```

### Mock Infrastructure (`tests/common/mod.rs`)

**CommandLog** -- In-memory call recorder for verifying CLI interactions:

- `CommandLog::record()` -- Record a command invocation
- `CommandLog::assert_called()` -- Assert a command was called
- `CommandLog::assert_called_with()` -- Assert called with specific args
- `CommandLog::assert_not_called()` -- Assert a command was NOT called
- `CommandLog::call_count()` -- Count invocations

**Process Mocks** -- Creates real executable scripts on the filesystem:

- `create_mock_executable()` -- Simple mock with fixed output and exit code
- `create_conditional_mock()` -- Pattern-matched output per argument combination
- `read_mock_log()` -- Read invocation log for a mock

**Config Fixtures** -- Creates sindri.yaml files for test scenarios:

- `create_docker_config()` -- Returns `(TempDir, SindriConfig)` for Docker
- `create_runpod_config_fixture()` -- RunPod YAML with GPU config
- `create_northflank_config_fixture()` -- Northflank YAML with compute plan

**State File Helpers**:

- `create_runpod_state()` -- Creates `.sindri/state/runpod.json`
- `create_northflank_state()` -- Creates `.sindri/state/northflank.json`

**Result Builders** -- Typed builders for test assertions:

- `deploy_result_ok()` -- Successful DeployResult
- `deployment_status()` -- DeploymentStatus with given state

### Test Coverage (55 Rust integration tests)

| Area                   | RunPod | Northflank |
| ---------------------- | ------ | ---------- |
| Prerequisites          | 2      | 2          |
| Deploy (process mocks) | 3      | 2          |
| Deploy (behavior)      | 5      | 6          |
| Deploy (config)        | 2      | 1          |
| Deploy (errors)        | 2      | 2          |
| Destroy                | 5      | 6          |
| Status                 | 3      | 3          |
| Connect                | 2      | 2          |
| Provider trait         | 3      | 3          |
| Deploy result          | 2      | 1          |
| **Total**              | **28** | **27**     |

### Code Fix Applied

Fixed non-exhaustive match in `v3/crates/sindri-core/src/templates/context.rs:67` to handle the new `Provider::Runpod` and `Provider::Northflank` variants.

### How to Run

```bash
cd v3

# All provider tests (unit + integration)
cargo test --package sindri-providers

# Integration tests only
cargo test --package sindri-providers --test runpod_tests --test northflank_tests

# Single test file
cargo test --package sindri-providers --test runpod_tests

# Specific test
cargo test --package sindri-providers --test runpod_tests deploy_should_call_docker_build

# Unit tests only (in lib)
cargo test --package sindri-providers --lib
```

### Verification

All tests pass:

```
running 28 tests ... test result: ok. 28 passed (runpod_tests)
running 27 tests ... test result: ok. 27 passed (northflank_tests)
running 101 tests ... test result: ok. 101 passed (unit tests)
```

## BATS Framework (Supplementary)

The BATS framework remains available at `tests/adapters/` for shell-level testing if needed. See `tests/README.md` for documentation.

## London School TDD Methodology

Both test layers follow London School principles:

1. **Mock all external dependencies** -- CLI tools (runpodctl, northflank, docker) are never invoked for real
2. **Test behavior through interactions** -- Assertions verify which commands are called with what arguments, not internal state
3. **Outside-in development** -- Tests specify behavior from the user perspective (deploy, destroy, status, connect)
4. **Contracts via types** -- The `Provider` trait defines the behavioral contract; tests verify implementations conform
