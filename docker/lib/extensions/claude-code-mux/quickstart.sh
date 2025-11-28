#!/bin/bash
set -euo pipefail

# CCM Quickstart Setup Script
# Interactive configuration for multi-model routing

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

CONFIG_FILE="/workspace/config/ccm-config.toml"

print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_status "  Claude Code Mux - Quickstart Setup"
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Check if config already exists
if [ -f "$CONFIG_FILE" ]; then
    print_warning "Configuration already exists at: $CONFIG_FILE"
    echo ""
    echo "What would you like to do?"
    echo "  1) Keep existing configuration and start CCM (recommended)"
    echo "  2) Edit existing config with \$EDITOR"
    echo "  3) Overwrite with new configuration (CAUTION: creates backup)"
    echo "  4) Cancel"
    echo ""
    read -rp "Choice [1-4]: " choice

    case $choice in
        1)
            print_success "Using existing configuration"
            exec ccmctl start
            exit 0
            ;;
        2)
            print_status "Opening config in editor..."
            ${EDITOR:-nano} "$CONFIG_FILE"
            print_success "Configuration updated"
            echo ""
            read -rp "Start CCM server now? [Y/n]: " start_now
            if [[ ! $start_now =~ ^[Nn]$ ]]; then
                exec ccmctl start
            fi
            exit 0
            ;;
        3)
            print_warning "Creating backup at: ${CONFIG_FILE}.backup"
            cp "$CONFIG_FILE" "${CONFIG_FILE}.backup"
            print_success "Backup created"
            # Continue to setup wizard below
            ;;
        *)
            print_status "Setup cancelled"
            exit 0
            ;;
    esac
fi

# Setup wizard for new installations
echo "Select your quickstart configuration:"
echo ""
echo "  1) Free OAuth Setup (Claude Pro/Max + ChatGPT Plus)"
echo "     • Zero cost for existing subscribers"
echo "     • Automatic failover between Anthropic and OpenAI"
echo "     • Requires web browser for OAuth login"
echo ""
echo "  2) API Key Setup with Failover"
echo "     • Commercial API access"
echo "     • You provide: Anthropic + OpenAI API keys"
echo "     • Automatic failover on provider outages"
echo ""
echo "  3) Cost-Optimized Multi-Provider"
echo "     • Budget-conscious routing"
echo "     • Cheap primary (Groq) + quality fallback (Anthropic)"
echo "     • Best for high-volume workloads"
echo ""
echo "  4) Custom Configuration"
echo "     • Use template file as starting point"
echo "     • Full control over all settings"
echo ""
echo "  5) Minimal (Manual Setup via Web UI)"
echo "     • Start with basic config"
echo "     • Configure everything through http://127.0.0.1:13456"
echo ""

read -rp "Choice [1-5]: " setup_choice

case $setup_choice in
    1)
        print_status "Setting up Free OAuth configuration..."
        cat > "$CONFIG_FILE" <<'EOF'
[server]
host = "127.0.0.1"
port = 13456

[[providers]]
name = "anthropic"
type = "anthropic"
auth_type = "oauth"
priority = 1

[[providers]]
name = "openai"
type = "openai"
auth_type = "oauth"
priority = 2

[[models]]
name = "claude-sonnet-4-20250514"
providers = [
  { name = "anthropic", priority = 1 },
  { name = "openai", priority = 2 }
]

[router]
enabled = true

[router.default]
model = "claude-sonnet-4-20250514"
enabled = true
EOF
        print_success "OAuth configuration created"
        echo ""
        print_status "Next steps:"
        print_status "  1. Run: ccmctl start"
        print_status "  2. Open: http://127.0.0.1:13456"
        print_status "  3. Click 'Login' in Providers tab for Anthropic and OpenAI"
        print_status "  4. Complete OAuth flow in browser"
        ;;

    2)
        print_status "Setting up API Key configuration..."
        echo ""
        read -rp "Anthropic API Key (sk-ant-...): " anthropic_key
        read -rp "OpenAI API Key (sk-...): " openai_key

        cat > "$CONFIG_FILE" <<EOF
[server]
host = "127.0.0.1"
port = 13456

[[providers]]
name = "anthropic"
type = "anthropic"
auth_type = "api_key"
api_key = "$anthropic_key"
priority = 1

[[providers]]
name = "openai"
type = "openai"
auth_type = "api_key"
api_key = "$openai_key"
priority = 2

[[models]]
name = "claude-sonnet-4-20250514"
providers = [
  { name = "anthropic", priority = 1 },
  { name = "openai", priority = 2 }
]

[[models]]
name = "gpt-4o"
providers = [{ name = "openai", priority = 1 }]

[router]
enabled = true

[router.default]
model = "claude-sonnet-4-20250514"
enabled = true
EOF
        print_success "API Key configuration created"
        echo ""
        print_status "Next steps:"
        print_status "  1. Run: ccmctl start"
        print_status "  2. Test failover: http://127.0.0.1:13456 (Test tab)"
        ;;

    3)
        print_status "Setting up Cost-Optimized configuration..."
        echo ""
        read -rp "Groq API Key (gsk_...): " groq_key
        read -rp "Anthropic API Key (sk-ant-...): " anthropic_key

        cat > "$CONFIG_FILE" <<EOF
[server]
host = "127.0.0.1"
port = 13456

[[providers]]
name = "groq"
type = "groq"
auth_type = "api_key"
api_key = "$groq_key"
priority = 1

[[providers]]
name = "anthropic"
type = "anthropic"
auth_type = "api_key"
api_key = "$anthropic_key"
priority = 2

[[models]]
name = "claude-sonnet-4-20250514"
providers = [
  { name = "groq", priority = 1 },
  { name = "anthropic", priority = 2 }
]

[router]
enabled = true

[router.default]
model = "claude-sonnet-4-20250514"
enabled = true
EOF
        print_success "Cost-optimized configuration created"
        echo ""
        print_status "Configuration uses:"
        print_status "  • Groq (ultra-fast, cheaper) as primary"
        print_status "  • Anthropic (Claude quality) as fallback"
        ;;

    4)
        print_status "Copying template for custom configuration..."
        TEMPLATE_FILE="/docker/lib/extensions/claude-code-mux/config-template.toml"
        cp "$TEMPLATE_FILE" "$CONFIG_FILE"
        print_success "Template copied to: $CONFIG_FILE"
        echo ""
        print_status "Opening in editor for customization..."
        sleep 1
        ${EDITOR:-nano} "$CONFIG_FILE"
        print_success "Custom configuration saved"
        ;;

    5)
        print_status "Creating minimal configuration..."
        cat > "$CONFIG_FILE" <<'EOF'
[server]
host = "127.0.0.1"
port = 13456

[router]
enabled = false
EOF
        print_success "Minimal configuration created"
        echo ""
        print_status "All configuration will be done via Web UI"
        ;;

    *)
        print_error "Invalid choice"
        exit 1
        ;;
esac

echo ""
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_success "Configuration created!"
print_status "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Configuration file:  $CONFIG_FILE"
echo "Start CCM server:    ccmctl start"
echo "Access Web UI:       http://127.0.0.1:13456"
echo "Edit config:         \$EDITOR $CONFIG_FILE"
echo ""
print_status "Claude Code is configured to route through CCM:"
echo "  ANTHROPIC_BASE_URL=http://127.0.0.1:13456"
echo ""

# Offer to start immediately
read -rp "Start CCM server now? [Y/n]: " start_now
if [[ ! $start_now =~ ^[Nn]$ ]]; then
    exec ccmctl start
fi
