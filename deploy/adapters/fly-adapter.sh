#!/bin/bash
# Fly.io adapter - Enhanced with comprehensive fly.toml generation
#
# Usage:
#   fly-adapter.sh [OPTIONS] [sindri.yaml]
#
# Options:
#   --config-only    Generate fly.toml without deploying
#   --output-dir     Directory for generated files (default: current directory)
#   --output-vars    Output parsed variables for CI integration (JSON to stdout)
#   --app-name       Override app name from sindri.yaml
#   --help           Show this help message
#
# Examples:
#   fly-adapter.sh                           # Deploy using ./sindri.yaml
#   fly-adapter.sh --config-only             # Just generate fly.toml
#   fly-adapter.sh --output-dir /tmp myapp.yaml  # Generate to specific directory

set -e

# shellcheck disable=SC2034  # May be used in future adapter implementations
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Default values
SINDRI_YAML=""
CONFIG_ONLY=false
OUTPUT_DIR="."
OUTPUT_VARS=false
APP_NAME_OVERRIDE=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --config-only)
            CONFIG_ONLY=true
            shift
            ;;
        --output-dir)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --output-vars)
            OUTPUT_VARS=true
            shift
            ;;
        --app-name)
            APP_NAME_OVERRIDE="$2"
            shift 2
            ;;
        --help)
            head -20 "$0" | tail -18
            exit 0
            ;;
        -*)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
        *)
            SINDRI_YAML="$1"
            shift
            ;;
    esac
done

# Default sindri.yaml if not specified
SINDRI_YAML="${SINDRI_YAML:-sindri.yaml}"

if [[ ! -f "$SINDRI_YAML" ]]; then
    echo "Error: $SINDRI_YAML not found" >&2
    exit 1
fi

# Source common utilities and secrets manager
source "$BASE_DIR/docker/lib/common.sh"
if [[ "$CONFIG_ONLY" != "true" ]]; then
    source "$BASE_DIR/cli/secrets-manager"
fi

# Parse sindri.yaml
NAME=$(yq '.name' "$SINDRI_YAML")
# Apply app name override if provided
[[ -n "$APP_NAME_OVERRIDE" ]] && NAME="$APP_NAME_OVERRIDE"

MEMORY=$(yq '.deployment.resources.memory // "2GB"' "$SINDRI_YAML" | sed 's/GB/*1024/;s/MB//')
CPUS=$(yq '.deployment.resources.cpus // 1' "$SINDRI_YAML")
REGION=$(yq '.providers.fly.region // "sjc"' "$SINDRI_YAML")
ORG=$(yq '.providers.fly.organization // "personal"' "$SINDRI_YAML")
PROFILE=$(yq '.extensions.profile // ""' "$SINDRI_YAML")
CUSTOM_EXTENSIONS=$(yq '.extensions.active[]? // ""' "$SINDRI_YAML" | tr '\n' ',' | sed 's/,$//')
VOLUME_SIZE=$(yq '.deployment.volumes.workspace.size // "10GB"' "$SINDRI_YAML" | sed 's/GB//')
AUTO_STOP=$(yq '.providers.fly.autoStopMachines // true' "$SINDRI_YAML")
AUTO_START=$(yq '.providers.fly.autoStartMachines // true' "$SINDRI_YAML")
CPU_KIND=$(yq '.providers.fly.cpuKind // "shared"' "$SINDRI_YAML")
SSH_EXTERNAL_PORT=$(yq '.providers.fly.sshPort // 10022' "$SINDRI_YAML")

# Calculate memory in MB
MEMORY_MB=$(echo "$MEMORY" | bc)

# Calculate swap (1/2 of memory, min 2GB)
SWAP_MB=$((MEMORY_MB / 2))
[[ $SWAP_MB -lt 2048 ]] && SWAP_MB=2048

# Determine auto_stop mode
AUTO_STOP_MODE="suspend"
[[ "$AUTO_STOP" == "false" ]] && AUTO_STOP_MODE="off"

# Output variables for CI integration if requested
if [[ "$OUTPUT_VARS" == "true" ]]; then
    cat << EOJSON
{
  "name": "$NAME",
  "region": "$REGION",
  "organization": "$ORG",
  "profile": "$PROFILE",
  "memory_mb": $MEMORY_MB,
  "cpus": $CPUS,
  "volume_size": $VOLUME_SIZE,
  "ssh_port": $SSH_EXTERNAL_PORT,
  "cpu_kind": "$CPU_KIND"
}
EOJSON
    exit 0
