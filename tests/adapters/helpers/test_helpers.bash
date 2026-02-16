#!/usr/bin/env bash
# test_helpers.bash - Shared test utilities for Sindri adapter tests
#
# Provides environment setup, fixture loading, and common assertions
# used across all adapter test suites.

# ─── Path Resolution ──────────────────────────────────────────────────────────

# Root of the sindri repository
SINDRI_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../" && pwd)"

# Paths to BATS helper libraries (vendored in tests/lib/)
BATS_SUPPORT_DIR="${SINDRI_ROOT}/tests/lib/bats-support"
BATS_ASSERT_DIR="${SINDRI_ROOT}/tests/lib/bats-assert"
BATS_FILE_DIR="${SINDRI_ROOT}/tests/lib/bats-file"

# Path to the mock library
MOCKS_LIB="${SINDRI_ROOT}/tests/adapters/helpers/mocks.bash"

# ─── Library Loading ──────────────────────────────────────────────────────────

load_test_libraries() {
    load "${BATS_SUPPORT_DIR}/load.bash"
    load "${BATS_ASSERT_DIR}/load.bash"
    load "${BATS_FILE_DIR}/load.bash"
    source "${MOCKS_LIB}"
}

# ─── Test Environment ─────────────────────────────────────────────────────────

# Creates an isolated working directory for a test run, simulating
# a user's project directory with a sindri.yaml.
setup_test_environment() {
    TEST_WORK_DIR="$(mktemp -d "${BATS_TMPDIR:-/tmp}/sindri-test.XXXXXX")"
    export TEST_WORK_DIR
    mkdir -p "${TEST_WORK_DIR}/.sindri/state"

    setup_mock_environment
}

# Cleans up the test working directory and all mocks.
teardown_test_environment() {
    cleanup_mocks
    if [[ -n "${TEST_WORK_DIR:-}" && -d "${TEST_WORK_DIR}" ]]; then
        rm -rf "${TEST_WORK_DIR}"
    fi
}

# ─── Fixture Helpers ──────────────────────────────────────────────────────────

# create_sindri_yaml <provider_name> [extra_yaml]
#
# Writes a minimal sindri.yaml into TEST_WORK_DIR with the given provider.
# extra_yaml is appended verbatim (must be valid YAML).
#
# Example:
#   create_sindri_yaml "runpod" "
#   provider:
#     runpod:
#       gpu_type: 'NVIDIA RTX A4000'
#       container_disk_gb: 20
#       volume_size_gb: 50"
create_sindri_yaml() {
    local provider_name="$1"
    local extra_yaml="${2:-}"
    cat > "${TEST_WORK_DIR}/sindri.yaml" <<EOF
name: test-sindri-app
version: "3"
provider:
  name: ${provider_name}
${extra_yaml}
EOF
}

# create_runpod_config [gpu_type] [container_disk_gb] [volume_size_gb]
#
# Creates a sindri.yaml configured for RunPod with sensible test defaults.
create_runpod_config() {
    local gpu_type="${1:-NVIDIA RTX A4000}"
    local container_disk="${2:-20}"
    local volume_size="${3:-50}"

    create_sindri_yaml "runpod" "  runpod:
    gpu_type: '${gpu_type}'
    gpu_count: 1
    container_disk_gb: ${container_disk}
    volume_size_gb: ${volume_size}
    cloud_type: COMMUNITY
    volume_mount_path: /workspace"
}

# create_northflank_config [project_name] [service_name] [compute_plan]
#
# Creates a sindri.yaml configured for Northflank with sensible test defaults.
create_northflank_config() {
    local project_name="${1:-sindri-test}"
    local service_name="${2:-sindri-workspace}"
    local compute_plan="${3:-nf-compute-50}"

    create_sindri_yaml "northflank" "  northflank:
    project_name: '${project_name}'
    service_name: '${service_name}'
    compute_plan: '${compute_plan}'
    instances: 1
    volume_size_gb: 10
    volume_mount_path: /workspace"
}

# create_runpod_state [pod_id] [app_name]
#
# Creates a RunPod state file as if a deploy had already run.
create_runpod_state() {
    local pod_id="${1:-pod-test-123}"
    local app_name="${2:-test-sindri-app}"

    cat > "${TEST_WORK_DIR}/.sindri/state/runpod.json" <<EOF
{
  "pod_id": "${pod_id}",
  "app_name": "${app_name}",
  "gpu_type": "NVIDIA RTX A4000",
  "created_at": "2026-01-15T10:00:00Z"
}
EOF
}

