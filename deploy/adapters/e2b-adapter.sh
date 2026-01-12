#!/bin/bash
# E2B adapter - Full lifecycle management for E2B sandbox deployments
#
# Usage:
#   e2b-adapter.sh <command> [OPTIONS] [sindri.yaml]
#
# Commands:
#   deploy     Build template if needed, create/resume sandbox
#   connect    Connect to sandbox via PTY terminal
#   pause      Pause sandbox (preserve state)
#   destroy    Kill sandbox
#   plan       Show deployment plan
#   status     Show sandbox status
#   template   Manage templates (build, list, delete)
#
# Options:
#   --config-only    Generate template files without deploying (deploy only)
#   --output-dir     Directory for generated files (default: .e2b/)
#   --rebuild        Force template rebuild (deploy only)
#   --ephemeral      Create non-persistent sandbox (deploy only)
#   --force          Skip confirmation prompts (destroy only)
#   --sandbox-name   Override sandbox name from sindri.yaml
#   --help           Show this help message
#
# Examples:
#   e2b-adapter.sh deploy                       # Deploy using ./sindri.yaml
#   e2b-adapter.sh deploy --rebuild             # Force template rebuild
#   e2b-adapter.sh deploy --config-only         # Just generate template files
#   e2b-adapter.sh deploy --ephemeral           # Create non-persistent sandbox
#   e2b-adapter.sh connect                      # Connect to sandbox
#   e2b-adapter.sh pause                        # Pause sandbox (preserve state)
#   e2b-adapter.sh status                       # Show sandbox status
#   e2b-adapter.sh template list                # List available templates
#   e2b-adapter.sh destroy --force              # Teardown without confirmation

set -e

# Source common adapter functions
# shellcheck source=adapter-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/adapter-common.sh"

# Initialize adapter
adapter_init "${BASH_SOURCE[0]}"

# E2B-specific defaults
# shellcheck disable=SC2034  # Used via indirect expansion in adapter_parse_base_config
SANDBOX_NAME_OVERRIDE=""
REBUILD=false
EPHEMERAL=false
# shellcheck disable=SC2034  # Used for template subcommand
TEMPLATE_SUBCMD=""

# Override default output directory for E2B
OUTPUT_DIR=".e2b"

# Show help wrapper
show_help() {
    adapter_show_help "$0" 34
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
        --config-only)   CONFIG_ONLY=true; shift ;;
        --output-dir)    OUTPUT_DIR="$2"; shift 2 ;;
        --output-vars)   OUTPUT_VARS=true; shift ;;
        --sandbox-name)  SANDBOX_NAME_OVERRIDE="$2"; shift 2 ;;
        --rebuild)       REBUILD=true; shift ;;
        --ephemeral)     EPHEMERAL=true; shift ;;
        --ci-mode)       CI_MODE=true; shift ;;
        --force|-f)      FORCE=true; shift ;;
        --help|-h)       show_help ;;
        -*)              adapter_unknown_option "$1" ;;
        *)
            # For template subcommand, capture the subcmd
            if [[ "$COMMAND" == "template" ]] && [[ -z "${TEMPLATE_SUBCMD:-}" ]]; then
                TEMPLATE_SUBCMD="$1"
                shift
            else
                SINDRI_YAML="$1"
                shift
            fi
            ;;
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
    adapter_parse_base_config "SANDBOX_NAME_OVERRIDE"

    # E2B-specific configuration
    TEMPLATE_ALIAS=$(yq '.providers.e2b.templateAlias // ""' "$SINDRI_YAML")
    if [[ -z "$TEMPLATE_ALIAS" ]] || [[ "$TEMPLATE_ALIAS" == "null" ]]; then
        # Generate template alias from name (lowercase, alphanumeric with hyphens)
        TEMPLATE_ALIAS=$(echo "$NAME" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9-]/-/g')
    fi

    REUSE_TEMPLATE=$(yq '.providers.e2b.reuseTemplate // true' "$SINDRI_YAML")
    TIMEOUT=$(yq '.providers.e2b.timeout // 300' "$SINDRI_YAML")
    AUTO_PAUSE=$(yq '.providers.e2b.autoPause // true' "$SINDRI_YAML")
    AUTO_RESUME=$(yq '.providers.e2b.autoResume // true' "$SINDRI_YAML")
    BUILD_ON_DEPLOY=$(yq '.providers.e2b.buildOnDeploy // false' "$SINDRI_YAML")

    # Network configuration
    INTERNET_ACCESS=$(yq '.providers.e2b.internetAccess // true' "$SINDRI_YAML")
    ALLOWED_DOMAINS=$(yq '.providers.e2b.allowedDomains // []' "$SINDRI_YAML")
    BLOCKED_DOMAINS=$(yq '.providers.e2b.blockedDomains // []' "$SINDRI_YAML")
    PUBLIC_ACCESS=$(yq '.providers.e2b.publicAccess // false' "$SINDRI_YAML")

    # Metadata (stored as JSON for E2B API)
    E2B_METADATA=$(yq -o=json '.providers.e2b.metadata // {}' "$SINDRI_YAML")

    # Team/billing
    E2B_TEAM=$(yq '.providers.e2b.team // ""' "$SINDRI_YAML")

    # Convert memory to MB for E2B
    local memory_raw
    memory_raw=$(yq '.deployment.resources.memory // "2GB"' "$SINDRI_YAML")
    MEMORY_MB=$(echo "$memory_raw" | sed 's/GB/*1024/;s/MB//' | bc)
}

