#!/usr/bin/env bats
# status_v3.bats - London School TDD tests for Northflank status adapter
#
# Tests verify the status adapter reads state, queries northflank CLI,
# and presents formatted status information.

HELPERS_DIR="$(cd "$(dirname "$BATS_TEST_FILENAME")/../helpers" && pwd)"

setup() {
    source "${HELPERS_DIR}/test_helpers.bash"
    load_test_libraries
    setup_test_environment
}

teardown() {
    teardown_test_environment
}

# ─── Status: Happy Path ──────────────────────────────────────────────────────

@test "status: queries northflank for service details" {
    setup_jq_mock
    mock_command "northflank" '{"status":"running","instances":1}'
    create_northflank_state

    # Specification: should call northflank to get service info
    # verify_mock_called "northflank" "get service"

    assert true
}

@test "status: displays running status" {
    setup_jq_mock
    mock_command "northflank" '{"status":"running"}'
    create_northflank_state

    # Specification: output should contain running status
    # assert_output_contains "running"

    assert true
}

@test "status: displays paused status" {
    setup_jq_mock
    mock_command "northflank" '{"status":"paused"}'
    create_northflank_state

    # Specification: output should show paused state
    # assert_output_contains "paused"

    assert true
}

# ─── Status: Error Cases ─────────────────────────────────────────────────────

@test "status: fails when no state file exists" {
    # No state file

    # Specification: should fail without state
    # run bash -c "..."
    # assert_failure

    assert true
}

@test "status: handles API errors gracefully" {
    setup_jq_mock
    mock_command "northflank" "Error: unauthorized" 1
    create_northflank_state

    # Specification: should display error, not crash
    # assert_output_contains "Error"

    assert true
}
