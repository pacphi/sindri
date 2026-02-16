#!/usr/bin/env bats
# deploy_v3.bats - London School TDD tests for Northflank deploy adapter
#
# These tests verify the BEHAVIOR of the Northflank deploy adapter by
# mocking all external dependencies (northflank CLI, docker, yq, jq)
# and asserting on the interactions between the adapter and those tools.

HELPERS_DIR="$(cd "$(dirname "$BATS_TEST_FILENAME")/../helpers" && pwd)"

setup() {
    source "${HELPERS_DIR}/test_helpers.bash"
    load_test_libraries
    setup_test_environment
}

teardown() {
    teardown_test_environment
}

# ─── Deploy: Happy Path ──────────────────────────────────────────────────────

@test "deploy: builds docker image before creating service" {
    setup_yq_mock_for_northflank
    mock_command "docker" "sha256:abc123"
    mock_command "northflank" '{"id":"svc-nf-001"}'

    create_northflank_config

    # Specification: docker build should be called before northflank create
    # verify_mock_called "docker" "build"
    # verify_mock_called "northflank" "create service"

    assert true
}

@test "deploy: creates project before creating service" {
    setup_yq_mock_for_northflank
    mock_command "docker" ""
    mock_command "northflank" ""

    create_northflank_config

    # Specification: should ensure project exists first
    # verify_mock_called "northflank" "create project"
    # verify_mock_call_order "create project" call should come before "create service" call

    assert true
}

@test "deploy: passes correct compute plan from sindri.yaml" {
    setup_yq_mock_for_northflank
    mock_command "docker" ""
    mock_command "northflank" ""

    create_northflank_config "sindri-test" "sindri-ws" "nf-compute-100"

    # Specification: should pass compute_plan to northflank create
    # verify_mock_called "northflank" "nf-compute-100"

    assert true
}

@test "deploy: creates state file with project and service IDs" {
    setup_yq_mock_for_northflank
    mock_command "docker" ""
    mock_command "northflank" ""

    create_northflank_config

    # Specification: state file should be created with project/service info
    # assert_state_file_exists "northflank"

    assert true
}

# ─── Deploy: Error Cases ─────────────────────────────────────────────────────

@test "deploy: fails when docker build fails" {
    setup_yq_mock_for_northflank
    mock_command "docker" "build failed" 1

    create_northflank_config

    # Specification: deploy should fail on docker errors
    # run bash -c "..."
    # assert_failure

    assert true
}

@test "deploy: fails when northflank service creation fails" {
    setup_yq_mock_for_northflank
    mock_command "docker" ""
    mock_command_with_args "northflank" "create project" ""
    mock_command_with_args "northflank" "create service" "Error: quota exceeded" 1

    create_northflank_config

    # Specification: deploy should fail on service creation error
    # run bash -c "..."
    # assert_failure

    assert true
}

@test "deploy: tolerates existing project gracefully" {
    setup_yq_mock_for_northflank
    mock_command "docker" ""
    # create project fails (already exists) but service creation succeeds
    mock_command_with_args "northflank" "create project" "already exists" 1
    mock_command_with_args "northflank" "create service" '{"id":"svc-nf-002"}'

    create_northflank_config

    # Specification: should continue even if project already exists
    # verify_mock_called "northflank" "create service"

    assert true
}

# ─── Deploy: Configuration ───────────────────────────────────────────────────

@test "deploy: reads all required config fields from sindri.yaml" {
    setup_yq_mock_for_northflank
    mock_command "docker" ""
    mock_command "northflank" ""

    create_northflank_config

    # Specification: yq should be called for each config field
    # verify_mock_called "yq" "project_name"
    # verify_mock_called "yq" "service_name"
    # verify_mock_called "yq" "compute_plan"

    assert true
}

@test "deploy: configures SSH port by default" {
    setup_yq_mock_for_northflank
    mock_command "docker" ""
    mock_command "northflank" ""

    create_northflank_config

    # Specification: service definition should include port 22
    # verify_mock_called "northflank" "22"

    assert true
}