# ============================================================================
# Prerequisite Checks
# ============================================================================

require_e2b_cli() {
    if ! command -v e2b >/dev/null 2>&1; then
        print_error "E2B CLI is not installed"
        echo "Install with: npm install -g @e2b/cli"
        echo "Or see: https://e2b.dev/docs/cli"
        exit 1
    fi
}

validate_api_key() {
    if [[ -z "${E2B_API_KEY:-}" ]]; then
        print_error "E2B_API_KEY environment variable is not set"
        echo ""
        echo "To fix this:"
        echo "  1. Get your API key from: https://e2b.dev/dashboard"
        echo "  2. Set the environment variable:"
        echo "     export E2B_API_KEY=e2b_..."
        echo ""
        echo "Or add to sindri.yaml secrets:"
        echo "  secrets:"
        echo "    - name: E2B_API_KEY"
        echo "      source: env"
        echo "      required: true"
        exit 1
    fi
}

validate_no_gpu() {
    if [[ "$GPU_ENABLED" == "true" ]]; then
        print_error "GPU is not supported on E2B provider"
        echo ""
        echo "E2B sandboxes do not support GPU workloads."
        echo "For GPU support, use one of these providers:"
        echo "  - fly: Fly.io with GPU machines"
        echo "  - devpod: DevPod with cloud GPU providers"
        echo "  - docker: Local Docker with nvidia-container-toolkit"
        exit 1
    fi
}

# ============================================================================
# Helper Functions
# ============================================================================

# Find sandbox by name (using metadata)
find_sandbox_by_name() {
    local name="$1"
    e2b sandbox list --json 2>/dev/null | \
        jq -r ".[] | select(.metadata.sindri_name == \"$name\") | .sandboxId" 2>/dev/null | \
        head -1
}

# Get sandbox state
get_sandbox_state() {
    local sandbox_id="$1"
    e2b sandbox list --json 2>/dev/null | \
        jq -r ".[] | select(.sandboxId == \"$sandbox_id\") | .status" 2>/dev/null
}

# Get sandbox info as JSON
get_sandbox_info() {
    local sandbox_id="$1"
    e2b sandbox list --json 2>/dev/null | \
        jq ".[] | select(.sandboxId == \"$sandbox_id\")" 2>/dev/null
}

# Check if template exists
template_exists() {
    local alias="$1"
    e2b template list --json 2>/dev/null | \
        jq -e ".[] | select(.templateId == \"$alias\" or .alias == \"$alias\")" >/dev/null 2>&1
}

# Get template ID from alias
get_template_id() {
    local alias="$1"
    e2b template list --json 2>/dev/null | \
        jq -r ".[] | select(.alias == \"$alias\") | .templateId" 2>/dev/null | \
        head -1
}

