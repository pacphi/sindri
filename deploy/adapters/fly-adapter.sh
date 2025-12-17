#!/bin/bash
# Fly.io adapter - Full lifecycle management for Fly.io deployments
#
# Usage:
#   fly-adapter.sh <command> [OPTIONS] [sindri.yaml]
#
# Commands:
#   deploy     Deploy to Fly.io
#   connect    SSH console into app
#   destroy    Delete app and resources
#   plan       Show deployment plan
#   status     Show app status
#
# Options:
#   --config-only    Generate fly.toml without deploying (deploy only)
#   --output-dir     Directory for generated files (default: current directory)
#   --output-vars    Output parsed variables as JSON (deploy only)
#   --app-name       Override app name from sindri.yaml
#   --ci-mode        Enable CI mode (empty services, set CI_MODE=true env)
#   --rebuild        Force rebuild without Docker cache (deploy only)
#   --force          Skip confirmation prompts (destroy only)
#   --help           Show this help message
#
# Examples:
#   fly-adapter.sh deploy                        # Deploy using ./sindri.yaml
#   fly-adapter.sh deploy --rebuild              # Force full rebuild (no cache)
#   fly-adapter.sh deploy --config-only          # Just generate fly.toml
#   fly-adapter.sh deploy --ci-mode --config-only  # CI-compatible fly.toml
#   fly-adapter.sh status                        # Show app status
#   fly-adapter.sh destroy --force               # Teardown without confirmation

set -e

# Source common adapter functions
# shellcheck source=adapter-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/adapter-common.sh"

# Initialize adapter
adapter_init "${BASH_SOURCE[0]}"

# Fly.io-specific defaults
# shellcheck disable=SC2034  # Used via indirect expansion in adapter_parse_base_config
APP_NAME_OVERRIDE=""
REBUILD=false

# Show help wrapper
show_help() {
    adapter_show_help "$0" 30
}

# Parse command
if ! adapter_parse_command "$@"; then
    show_help
fi
set -- "${REMAINING_ARGS[@]}"

# Parse arguments
# shellcheck disable=SC2034  # Variables used by adapter_parse_base_config or sourced scripts
while [[ $# -gt 0 ]]; do
    case $1 in
        --config-only)  CONFIG_ONLY=true; shift ;;
        --output-dir)   OUTPUT_DIR="$2"; shift 2 ;;
        --output-vars)  OUTPUT_VARS=true; shift ;;
        --app-name)     APP_NAME_OVERRIDE="$2"; shift 2 ;;
        --ci-mode)      CI_MODE=true; shift ;;
        --rebuild)      REBUILD=true; shift ;;
        --force|-f)     FORCE=true; shift ;;
        --help|-h)      show_help ;;
        -*)             adapter_unknown_option "$1" ;;
        *)              SINDRI_YAML="$1"; shift ;;
    esac
done

# Validate config file
adapter_validate_config

# Source common utilities (print_*, etc.)
source "$BASE_DIR/docker/lib/common.sh"

# ============================================================================
# Configuration Parsing
# ============================================================================

parse_config() {
    # Parse base configuration
    adapter_parse_base_config "APP_NAME_OVERRIDE"

    # Fly.io-specific: Convert memory to MB (security fix H-9)
    local memory_raw
    memory_raw=$(yq '.deployment.resources.memory // "2GB"' "$SINDRI_YAML")

    # Validate memory format to prevent command injection
    if [[ ! "$memory_raw" =~ ^[0-9]+[GM]B$ ]]; then
        print_error "Invalid memory format: $memory_raw (expected format: 2GB, 512MB, etc.)"
        exit 1
    fi

    # Safe conversion after validation
    MEMORY=$(echo "$memory_raw" | sed 's/GB/*1024/;s/MB//')
    MEMORY_MB=$(echo "$MEMORY" | bc)

    # Fly.io-specific config
    REGION=$(yq '.providers.fly.region // "sjc"' "$SINDRI_YAML")
    ORG=$(yq '.providers.fly.organization // "personal"' "$SINDRI_YAML")
    AUTO_STOP=$(yq '.providers.fly.autoStopMachines // true' "$SINDRI_YAML")
    AUTO_START=$(yq '.providers.fly.autoStartMachines // true' "$SINDRI_YAML")
    CPU_KIND=$(yq '.providers.fly.cpuKind // "shared"' "$SINDRI_YAML")
    SSH_EXTERNAL_PORT=$(yq '.providers.fly.sshPort // 10022' "$SINDRI_YAML")

    # Calculated values
    SWAP_MB=$((MEMORY_MB / 2))
    if [[ $SWAP_MB -lt 2048 ]]; then
        SWAP_MB=2048
    fi
    AUTO_STOP_MODE="suspend"
    if [[ "$AUTO_STOP" == "false" ]]; then
        AUTO_STOP_MODE="off"
    fi
}

