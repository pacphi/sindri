#!/bin/bash
# Management API Health Check Script
# Runs after supervisord startup to verify Management API is responding

set -e

MANAGEMENT_API_HOST="${MANAGEMENT_API_HOST:-localhost}"
MANAGEMENT_API_PORT="${MANAGEMENT_API_PORT:-9090}"
MAX_RETRIES=30
RETRY_DELAY=2

echo "=== Management API Health Check ==="
echo "Target: http://${MANAGEMENT_API_HOST}:${MANAGEMENT_API_PORT}/health"
echo "Max retries: ${MAX_RETRIES} (${RETRY_DELAY}s interval)"
echo ""

for i in $(seq 1 $MAX_RETRIES); do
    if curl -s -f "http://${MANAGEMENT_API_HOST}:${MANAGEMENT_API_PORT}/health" > /dev/null 2>&1; then
        RESPONSE=$(curl -s "http://${MANAGEMENT_API_HOST}:${MANAGEMENT_API_PORT}/health")
        echo "✅ Management API is healthy (attempt $i/$MAX_RETRIES)"
        echo "   Response: $RESPONSE"
        echo ""
        echo "=== Health Check Passed ==="
        exit 0
    else
        echo "⏳ Attempt $i/$MAX_RETRIES: Management API not ready yet..."

        # Check if process is running
        if /opt/venv/bin/supervisorctl status management-api | grep -q "RUNNING"; then
            echo "   Process status: RUNNING (waiting for HTTP response)"
        else
            echo "   ⚠️  Process not running! Attempting restart..."
            /opt/venv/bin/supervisorctl restart management-api
        fi

        sleep $RETRY_DELAY
    fi
done

# If we get here, health check failed
echo "❌ Management API health check FAILED after ${MAX_RETRIES} attempts"
echo ""
echo "Diagnostic information:"
/opt/venv/bin/supervisorctl status management-api
echo ""
echo "Recent logs:"
/opt/venv/bin/supervisorctl tail management-api stderr
echo ""
echo "=== Health Check Failed ==="
exit 1
