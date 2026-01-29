#!/usr/bin/env bash
# Extension Discovery Script for Sindri v3
# Scans v3/extensions/*/extension.yaml and enriches with registry.yaml metadata
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
EXTENSIONS_DIR="$REPO_ROOT/v3/extensions"
REGISTRY_FILE="$REPO_ROOT/v3/registry.yaml"
PROFILES_FILE="$REPO_ROOT/v3/profiles.yaml"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $*" >&2; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $*" >&2; }
log_error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $*" >&2; }

usage() {
  cat <<EOF
Extension Discovery Script for Sindri v3

USAGE:
    $(basename "$0") <command> [options]

COMMANDS:
    discover [format]     List all extensions with metadata
                         format: json (default), yaml, table

    profile <name>        Get extensions for a specific profile
                         Profiles: minimal, fullstack, ai-dev, anthropic-dev,
                                   systems, enterprise, devops, mobile

    category <name>       Get extensions for a specific category
                         Categories: ai-agents, ai-dev, claude, cloud, desktop,
                                     devops, documentation, languages, mcp,
                                     productivity, research, testing

    categories            List all available categories

    profiles              List all available profiles with their extensions

    changed [base-ref]    Get extensions changed compared to base ref (default: main)

    validate              Validate all extension.yaml files

OPTIONS:
    --filter-memory <MB>  Filter extensions by max memory requirement
    --filter-gpu          Exclude extensions requiring GPU
    --filter-heavy        Exclude heavy extensions (>4GB memory, >10min install)

EXAMPLES:
    $(basename "$0") discover json
    $(basename "$0") profile minimal
    $(basename "$0") category languages
    $(basename "$0") discover json --filter-memory 2048 --filter-gpu
EOF
  exit 1
}

# Parse extension.yaml and extract metadata
parse_extension() {
  local ext_dir="$1"
  local ext_name
  ext_name=$(basename "$ext_dir")
  local ext_file="$ext_dir/extension.yaml"

  if [[ ! -f "$ext_file" ]]; then
    return 1
  fi

  # Extract fields using yq (if available) or basic grep/sed
  if command -v yq &>/dev/null; then
    local version description category memory disk install_time gpu_required
    version=$(yq -r '.metadata.version // "1.0.0"' "$ext_file")
    description=$(yq -r '.metadata.description // ""' "$ext_file")
    category=$(yq -r '.metadata.category // "unknown"' "$ext_file")
    memory=$(yq -r '.requirements.memory // 256' "$ext_file")
    disk=$(yq -r '.requirements.diskSpace // 100' "$ext_file")
    install_time=$(yq -r '.requirements.installTime // 60' "$ext_file")
    gpu_required=$(yq -r '.requirements.gpu.required // false' "$ext_file")

    jq -n \
      --arg name "$ext_name" \
      --arg version "$version" \
      --arg description "$description" \
      --arg category "$category" \
      --argjson memory "${memory:-256}" \
      --argjson disk "${disk:-100}" \
      --argjson install_time "${install_time:-60}" \
      --argjson gpu_required "${gpu_required:-false}" \
      '{
        name: $name,
        version: $version,
        description: $description,
        category: $category,
        requirements: {
          memory: $memory,
          disk: $disk,
          install_time: $install_time,
          gpu_required: $gpu_required
        }
      }'
  else
    # Fallback: basic parsing without yq
    local category
    category=$(grep -E "^\s*category:" "$ext_file" | head -1 | sed 's/.*category:\s*//' | tr -d '"' || echo "unknown")

    jq -n \
      --arg name "$ext_name" \
      --arg category "$category" \
      '{
        name: $name,
        version: "1.0.0",
        description: "",
        category: $category,
        requirements: {
          memory: 256,
          disk: 100,
          install_time: 60,
          gpu_required: false
        }
      }'
  fi
}

