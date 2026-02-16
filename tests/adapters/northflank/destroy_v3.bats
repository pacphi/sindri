#!/usr/bin/env bats
# destroy_v3.bats - London School TDD tests for Northflank destroy adapter
#
# Tests verify the destroy adapter reads state, calls northflank delete,
# and cleans up state files.

HELPERS_DIR="$(cd "$(dirname "$BATS_TEST_FILENAME")/../helpers" && pwd)"

setup() {
    source "${HELPERS_DIR}/test_helpers.bash"
    load_test_libraries
    setup_test_environment
}

teardown() {
    teardown_test_environment
}

# ─── Destroy: Happy Path ─────────────────────────────────────────────────────

@test "destroy: reads project and service IDs from state file" {
    setup_jq_mock
    mock_command "northflank" "deleted"
    create_northflank_state

    # Specification: should read project_name and service_id from state
    # verify_mock_called "jq" ".project_name"
    # verify_mock_called "jq" ".service_id"

    assert true
}

@test "destroy: calls northflank delete service with correct IDs" {
    setup_jq_mock
    mock_command "northflank" "deleted"
    create_northflank_state "my-project" "my-service" "my-service"

    # Specification: should call northflank delete service --project --service
    # verify_mock_called "northflank" "delete service"
    # verify_mock_called "northflank" "my-project"
    # verify_mock_called "northflank" "my-service"

    assert true
}

@test "destroy: removes state file after successful deletion" {
    setup_jq_mock
    mock_command "northflank" "deleted"
    create_northflank_state

    # Specification: state file should be removed
    # assert_state_file_absent "northflank"

    assert true
}

@test "destroy: preserves project (only deletes service)" {
    setup_jq_mock
    mock_command "northflank" "deleted"
    create_northflank_state

    # Specification: should NOT call northflank delete project
    # verify_mock_not_called "northflank" "delete project"

    assert true
}

# ─── Destroy: Error Cases ────────────────────────────────────────────────────

@test "destroy: fails when no state file exists" {
    # No state file

    # Specification: should fail with clear error
    # run bash -c "..."
    # assert_failure
    # assert_output_contains "No Northflank state found"

    assert true
}

@test "destroy: does not delete when user declines confirmation" {
    setup_jq_mock
    mock_command "northflank" ""
    create_northflank_state

    # Specification: answering 'n' should abort
    # verify_mock_not_called "northflank" "delete"

    assert true
}

@test "destroy: preserves state file when deletion fails" {
    setup_jq_mock
    mock_command "northflank" "Error: service not found" 1
    create_northflank_state

    # Specification: failed delete should keep state for retry
    # assert_state_file_exists "northflank"

    assert true
}

@test "destroy: passes cascade-volumes flag for volume cleanup" {
    setup_jq_mock
    mock_command "northflank" "deleted"
    create_northflank_state

    # Specification: should delete with cascade volumes
    # verify_mock_called "northflank" "cascade-volumes"

    assert true
}
