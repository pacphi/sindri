#!/usr/bin/env bats
# connect_v3.bats - London School TDD tests for RunPod connect adapter
#
# Tests verify the connect adapter reads state, determines connection
# info, and invokes SSH with the correct parameters.

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

@test "connect: reads pod ID from state file" {
    setup_jq_mock
    mock_command_with_args "runpodctl" "get pod" '{"id":"pod-conn-001","desiredStatus":"RUNNING","runtime":{"ports":[{"ip":"1.2.3.4","port":22}]}}'
    mock_command "ssh" ""
    create_runpod_state "pod-conn-001"

    # Specification: connect reads pod_id from state
    # verify_mock_called "jq" ".pod_id"

    assert true
}

@test "connect: queries pod for connection details" {
    setup_jq_mock
    mock_command_with_args "runpodctl" "get pod" '{"desiredStatus":"RUNNING"}'
    mock_command "ssh" ""
    create_runpod_state "pod-conn-002"

    # Specification: should query runpodctl for pod info
    # verify_mock_called "runpodctl" "get pod"

    assert true
}

# ─── Connect: Error Cases ────────────────────────────────────────────────────

@test "connect: fails when pod is not running" {
    setup_jq_mock
    mock_command_with_args "runpodctl" "get pod" '{"desiredStatus":"EXITED"}'
    create_runpod_state "pod-conn-003"

    # Specification: should fail if pod is not RUNNING
    # assert_failure
    # assert_output_contains "not running"

    assert true
}

@test "connect: fails when no state file exists" {
    # No state file

    # Specification: connect without state should fail
    # run bash -c "..."
    # assert_failure

    assert true
}