require_flyctl() {
    if ! command -v flyctl >/dev/null 2>&1; then
        print_error "flyctl CLI is not installed"
        echo "Install from: https://fly.io/docs/hands-on/install-flyctl/"
        exit 1
    fi
}

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

    print_warning "GPU machines may not be available in region: $region"
    echo "GPU-enabled regions: ${valid_regions[*]}" >&2
    return 1
}

# ============================================================================
# fly.toml Generation
# ============================================================================

generate_fly_toml() {
    mkdir -p "$OUTPUT_DIR"

    # Get skip_auto_install value
    local skip_auto_install
    skip_auto_install=$(adapter_get_skip_auto_install)

    # Determine if CI mode is active
    local ci_mode_env=""
    if [[ "$CI_MODE" == "true" ]]; then
        ci_mode_env='
  # CI Mode enabled - SSH daemon is skipped, use flyctl ssh console for access
  CI_MODE = "true"'
    fi

    # Generate fly.toml with comprehensive configuration and documentation
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
  ADDITIONAL_EXTENSIONS = "${ADDITIONAL_EXTENSIONS}"
  # Skip auto-install (set to true for manual control)
  SKIP_AUTO_INSTALL = "${skip_auto_install}"
  # Workspace initialization
  INIT_WORKSPACE = "true"${ci_mode_env}

# Volume mounts for persistent storage
[[mounts]]
  # Mount persistent volume as developer's home directory
  # This ensures \$HOME is persistent and contains workspace, config, and tool data
  source = "home_data"
  destination = "/alt/home/developer"
  initial_size = "${VOLUME_SIZE}gb"
  # Snapshot retention for disaster recovery
  snapshot_retention = 7
  # Auto-extend storage when capacity is reached
  auto_extend_size_threshold = 80
  auto_extend_size_increment = "5GB"
  auto_extend_size_limit = "250GB"

EOFT

    # Add services section (or empty for CI mode)
    if [[ "$CI_MODE" == "true" ]]; then
        cat >> "$OUTPUT_DIR/fly.toml" << EOFT
# CI Mode: No services exposed (use flyctl ssh console for access)
services = []

EOFT
    else
        cat >> "$OUTPUT_DIR/fly.toml" << EOFT
# SSH service configuration
# Exposes SSH on external port ${SSH_EXTERNAL_PORT} (internal 2222)
[[services]]
  protocol = "tcp"
  internal_port = 2222
  # Auto-suspend after 5 minutes of no SSH connections
  auto_stop_machines = "${AUTO_STOP_MODE}"
  auto_start_machines = ${AUTO_START}
  min_machines_running = 0

  [[services.ports]]
    port = ${SSH_EXTERNAL_PORT}

  # Health check - verifies SSH daemon is running
  # SSH daemon starts immediately; extensions install in background
  [[services.tcp_checks]]
    interval = "30s"
    timeout = "10s"
    grace_period = "30s"

EOFT
    fi

    # Add VM configuration (GPU or standard)
    if [[ "$GPU_ENABLED" == "true" ]]; then
        local gpu_config gpu_guest_type gpu_cpus gpu_memory_mb
        gpu_config=$(get_fly_gpu_config "$GPU_TIER")
        gpu_guest_type=$(echo "$gpu_config" | cut -d: -f1)
        gpu_cpus=$(echo "$gpu_config" | cut -d: -f2)
        gpu_memory_mb=$(echo "$gpu_config" | cut -d: -f3)

        cat >> "$OUTPUT_DIR/fly.toml" << EOFT
# GPU-enabled VM configuration
# Using ${GPU_TIER} tier with ${gpu_guest_type}
[vm]
  guest_type = "${gpu_guest_type}"
  cpus = ${gpu_cpus}
  memory = "${gpu_memory_mb}mb"

EOFT
    else
        cat >> "$OUTPUT_DIR/fly.toml" << EOFT
# VM sizing configuration
# Adjust based on your workload requirements
[vm]
  cpu_kind = "${CPU_KIND}"
  cpus = ${CPUS}
  memory = "${MEMORY_MB}mb"
  # Swap provides overflow memory capacity
  swap_size_mb = ${SWAP_MB}

EOFT
    fi

    # Add deployment strategy
    cat >> "$OUTPUT_DIR/fly.toml" << EOFT
# Deployment strategy
[deploy]
  strategy = "rolling"

EOFT

    # Add health checks (skip in CI mode)
    if [[ "$CI_MODE" != "true" ]]; then
        cat >> "$OUTPUT_DIR/fly.toml" << EOFT
# Health checks
# SSH daemon starts immediately; extensions install in background
[checks]
  [checks.ssh]
    type = "tcp"
    port = 2222
    interval = "30s"
    timeout = "10s"
    grace_period = "30s"
EOFT
    fi

    # Add documentation comments
    cat >> "$OUTPUT_DIR/fly.toml" << EOFT

# =============================================================================
# Documentation
# =============================================================================

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
}

