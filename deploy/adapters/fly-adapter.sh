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
#   --ci-mode        Enable CI mode (empty services, set CI_MODE=true env)
#   --help           Show this help message
#
# Examples:
#   fly-adapter.sh                           # Deploy using ./sindri.yaml
#   fly-adapter.sh --config-only             # Just generate fly.toml
#   fly-adapter.sh --ci-mode --config-only   # Generate CI-compatible fly.toml
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
CI_MODE=false

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
        --ci-mode)
            CI_MODE=true
            shift
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

# Parse GPU configuration
GPU_ENABLED=$(yq '.deployment.resources.gpu.enabled // false' "$SINDRI_YAML")
GPU_TIER=$(yq '.deployment.resources.gpu.tier // "gpu-small"' "$SINDRI_YAML")
GPU_COUNT=$(yq '.deployment.resources.gpu.count // 1' "$SINDRI_YAML")

# GPU tier to Fly.io machine type mapping
get_fly_gpu_config() {
    local tier="${1:-gpu-small}"
    case "$tier" in
        gpu-small)   echo "a100-40gb:8:32768" ;;   # guest_type:cpus:memory_mb
        gpu-medium)  echo "a100-40gb:16:65536" ;;
        gpu-large)   echo "l40s:16:65536" ;;
        gpu-xlarge)  echo "a100-80gb:32:131072" ;;
        *)           echo "a100-40gb:8:32768" ;;
    esac
}

# Validate Fly.io GPU region
validate_fly_gpu_region() {
    local region="$1"
    local valid_regions=("ord" "sjc")

    for valid in "${valid_regions[@]}"; do
        if [[ "$region" == "$valid" ]]; then
            return 0
        fi
    done

    echo "Warning: GPU machines may not be available in region: $region" >&2
    echo "GPU-enabled regions: ${valid_regions[*]}" >&2
    return 1
}

# Validate GPU configuration
if [[ "$GPU_ENABLED" == "true" ]]; then
    validate_fly_gpu_region "$REGION" || true
fi

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
  "cpu_kind": "$CPU_KIND",
  "ci_mode": $CI_MODE,
  "gpu_enabled": $GPU_ENABLED,
  "gpu_tier": "$GPU_TIER",
  "gpu_count": $GPU_COUNT
}
EOJSON
    exit 0
fi

# Determine if CI mode is active (controls SSH daemon startup in entrypoint)
CI_MODE_ENV=""
[[ "$CI_MODE" == "true" ]] && CI_MODE_ENV='
  # CI Mode enabled - SSH daemon is skipped, use flyctl ssh console for access
  CI_MODE = "true"'

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
  dockerfile = "Dockerfile"

# Note: No [processes] section needed - Docker's ENTRYPOINT runs the entrypoint script
# The entrypoint checks CI_MODE to decide whether to start SSH daemon
# See: https://fly.io/docs/blueprints/opensshd/

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
  INIT_WORKSPACE = "true"${CI_MODE_ENV}

# Volume mounts for persistent storage
[[mounts]]
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

EOFT

# Add services section based on CI_MODE
if [[ "$CI_MODE" == "true" ]]; then
    # CI Mode: Empty services to avoid hallpass conflicts
    cat >> "$OUTPUT_DIR/fly.toml" << 'CISERVICES'
# Services configuration - empty for CI mode to prevent hallpass conflicts
# In CI mode, use flyctl ssh console for access instead of custom SSH service
services = []

CISERVICES
else
    # Normal mode: Full SSH services configuration
    cat >> "$OUTPUT_DIR/fly.toml" << NORMALSERVICES
# SSH service configuration (primary access method)
# Note: sshd listens internally on 2222 to avoid conflicts with Fly.io's internal SSH on port 22
# See: https://fly.io/docs/blueprints/opensshd/
[[services]]
  protocol = "tcp"
  internal_port = 2222

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
    timeout = "5s"
    grace_period = "30s"
    restart_limit = 3

NORMALSERVICES
fi

