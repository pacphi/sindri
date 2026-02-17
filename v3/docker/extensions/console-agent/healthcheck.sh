#!/bin/bash
set -euo pipefail

# healthcheck.sh
# Verifies that sindri-agent is running and healthy.
# Exits 0 if healthy, non-zero if not.

BINARY_PATH="${HOME}/.local/bin/sindri-agent"
PID_FILE="/tmp/sindri-agent.pid"
LOG_FILE="/tmp/sindri-agent.log"
SERVICE_NAME="sindri-agent"

HEALTHY=true

print_status "Checking sindri-agent health..."

# 1. Verify binary is installed
if [[ ! -f "${BINARY_PATH}" ]]; then
    print_error "FAIL: sindri-agent binary not found at ${BINARY_PATH}"
    HEALTHY=false
else
    print_success "OK:   Binary present at ${BINARY_PATH}"
fi

# 2. Check systemd service if available
USING_SYSTEMD=false
if command -v systemctl >/dev/null 2>&1 && systemctl --user list-units --type=service 2>/dev/null | grep -q "${SERVICE_NAME}"; then
    USING_SYSTEMD=true
    if systemctl --user is-active --quiet "${SERVICE_NAME}.service" 2>/dev/null; then
        print_success "OK:   systemd user service '${SERVICE_NAME}' is active"
    else
        service_state=$(systemctl --user show "${SERVICE_NAME}.service" --property=ActiveState 2>/dev/null | cut -d= -f2 || echo "unknown")
        print_error "FAIL: systemd user service '${SERVICE_NAME}' is not active (state: ${service_state})"
        HEALTHY=false
    fi
fi

# 3. Check PID file and process liveness (for non-systemd or additional verification)
if [[ "${USING_SYSTEMD}" == "false" ]]; then
    if [[ ! -f "${PID_FILE}" ]]; then
        print_error "FAIL: PID file not found at ${PID_FILE}"
        HEALTHY=false
    else
        pid=$(cat "${PID_FILE}" 2>/dev/null || echo "")
        if [[ -z "${pid}" ]]; then
            print_error "FAIL: PID file is empty"
            HEALTHY=false
        elif kill -0 "${pid}" 2>/dev/null; then
            print_success "OK:   Process is running (PID: ${pid})"
        else
            print_error "FAIL: Process with PID ${pid} is not running (stale PID file)"
            HEALTHY=false
        fi
    fi
fi

# 4. Check configuration file exists
CONFIG_FILE="${HOME}/.config/sindri-agent/config.yaml"
if [[ ! -f "${CONFIG_FILE}" ]]; then
    print_error "FAIL: Config file not found at ${CONFIG_FILE}"
    print_status "      Run configure-agent.sh to generate it"
    HEALTHY=false
else
    print_success "OK:   Config file present at ${CONFIG_FILE}"
fi

# 5. Check console URL is configured
if [[ -f "${CONFIG_FILE}" ]]; then
    console_url=$(grep -E '^\s*url\s*:' "${CONFIG_FILE}" 2>/dev/null | \
                  sed 's/.*url\s*:\s*//' | \
                  tr -d "\"'" | \
                  head -n1 | \
                  xargs 2>/dev/null || true)
    if [[ -z "${console_url}" ]]; then
        print_warning "WARN: Console URL is not configured in ${CONFIG_FILE}"
        print_warning "      Set SINDRI_CONSOLE_URL and re-run configure-agent.sh"
    else
        print_success "OK:   Console URL: ${console_url}"
    fi
fi

# 6. Check log file for recent activity
if [[ -f "${LOG_FILE}" ]]; then
    log_size=$(wc -c < "${LOG_FILE}" 2>/dev/null || echo "0")
    if [[ "${log_size}" -gt 0 ]]; then
        print_success "OK:   Log file present (${log_size} bytes): ${LOG_FILE}"
        # Show last few lines of log for context
        print_status "      Recent log entries:"
        tail -n 5 "${LOG_FILE}" 2>/dev/null | while IFS= read -r line; do
            print_status "        ${line}"
        done
    else
        print_warning "WARN: Log file is empty: ${LOG_FILE}"
    fi
else
    print_warning "WARN: Log file not found at ${LOG_FILE} (agent may not have produced output yet)"
fi

# Summary
echo ""
if [[ "${HEALTHY}" == "true" ]]; then
    print_success "Health check PASSED: sindri-agent is running and healthy"
    exit 0
else
    print_error "Health check FAILED: sindri-agent is not healthy"
    print_status "Troubleshooting steps:"
    print_status "  1. Re-run install:   sindri extension install console-agent"
    print_status "  2. Re-configure:     configure-agent.sh"
    print_status "  3. Restart agent:    start-agent.sh"
    print_status "  4. View logs:        ${LOG_FILE}"
    exit 1
fi