# ============================================================================
# SSH Key Configuration
# ============================================================================

# Validate SSH key configuration from sindri.yaml
# Returns 0 if valid, 1 if RSA key detected (should stop deploy)
validate_ssh_key_config() {
    local config_file="$1"

    # Check if AUTHORIZED_KEYS has a fromFile pointing to an RSA key
    local from_file
    from_file=$(yq '.secrets[] | select(.name == "AUTHORIZED_KEYS") | .fromFile // ""' "$config_file" 2>/dev/null)

    if [[ -z "$from_file" ]]; then
        # No fromFile specified, proceed normally
        return 0
    fi

    # Expand ~ to $HOME
    from_file="${from_file/#\~/$HOME}"

    # Check if the file path contains "rsa" (indicating RSA key)
    if [[ "$from_file" == *"rsa"* ]]; then
        print_error "RSA SSH key detected in sindri.yaml"
        echo ""
        echo "Your configuration specifies an RSA key:"
        echo "  fromFile: $from_file"
        echo ""
        echo "RSA keys are deprecated and may cause authentication issues with modern"
        echo "SSH servers. ED25519 keys are recommended for better security and compatibility."
        echo ""
        echo "To fix this:"
        echo ""
        echo "  1. Generate an ED25519 key (if you don't have one):"
        echo "     ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519 -C \"your-email@example.com\""
        echo ""
        echo "  2. Update your sindri.yaml to use the ED25519 key:"
        echo "     secrets:"
        echo "       - name: AUTHORIZED_KEYS"
        echo "         source: env"
        echo "         fromFile: ~/.ssh/id_ed25519.pub"
        echo ""
        echo "  3. Re-run the deploy command"
        echo ""
        return 1
    fi

    # Check if ED25519 key file exists, create if not
    if [[ "$from_file" == *"ed25519"* ]]; then
        if [[ ! -f "$from_file" ]]; then
            # Check for the private key too
            local private_key="${from_file%.pub}"
            if [[ ! -f "$private_key" ]]; then
                print_warning "ED25519 key not found: $from_file"
                echo ""

                # In CI mode or non-interactive, fail
                if [[ "${CI_MODE:-}" == "true" ]] || [[ "${CI:-}" == "true" ]] || [[ ! -t 0 ]]; then
                    print_error "Cannot create SSH key in non-interactive mode"
                    echo "Generate the key manually: ssh-keygen -t ed25519 -f $private_key"
                    return 1
                fi

                read -p "Generate ED25519 key pair? (Y/n) " -n 1 -r
                echo ""
                if [[ ! $REPLY =~ ^[Nn]$ ]]; then
                    print_status "Generating ED25519 SSH key pair..."
                    mkdir -p "$(dirname "$private_key")"
                    ssh-keygen -t ed25519 -f "$private_key" -N "" -C "sindri-dev-$(date +%Y%m%d)"
                    print_success "SSH key generated: $private_key"
                else
                    print_error "SSH key required for deployment"
                    return 1
                fi
            fi
        fi
        print_status "Using ED25519 key: $from_file"
    fi

    return 0
}

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

    # Look for common SSH public keys (prefer ED25519)
    local ssh_key=""
    local ssh_key_type=""
    for key_file in ~/.ssh/id_ed25519.pub ~/.ssh/id_ecdsa.pub; do
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
            # Use stdin to prevent secrets exposure in process arguments (C-4 security fix)
            echo "AUTHORIZED_KEYS=$ssh_key" | flyctl secrets import -a "$app_name"
            print_success "SSH key configured successfully"
            return 0
        fi
    else
        print_warning "No ED25519/ECDSA SSH keys found in ~/.ssh/"
        echo ""
        read -p "Generate a new ED25519 SSH key pair for Sindri? (Y/n) " -n 1 -r
        echo ""
        if [[ ! $REPLY =~ ^[Nn]$ ]]; then
            local key_path="$HOME/.ssh/id_ed25519"
            print_status "Generating ED25519 SSH key pair..."
            ssh-keygen -t ed25519 -f "$key_path" -N "" -C "sindri-dev-$(date +%Y%m%d)"
            ssh_key=$(cat "${key_path}.pub")
            print_success "SSH key generated: $key_path"

            print_status "Configuring SSH key on Fly.io..."
            # Use stdin to prevent secrets exposure in process arguments (C-4 security fix)
            echo "AUTHORIZED_KEYS=$ssh_key" | flyctl secrets import -a "$app_name"
            print_success "SSH key configured successfully"

            echo ""
            print_status "To connect, use:"
            echo "    ssh developer@${app_name}.fly.dev -p ${SSH_EXTERNAL_PORT}"
            echo ""
            return 0
        fi
    fi

    print_warning "Skipping SSH key setup"
    print_status "SSH access available via: flyctl ssh console -a $app_name"
}

