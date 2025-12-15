#!/bin/bash
# cli.sh - CLI argument parsing and help (declarative)

MODULE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Detect environment and source common functions
if [[ -f "/docker/lib/common.sh" ]]; then
    # Running inside container
    source /docker/lib/common.sh
else
    # Running on host
    source "${MODULE_DIR}/../../docker/lib/common.sh"
fi

show_help() {
    cat << 'EOF'
extension-manager - Manage Sindri extensions

USAGE:
    extension-manager <command> [options]

COMMANDS:
    list                    List all available extensions
    list-profiles           List all available profiles
    list-categories         List all available categories
    install <name>          Install specific extension
    install-profile <name>  Install extension profile
    install-all             Install all active extensions
    reinstall <name>        Remove and reinstall extension
    reinstall-profile <name> Remove and reinstall all extensions in profile
    remove <name>           Remove extension
    validate <name>         Validate extension installation
    validate-all            Validate all installed extensions
    validate-domains [name] Validate domain requirements (format, DNS)
    status [name]           Show extension status
    resolve <name>          Show dependency resolution order
    search <term>           Search extensions by name or description
    info <name>             Show detailed extension information
    bom [name]              Show Bill of Materials (BOM) for extension or all
    bom-regenerate          Regenerate all BOMs

OPTIONS:
    -h, --help              Show this help message
    -v, --verbose           Verbose output
    --dry-run               Show what would be done
    --force                 Force operation without confirmation (for remove)
    --category <name>       Filter by category
    --profile <name>        Use specific profile
    --format <format>       BOM output format (yaml|json|csv|cyclonedx|spdx)
    --check-dns             Include DNS resolution check (validate-domains)

EXAMPLES:
    extension-manager list
    extension-manager list --category language
    extension-manager install nodejs
    extension-manager install-profile fullstack
    extension-manager reinstall supabase-cli  # Remove and reinstall extension
    extension-manager reinstall-profile base  # Reinstall all extensions in profile
    extension-manager search python
    extension-manager info docker
    extension-manager bom                    # Show complete BOM
    extension-manager bom nodejs             # Show BOM for nodejs
    extension-manager bom --format json      # Export as JSON
    extension-manager bom --format cyclonedx # Export as CycloneDX SBOM
    extension-manager bom-regenerate         # Regenerate all BOMs
EOF
}

parse_args() {
    # Global flags - initialize defaults
    VERBOSE="${VERBOSE:-false}"
    DRY_RUN="${DRY_RUN:-false}"
    FORCE_MODE="${FORCE_MODE:-false}"
    CHECK_DNS="${CHECK_DNS:-false}"
    FILTER_CATEGORY="${FILTER_CATEGORY:-}"
    USE_PROFILE="${USE_PROFILE:-}"
    FORMAT="${FORMAT:-yaml}"

    # Parse global flags
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -h|--help)
                show_help
                exit 0
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --force)
                FORCE_MODE=true
                shift
                ;;
            --check-dns)
                CHECK_DNS=true
                shift
                ;;
            --category)
                FILTER_CATEGORY="$2"
                shift 2
                ;;
            --profile)
                USE_PROFILE="$2"
                shift 2
                ;;
            --format)
                FORMAT="$2"
                shift 2
                ;;
            *)
                # Not a global flag, break to command parsing
                break
                ;;
        esac
    done

    # Store remaining args in global array (avoids subshell issue)
    # shellcheck disable=SC2034  # Used by extension-manager main script
    PARSED_ARGS=("$@")
}

# Export functions
export -f show_help parse_args