# Discover all extensions
cmd_discover() {
  local format="${1:-json}"
  local filter_memory="${FILTER_MEMORY:-0}"
  local filter_gpu="${FILTER_GPU:-false}"
  local filter_heavy="${FILTER_HEAVY:-false}"

  local extensions=()

  for ext_dir in "$EXTENSIONS_DIR"/*/; do
    [[ -d "$ext_dir" ]] || continue
    local ext_json
    ext_json=$(parse_extension "$ext_dir" 2>/dev/null) || continue

    # Apply filters
    if [[ "$filter_memory" -gt 0 ]]; then
      local mem
      mem=$(echo "$ext_json" | jq -r '.requirements.memory')
      [[ "$mem" -gt "$filter_memory" ]] && continue
    fi

    if [[ "$filter_gpu" == "true" ]]; then
      local gpu
      gpu=$(echo "$ext_json" | jq -r '.requirements.gpu_required')
      [[ "$gpu" == "true" ]] && continue
    fi

    if [[ "$filter_heavy" == "true" ]]; then
      local mem time
      mem=$(echo "$ext_json" | jq -r '.requirements.memory')
      time=$(echo "$ext_json" | jq -r '.requirements.install_time')
      [[ "$mem" -gt 4096 || "$time" -gt 600 ]] && continue
    fi

    extensions+=("$ext_json")
  done

  case "$format" in
    json)
      printf '%s\n' "${extensions[@]}" | jq -s '.'
      ;;
    yaml)
      printf '%s\n' "${extensions[@]}" | jq -s '.' | yq -P
      ;;
    table)
      echo "NAME|VERSION|CATEGORY|MEMORY|DISK|INSTALL_TIME|GPU"
      echo "----|-------|--------|------|----|-----------|----|"
      for ext in "${extensions[@]}"; do
        echo "$ext" | jq -r '[.name, .version, .category, .requirements.memory, .requirements.disk, .requirements.install_time, .requirements.gpu_required] | join("|")'
      done | column -t -s'|'
      ;;
    names)
      for ext in "${extensions[@]}"; do
        echo "$ext" | jq -r '.name'
      done
      ;;
    *)
      log_error "Unknown format: $format"
      exit 1
      ;;
  esac
}

# Get extensions for a profile
cmd_profile() {
  local profile_name="$1"

  if [[ ! -f "$PROFILES_FILE" ]]; then
    log_error "Profiles file not found: $PROFILES_FILE"
    exit 1
  fi

  if command -v yq &>/dev/null; then
    local extensions
    extensions=$(yq -r ".profiles.${profile_name}.extensions[]?" "$PROFILES_FILE" 2>/dev/null)

    if [[ -z "$extensions" ]]; then
      log_error "Profile not found: $profile_name"
      log_info "Available profiles: $(yq -r '.profiles | keys | .[]' "$PROFILES_FILE" | tr '\n' ', ')"
      exit 1
    fi

    # Output as JSON array
    echo "$extensions" | jq -R . | jq -s .
  else
    log_error "yq is required for profile parsing"
    exit 1
  fi
}

# Get extensions for a category
cmd_category() {
  local category_name="$1"

  local extensions=()

  for ext_dir in "$EXTENSIONS_DIR"/*/; do
    [[ -d "$ext_dir" ]] || continue
    local ext_file="$ext_dir/extension.yaml"
    [[ -f "$ext_file" ]] || continue

    local cat
    if command -v yq &>/dev/null; then
      cat=$(yq -r '.metadata.category // ""' "$ext_file")
    else
      cat=$(grep -E "^\s*category:" "$ext_file" | head -1 | sed 's/.*category:\s*//' | tr -d '"')
    fi

    if [[ "$cat" == "$category_name" ]]; then
      extensions+=("$(basename "$ext_dir")")
    fi
  done

  if [[ ${#extensions[@]} -eq 0 ]]; then
    log_error "No extensions found for category: $category_name"
    exit 1
  fi

  printf '%s\n' "${extensions[@]}" | jq -R . | jq -s .
}

# List all categories
cmd_categories() {
  if [[ -f "$REGISTRY_FILE" ]] && command -v yq &>/dev/null; then
    yq -r '.categories | keys | .[]' "$REGISTRY_FILE" | jq -R . | jq -s .
  else
    # Discover from extension files
    local categories=()
    for ext_dir in "$EXTENSIONS_DIR"/*/; do
      [[ -d "$ext_dir" ]] || continue
      local ext_file="$ext_dir/extension.yaml"
      [[ -f "$ext_file" ]] || continue

      local cat
      if command -v yq &>/dev/null; then
        cat=$(yq -r '.metadata.category // ""' "$ext_file")
      else
        cat=$(grep -E "^\s*category:" "$ext_file" | head -1 | sed 's/.*category:\s*//' | tr -d '"')
      fi

      [[ -n "$cat" ]] && categories+=("$cat")
    done

    printf '%s\n' "${categories[@]}" | sort -u | jq -R . | jq -s .
  fi
}

# List all profiles
cmd_profiles() {
  if [[ ! -f "$PROFILES_FILE" ]]; then
    log_error "Profiles file not found: $PROFILES_FILE"
    exit 1
  fi

  if command -v yq &>/dev/null; then
    yq -r '.profiles | to_entries[] | {name: .key, description: .value.description, extensions: .value.extensions}' "$PROFILES_FILE" | jq -s .
  else
    log_error "yq is required for profile listing"
    exit 1
  fi
}

# Get changed extensions compared to base ref
cmd_changed() {
  local base_ref="${1:-main}"

  # Get changed files in extensions directory
  local changed_files
  changed_files=$(git diff --name-only "$base_ref"...HEAD -- "v3/extensions/" 2>/dev/null || git diff --name-only "$base_ref" -- "v3/extensions/" 2>/dev/null || echo "")

  if [[ -z "$changed_files" ]]; then
    echo "[]"
    return
  fi

  # Extract unique extension names
  local extensions=()
  while IFS= read -r file; do
    local ext_name
    ext_name=$(echo "$file" | sed -n 's|v3/extensions/\([^/]*\)/.*|\1|p')
    [[ -n "$ext_name" ]] && extensions+=("$ext_name")
  done <<< "$changed_files"

  printf '%s\n' "${extensions[@]}" | sort -u | jq -R . | jq -s .
}

# Validate all extension.yaml files
cmd_validate() {
  local errors=0
  local count=0

  for ext_dir in "$EXTENSIONS_DIR"/*/; do
    [[ -d "$ext_dir" ]] || continue
    local ext_name
    ext_name=$(basename "$ext_dir")
    local ext_file="$ext_dir/extension.yaml"

    ((count++))

    if [[ ! -f "$ext_file" ]]; then
      log_error "Missing extension.yaml: $ext_name"
      ((errors++))
      continue
    fi

    # Check required fields
    if command -v yq &>/dev/null; then
      local name version description category
      name=$(yq -r '.metadata.name // ""' "$ext_file")
      version=$(yq -r '.metadata.version // ""' "$ext_file")
      description=$(yq -r '.metadata.description // ""' "$ext_file")
      category=$(yq -r '.metadata.category // ""' "$ext_file")

      if [[ -z "$name" ]]; then
        log_error "[$ext_name] Missing metadata.name"
        ((errors++))
      fi

      if [[ -z "$version" ]]; then
        log_error "[$ext_name] Missing metadata.version"
        ((errors++))
      fi

      if [[ -z "$category" ]]; then
        log_error "[$ext_name] Missing metadata.category"
        ((errors++))
      fi
    fi
  done

  if [[ $errors -eq 0 ]]; then
    log_success "Validated $count extensions with no errors"
  else
    log_error "Found $errors errors in $count extensions"
    exit 1
  fi
}

# Main
main() {
  local cmd="${1:-}"
  shift || true

  # Parse global options
  FILTER_MEMORY=0
  FILTER_GPU=false
  FILTER_HEAVY=false

  local args=()
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --filter-memory)
        FILTER_MEMORY="$2"
        shift 2
        ;;
      --filter-gpu)
        FILTER_GPU=true
        shift
        ;;
      --filter-heavy)
        FILTER_HEAVY=true
        shift
        ;;
      *)
        args+=("$1")
        shift
        ;;
    esac
  done

  export FILTER_MEMORY FILTER_GPU FILTER_HEAVY

  case "$cmd" in
    discover)
      cmd_discover "${args[0]:-json}"
      ;;
    profile)
      [[ ${#args[@]} -lt 1 ]] && { log_error "Profile name required"; exit 1; }
      cmd_profile "${args[0]}"
      ;;
    category)
      [[ ${#args[@]} -lt 1 ]] && { log_error "Category name required"; exit 1; }
      cmd_category "${args[0]}"
      ;;
    categories)
      cmd_categories
      ;;
    profiles)
      cmd_profiles
      ;;
    changed)
      cmd_changed "${args[0]:-main}"
      ;;
    validate)
      cmd_validate
      ;;
    -h|--help|help)
      usage
      ;;
    "")
      usage
      ;;
    *)
      log_error "Unknown command: $cmd"
      usage
      ;;
  esac
}

main "$@"
