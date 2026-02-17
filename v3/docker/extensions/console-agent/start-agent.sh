#!/bin/bash
set -euo pipefail

# start-agent.sh
# Starts the sindri-agent as a background service.
# Supports systemd user services (if available) or a background process with PID tracking.
# Also enables auto-start on boot via systemd or .bashrc.

BINARY_PATH="${HOME}/.local/bin/sindri-agent"
CONFIG_FILE="${HOME}/.config/sindri-agent/config.yaml"
PID_FILE="/tmp/sindri-agent.pid"
LOG_FILE="/tmp/sindri-agent.log"
SERVICE_NAME="sindri-agent"
EXTENSION_DIR="${HOME}/.local/share/sindri/extensions/console-agent"

start_background_process() {
    print_status "Starting sindri-agent as background process..."
    nohup "${BINARY_PATH}" --config "${CONFIG_FILE}" >> "${LOG_FILE}" 2>&1 &
    local agent_pid=$!
    echo "${agent_pid}" > "${PID_FILE}"

    # Wait briefly and verify startup
    sleep 2
    if kill -0 "${agent_pid}" 2>/dev/null; then
        print_success "sindri-agent started (PID: ${agent_pid})"
    else
        print_error "sindri-agent failed to start - check logs: ${LOG_FILE}"
        rm -f "${PID_FILE}"
        exit 1
    fi
}

enable_bashrc_autostart() {
    local startup_file="${HOME}/.bashrc"
    local autostart_marker="# sindri-agent autostart"

    if ! grep -q "${autostart_marker}" "${startup_file}" 2>/dev/null; then
        print_status "Enabling auto-start on login in ${startup_file}..."
        cat >> "${startup_file}" <<BASHRC

${autostart_marker}
if [[ -f "${HOME}/.local/bin/sindri-agent" ]] && ! kill -0 "\$(cat /tmp/sindri-agent.pid 2>/dev/null)" 2>/dev/null; then
    bash "${EXTENSION_DIR}/start-agent.sh" >/dev/null 2>&1 &
fi
BASHRC
        print_success "Auto-start enabled in ${startup_file}"
    fi
}

print_status "Starting sindri-agent..."

# Verify binary exists
if [[ ! -f "${BINARY_PATH}" ]]; then
    print_error "sindri-agent binary not found at ${BINARY_PATH}"
    print_error "Run the install script first: sindri extension install console-agent"
    exit 1
fi

# Ensure config exists, run configure if missing
if [[ ! -f "${CONFIG_FILE}" ]]; then
    print_warning "Config file not found at ${CONFIG_FILE}"
    print_status "Running configure-agent.sh first..."
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    if [[ -f "${SCRIPT_DIR}/configure-agent.sh" ]]; then
        bash "${SCRIPT_DIR}/configure-agent.sh"
    else
        print_error "configure-agent.sh not found at ${SCRIPT_DIR} - cannot auto-configure"
        exit 1
    fi
fi

# Stop any existing instance
if [[ -f "${PID_FILE}" ]]; then
    existing_pid=$(cat "${PID_FILE}" 2>/dev/null || echo "")
    if [[ -n "${existing_pid}" ]] && kill -0 "${existing_pid}" 2>/dev/null; then
        print_status "Stopping existing sindri-agent (PID: ${existing_pid})..."
        kill "${existing_pid}" 2>/dev/null || true
        sleep 1
    fi
    rm -f "${PID_FILE}"
fi

# Prefer systemd user service if available
USING_SYSTEMD=false
if command -v systemctl >/dev/null 2>&1 && systemctl --user daemon-reload >/dev/null 2>&1; then
    print_status "Setting up systemd user service..."
    SERVICE_DIR="${HOME}/.config/systemd/user"
    mkdir -p "${SERVICE_DIR}"

    cat > "${SERVICE_DIR}/${SERVICE_NAME}.service" <<EOF
[Unit]
Description=Sindri Console Agent
Documentation=https://github.com/pacphi/sindri
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=${BINARY_PATH} --config ${CONFIG_FILE}
Restart=on-failure
RestartSec=10
StandardOutput=append:${LOG_FILE}
StandardError=append:${LOG_FILE}
Environment=HOME=${HOME}

[Install]
WantedBy=default.target
EOF

    systemctl --user daemon-reload
    systemctl --user enable "${SERVICE_NAME}.service" >/dev/null 2>&1 || true
    systemctl --user restart "${SERVICE_NAME}.service"

    sleep 2
    if systemctl --user is-active --quiet "${SERVICE_NAME}.service"; then
        USING_SYSTEMD=true
        print_success "sindri-agent started via systemd user service (auto-restart on failure enabled)"
        print_status "View status: systemctl --user status ${SERVICE_NAME}"
        print_status "View logs:   journalctl --user -u ${SERVICE_NAME} -f"
    else
        print_warning "systemd service failed to start - falling back to background process"
        systemctl --user disable "${SERVICE_NAME}.service" >/dev/null 2>&1 || true
    fi
fi

# Fall back to background process if systemd is not available or failed
if [[ "${USING_SYSTEMD}" == "false" ]]; then
    start_background_process
    enable_bashrc_autostart
    print_status "View logs: ${LOG_FILE}"
    print_status "PID file:  ${PID_FILE}"
fi

print_success "sindri-agent is running"