fi

# Ensure output directory exists
mkdir -p "$OUTPUT_DIR"

# Generate fly.toml with comprehensive configuration
cat > "$OUTPUT_DIR/fly.toml" << EOFT
# fly.toml configuration for Sindri
# AI-powered cloud development forge with cost-effective remote development,
# scale-to-zero capabilities, and persistent storage

app = "${NAME}"
# Change to your preferred region
# Consult https://fly.io/docs/reference/regions/ for available regions
primary_region = "${REGION}"

# Build configuration
[build]
  dockerfile = "docker/Dockerfile"

# Environment variables
[env]
  # User configuration
  DEV_USER = "developer"
  # SSH port (internal) - use 2222 to avoid conflicts with Fly.io's hallpass service on port 22
  SSH_PORT = "2222"
  # Timezone
  TZ = "UTC"
  # Extension profile or custom list
  INSTALL_PROFILE = "${PROFILE}"
  CUSTOM_EXTENSIONS = "${CUSTOM_EXTENSIONS}"
  # Workspace initialization
  INIT_WORKSPACE = "true"

# Volume mounts for persistent storage
[mounts]
  # Mount persistent volume as developer's home directory
  # This ensures $HOME is persistent and contains workspace, config, and tool data
  source = "home_data"
  destination = "/alt/home/developer"
  # Initial size matches the volume size specified during creation
  initial_size = "${VOLUME_SIZE}gb"
  # Keep snapshots for a week
  snapshot_retention = 7
  # Auto-extend when 80% full
  auto_extend_size_threshold = 80
  # Grow by 5GB increments
  auto_extend_size_increment = "5GB"
  # Maximum size limit
  auto_extend_size_limit = "250GB"

# SSH service configuration (primary access method)
[[services]]
  protocol = "tcp"
  internal_port = 2222  # Use port 2222 to avoid conflicts with Fly.io's hallpass service on port 22

  # Cost optimization settings
  auto_stop_machines = "${AUTO_STOP_MODE}"
  auto_start_machines = ${AUTO_START}
  min_machines_running = 0

  # Port mapping for SSH access
  [[services.ports]]
    port = ${SSH_EXTERNAL_PORT}  # External port for SSH - configurable for testing

  # Health check for SSH service
  [[services.tcp_checks]]
    interval = "15s"
    timeout = "2s"
    grace_period = "10s"
    restart_limit = 0

# Machine configuration
[machine]
  # Auto-restart on failure
  auto_restart = true

  # Restart policy
  restart_policy = "always"

# VM resource allocation
# Start small and scale up if needed
[vm]
  # CPU and memory settings (adjust based on needs)
  cpu_kind = "${CPU_KIND}"     # Options: "shared", "performance"
  cpus = ${CPUS}               # Number of CPUs
  memory = "${MEMORY_MB}mb"
  # Swap space for memory pressure relief
  swap_size_mb = ${SWAP_MB}

# Deployment settings
[deploy]
  # Deployment strategy for zero-downtime updates
  strategy = "rolling"

  # Release command (runs once per deployment)
  release_command = "echo 'Deployment complete'"

# Monitoring and health checks
[checks]
  # SSH service health check
  [checks.ssh]
    type = "tcp"
    port = 2222  # Updated to match SSH daemon port
    interval = "15s"
    timeout = "2s"

# Optional: Metrics and observability
[metrics]
  port = 9090
  path = "/metrics"

# Optional: Process groups for complex applications
# Uncomment if you need separate processes
# [processes]
#   app = "ssh-server"
#   worker = "background-tasks"

# Volume configuration reference
# Create volume with: flyctl volumes create home_data --region ${REGION} --size ${VOLUME_SIZE}
# Volume naming pattern: home_data (mounts as developer's home directory)
# Pricing: ~\$0.15/GB/month

# Cost optimization notes:
# 1. auto_stop_machines = "${AUTO_STOP_MODE}" - Fastest restart, lowest cost when idle
# 2. min_machines_running = 0 - Allows complete scale-to-zero
# 3. ${CPU_KIND} CPU - Cost-effective for development workloads
# 4. ${MEMORY_MB}MB RAM - Good performance for AI-powered development

