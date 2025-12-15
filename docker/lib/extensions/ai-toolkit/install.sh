#!/bin/bash
set -euo pipefail

# ai-toolkit install script - Simplified for YAML-driven architecture
# Installs AI CLI tools using hybrid approach: Native + mise + platform-specific

# Source common utilities
source "$(dirname "$(dirname "$(dirname "${BASH_SOURCE[0]}")")")/common.sh"

print_status "Installing AI CLI tools using hybrid approach..."

# ===========================================================================
# GO ENVIRONMENT SETUP
# ===========================================================================
# Ensure GOPATH and GOMODCACHE use absolute paths (not ~)
# This MUST be set unconditionally before any Go-based tool installations,
# including mise install which may install Go packages
export GOPATH="${HOME}/go"
export GOMODCACHE="${GOPATH}/pkg/mod"
export GOBIN="${GOPATH}/bin"
export PATH="${GOBIN}:${PATH}"
mkdir -p "${GOPATH}" "${GOMODCACHE}" "${GOBIN}"

# ===========================================================================
# NATIVE INSTALLATIONS
# ===========================================================================

# Check if running in CI mode
if [[ "${CI:-}" == "true" ]] || [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
  print_status "CI mode detected - skipping native tools (Fabric)"
else
  # Note: Ollama is now a separate extension for better isolation
  # Install it with: extension-manager install ollama

  # Fabric - Go install (standard Go module installation)
  print_status "Installing Fabric (go install)..."
  if command_exists fabric; then
    print_warning "Fabric already installed"
  elif ! command_exists go; then
    print_warning "Go not found - skipping Fabric (requires Go)"
    print_status "Install golang extension for Fabric"
  else
    mkdir -p "$HOME/.local/bin"
    # Use go install for proper Go module installation
    # The fabric CLI binary is in cmd/fabric subdirectory
    if timeout 300 go install github.com/danielmiessler/fabric/cmd/fabric@latest 2>&1; then
      print_success "Fabric installed via go install"
      print_status "Initialize with: fabric --setup"
    else
      print_warning "Failed to install Fabric"
    fi
  fi
fi

# ===========================================================================
# MISE-MANAGED INSTALLATIONS (Preferred)
# ===========================================================================

if command_exists mise; then
  print_status "Installing AI tools via mise..."

  MISE_CONF_DIR="$HOME/.config/mise/conf.d"
  mkdir -p "$MISE_CONF_DIR"
  TOML_FILE="$MISE_CONF_DIR/ai-toolkit.toml"

  # Build toml content dynamically
  toml_content="# AI Toolkit - mise configuration\n\n[tools]\n"

  # Add npm-based tools if Node.js available
  if command_exists npm || command_exists node; then
    toml_content+="# npm-based tools\n"
    toml_content+='"npm:@openai/codex" = "latest"\n'
    toml_content+='"npm:@google/gemini-cli" = "latest"\n'
    toml_content+='"npm:@vibe-kit/grok-cli" = "latest"\n'
    toml_content+="\n"
  fi

  # Add Go-based tools if Go available
  if command_exists go; then
    toml_content+="# Go-based tools\n"
    toml_content+='"go:github.com/kadirpekel/hector/cmd/hector" = "latest"\n'
    toml_content+="\n"
  fi

  echo -e "$toml_content" > "$TOML_FILE"
  print_success "Created mise config: $TOML_FILE"

  if mise install 2>&1 | tee /tmp/mise-install.log; then
    print_success "mise install completed"
  else
    print_warning "mise install encountered issues"
  fi

else
  # =========================================================================
  # FALLBACK INSTALLATIONS (Direct npm/go)
  # =========================================================================
  print_warning "mise not available - using fallback installations"

  # npm global installs
  if command_exists npm; then
    print_status "Installing npm-based tools (fallback)..."

    if ! command_exists codex; then
      npm install -g @openai/codex 2>&1 && print_success "Codex CLI installed"
    fi

    if ! command_exists gemini; then
      npm install -g @google/gemini-cli 2>&1 && print_success "Gemini CLI installed"
    fi
  else
    print_warning "npm not found - skipping npm-based tools"
  fi

  # go install (GOPATH already configured at top of script)
  if command_exists go; then
    print_status "Installing Go-based tools (fallback)..."

    if ! command_exists hector; then
      timeout 300 go install github.com/kadirpekel/hector/cmd/hector@latest 2>&1 && print_success "Hector installed"
    fi
  else
    print_warning "Go not found - skipping Go-based tools"
  fi
fi

# ===========================================================================
# PLATFORM CLIs
# ===========================================================================

# Factory AI Droid
print_status "Installing Factory AI CLI (Droid)..."
# Ensure ~/.local/bin is in PATH for post-install check
export PATH="${HOME}/.local/bin:${PATH}"
if command_exists droid; then
  print_warning "Factory AI CLI already installed"
else
  case "$(uname)" in
    Darwin|Linux)
      if curl -fsSL https://app.factory.ai/cli | bash 2>&1; then
        if command_exists droid; then
          print_success "Factory AI CLI installed"
          print_status "Authenticate with: droid"
        else
          print_warning "Factory AI CLI installed but command not found"
        fi
      else
        print_warning "Failed to install Factory AI CLI"
      fi
      ;;
    *)
      print_warning "Factory AI CLI not supported on $(uname)"
      ;;
  esac
fi

# GitHub Copilot CLI
print_status "Installing GitHub Copilot CLI..."
if command_exists gh; then
  if gh extension list 2>/dev/null | grep -q "github/gh-copilot"; then
    print_warning "GitHub Copilot CLI already installed"
  else
    gh extension install github/gh-copilot 2>&1 && print_success "GitHub Copilot CLI installed"
  fi
else
  print_warning "GitHub CLI (gh) not found - skipping Copilot CLI"
fi

# AWS Q Developer
if command_exists aws; then
  print_status "AWS CLI available - Amazon Q Developer accessible via 'aws q'"
else
  print_warning "AWS CLI not found - Amazon Q unavailable"
fi

# Grok CLI - now installed via mise npm: backend (added to TOML above)
# This ensures shims are created and grok is available in PATH during validation
print_status "Grok CLI will be installed via mise npm: backend"

# Create workspace
WORKSPACE="${WORKSPACE:-${HOME}/workspace}"
mkdir -p "${WORKSPACE}/extensions/ai-tools"/{ollama-models,fabric-patterns,projects}

# Refresh mise shims for all installed tools
if command_exists mise; then
    mise reshim 2>/dev/null || true
fi
hash -r 2>/dev/null || true

print_success "AI toolkit installation complete"