# ============================================================================
# Commands
# ============================================================================

cmd_deploy() {
    parse_config

    # Validate SSH key configuration early (before any resources are created)
    # This stops the deploy if an RSA key is specified, or creates ED25519 if missing
    if ! validate_ssh_key_config "$SINDRI_YAML"; then
        exit 1
    fi

    # Output variables for CI integration
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
  "gpu_tier": "$GPU_TIER"
}
EOJSON
        exit 0
    fi

    # Validate GPU region if enabled
    if [[ "$GPU_ENABLED" == "true" ]]; then
        validate_fly_gpu_region "$REGION" || true
    fi

    generate_fly_toml

    if [[ "$CONFIG_ONLY" == "true" ]]; then
        print_success "Generated fly.toml at $OUTPUT_DIR/fly.toml"
        echo "  App name: $NAME"
        echo "  Region: $REGION"
        echo "  Profile: $PROFILE"
        echo "  CI Mode: $CI_MODE"
        if [[ "$GPU_ENABLED" == "true" ]]; then
            local gpu_config gpu_guest_type
            gpu_config=$(get_fly_gpu_config "$GPU_TIER")
            gpu_guest_type=$(echo "$gpu_config" | cut -d: -f1)
            echo "  GPU: $GPU_TIER ($gpu_guest_type)"
        fi
        return 0
    fi

    require_flyctl
    source "$BASE_DIR/cli/secrets-manager"

    print_header "Deploying to Fly.io"
    echo "  App: $NAME"
    echo "  Region: $REGION"
    echo "  Profile: $PROFILE"
    echo "  Resources: ${CPUS} CPUs, ${MEMORY_MB}MB memory"
    if [[ "$GPU_ENABLED" == "true" ]]; then
        echo "  GPU: $GPU_TIER"
    fi
    echo ""

    # Create app if it doesn't exist
    if ! flyctl apps list 2>/dev/null | grep -q "$NAME"; then
        print_status "Creating app: $NAME"
        flyctl apps create "$NAME" --org "$ORG"
    fi

    # Create volume if it doesn't exist
    if ! flyctl volumes list -a "$NAME" 2>/dev/null | grep -q "home_data"; then
        print_status "Creating volume: home_data (${VOLUME_SIZE}GB)"
        flyctl volumes create home_data -s "$VOLUME_SIZE" -r "$REGION" -a "$NAME" --yes
    fi

    # Configure SSH keys
    ensure_ssh_keys "$NAME"

    # Resolve and inject secrets
    print_status "Resolving secrets..."
    if secrets_resolve_all "$SINDRI_YAML"; then
        secrets_inject_fly "$NAME"
    fi

    # Copy fly.toml to working directory if generated elsewhere
    if [[ "$OUTPUT_DIR" != "." ]]; then
        cp "$OUTPUT_DIR/fly.toml" ./fly.toml
    fi

    # Deploy
    print_status "Deploying application..."
    local deploy_args=("--ha=false" "--wait-timeout" "600")
    if [[ "$REBUILD" == "true" ]]; then
        print_status "Forcing full rebuild (--no-cache)..."
        deploy_args+=("--no-cache")
    fi
    flyctl deploy "${deploy_args[@]}"

    print_success "App '$NAME' deployed successfully"
    echo ""
    echo "Connect:"
    echo "  sindri connect"
    echo "  flyctl ssh console -a $NAME"
    if [[ "$CI_MODE" != "true" ]]; then
        echo "  ssh developer@$NAME.fly.dev -p $SSH_EXTERNAL_PORT"
    fi
    echo ""
    echo "Manage:"
    echo "  sindri status"
    echo "  sindri destroy"
}