# Security notes:
# 1. SSH access only via key authentication (configured in Dockerfile)
# 2. Non-standard SSH port (${SSH_EXTERNAL_PORT}) reduces automated attacks
# 3. Auto-restart on failure provides resilience
# 4. No root access via SSH (configured in Dockerfile)
# 5. Secrets management via Fly.io secrets:
#    - ANTHROPIC_API_KEY: Claude API authentication
#    - GITHUB_TOKEN: GitHub authentication for git operations
#    - GIT_USER_NAME: Git config user.name
#    - GIT_USER_EMAIL: Git config user.email
#    - GITHUB_USER: GitHub username for gh CLI
#    - OPENROUTER_API_KEY: OpenRouter API for cost-optimized models
#    - GOOGLE_GEMINI_API_KEY: Google Gemini API for free-tier access
#    - PERPLEXITY_API_KEY: Perplexity API for research assistant
#    - XAI_API_KEY: xAI Grok SDK authentication
#    - NPM_TOKEN: npm private package access (optional)
#    - PYPI_TOKEN: PyPI package publishing (optional)

# Scaling notes:
# 1. Machines will automatically start on incoming connections
# 2. Adjust concurrency limits based on expected load
# 3. Consider performance CPU for intensive tasks
# 4. Increase memory if running memory-intensive operations

# Development workflow:
# 1. Deploy: flyctl deploy
# 2. Set secrets (optional):
#    flyctl secrets set ANTHROPIC_API_KEY=sk-ant-... -a ${NAME}
#    flyctl secrets set GITHUB_TOKEN=ghp_... -a ${NAME}
#    flyctl secrets set GIT_USER_NAME="Your Name" -a ${NAME}
#    flyctl secrets set GIT_USER_EMAIL="you@example.com" -a ${NAME}
#    flyctl secrets set OPENROUTER_API_KEY=sk-or-... -a ${NAME}
#    flyctl secrets set GOOGLE_GEMINI_API_KEY=... -a ${NAME}
#    flyctl secrets set PERPLEXITY_API_KEY=pplx-... -a ${NAME}
# 3. Connect: ssh developer@${NAME}.fly.dev -p ${SSH_EXTERNAL_PORT}
#    (External port ${SSH_EXTERNAL_PORT} maps to internal SSH daemon on port 2222)
#    Alternative: flyctl ssh console -a ${NAME} (uses Fly.io's hallpass service)
# 4. Work: All files in \$HOME (/alt/home/developer) are persistent
#    Projects go in \$WORKSPACE (/alt/home/developer/workspace)
# 5. Idle: VM automatically suspends after inactivity
# 6. Resume: VM starts automatically on next connection
EOFT

# If config-only mode, just report success and exit
if [[ "$CONFIG_ONLY" == "true" ]]; then
    echo "==> Generated fly.toml at $OUTPUT_DIR/fly.toml"
    echo "    App name: $NAME"
    echo "    Region: $REGION"
    echo "    Profile: $PROFILE"
    exit 0
fi

echo "==> Deploying to Fly.io..."

# Create app if not exists
if ! flyctl apps list | grep -q "$NAME"; then
    flyctl apps create "$NAME" --org "$ORG"
fi

# Create volume if not exists
if ! flyctl volumes list -a "$NAME" | grep -q "home_data"; then
    flyctl volumes create home_data -s "$VOLUME_SIZE" -r "$REGION" -a "$NAME" --yes
fi

# Resolve and inject secrets
print_status "Resolving secrets..."
if secrets_resolve_all "$SINDRI_YAML"; then
    print_status "Injecting secrets into Fly.io app..."
    secrets_inject_fly "$NAME"
else
    print_warning "Secret resolution failed, continuing without secrets..."
fi

# Deploy (use generated fly.toml from output directory if different from current)
if [[ "$OUTPUT_DIR" != "." ]]; then
    cp "$OUTPUT_DIR/fly.toml" ./fly.toml
fi
flyctl deploy --ha=false --wait-timeout 600

echo "==> Deployed to Fly.io"
echo "    Connect with: flyctl ssh console -a ${NAME}"
