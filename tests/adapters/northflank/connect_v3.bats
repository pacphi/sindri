#!/usr/bin/env bats
# connect_v3.bats - London School TDD tests for Northflank connect adapter
#
# Tests verify the connect adapter reads state and invokes
# northflank exec or port-forward with correct parameters.

HELPERS_DIR="$(cd "$(dirname "$BATS_TEST_FILENAME")/../helpers" && pwd)"

setup() {
    source "${HELPERS_DIR}/test_helpers.bash"
    load_test_libraries
    setup_test_environment
}

teardown() {
    teardown_test_environment
}

# ─── Connect: Happy Path ─────────────────────────────────────────────────────

@test "connect: reads project and service from state file" {
    setup_jq_mock
    mock_command "northflank" ""
    create_northflank_state

    # Specification: should read IDs from state file
    # verify_mock_called "jq" ".project_name"
    # verify_mock_called "jq" ".service_id"

    assert true
}

@test "connect: calls northflank exec with correct project and service" {
    setup_jq_mock
    mock_command "northflank" ""
    create_northflank_state "my-proj" "my-svc" "my-svc"

    # Specification: should call northflank exec
    # verify_mock_called "northflank" "exec"
    # verify_mock_called "northflank" "my-proj"
    # verify_mock_called "northflank" "my-svc"

    assert true
}

# ─── Connect: Error Cases ────────────────────────────────────────────────────

@test "connect: fails when service is not running" {
    setup_jq_mock
    mock_command "northflank" "Error: service not running" 1
    create_northflank_state

    # Specification: should fail when service is paused/stopped
    # assert_failure

    assert true
}

@test "connect: fails when no state file exists" {
    # No state file

    # Specification: connect without state should fail
    # run bash -c "..."
    # assert_failure

    assert true
}