# Continue with VM and remaining configuration
if [[ "$GPU_ENABLED" == "true" ]]; then
    # GPU-enabled VM configuration
    GPU_CONFIG=$(get_fly_gpu_config "$GPU_TIER")
    GPU_GUEST_TYPE=$(echo "$GPU_CONFIG" | cut -d: -f1)
    GPU_CPUS=$(echo "$GPU_CONFIG" | cut -d: -f2)
    GPU_MEMORY_MB=$(echo "$GPU_CONFIG" | cut -d: -f3)

    cat >> "$OUTPUT_DIR/fly.toml" << EOFT
# GPU-enabled VM resource allocation
# Using Fly.io GPU machines with ${GPU_TIER} tier
[vm]
  # GPU machine type - includes GPU, CPU, and memory
  guest_type = "${GPU_GUEST_TYPE}"
  cpus = ${GPU_CPUS}
  memory = "${GPU_MEMORY_MB}mb"
  # Note: GPU machines include swap configured automatically

# Deployment settings
EOFT
else
    cat >> "$OUTPUT_DIR/fly.toml" << EOFT
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
EOFT
fi

cat >> "$OUTPUT_DIR/fly.toml" << EOFT
[deploy]
  # Deployment strategy for zero-downtime updates
  strategy = "rolling"
  # No release_command - initialization happens in entrypoint

EOFT

# Add health checks section based on CI_MODE
if [[ "$CI_MODE" != "true" ]]; then
    # Normal mode: Add SSH health checks
    cat >> "$OUTPUT_DIR/fly.toml" << 'HEALTHCHECKS'
# Monitoring and health checks
[checks]
  # SSH service health check
  [checks.ssh]
    type = "tcp"
    port = 2222
    interval = "15s"
    timeout = "5s"
    grace_period = "30s"

HEALTHCHECKS
else
    # CI Mode: Skip health checks
    cat >> "$OUTPUT_DIR/fly.toml" << 'NOHEALTHCHECKS'
# Monitoring and health checks - disabled for CI mode
# Health checks are skipped in CI to allow faster deployment and avoid timeout issues

NOHEALTHCHECKS
fi

# Add documentation comments
cat >> "$OUTPUT_DIR/fly.toml" << EOFT

# Volume configuration reference
# Volume is automatically created by fly deploy if it doesn't exist
# Manual creation: flyctl volumes create home_data --region ${REGION} --size ${VOLUME_SIZE}
# Volume naming pattern: home_data (mounts as developer's home directory at /alt/home/developer)
# Pricing: ~\$0.15/GB/month

# Process configuration notes:
# No [processes] section is used - Docker's ENTRYPOINT handles container startup
# The entrypoint script checks CI_MODE to determine whether to start SSH daemon
# In CI mode, SSH is skipped and access is via flyctl ssh console (hallpass)
# See: https://fly.io/docs/blueprints/opensshd/

# Cost optimization notes:
# 1. auto_stop_machines = "${AUTO_STOP_MODE}" - Suspends when idle, fastest restart
# 2. min_machines_running = 0 - Allows complete scale-to-zero
# 3. ${CPU_KIND} CPU - Cost-effective for development workloads
# 4. ${MEMORY_MB}MB RAM - Good performance for AI-powered development

# Security notes:
# 1. SSH server listens on port 2222 internally (avoids conflict with Fly.io's port 22)
# 2. External port ${SSH_EXTERNAL_PORT} maps to internal port 2222
# 3. Password authentication enabled for developer user (can use key-based auth instead)
# 4. Root login disabled via SSH
# 5. SSH host keys are persisted to volume (~/.ssh/host_keys/) for stable fingerprints
# 6. Secrets management via Fly.io secrets:
#    - AUTHORIZED_KEYS: SSH public keys for key-based authentication (recommended)
#      Set with: flyctl secrets set "AUTHORIZED_KEYS=\$(cat ~/.ssh/id_ed25519.pub)" -a ${NAME}
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
#    flyctl secrets set "AUTHORIZED_KEYS=\$(cat ~/.ssh/id_ed25519.pub)" -a ${NAME}
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
    echo "    CI Mode: $CI_MODE"
    if [[ "$GPU_ENABLED" == "true" ]]; then
        echo "    GPU: $GPU_TIER ($GPU_GUEST_TYPE)"
    fi
    exit 0
fi

