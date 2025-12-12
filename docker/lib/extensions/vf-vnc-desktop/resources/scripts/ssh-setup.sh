#!/bin/bash
# SSH Credential Setup Helper for Agentic Workstation
# Provides options for configuring SSH credentials in the container

set -e

CONTAINER_NAME="${CONTAINER_NAME:-agentic-workstation}"

show_usage() {
    cat <<EOF
SSH Credential Setup Helper

Usage:
  $0 <command>

Commands:
  status      - Show SSH configuration status
  verify      - Verify SSH keys are accessible
  copy        - Copy specific SSH key to container (if not using mount)
  agent       - Show SSH agent status
  test        - Test SSH connection to a host

Examples:
  # Check SSH status
  $0 status

  # Verify keys are accessible
  $0 verify

  # Test SSH connection
  $0 test git@github.com

Environment Variables:
  CONTAINER_NAME  - Container name (default: agentic-workstation)

Notes:
  - SSH credentials are mounted from host ~/.ssh (read-only)
  - No rebuild needed - just restart container to update keys
  - SSH agent auto-starts in each shell session
EOF
}

cmd_status() {
    echo "=== SSH Configuration Status ==="
    echo ""

    # Check if container is running
    if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        echo "❌ Container '${CONTAINER_NAME}' is not running"
        exit 1
    fi

    # Check SSH mount
    if docker exec "$CONTAINER_NAME" test -d /home/devuser/.ssh 2>/dev/null; then
        echo "✓ SSH directory mounted: /home/devuser/.ssh"

        # Count keys
        PRIVATE_KEYS=$(docker exec "$CONTAINER_NAME" find /home/devuser/.ssh -type f -name "id_*" ! -name "*.pub" 2>/dev/null | wc -l)
        PUBLIC_KEYS=$(docker exec "$CONTAINER_NAME" find /home/devuser/.ssh -type f -name "*.pub" 2>/dev/null | wc -l)

        echo "  - Private keys: $PRIVATE_KEYS"
        echo "  - Public keys: $PUBLIC_KEYS"

        # List keys
        if [ "$PRIVATE_KEYS" -gt 0 ]; then
            echo ""
            echo "Available keys:"
            docker exec "$CONTAINER_NAME" ls -lh /home/devuser/.ssh/id_* 2>/dev/null | grep -v ".pub$" || true
        fi
    else
        echo "⚠️  SSH directory not mounted"
        echo ""
        echo "To enable SSH credentials:"
        echo "  1. Ensure docker-compose.unified.yml has the volume mount:"
        echo "     - \${HOME}/.ssh:/home/devuser/.ssh:ro"
        echo "  2. Restart container: docker-compose -f docker-compose.unified.yml restart"
    fi
}

cmd_verify() {
    echo "=== Verifying SSH Keys ==="
    echo ""

    if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        echo "❌ Container '${CONTAINER_NAME}' is not running"
        exit 1
    fi

    # Check for keys
    KEYS=$(docker exec "$CONTAINER_NAME" find /home/devuser/.ssh -type f -name "id_*" ! -name "*.pub" 2>/dev/null || echo "")

    if [ -z "$KEYS" ]; then
        echo "❌ No SSH private keys found"
        exit 1
    fi

    echo "Found SSH keys:"
    for key in $KEYS; do
        KEY_NAME=$(basename "$key")
        # Check permissions
        PERMS=$(docker exec "$CONTAINER_NAME" stat -c "%a" "$key" 2>/dev/null || echo "unknown")

        # Get key type
        KEY_TYPE=$(docker exec "$CONTAINER_NAME" ssh-keygen -l -f "$key" 2>/dev/null | awk '{print $4}' | tr -d '()' || echo "unknown")

        echo "  ✓ $KEY_NAME"
        echo "    Type: $KEY_TYPE"
        echo "    Permissions: $PERMS"

        # Check if public key exists
        if docker exec "$CONTAINER_NAME" test -f "${key}.pub" 2>/dev/null; then
            echo "    Public key: ✓"
        else
            echo "    Public key: ⚠️  missing"
        fi
        echo ""
    done

    echo "✓ SSH verification complete"
}