# Build E2B template from Sindri Dockerfile
build_template() {
    print_status "Building E2B template: $TEMPLATE_ALIAS"

    # Create output directory for template
    mkdir -p "$OUTPUT_DIR/template"

    # Generate e2b.Dockerfile from Sindri Dockerfile
    generate_e2b_dockerfile

    # Generate e2b.toml configuration
    generate_e2b_toml

    if [[ "$CONFIG_ONLY" == "true" ]]; then
        print_success "Template files generated at $OUTPUT_DIR/template/"
        return 0
    fi

    # Build template using E2B CLI
    print_status "Building template with E2B (this may take 2-5 minutes)..."
    local build_args=()

    if [[ -n "$E2B_TEAM" ]] && [[ "$E2B_TEAM" != "null" ]]; then
        build_args+=(--team "$E2B_TEAM")
    fi

    # Run e2b template build from the template directory
    (
        cd "$OUTPUT_DIR/template"
        e2b template build \
            --name "$TEMPLATE_ALIAS" \
            --dockerfile e2b.Dockerfile \
            --cpu-count "$CPUS" \
            --memory-mb "$MEMORY_MB" \
            "${build_args[@]}"
    )

    print_success "Template built: $TEMPLATE_ALIAS"
}

# Generate E2B Dockerfile (based on Sindri Dockerfile)
generate_e2b_dockerfile() {
    local skip_auto_install
    skip_auto_install=$(adapter_get_skip_auto_install)

    # Get the base Dockerfile path
    local dockerfile_path="$BASE_DIR/Dockerfile"
    if [[ ! -f "$dockerfile_path" ]]; then
        print_error "Dockerfile not found at $dockerfile_path"
        exit 1
    fi

    # Copy the Dockerfile and add E2B-specific environment variables
    cat > "$OUTPUT_DIR/template/e2b.Dockerfile" << 'EODF'
# E2B Template Dockerfile for Sindri
# Generated from Sindri Dockerfile with E2B-specific configuration

EODF

    # Append the original Dockerfile content
    cat "$dockerfile_path" >> "$OUTPUT_DIR/template/e2b.Dockerfile"

    # Add E2B-specific environment variables and configuration
    cat >> "$OUTPUT_DIR/template/e2b.Dockerfile" << EODF

# E2B-specific configuration
ENV E2B_PROVIDER=true
ENV INSTALL_PROFILE="${PROFILE}"
ENV ADDITIONAL_EXTENSIONS="${ADDITIONAL_EXTENSIONS}"
ENV SKIP_AUTO_INSTALL="${skip_auto_install}"
ENV INIT_WORKSPACE=true

# Set working directory for E2B
WORKDIR /alt/home/developer/workspace

# Switch to developer user
USER developer

# E2B ready command - verify environment is initialized
# E2B will wait for this to succeed before marking sandbox as ready
EODF

    # Add NPM_TOKEN if set (for CI or when passed from environment)
    # This bypasses npm registry rate limits during extension installation
    if [[ -n "${NPM_TOKEN:-}" ]]; then
        # Insert NPM_TOKEN before WORKDIR line
        sed -i.bak 's|^WORKDIR /alt/home/developer/workspace|ENV NPM_TOKEN="'"${NPM_TOKEN}"'"\nWORKDIR /alt/home/developer/workspace|' "$OUTPUT_DIR/template/e2b.Dockerfile"
        rm -f "$OUTPUT_DIR/template/e2b.Dockerfile.bak"
    fi

    print_status "Generated e2b.Dockerfile"
}

# Generate e2b.toml configuration
generate_e2b_toml() {
    cat > "$OUTPUT_DIR/template/e2b.toml" << EOTOML
# E2B Template Configuration for Sindri
# Generated from sindri.yaml

# Template identification
[template]
name = "${TEMPLATE_ALIAS}"
dockerfile = "e2b.Dockerfile"

# Resource configuration
[resources]
cpu_count = ${CPUS}
memory_mb = ${MEMORY_MB}

# Build configuration
[build]
# Copy docker lib directory for extensions
start_cmd = "/docker/scripts/entrypoint.sh echo 'Sindri environment ready'"

# Network configuration (applied at sandbox creation)
# Note: These are informational; actual config is passed via SDK/CLI
EOTOML

    print_status "Generated e2b.toml"
}