# ------------------------------------------------------------------------------
# ensure_ssh_keys - Ensure AUTHORIZED_KEYS is configured for SSH access
# ------------------------------------------------------------------------------
# Skips interactive prompts in CI (non-interactive shell) or when CI_MODE=true
ensure_ssh_keys() {
    local app_name="$1"

    # Check if AUTHORIZED_KEYS is already set in environment
    if [[ -n "${AUTHORIZED_KEYS:-}" ]]; then
        print_status "SSH keys found in environment"
        return 0
    fi

    # Check if AUTHORIZED_KEYS is set in .env or .env.local
    if [[ -f .env.local ]] && grep -q "^AUTHORIZED_KEYS=" .env.local 2>/dev/null; then
        print_status "SSH keys found in .env.local"
        return 0
    fi
    if [[ -f .env ]] && grep -q "^AUTHORIZED_KEYS=" .env 2>/dev/null; then
        print_status "SSH keys found in .env"
        return 0
    fi

    # Check if AUTHORIZED_KEYS is already set on Fly.io
    if flyctl secrets list -a "$app_name" 2>/dev/null | grep -q "AUTHORIZED_KEYS"; then
        print_status "SSH keys already configured on Fly.io"
        return 0
    fi

    # In CI mode or non-interactive shell, skip prompts
    if [[ "${CI_MODE:-}" == "true" ]] || [[ "${CI:-}" == "true" ]] || [[ ! -t 0 ]]; then
        print_warning "No SSH keys configured (CI mode - skipping interactive setup)"
        print_status "SSH access available via: flyctl ssh console -a $app_name"
        return 0
    fi

    print_warning "No SSH keys configured - SSH access will not be available"
    print_status "Checking for local SSH keys..."

    # Look for common SSH public keys
    local ssh_key=""
    local ssh_key_type=""
    for key_file in ~/.ssh/id_ed25519.pub ~/.ssh/id_rsa.pub ~/.ssh/id_ecdsa.pub; do
        if [[ -f "$key_file" ]]; then
            ssh_key=$(cat "$key_file")
            ssh_key_type=$(basename "$key_file" .pub)
            break
        fi
    done

    if [[ -n "$ssh_key" ]]; then
        print_success "Found local SSH key: $ssh_key_type"
        echo ""
        read -p "Use this key for SSH access to Sindri? (Y/n) " -n 1 -r
        echo ""
        if [[ ! $REPLY =~ ^[Nn]$ ]]; then
            print_status "Configuring SSH key on Fly.io..."
            flyctl secrets set "AUTHORIZED_KEYS=$ssh_key" -a "$app_name"
            print_success "SSH key configured successfully"
            return 0
        fi
    else
        print_warning "No local SSH keys found in ~/.ssh/"
        echo ""
        read -p "Generate a new SSH key pair for Sindri? (Y/n) " -n 1 -r
        echo ""
        if [[ ! $REPLY =~ ^[Nn]$ ]]; then
            local key_path="$HOME/.ssh/sindri_ed25519"
            print_status "Generating SSH key pair..."
            ssh-keygen -t ed25519 -f "$key_path" -N "" -C "sindri-dev-$(date +%Y%m%d)"
            ssh_key=$(cat "${key_path}.pub")
            print_success "SSH key generated: $key_path"

            print_status "Configuring SSH key on Fly.io..."
            flyctl secrets set "AUTHORIZED_KEYS=$ssh_key" -a "$app_name"
            print_success "SSH key configured successfully"

            echo ""
            print_status "To connect, use:"
            echo "    ssh -i $key_path developer@${app_name}.fly.dev -p ${SSH_EXTERNAL_PORT}"
            echo ""
            return 0
        fi
    fi

    # User declined to configure SSH keys
    print_warning "Continuing without SSH key configuration"
    print_status "SSH access will only be available via: flyctl ssh console -a $app_name"
    return 0
}

echo "==> Deploying to Fly.io..."

# Create app if not exists
if ! flyctl apps list | grep -q "$NAME"; then
    flyctl apps create "$NAME" --org "$ORG"
fi

# Create volume if not exists
if ! flyctl volumes list -a "$NAME" | grep -q "home_data"; then
    flyctl volumes create home_data -s "$VOLUME_SIZE" -r "$REGION" -a "$NAME" --yes
fi

# Ensure SSH keys are configured (interactive prompt if missing)
ensure_ssh_keys "$NAME"

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
