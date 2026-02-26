#!/bin/bash
set -euo pipefail

# OpenFang install script - Open-source agent OS for autonomous AI agents
# Installs OpenFang CLI v0.1.1 via official installer or cargo fallback

OPENFANG_VERSION="0.1.1"

print_status "Installing OpenFang agent OS v${OPENFANG_VERSION}..."

# Check if already installed at correct version
if command_exists openfang; then
    current_version=$(openfang --version 2>/dev/null || echo "")
    if echo "$current_version" | grep -q "$OPENFANG_VERSION"; then
        print_warning "OpenFang v${OPENFANG_VERSION} is already installed: $current_version"
        exit 0
    else
        print_status "OpenFang found but version mismatch (have: $current_version, want: $OPENFANG_VERSION)"
        print_status "Upgrading..."
    fi
fi

# Ollama detection - informational only
if command_exists ollama; then
    print_status "Ollama detected on this system - OpenFang can use local models"
    print_status "Example config.toml for Ollama:"
    print_status "  [llm]"
    print_status "  provider = \"ollama\""
    print_status "  model = \"llama3.2\""
    print_status "  endpoint = \"http://localhost:11434\""
else
    print_status "Ollama not found - OpenFang will use cloud LLM providers"
    print_status "Install the 'ollama' extension for local model support"
fi

# API key awareness
if [[ -n "${ANTHROPIC_API_KEY:-}" ]]; then
    print_status "ANTHROPIC_API_KEY detected - Anthropic provider available"
elif [[ -n "${OPENAI_API_KEY:-}" ]]; then
    print_status "OPENAI_API_KEY detected - OpenAI provider available"
elif [[ -n "${GROQ_API_KEY:-}" ]]; then
    print_status "GROQ_API_KEY detected - Groq provider available"
else
    if ! command_exists ollama; then
        print_warning "No LLM API key found and Ollama not installed"
        print_status "Set one of: ANTHROPIC_API_KEY, OPENAI_API_KEY, or GROQ_API_KEY"
        print_status "Or install the 'ollama' extension for local models"
    fi
fi

# Ensure install directory exists
mkdir -p "$HOME/.openfang/bin"

# Method 1: Official installer
install_success=false
print_status "Attempting install via official installer..."
if curl -fsSL https://openfang.sh/install | VERSION="${OPENFANG_VERSION}" sh 2>&1; then
    install_success=true
    print_status "Official installer completed"
fi

# Method 2: Cargo fallback
if [[ "$install_success" != "true" ]]; then
    print_warning "Official installer failed, trying cargo fallback..."
    if command_exists cargo; then
        if cargo install openfang-cli --version "$OPENFANG_VERSION" --locked 2>&1; then
            install_success=true
            print_status "Cargo install completed"
        else
            print_error "Cargo install failed"
        fi
    else
        print_error "Cargo not found - cannot use fallback install method"
        print_status "Install Rust first: https://rustup.rs"
    fi
fi

if [[ "$install_success" != "true" ]]; then
    print_error "Failed to install OpenFang v${OPENFANG_VERSION}"
    print_status "Try manual installation: https://openfang.sh/docs/install"
    exit 1
fi

# PATH setup - add ~/.openfang/bin to shell configs (idempotent)
OPENFANG_PATH_LINE='export PATH="$HOME/.openfang/bin:$PATH"'
for rcfile in "$HOME/.bashrc" "$HOME/.zshrc"; do
    if [[ -f "$rcfile" ]]; then
        if ! grep -q '.openfang/bin' "$rcfile" 2>/dev/null; then
            echo "" >> "$rcfile"
            echo "# OpenFang" >> "$rcfile"
            echo "$OPENFANG_PATH_LINE" >> "$rcfile"
            print_status "Added OpenFang to PATH in $(basename "$rcfile")"
        fi
    fi
done

# Ensure PATH is available in current session
export PATH="$HOME/.openfang/bin:$PATH"
hash -r 2>/dev/null || true

# Verification
if command_exists openfang; then
    version=$(openfang --version 2>/dev/null || echo "installed")
    print_success "OpenFang installed successfully: $version"
    print_status ""
    print_status "Next steps:"
    print_status "  1. Initialize configuration: openfang init"
    print_status "  2. Check system health:      openfang doctor"
    print_status "  3. Spawn an agent:           openfang agent spawn --name my-agent --platform slack"
    print_status ""
    print_status "Documentation: https://openfang.sh/docs/"
else
    print_error "OpenFang installation completed but binary not found in PATH"
    print_status "Expected location: ~/.openfang/bin/openfang"
    exit 1
fi
