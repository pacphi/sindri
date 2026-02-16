# Sindri Test Suite

London School TDD testing framework for Sindri provider adapters using [BATS](https://github.com/bats-core/bats-core) (Bash Automated Testing System).

## Quick Start

```bash
# Run all adapter tests
bats tests/adapters/

# Run tests for a specific provider
bats tests/adapters/runpod/
bats tests/adapters/northflank/

# Run a single test file
bats tests/adapters/runpod/deploy_v3.bats

# Run with verbose (TAP) output
bats --verbose-run tests/adapters/

# Run with timing information
bats --timing tests/adapters/
```

## Directory Structure

```
tests/
├── README.md                          # This file
├── lib/                               # Vendored BATS libraries
│   ├── bats-support/                  # Core assertion support
│   ├── bats-assert/                   # Output/status assertions
│   └── bats-file/                     # File system assertions
├── adapters/
│   ├── helpers/
│   │   ├── mocks.bash                 # Mock library for external commands
│   │   └── test_helpers.bash          # Shared test utilities and fixtures
│   ├── runpod/
│   │   ├── deploy_v3.bats            # Deploy adapter tests
│   │   ├── destroy_v3.bats           # Destroy adapter tests
│   │   ├── status_v3.bats            # Status adapter tests
│   │   └── connect_v3.bats           # Connect adapter tests
│   └── northflank/
│       ├── deploy_v3.bats            # Deploy adapter tests
│       ├── destroy_v3.bats           # Destroy adapter tests
│       ├── status_v3.bats            # Status adapter tests
│       └── connect_v3.bats           # Connect adapter tests
```

## London School TDD Approach

This test suite follows the **London School** (mockist) approach to TDD:

1. **Mock all external dependencies** -- Every external CLI tool (`runpodctl`, `northflank`, `docker`, `yq`, `jq`) is mocked. Tests never call real infrastructure.

2. **Test behavior, not implementation** -- Tests assert _what interactions happen_ (which commands are called with what arguments) rather than _how_ internal state is structured.

3. **Outside-in development** -- Tests are written from the user's perspective first (deploy, destroy, status, connect), then drive the implementation inward.

4. **Verify collaborations** -- The core assertions verify that objects/scripts correctly coordinate with their dependencies in the right order.

## Writing Tests

### Test Anatomy

Every test file follows this structure:

```bash
#!/usr/bin/env bats

HELPERS_DIR="$(cd "$(dirname "$BATS_TEST_FILENAME")/../helpers" && pwd)"

setup() {
    source "${HELPERS_DIR}/test_helpers.bash"
    load_test_libraries           # Load bats-assert, bats-support, bats-file
    setup_test_environment        # Create temp dir, set up mock PATH
}

teardown() {
    teardown_test_environment     # Clean up temp dir and mocks
}

@test "descriptive test name" {
    # 1. Arrange: set up mocks and fixtures
    mock_command "runpodctl" '{"id":"pod-123"}'
    create_runpod_config

    # 2. Act: run the code under test
    run bash -c "cd ${TEST_WORK_DIR} && source deploy_v3.sh && deploy_runpod_v3"

    # 3. Assert: verify interactions and outcomes
    assert_success
    verify_mock_called "runpodctl" "create pods"
    verify_mock_called "docker" "build"
}
```

### Mock Library Reference

The mock library (`tests/adapters/helpers/mocks.bash`) provides:

#### Creating Mocks

```bash
# Simple mock: always returns the same output
mock_command "runpodctl" '{"id":"pod-123"}' 0

# Conditional mock: different output based on arguments
mock_command_with_args "runpodctl" "create pods" '{"id":"pod-123"}'
mock_command_with_args "runpodctl" "get pod"     '{"status":"RUNNING"}'

# Callback mock: delegate to a function for complex logic
my_yq_mock() {
    case "$1" in
        '.name') echo "my-app" ;;
        *)       echo ""       ;;
    esac
}
mock_command_with_callback "yq" "my_yq_mock"

# Simulate failure
mock_command "docker" "Error: build failed" 1
```

#### Verifying Mock Calls

```bash
# Was the command called at all?
verify_mock_called "runpodctl"

# Was it called with specific arguments?
verify_mock_called "runpodctl" "create pods --name test"

# Was it NOT called?
verify_mock_not_called "runpodctl" "remove"

# Was it called exactly N times?
verify_mock_call_count "docker" 2

# Get the Nth call for manual inspection
call=$(get_mock_call "runpodctl" 1)

# Get total call count
count=$(get_mock_call_count "docker")
```

#### Preset Mocks

```bash
# Pre-configured yq mock for RunPod config parsing
setup_yq_mock_for_runpod

# Pre-configured yq mock for Northflank config parsing
setup_yq_mock_for_northflank

# Pre-configured jq mock for state file parsing
setup_jq_mock

# Mock all common external commands at once (no-op defaults)
mock_all_external_commands
```

### Fixture Helpers

```bash
# Create a sindri.yaml with RunPod provider config
create_runpod_config "NVIDIA A100 80GB" 40 100

# Create a sindri.yaml with Northflank provider config
create_northflank_config "my-project" "my-service" "nf-compute-100"

# Create a state file as if deploy already ran
create_runpod_state "pod-abc-123" "my-app"
create_northflank_state "my-project" "my-service" "my-service"
```

### Custom Assertions

```bash
# Check state file exists/absent
assert_state_file_exists "runpod"
assert_state_file_absent "northflank"

# Check output contains substring (works with bats `run`)
assert_output_contains "Pod is running"
```

## Example: Good London-Style Test

```bash
@test "deploy: orchestrates docker build, push, and pod creation in order" {
    # Arrange: mock all collaborators
    setup_yq_mock_for_runpod
    mock_command "docker" "sha256:abc123"
    mock_command_with_args "runpodctl" "create pods" '{"id":"pod-new-001"}'
    mock_command_with_args "runpodctl" "get pod" '{"desiredStatus":"RUNNING"}'

    create_runpod_config

    # Act: run the deploy function
    run bash -c "cd ${TEST_WORK_DIR} && source deploy_v3.sh && deploy_runpod_v3"

    # Assert: verify the collaboration sequence
    assert_success

    # Docker image was built
    verify_mock_called "docker" "build"

    # Image was pushed to registry
    verify_mock_called "docker" "push"

    # Pod was created with correct GPU type
    verify_mock_called "runpodctl" "create pods"
    verify_mock_called "runpodctl" "NVIDIA RTX A4000"

    # Pod status was polled
    verify_mock_called "runpodctl" "get pod"

    # State file was written
    assert_state_file_exists "runpod"
}
```

## BATS Helper Libraries

- **bats-support**: Core assertion support (`fail`, `batslib_*`)
- **bats-assert**: Output assertions (`assert_success`, `assert_failure`, `assert_output`, `assert_line`)
- **bats-file**: File system assertions (`assert_file_exists`, `assert_dir_exists`, `assert_file_contains`)

See [bats-core docs](https://bats-core.readthedocs.io/) for the full reference.

## Prerequisites

- **BATS** >= 1.13.0 (installed at `~/.local/bin/bats`)
- **Bash** >= 4.0
- No network or cloud access required -- all external calls are mocked