# Create sandbox from template
create_sandbox() {
    local template_id="$1"
    local timeout_ms=$((TIMEOUT * 1000))

    print_status "Creating sandbox from template: $template_id"

    # Build metadata JSON
    local metadata_json
    metadata_json=$(jq -n \
        --arg name "$NAME" \
        --arg profile "$PROFILE" \
        --argjson custom "$E2B_METADATA" \
        '{sindri_name: $name, sindri_profile: $profile} + $custom')

    # Build create command arguments
    local create_args=()
    create_args+=(--timeout "$timeout_ms")

    if [[ "$AUTO_PAUSE" == "true" ]] && [[ "$EPHEMERAL" != "true" ]]; then
        create_args+=(--on-timeout "pause")
    fi

    if [[ -n "$E2B_TEAM" ]] && [[ "$E2B_TEAM" != "null" ]]; then
        create_args+=(--team "$E2B_TEAM")
    fi

    # Create sandbox with metadata
    local sandbox_id
    sandbox_id=$(e2b sandbox create "$template_id" \
        "${create_args[@]}" \
        --metadata "$metadata_json" \
        --json 2>/dev/null | jq -r '.sandboxId')

    if [[ -z "$sandbox_id" ]] || [[ "$sandbox_id" == "null" ]]; then
        print_error "Failed to create sandbox"
        exit 1
    fi

    echo "$sandbox_id"
}

# Resume a paused sandbox
resume_sandbox() {
    local sandbox_id="$1"
    print_status "Resuming sandbox: $sandbox_id"
    e2b sandbox resume "$sandbox_id"
    print_success "Sandbox resumed"
}

# Connect via PTY (WebSocket terminal)
connect_pty() {
    local sandbox_id="$1"

    print_status "Connecting to sandbox: $sandbox_id"
    echo ""

    # Use e2b CLI's built-in terminal command
    # --shell specifies the shell to use inside the sandbox
    e2b sandbox terminal "$sandbox_id" --shell /bin/bash
}

# ============================================================================
# Commands
# ============================================================================

