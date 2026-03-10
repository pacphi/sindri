#!/usr/bin/env bash
set -euo pipefail

# Start draupnir agent as a background daemon after extension installation.
# Called by the post-install hook in extension.yaml.

DRAUPNIR_BIN="${HOME}/.local/bin/draupnir"
PID_FILE="${HOME}/.sindri/draupnir.pid"
LOG_FILE="${HOME}/.sindri/logs/draupnir.log"

mkdir -p "$(dirname "$PID_FILE")" "$(dirname "$LOG_FILE")"

# Check if already running
if [ -f "$PID_FILE" ]; then
  OLD_PID=$(cat "$PID_FILE")
  if kill -0 "$OLD_PID" 2>/dev/null; then
    echo "draupnir already running (PID $OLD_PID)"
    exit 0
  fi
  rm -f "$PID_FILE"
fi

# Verify binary exists
if [ ! -x "$DRAUPNIR_BIN" ]; then
  echo "draupnir binary not found at $DRAUPNIR_BIN"
  exit 1
fi

# Require console URL for agent connectivity
if [ -z "${SINDRI_CONSOLE_URL:-}" ]; then
  echo "SINDRI_CONSOLE_URL not set — draupnir cannot connect to Mimir"
  echo "Agent will need to be started manually after configuration"
  exit 0
fi

echo "Starting draupnir agent..."
echo "  Console: ${SINDRI_CONSOLE_URL}"
echo "  Log: ${LOG_FILE}"

nohup "$DRAUPNIR_BIN" > "$LOG_FILE" 2>&1 &
DRAUPNIR_PID=$!
echo "$DRAUPNIR_PID" > "$PID_FILE"

# Brief wait to check it didn't crash immediately
sleep 2
if kill -0 "$DRAUPNIR_PID" 2>/dev/null; then
  echo "draupnir started (PID $DRAUPNIR_PID)"
else
  echo "draupnir failed to start — check $LOG_FILE"
  rm -f "$PID_FILE"
  exit 1
fi