cmd_agent() {
    echo "=== SSH Agent Status ==="
    echo ""

    if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        echo "❌ Container '${CONTAINER_NAME}' is not running"
        exit 1
    fi

    # Check SSH agent
    docker exec -u devuser "$CONTAINER_NAME" bash -c '
        if [ -n "$SSH_AUTH_SOCK" ]; then
            echo "✓ SSH agent is running"
            echo "  Socket: $SSH_AUTH_SOCK"
            echo ""
            echo "Loaded keys:"
            ssh-add -l || echo "  (no keys loaded)"
        else
            echo "⚠️  SSH agent not running"
            echo ""
            echo "Start SSH agent:"
            echo "  eval \"\$(ssh-agent -s)\""
            echo "  ssh-add ~/.ssh/id_rsa  # or your key name"
        fi
    '
}

cmd_test() {
    local HOST="${1:-git@github.com}"

    echo "=== Testing SSH Connection ==="
    echo "Host: $HOST"
    echo ""

    if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        echo "❌ Container '${CONTAINER_NAME}' is not running"
        exit 1
    fi

    # Test connection
    echo "Testing connection (this may take a few seconds)..."
    docker exec -u devuser "$CONTAINER_NAME" bash -c "
        eval \"\$(ssh-agent -s)\" > /dev/null 2>&1
        find ~/.ssh -type f -name 'id_*' ! -name '*.pub' -exec ssh-add {} \; 2>/dev/null
        ssh -T -o StrictHostKeyChecking=no $HOST
    " || echo ""

    echo ""
    echo "✓ Connection test complete"
}

cmd_copy() {
    local KEY_FILE="${1}"

    if [ -z "$KEY_FILE" ]; then
        echo "Error: Key file path required"
        echo "Usage: $0 copy <key_file_path>"
        echo "Example: $0 copy ~/.ssh/id_rsa"
        exit 1
    fi

    if [ ! -f "$KEY_FILE" ]; then
        echo "❌ Key file not found: $KEY_FILE"
        exit 1
    fi

    echo "⚠️  Manual copy mode (not recommended - use volume mount instead)"
    echo ""
    echo "Copying: $KEY_FILE"

    # Copy private key
    docker cp "$KEY_FILE" "${CONTAINER_NAME}:/tmp/ssh_key_temp"
    docker exec "$CONTAINER_NAME" bash -c "
        mkdir -p /home/devuser/.ssh
        mv /tmp/ssh_key_temp /home/devuser/.ssh/$(basename $KEY_FILE)
        chmod 600 /home/devuser/.ssh/$(basename $KEY_FILE)
        chown devuser:devuser /home/devuser/.ssh/$(basename $KEY_FILE)
    "

    # Copy public key if exists
    if [ -f "${KEY_FILE}.pub" ]; then
        docker cp "${KEY_FILE}.pub" "${CONTAINER_NAME}:/tmp/ssh_key_temp.pub"
        docker exec "$CONTAINER_NAME" bash -c "
            mv /tmp/ssh_key_temp.pub /home/devuser/.ssh/$(basename $KEY_FILE).pub
            chmod 644 /home/devuser/.ssh/$(basename $KEY_FILE).pub
            chown devuser:devuser /home/devuser/.ssh/$(basename $KEY_FILE).pub
        "
    fi

    echo "✓ Key copied successfully"
    echo ""
    echo "Note: Manual copies will be lost on container restart"
    echo "Recommendation: Use volume mount in docker-compose.unified.yml"
}

# Main command dispatcher
case "${1:-}" in
    status)
        cmd_status
        ;;
    verify)
        cmd_verify
        ;;
    agent)
        cmd_agent
        ;;
    test)
        cmd_test "$2"
        ;;
    copy)
        cmd_copy "$2"
        ;;
    help|--help|-h)
        show_usage
        ;;
    "")
        show_usage
        exit 1
        ;;
    *)
        echo "Unknown command: $1"
        echo ""
        show_usage
        exit 1
        ;;
esac