cmd_deploy() {
    parse_config
    validate_no_gpu

    # Output variables for CI integration
    if [[ "$OUTPUT_VARS" == "true" ]]; then
        cat << EOJSON
{
  "name": "$NAME",
  "templateAlias": "$TEMPLATE_ALIAS",
  "profile": "$PROFILE",
  "memory_mb": $MEMORY_MB,
  "cpus": $CPUS,
  "timeout": $TIMEOUT,
  "autoPause": $AUTO_PAUSE,
  "autoResume": $AUTO_RESUME,
  "ephemeral": $EPHEMERAL
}
EOJSON
        exit 0
    fi

    # Handle config-only mode (doesn't require CLI or API key)
    if [[ "$CONFIG_ONLY" == "true" ]]; then
        # Generate template files without building or deploying
        mkdir -p "$OUTPUT_DIR/template"
        generate_e2b_dockerfile
        generate_e2b_toml
        print_success "Generated E2B template files at $OUTPUT_DIR/template/"
        echo "  Template alias: $TEMPLATE_ALIAS"
        echo "  Profile: $PROFILE"
        return 0
    fi

    # For actual deployment, require CLI and API key
    require_e2b_cli
    validate_api_key

    # Check for existing sandbox
    local existing_sandbox
    existing_sandbox=$(find_sandbox_by_name "$NAME")

    if [[ -n "$existing_sandbox" ]]; then
        local state
        state=$(get_sandbox_state "$existing_sandbox")

        case "$state" in
            running)
                print_success "Sandbox '$NAME' already running"
                echo ""
                echo "Sandbox ID: $existing_sandbox"
                echo "Connect: sindri connect"
                return 0
                ;;
            paused)
                if [[ "$AUTO_RESUME" == "true" ]]; then
                    print_status "Found paused sandbox, resuming..."
                    resume_sandbox "$existing_sandbox"
                    echo ""
                    echo "Sandbox ID: $existing_sandbox"
                    echo "Connect: sindri connect"
                    return 0
                else
                    print_warning "Sandbox '$NAME' is paused"
                    echo "Resume with: sindri connect (if autoResume is enabled)"
                    echo "Or manually: e2b sandbox resume $existing_sandbox"
                    return 0
                fi
                ;;
        esac
    fi

    # Determine if we need to build template
    local need_build=false
    if [[ "$BUILD_ON_DEPLOY" == "true" ]] || [[ "$REBUILD" == "true" ]]; then
        need_build=true
    elif ! template_exists "$TEMPLATE_ALIAS"; then
        print_status "Template '$TEMPLATE_ALIAS' not found, building..."
        need_build=true
    elif [[ "$REUSE_TEMPLATE" != "true" ]]; then
        need_build=true
    fi

    # Build template if needed
    if [[ "$need_build" == "true" ]]; then
        build_template
    fi

    print_header "Deploying to E2B"
    echo "  Sandbox: $NAME"
    echo "  Template: $TEMPLATE_ALIAS"
    echo "  Profile: $PROFILE"
    echo "  Resources: ${CPUS} CPUs, ${MEMORY_MB}MB memory"
    echo "  Timeout: ${TIMEOUT}s (auto-pause: $AUTO_PAUSE)"
    if [[ "$EPHEMERAL" == "true" ]]; then
        echo "  Mode: Ephemeral (no state persistence)"
    fi
    echo ""

    # Get template ID
    local template_id
    template_id=$(get_template_id "$TEMPLATE_ALIAS")
    if [[ -z "$template_id" ]] || [[ "$template_id" == "null" ]]; then
        # Template alias might be the template ID itself
        template_id="$TEMPLATE_ALIAS"
    fi

    # Create sandbox
    local sandbox_id
    sandbox_id=$(create_sandbox "$template_id")

    print_success "Sandbox '$NAME' deployed successfully"
    echo ""
    echo "Sandbox ID: $sandbox_id"
    echo ""
    echo "Connect:"
    echo "  sindri connect"
    echo "  e2b sandbox terminal $sandbox_id"
    echo ""
    echo "Manage:"
    echo "  sindri status"
    echo "  sindri pause       # Preserve state (free while paused)"
    echo "  sindri destroy     # Kill sandbox"
}

cmd_connect() {
    parse_config
    require_e2b_cli
    validate_api_key

    local sandbox_id
    sandbox_id=$(find_sandbox_by_name "$NAME")

    if [[ -z "$sandbox_id" ]]; then
        print_error "Sandbox '$NAME' not found"
        echo "Deploy first: sindri deploy --provider e2b"
        exit 1
    fi

    local state
    state=$(get_sandbox_state "$sandbox_id")

    case "$state" in
        running)
            # Already running, connect directly
            ;;
        paused)
            if [[ "$AUTO_RESUME" == "true" ]]; then
                print_status "Sandbox is paused, resuming..."
                resume_sandbox "$sandbox_id"
            else
                print_error "Sandbox is paused"
                echo "Enable auto-resume in sindri.yaml or resume manually:"
                echo "  e2b sandbox resume $sandbox_id"
                exit 1
            fi
            ;;
        *)
            print_error "Sandbox is in unexpected state: $state"
            exit 1
            ;;
    esac

    # Connect via PTY
    connect_pty "$sandbox_id"
}

cmd_pause() {
    parse_config
    require_e2b_cli
    validate_api_key

    local sandbox_id
    sandbox_id=$(find_sandbox_by_name "$NAME")

    if [[ -z "$sandbox_id" ]]; then
        print_error "Sandbox '$NAME' not found"
        exit 1
    fi

    local state
    state=$(get_sandbox_state "$sandbox_id")

    if [[ "$state" == "paused" ]]; then
        print_warning "Sandbox '$NAME' is already paused"
        return 0
    fi

    if [[ "$state" != "running" ]]; then
        print_error "Cannot pause sandbox in state: $state"
        exit 1
    fi

    print_status "Pausing sandbox '$NAME'..."
    print_status "Note: Pause takes ~4 seconds per 1 GiB of RAM"

    e2b sandbox pause "$sandbox_id"

    print_success "Sandbox paused"
    echo ""
    echo "Your sandbox state is preserved (memory + filesystem)."
    echo "No compute charges while paused (only snapshot storage)."
    echo ""
    echo "Resume with:"
    echo "  sindri connect    # Auto-resume (if enabled)"
    echo "  sindri deploy     # Explicit resume"
    echo ""
    echo "Important: Data expires 30 days from initial sandbox creation."
}

