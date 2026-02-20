#!/bin/bash
# Install starship (shell prompt) system-wide
# Binary goes to /usr/local/bin, initialized via profile.d for all shells

set -euo pipefail

# Source common utilities if available
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$SCRIPT_DIR/../lib/common.sh" ]]; then
    source "$SCRIPT_DIR/../lib/common.sh"
else
    print_status() { echo "==> $1"; }
    print_success() { echo "[OK] $1"; }
    print_error() { echo "[ERROR] $1" >&2; }
fi

STARSHIP_VERSION="${STARSHIP_VERSION:-latest}"
INSTALL_DIR="/usr/local/bin"
PROFILE_SCRIPT="/etc/profile.d/02-starship.sh"
SKEL_CONFIG_DIR="/etc/skel/.config/starship"

print_status "Installing starship to $INSTALL_DIR/starship..."

# Fetch latest version if not specified
if [[ "$STARSHIP_VERSION" == "latest" ]]; then
    CURL_ARGS=(-sL)
    if [[ -n "${GITHUB_TOKEN:-}" ]]; then
        CURL_ARGS+=(-H "Authorization: token $GITHUB_TOKEN")
        print_status "Using GITHUB_TOKEN for GitHub API request..."
    fi
    STARSHIP_VERSION=$(curl "${CURL_ARGS[@]}" https://api.github.com/repos/starship/starship/releases/latest | grep '"tag_name":' | sed -E 's/.*"v([^"]+)".*/\1/')
    print_status "Latest version: v$STARSHIP_VERSION"
fi

# Download and install binary (use musl static binary for no libc dependencies)
DOWNLOAD_URL="https://github.com/starship/starship/releases/download/v${STARSHIP_VERSION}/starship-x86_64-unknown-linux-musl.tar.gz"
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

print_status "Downloading starship v${STARSHIP_VERSION}..."
curl -fsSL "$DOWNLOAD_URL" -o "$TEMP_DIR/starship.tar.gz"

# Extract and install
tar -xzf "$TEMP_DIR/starship.tar.gz" -C "$TEMP_DIR"
cp "$TEMP_DIR/starship" "$INSTALL_DIR/starship"
chmod +x "$INSTALL_DIR/starship"

# Verify installation
if [[ ! -x "$INSTALL_DIR/starship" ]]; then
    print_error "starship installation failed"
    exit 1
fi

INSTALLED_VERSION=$("$INSTALL_DIR/starship" --version | grep -oP 'starship \K[0-9.]+')
print_success "starship installed: v${INSTALLED_VERSION}"

# Create profile.d script for shell initialization
# This runs for all login shells (SSH, su -, etc.)
# Note: PS1 is not yet set when profile.d runs, so we check $- for interactive shells
print_status "Creating starship profile script..."
cat > "$PROFILE_SCRIPT" << 'EOF'
# starship - cross-shell prompt
# Automatically initializes for bash shells

if command -v starship >/dev/null 2>&1; then
    # Only initialize for interactive bash shells
    # Check $- instead of $PS1 because PS1 isn't set yet when profile.d runs
    if [ -n "$BASH_VERSION" ]; then
        case $- in
            *i*)
                eval "$(starship init bash)"
                ;;
        esac
    fi
fi
EOF
chmod 644 "$PROFILE_SCRIPT"

# Create skel config directory for default configuration
print_status "Setting up default starship configuration..."
mkdir -p "$SKEL_CONFIG_DIR"

# Create a default starship.toml configuration
cat > "$SKEL_CONFIG_DIR/starship.toml" << 'EOF'
# Starship configuration for Sindri development environment
# Users can customize this in ~/.config/starship/starship.toml
# Documentation: https://starship.rs/config/

# Get editor completions based on the config schema
"$schema" = 'https://starship.rs/config-schema.json'

# Optimized format for cloud development environments
format = """
[┌─](bold green)$username$hostname$directory$git_branch$git_status
[└─>](bold green) """

[character]
success_symbol = "[➜](bold green)"
error_symbol = "[✗](bold red)"

[directory]
truncation_length = 3
truncate_to_repo = true
style = "bold cyan"

[git_branch]
symbol = " "
style = "bold purple"

[git_status]
style = "bold yellow"
ahead = "⇡${count}"
diverged = "⇕⇡${ahead_count}⇣${behind_count}"
behind = "⇣${count}"

# Show mise-managed tool versions in prompt
[mise]
symbol = "mise "
style = "bold blue"
format = '[$symbol($version )]($style)'

# Show command duration for slow commands
[cmd_duration]
min_time = 500
format = "took [$duration](bold yellow)"
EOF

chmod 644 "$SKEL_CONFIG_DIR/starship.toml"

# Create bashrc snippet for appending to user's .bashrc
# This follows the same pattern as JVM extension (mode: append)
BASHRC_SNIPPET="/etc/skel/.bashrc-starship"
print_status "Creating starship bashrc snippet..."
cat > "$BASHRC_SNIPPET" << 'EOF'

# Starship prompt initialization (added by Sindri)
if command -v starship >/dev/null 2>&1; then
    eval "$(starship init bash)"
fi
EOF
chmod 644 "$BASHRC_SNIPPET"

print_success "starship installation complete"
print_status "Users will have starship initialized automatically on login"
print_status "Default config available at ~/.config/starship/starship.toml"
