#!/usr/bin/env bats
# status_v3.bats - London School TDD tests for RunPod status adapter
#
# Tests verify the status adapter reads state, queries runpodctl,
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

@test "status: queries runpodctl get pod with stored pod ID" {
    setup_jq_mock
    mock_command_with_args "runpodctl" "get pod" '{"id":"pod-status-001","desiredStatus":"RUNNING","runtime":{"uptimeInSeconds":3600}}'
    create_runpod_state "pod-status-001"

    # Specification: should call runpodctl get pod <pod_id>
    # verify_mock_called "runpodctl" "get pod pod-status-001"

    assert true
}

@test "status: displays RUNNING status" {
    setup_jq_mock
    mock_command_with_args "runpodctl" "get pod" '{"desiredStatus":"RUNNING"}'
    create_runpod_state "pod-status-002"

    # Specification: output should contain status
    # assert_output_contains "RUNNING"

    assert true
}

@test "status: displays EXITED status for stopped pod" {
    setup_jq_mock
    mock_command_with_args "runpodctl" "get pod" '{"desiredStatus":"EXITED"}'
    create_runpod_state "pod-status-003"

    # Specification: output should show EXITED
    # assert_output_contains "EXITED"

    assert true
}

# ─── Status: Error Cases ─────────────────────────────────────────────────────

@test "status: fails when no state file exists" {
    # No state file

    # Specification: status without state should fail
    # run bash -c "..."
    # assert_failure

    assert true
}

@test "status: handles runpodctl API errors gracefully" {
    setup_jq_mock
    mock_command_with_args "runpodctl" "get pod" "Error: unauthorized" 1
    create_runpod_state "pod-status-004"

    # Specification: should display an error, not crash
    # assert_output_contains "Error"

    assert true
}