cmd_destroy() {
    parse_config

    if [[ "$FORCE" != "true" ]]; then
        print_warning "This will destroy sandbox '$NAME' and all its data"
        echo "Note: Unlike 'pause', this permanently deletes all data."
        read -p "Are you sure? (y/N) " -n 1 -r
        echo
        [[ ! $REPLY =~ ^[Yy]$ ]] && { print_status "Cancelled"; exit 0; }
    fi

    require_e2b_cli
    validate_api_key

    print_header "Destroying E2B sandbox: $NAME"

    local sandbox_id
    sandbox_id=$(find_sandbox_by_name "$NAME")

    if [[ -n "$sandbox_id" ]]; then
        print_status "Killing sandbox: $sandbox_id"
        e2b sandbox kill "$sandbox_id"
        print_success "Sandbox destroyed"
    else
        print_warning "Sandbox '$NAME' not found"
    fi

    # Clean up local files
    if [[ -d "$OUTPUT_DIR" ]]; then
        print_status "Cleaning up local files..."
        rm -rf "$OUTPUT_DIR"
    fi
}

cmd_plan() {
    parse_config

    print_header "E2B Deployment Plan"
    echo ""
    echo "Sandbox:    $NAME"
    echo "Template:   $TEMPLATE_ALIAS"
    echo "Profile:    $PROFILE"
    echo ""
    echo "Resources:"
    echo "  CPUs:     $CPUS"
    echo "  Memory:   ${MEMORY_MB}MB"
    echo "  Timeout:  ${TIMEOUT}s"
    echo ""
    echo "Behavior:"
    echo "  Auto-pause:   $AUTO_PAUSE"
    echo "  Auto-resume:  $AUTO_RESUME"
    echo "  Ephemeral:    $EPHEMERAL"
    echo "  Reuse template: $REUSE_TEMPLATE"
    echo ""
    echo "Network:"
    echo "  Internet:  $INTERNET_ACCESS"
    echo "  Public:    $PUBLIC_ACCESS"
    if [[ "$ALLOWED_DOMAINS" != "[]" ]]; then
        echo "  Allowed:   $ALLOWED_DOMAINS"
    fi
    if [[ "$BLOCKED_DOMAINS" != "[]" ]]; then
        echo "  Blocked:   $BLOCKED_DOMAINS"
    fi
    echo ""
    echo "Actions:"
    echo "  1. Check for existing sandbox '$NAME'"

    if template_exists "$TEMPLATE_ALIAS" 2>/dev/null; then
        echo "  2. Use existing template: $TEMPLATE_ALIAS"
    else
        echo "  2. Build E2B template from Sindri Dockerfile"
        echo "     - Generate e2b.Dockerfile"
        echo "     - Generate e2b.toml"
        echo "     - Upload and build template (2-5 minutes)"
    fi

    echo "  3. Create sandbox from template"
    if [[ "$AUTO_PAUSE" == "true" ]] && [[ "$EPHEMERAL" != "true" ]]; then
        echo "     - Configure auto-pause on timeout"
    fi
    echo "  4. Inject metadata for sandbox identification"
    echo ""
    echo "Estimated startup time: ~150ms (after template is built)"
}

