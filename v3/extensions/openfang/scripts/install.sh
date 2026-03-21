#!/bin/bash
set -euo pipefail

# OpenFang install script - Open-source agent OS for autonomous AI agents
# Installs OpenFang CLI v0.4.6 via official installer or cargo fallback

OPENFANG_VERSION="0.4.6"

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

# Detect OpenSSL major version to choose install strategy.
# The official prebuilt binary is linked against OpenSSL 1.x.
# Systems with OpenSSL 3.x (e.g., Ubuntu 24.04+) must build from source
# via cargo so the binary links against the system's libssl.
needs_cargo_build=false
if command_exists openssl; then
    openssl_major=$(openssl version 2>/dev/null | grep -oP '(?<=OpenSSL )\d+' | head -1 || echo "")
    if [[ "$openssl_major" -ge 3 ]] 2>/dev/null; then
        print_status "OpenSSL ${openssl_major}.x detected — prebuilt binary requires OpenSSL 1.x"
        print_status "Will build from source via cargo to link against system OpenSSL"
        needs_cargo_build=true
    fi
fi

install_success=false

if [[ "$needs_cargo_build" == "true" ]]; then
    # OpenSSL 3.x: build from source first, prebuilt binary as last resort
    if command_exists cargo; then
        print_status "Building OpenFang from source via cargo (OpenSSL 3.x compatibility)..."
        if cargo install openfang-cli --version "$OPENFANG_VERSION" --locked 2>&1; then
            # cargo installs to ~/.cargo/bin; copy to ~/.openfang/bin for consistency
            if [[ -f "$HOME/.cargo/bin/openfang" ]]; then
                cp "$HOME/.cargo/bin/openfang" "$HOME/.openfang/bin/openfang"
                print_status "Copied binary to ~/.openfang/bin/"
            fi
            install_success=true
            print_status "Cargo build completed"
        else
            print_warning "Cargo build failed, trying official installer as fallback..."
            if curl -fsSL https://openfang.sh/install | VERSION="${OPENFANG_VERSION}" sh 2>&1; then
                install_success=true
                print_status "Official installer completed"
            fi
        fi
    else
        print_warning "Cargo not available — trying official installer (may fail with OpenSSL 3.x)..."
        if curl -fsSL https://openfang.sh/install | VERSION="${OPENFANG_VERSION}" sh 2>&1; then
            install_success=true
            print_status "Official installer completed"
        fi
    fi
else
    # OpenSSL 1.x or unknown: prebuilt binary first, cargo fallback
    print_status "Attempting install via official installer..."
    if curl -fsSL https://openfang.sh/install | VERSION="${OPENFANG_VERSION}" sh 2>&1; then
        install_success=true
        print_status "Official installer completed"
    fi

    if [[ "$install_success" != "true" ]]; then
        print_warning "Official installer failed, trying cargo fallback..."
        if command_exists cargo; then
            if cargo install openfang-cli --version "$OPENFANG_VERSION" --locked 2>&1; then
                if [[ -f "$HOME/.cargo/bin/openfang" ]]; then
                    cp "$HOME/.cargo/bin/openfang" "$HOME/.openfang/bin/openfang"
                fi
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