# create_northflank_state [project_name] [service_name] [service_id]
#
# Creates a Northflank state file as if a deploy had already run.
create_northflank_state() {
    local project_name="${1:-sindri-test}"
    local service_name="${2:-sindri-workspace}"
    local service_id="${3:-sindri-workspace}"

    cat > "${TEST_WORK_DIR}/.sindri/state/northflank.json" <<EOF
{
  "project_name": "${project_name}",
  "service_name": "${service_name}",
  "service_id": "${service_id}",
  "compute_plan": "nf-compute-50",
  "created_at": "2026-01-15T10:00:00Z"
}
EOF
}

# ─── Mock Presets ─────────────────────────────────────────────────────────────

# setup_yq_mock_for_runpod
#
# Configures a yq mock that returns expected values when parsing
# a standard RunPod sindri.yaml.
setup_yq_mock_for_runpod() {
    yq_runpod_callback() {
        local query="$1"
        case "${query}" in
            '.name')
                echo "test-sindri-app" ;;
            *'.runpod.image'*)
                echo "sindri-dev" ;;
            *'.runpod.gpu_type'*)
                echo "NVIDIA RTX A4000" ;;
            *'.runpod.gpu_count'*)
                echo "1" ;;
            *'.runpod.container_disk_gb'*)
                echo "20" ;;
            *'.runpod.volume_size_gb'*)
                echo "50" ;;
            *'.runpod.cloud_type'*)
                echo "COMMUNITY" ;;
            *'.runpod.volume_mount_path'*)
                echo "/workspace" ;;
            *'.provider.name'*)
                echo "runpod" ;;
            *)
                echo "" ;;
        esac
    }
    mock_command_with_callback "yq" "yq_runpod_callback"
}

# setup_yq_mock_for_northflank
#
# Configures a yq mock that returns expected values when parsing
# a standard Northflank sindri.yaml.
setup_yq_mock_for_northflank() {
    yq_northflank_callback() {
        local query="$1"
        case "${query}" in
            '.name')
                echo "test-sindri-app" ;;
            *'.northflank.project_name'*)
                echo "sindri-test" ;;
            *'.northflank.service_name'*)
                echo "sindri-workspace" ;;
            *'.northflank.compute_plan'*)
                echo "nf-compute-50" ;;
            *'.northflank.image'*)
                echo "sindri-dev" ;;
            *'.northflank.volume_size_gb'*)
                echo "10" ;;
            *'.northflank.instances'*)
                echo "1" ;;
            *'.provider.name'*)
                echo "northflank" ;;
            *)
                echo "" ;;
        esac
    }
    mock_command_with_callback "yq" "yq_northflank_callback"
}

# setup_jq_mock
#
# Configures a jq mock that handles common state file queries.
setup_jq_mock() {
    jq_callback() {
        local query="$1"
        local file="${2:-}"
        case "${query}" in
            '-r .pod_id')
                echo "pod-test-123" ;;
            '-r .app_name')
                echo "test-sindri-app" ;;
            '-r .project_name')
                echo "sindri-test" ;;
            '-r .service_id')
                echo "sindri-workspace" ;;
            '-r .service_name')
                echo "sindri-workspace" ;;
            *)
                echo "" ;;
        esac
    }
    mock_command_with_callback "jq" "jq_callback"
}

# ─── Common Assertions ────────────────────────────────────────────────────────

# assert_state_file_exists <provider>
#
# Asserts the state file for the given provider exists in TEST_WORK_DIR.
assert_state_file_exists() {
    local provider="$1"
    local state_file="${TEST_WORK_DIR}/.sindri/state/${provider}.json"
    if [[ ! -f "${state_file}" ]]; then
        echo "FAIL: State file not found: ${state_file}"
        return 1
    fi
}

# assert_state_file_absent <provider>
#
# Asserts the state file for the given provider does NOT exist.
assert_state_file_absent() {
    local provider="$1"
    local state_file="${TEST_WORK_DIR}/.sindri/state/${provider}.json"
    if [[ -f "${state_file}" ]]; then
        echo "FAIL: State file should not exist: ${state_file}"
        return 1
    fi
}

# assert_output_contains <substring>
#
# Asserts that $output (set by bats `run`) contains the given substring.
assert_output_contains() {
    local expected="$1"
    if [[ "${output}" != *"${expected}"* ]]; then
        echo "FAIL: Output does not contain '${expected}'"
        echo "Actual output:"
        echo "${output}" | sed 's/^/  /'
        return 1
    fi
}