cmd_status() {
    parse_config
    require_e2b_cli
    validate_api_key

    print_header "E2B Deployment Status"
    echo ""
    echo "Sandbox: $NAME"
    echo "Template: $TEMPLATE_ALIAS"
    echo ""

    local sandbox_id
    sandbox_id=$(find_sandbox_by_name "$NAME")

    if [[ -n "$sandbox_id" ]]; then
        local info
        info=$(get_sandbox_info "$sandbox_id")

        local state started_at
        state=$(echo "$info" | jq -r '.status // "unknown"')
        started_at=$(echo "$info" | jq -r '.startedAt // "unknown"')

        echo "Sandbox ID: $sandbox_id"
        echo "State:      $state"
        echo "Started:    $started_at"

        # Show template info
        local template_id
        template_id=$(echo "$info" | jq -r '.templateId // "unknown"')
        echo "Template:   $template_id"

        # Show resource info if available
        local cpu_count memory_mb
        cpu_count=$(echo "$info" | jq -r '.cpuCount // "unknown"')
        memory_mb=$(echo "$info" | jq -r '.memoryMB // "unknown"')
        echo "Resources:  ${cpu_count} CPUs, ${memory_mb}MB RAM"

        echo ""
        case "$state" in
            running)
                echo "Commands:"
                echo "  Connect: sindri connect"
                echo "  Pause:   sindri pause"
                echo "  Destroy: sindri destroy"
                ;;
            paused)
                echo "Sandbox is paused (no compute charges)."
                echo "Commands:"
                echo "  Resume:  sindri connect (auto-resume)"
                echo "  Resume:  e2b sandbox resume $sandbox_id"
                echo "  Destroy: sindri destroy"
                ;;
            *)
                echo "Sandbox is in state: $state"
                ;;
        esac
    else
        echo "Status: Not deployed"
        echo ""

        # Check if template exists
        if template_exists "$TEMPLATE_ALIAS" 2>/dev/null; then
            echo "Template '$TEMPLATE_ALIAS' exists."
            echo "Deploy with: sindri deploy --provider e2b"
        else
            echo "Template '$TEMPLATE_ALIAS' not found."
            echo "Deploy will build template first: sindri deploy --provider e2b"
        fi
    fi
}

cmd_template() {
    require_e2b_cli
    validate_api_key
    parse_config

    case "${TEMPLATE_SUBCMD:-}" in
        build)
            print_header "Building E2B Template"
            REBUILD=true
            build_template
            ;;
        list)
            print_header "E2B Templates"
            echo ""
            e2b template list
            ;;
        delete)
            local template_id
            template_id=$(get_template_id "$TEMPLATE_ALIAS")

            if [[ -z "$template_id" ]] || [[ "$template_id" == "null" ]]; then
                print_error "Template '$TEMPLATE_ALIAS' not found"
                exit 1
            fi

            if [[ "$FORCE" != "true" ]]; then
                print_warning "This will delete template '$TEMPLATE_ALIAS'"
                read -p "Are you sure? (y/N) " -n 1 -r
                echo
                [[ ! $REPLY =~ ^[Yy]$ ]] && { print_status "Cancelled"; exit 0; }
            fi

            print_status "Deleting template: $TEMPLATE_ALIAS"
            e2b template delete "$template_id"
            print_success "Template deleted"
            ;;
        ""|help)
            echo "Usage: e2b-adapter.sh template <subcommand>"
            echo ""
            echo "Subcommands:"
            echo "  build   Build/rebuild the E2B template"
            echo "  list    List all available templates"
            echo "  delete  Delete the template"
            echo ""
            echo "Options:"
            echo "  --force    Skip confirmation prompts (delete only)"
            ;;
        *)
            print_error "Unknown template subcommand: $TEMPLATE_SUBCMD"
            echo "Use 'e2b-adapter.sh template help' for usage"
            exit 1
            ;;
    esac
}

# ============================================================================
# Extended Command Dispatch
# ============================================================================

# E2B has additional commands (pause, template), so we use custom dispatch
case "$COMMAND" in
    deploy)   cmd_deploy ;;
    connect)  cmd_connect ;;
    pause)    cmd_pause ;;
    destroy)  cmd_destroy ;;
    plan)     cmd_plan ;;
    status)   cmd_status ;;
    template) cmd_template ;;
    help|--help|-h) show_help ;;
    *)
        echo "Unknown command: $COMMAND" >&2
        echo "Commands: deploy, connect, pause, destroy, plan, status, template"
        exit 1
        ;;
esac
