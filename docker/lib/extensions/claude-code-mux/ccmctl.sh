#!/bin/bash
set -euo pipefail

# CCM Control Script
# Manages the Claude Code Mux server lifecycle

CONFIG_FILE="/workspace/config/ccm-config.toml"
PIDFILE="$HOME/.claude-code-mux/ccm.pid"
LOGFILE="$HOME/.claude-code-mux/ccm.log"

case "${1:-start}" in
    start)
        if [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then
            echo "CCM server is already running (PID: $(cat "$PIDFILE"))"
            exit 0
        fi

        if [ ! -f "$CONFIG_FILE" ]; then
            echo "Error: Config file not found at $CONFIG_FILE"
            echo "Run 'ccm-quickstart' to create a configuration"
            exit 1
        fi

        echo "Starting CCM server with config: $CONFIG_FILE"
        mkdir -p "$(dirname "$PIDFILE")"
        nohup ccm start --config "$CONFIG_FILE" > "$LOGFILE" 2>&1 &
        echo $! > "$PIDFILE"
        sleep 2

        if kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then
            echo "CCM server started successfully (PID: $(cat "$PIDFILE"))"
            echo "Admin UI: http://127.0.0.1:13456"
            echo "Config: $CONFIG_FILE"
        else
            echo "Failed to start CCM server. Check logs: $LOGFILE"
            exit 1
        fi
        ;;

    stop)
        if [ -f "$PIDFILE" ]; then
            PID=$(cat "$PIDFILE")
            if kill -0 "$PID" 2>/dev/null; then
                echo "Stopping CCM server (PID: $PID)..."
                kill "$PID"
                rm -f "$PIDFILE"
                echo "CCM server stopped"
            else
                echo "CCM server not running (stale PID file removed)"
                rm -f "$PIDFILE"
            fi
        else
            echo "CCM server is not running"
        fi
        ;;

    restart)
        $0 stop
        sleep 1
        $0 start
        ;;

    status)
        if [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then
            echo "CCM server is running (PID: $(cat "$PIDFILE"))"
            echo "Admin UI: http://127.0.0.1:13456"
            exit 0
        else
            echo "CCM server is not running"
            exit 1
        fi
        ;;

    logs)
        if [ -f "$LOGFILE" ]; then
            tail -f "$LOGFILE"
        else
            echo "No log file found at $LOGFILE"
            exit 1
        fi
        ;;

    *)
        echo "Usage: $0 {start|stop|restart|status|logs}"
        echo ""
        echo "Commands:"
        echo "  start   - Start the CCM server"
        echo "  stop    - Stop the CCM server"
        echo "  restart - Restart the CCM server"
        echo "  status  - Check if CCM server is running"
        echo "  logs    - Tail CCM server logs"
        exit 1
        ;;
esac