cmd_connect() {
    parse_config
    require_flyctl

    if ! flyctl apps list 2>/dev/null | grep -q "$NAME"; then
        print_error "App '$NAME' not found on Fly.io"
        echo "Deploy first: sindri deploy --provider fly"
        exit 1
    fi

    # Check if machine is suspended and wake it first
    local machine_state
    machine_state=$(flyctl machines list -a "$NAME" --json 2>/dev/null | yq -r '.[0].state // "unknown"')

    if [[ "$machine_state" == "suspended" ]] || [[ "$machine_state" == "stopped" ]]; then
        print_status "Machine is $machine_state, waking up..."
        local machine_id
        machine_id=$(flyctl machines list -a "$NAME" --json 2>/dev/null | yq -r '.[0].id // ""')
        if [[ -n "$machine_id" ]]; then
            flyctl machine start "$machine_id" -a "$NAME" 2>/dev/null || true
            print_status "Waiting for machine to start..."
            sleep 5
        fi
    fi

    # Connect as developer user with proper login shell and home directory
    # --pty allocates a pseudo-terminal for interactive shell
    # Display MOTD first (flyctl ssh doesn't trigger PAM's pam_motd)
    # Then 'su - developer' ensures HOME is set correctly and we land in $HOME
    # Note: sh -c required because flyctl ssh -C doesn't parse shell operators
    flyctl ssh console -a "$NAME" --pty -C "sh -c 'cat /etc/motd 2>/dev/null; exec su - developer'"
}

cmd_destroy() {
    parse_config

    if [[ "$FORCE" != "true" ]]; then
        print_warning "This will destroy app '$NAME' and all its resources (volumes, secrets)"
        read -p "Are you sure? (y/N) " -n 1 -r
        echo
        [[ ! $REPLY =~ ^[Yy]$ ]] && { print_status "Cancelled"; exit 0; }
    fi

    require_flyctl
    print_header "Destroying Fly.io app: $NAME"

    if flyctl apps list 2>/dev/null | grep -q "$NAME"; then
        print_status "Deleting app (includes volumes and secrets)..."
        flyctl apps destroy "$NAME" --yes
        print_success "App destroyed"
    else
        print_warning "App '$NAME' not found on Fly.io"
    fi

    if [[ -f "$OUTPUT_DIR/fly.toml" ]]; then
        rm -f "$OUTPUT_DIR/fly.toml"
        print_status "Removed fly.toml"
    fi
}

cmd_plan() {
    parse_config

    print_header "Fly.io Deployment Plan"
    echo ""
    echo "App:        $NAME"
    echo "Region:     $REGION"
    echo "Org:        $ORG"
    echo "Profile:    $PROFILE"
    echo ""
    echo "Resources:"
    echo "  CPUs:     $CPUS ($CPU_KIND)"
    echo "  Memory:   ${MEMORY_MB}MB"
    echo "  Swap:     ${SWAP_MB}MB"
    echo "  Volume:   ${VOLUME_SIZE}GB"
    echo "  SSH Port: $SSH_EXTERNAL_PORT"
    if [[ "$GPU_ENABLED" == "true" ]]; then
        echo "  GPU:      $GPU_TIER"
    fi
    echo ""
    echo "Features:"
    echo "  Auto-stop:  $AUTO_STOP_MODE"
    echo "  Auto-start: $AUTO_START"
    echo "  CI Mode:    $CI_MODE"
    echo ""
    echo "Actions:"
    echo "  1. Generate fly.toml with documentation"
    echo "  2. Create app: $NAME (if needed)"
    echo "  3. Create volume: home_data (${VOLUME_SIZE}GB)"
    echo "  4. Configure SSH keys"
    echo "  5. Inject secrets from sindri.yaml"
    echo "  6. Deploy: flyctl deploy --ha=false"
}

cmd_status() {
    parse_config
    require_flyctl

    print_header "Fly.io Deployment Status"
    echo ""
    echo "App: $NAME"
    echo "Region: $REGION"
    echo ""

    if flyctl apps list 2>/dev/null | grep -q "$NAME"; then
        flyctl status -a "$NAME" 2>/dev/null || true
        echo ""
        echo "Machines:"
        flyctl machines list -a "$NAME" 2>/dev/null || true
        echo ""
        echo "Volumes:"
        flyctl volumes list -a "$NAME" 2>/dev/null || true
    else
        echo "Status: Not deployed"
        echo ""
        echo "Deploy with: sindri deploy --provider fly"
    fi
}

# ============================================================================
# Main Dispatch
# ============================================================================

adapter_dispatch "$COMMAND" cmd_deploy cmd_connect cmd_destroy cmd_plan cmd_status show_help
