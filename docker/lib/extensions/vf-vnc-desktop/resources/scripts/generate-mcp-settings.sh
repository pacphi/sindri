#!/bin/bash
# ============================================================================
# Dynamic MCP Settings Generator
# Discovers skills from SKILL.md frontmatter and generates mcp_settings.json
# ============================================================================

set -e

SKILLS_DIR="${SKILLS_DIR:-/home/devuser/.claude/skills}"
OUTPUT_FILE="${OUTPUT_FILE:-/home/devuser/.config/claude/mcp_settings.json}"
VERBOSE="${VERBOSE:-false}"

log() {
    if [ "$VERBOSE" = "true" ]; then
        echo "[mcp-gen] $1"
    fi
}

# Parse YAML frontmatter from SKILL.md
# Returns: name|protocol|entry_point
parse_skill_frontmatter() {
    local skill_md="$1"
    local in_frontmatter=false
    local name="" protocol="" entry_point="" mcp_server="false"

    while IFS= read -r line; do
        if [ "$line" = "---" ]; then
            if [ "$in_frontmatter" = "false" ]; then
                in_frontmatter=true
                continue
            else
                break
            fi
        fi

        if [ "$in_frontmatter" = "true" ]; then
            # Extract key: value pairs
            case "$line" in
                name:*)
                    name=$(echo "$line" | sed 's/^name:[[:space:]]*//')
                    ;;
                protocol:*)
                    protocol=$(echo "$line" | sed 's/^protocol:[[:space:]]*//')
                    ;;
                entry_point:*)
                    entry_point=$(echo "$line" | sed 's/^entry_point:[[:space:]]*//')
                    ;;
                mcp_server:*)
                    mcp_server=$(echo "$line" | sed 's/^mcp_server:[[:space:]]*//')
                    ;;
            esac
        fi
    done < "$skill_md"

    # Only return skills with mcp_server: true
    if [ "$mcp_server" = "true" ] && [ -n "$name" ] && [ -n "$entry_point" ]; then
        echo "${name}|${protocol:-stdio}|${entry_point}"
    fi
}

# Determine command and args based on entry point extension
get_command_for_entry() {
    local entry_point="$1"
    local protocol="$2"

    case "$entry_point" in
        *.py)
            echo "python3|-u"
            ;;
        *.js)
            echo "node|"
            ;;
        *)
            echo "python3|-u"
            ;;
    esac
}

# Build environment variables for specific skills
get_skill_env() {
    local skill_name="$1"

    case "$skill_name" in
        web-summary)
            cat <<EOF
        "ZAI_URL": "http://localhost:9600/chat",
        "ZAI_TIMEOUT": "60"
EOF
            ;;
        qgis)
            cat <<EOF
        "QGIS_HOST": "localhost",
        "QGIS_PORT": "9877"
EOF
            ;;
        blender)
            cat <<EOF
        "BLENDER_HOST": "localhost",
        "BLENDER_PORT": "9876"
EOF
            ;;
        playwright)
            cat <<EOF
        "DISPLAY": ":1",
        "CHROMIUM_PATH": "/usr/bin/chromium",
        "SCREENSHOT_DIR": "/tmp/playwright-screenshots"
EOF
            ;;
        imagemagick)
            cat <<EOF
        "IMAGEMAGICK_TEMP": "/tmp/imagemagick"
EOF
            ;;
        comfyui)
            cat <<EOF
        "COMFYUI_URL": "http://localhost:8188",
        "COMFYUI_OUTPUT_DIR": "/tmp/comfyui-outputs"
EOF
            ;;
        perplexity)
            cat <<EOF
        "PERPLEXITY_API_KEY": "\$PERPLEXITY_API_KEY"
EOF
            ;;
        deepseek-reasoning)
            cat <<EOF
        "DEEPSEEK_API_KEY": "\$DEEPSEEK_API_KEY",
        "DEEPSEEK_BASE_URL": "https://api.deepseek.com/v1"
EOF
            ;;
        *)
            echo ""
            ;;
    esac
}

# Main generation
generate_mcp_settings() {
    local first=true
    local skill_count=0

    echo "{"
    echo '  "mcpServers": {'

    # Find all SKILL.md files
    for skill_md in "$SKILLS_DIR"/*/SKILL.md; do
        [ -f "$skill_md" ] || continue

        local skill_dir=$(dirname "$skill_md")
        local parsed=$(parse_skill_frontmatter "$skill_md")

        [ -z "$parsed" ] && continue

        local name=$(echo "$parsed" | cut -d'|' -f1)
        local protocol=$(echo "$parsed" | cut -d'|' -f2)
        local entry_point=$(echo "$parsed" | cut -d'|' -f3)
        local full_path="$skill_dir/$entry_point"

        # Verify entry point exists
        if [ ! -f "$full_path" ]; then
            log "Warning: Entry point not found for $name: $full_path"
            continue
        fi

        # Get command configuration
        local cmd_info=$(get_command_for_entry "$entry_point" "$protocol")
        local command=$(echo "$cmd_info" | cut -d'|' -f1)
        local extra_args=$(echo "$cmd_info" | cut -d'|' -f2)

        # Comma handling
        if [ "$first" = "true" ]; then
            first=false
        else
            echo ","
        fi

        # Build args array
        local args_json=""
        if [ -n "$extra_args" ]; then
            args_json="[\"$extra_args\", \"$full_path\"]"
        else
            args_json="[\"$full_path\"]"
        fi

        # Get environment variables
        local env_vars=$(get_skill_env "$name")

        # Output skill entry
        echo -n "    \"$name\": {"
        echo -n "\"command\": \"$command\", \"args\": $args_json"

        if [ -n "$env_vars" ]; then
            echo -n ", \"env\": {"
            echo -n "$env_vars"
            echo -n "}"
        fi

        echo -n "}"

        skill_count=$((skill_count + 1))
        log "Registered: $name ($protocol) -> $full_path"
    done

    echo ""
    echo "  },"

    # VisionFlow integration section
    cat <<EOF
  "visionflow": {
    "tcp_bridge": {
      "host": "localhost",
      "port": 9500
    },
    "discovery": {
      "resource_pattern": "{skill}://capabilities",
      "refresh_interval": 300
    }
  },
  "metadata": {
    "generated_at": "$(date -Iseconds)",
    "skills_count": $skill_count,
    "generator": "generate-mcp-settings.sh v2.0.0"
  }
}
EOF

    log "Generated MCP settings with $skill_count skills"
}

# Create output directory
mkdir -p "$(dirname "$OUTPUT_FILE")"

# Generate and write
generate_mcp_settings > "$OUTPUT_FILE"

echo "✓ Generated $OUTPUT_FILE with $(grep -c '"command":' "$OUTPUT_FILE") MCP servers"
