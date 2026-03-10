#!/usr/bin/env bash
set -euo pipefail

# Gracefully stop and remove draupnir agent.
# Called by the remove.script block in extension.yaml.

PID_FILE="${HOME}/.sindri/draupnir.pid"
LOG_FILE="${HOME}/.sindri/logs/draupnir.log"
DRAUPNIR_BIN="${HOME}/.local/bin/draupnir"

# Stop the daemon
if [ -f "$PID_FILE" ]; then
  PID=$(cat "$PID_FILE")
  if kill -0 "$PID" 2>/dev/null; then
    echo "Stopping draupnir (PID $PID)..."
    kill -TERM "$PID" 2>/dev/null || true
    # Wait up to 10 seconds for graceful shutdown
    for i in $(seq 1 10); do
      if ! kill -0 "$PID" 2>/dev/null; then
        echo "draupnir stopped gracefully"
        break
      fi
      sleep 1
    done
    # Force kill if still running
    if kill -0 "$PID" 2>/dev/null; then
      echo "Force killing draupnir..."
      kill -KILL "$PID" 2>/dev/null || true
    fi
  fi
  rm -f "$PID_FILE"
fi

# Clean up files
rm -f "$DRAUPNIR_BIN"
rm -f "$LOG_FILE"
echo "draupnir removed"
