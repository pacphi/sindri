#!/bin/bash
# git.sh - Git configuration and utilities
# This library provides functions for Git setup and configuration

# Note: set -euo pipefail is set by the calling script
export GIT_SH_LOADED="true"

# Source common utilities
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
[[ "${COMMON_SH_LOADED:-}" != "true" ]] && source "${SCRIPT_DIR}/common.sh"

# Function to setup Git hooks
setup_git_hooks() {
    local project_dir="${1:-$(pwd)}"

    if [[ ! -d "$project_dir/.git" ]]; then
        print_warning "Not a Git repository: $project_dir"
        return 1
    fi

    print_status "Setting up Git hooks in $project_dir..."

    local hooks_dir="$project_dir/.git/hooks"
    mkdir -p "$hooks_dir"

    # Pre-commit hook for code quality
    cat > "$hooks_dir/pre-commit" << 'EOF'
#!/bin/bash
# Pre-commit hook for code quality checks

# Source common utilities if available
if [ -f "/docker/lib/common.sh" ]; then
    source "/docker/lib/common.sh"
else
    print_status() { echo "[INFO] $1"; }
    print_error() { echo "[ERROR] $1"; }
fi

# Check for debugging code
if git diff --cached --name-only | xargs grep -E "console\.(log|debug|info|warn|error)" 2>/dev/null; then
    print_error "Debugging code detected. Please remove console statements."
    exit 1
fi

# Run prettier if available
if command -v prettier >/dev/null 2>&1; then
    files=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(js|jsx|ts|tsx|json|css|scss|md)$')
    if [ -n "$files" ]; then
        echo "$files" | xargs prettier --write
        echo "$files" | xargs git add
    fi
fi

# Run eslint if available
if command -v eslint >/dev/null 2>&1; then
    files=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(js|jsx|ts|tsx)$')
    if [ -n "$files" ]; then
        echo "$files" | xargs eslint --fix
        echo "$files" | xargs git add
    fi
fi

exit 0
EOF
    chmod +x "$hooks_dir/pre-commit"

    # Commit message hook
    cat > "$hooks_dir/commit-msg" << 'EOF'
#!/bin/bash
# Commit message validation hook

commit_regex='^(feat|fix|docs|style|refactor|test|chore|perf|ci|build|revert)(\(.+\))?: .{1,50}'

if ! grep -qE "$commit_regex" "$1"; then
    echo "Invalid commit message format!"
    echo "Format: <type>(<scope>): <subject>"
    echo "Example: feat(auth): add login functionality"
    echo ""
    echo "Types: feat, fix, docs, style, refactor, test, chore, perf, ci, build, revert"
    exit 1
fi
EOF
    chmod +x "$hooks_dir/commit-msg"

    print_success "Git hooks configured"
}

# Function to create gitignore file
create_gitignore() {
    local project_type="${1:-general}"
    local gitignore_file="${2:-.gitignore}"

    print_status "Creating .gitignore for $project_type project..."

    case "$project_type" in
        node|nodejs)
            cat > "$gitignore_file" << 'EOF'
# Dependencies
node_modules/
jspm_packages/

# Build outputs
dist/
build/
*.min.js
*.min.css

# Logs
logs/
*.log
npm-debug.log*
yarn-debug.log*
yarn-error.log*

# Environment
.env
.env.local
.env.*.local

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Testing
coverage/
.nyc_output/

# Temporary
tmp/
temp/
EOF
            ;;
        python)
            cat > "$gitignore_file" << 'EOF'
# Byte-compiled / optimized
__pycache__/
*.py[cod]
*$py.class
*.so

# Virtual Environment
venv/
env/
ENV/
.venv

# Distribution / packaging
.Python
build/
develop-eggs/
dist/
downloads/
eggs/
.eggs/
lib/
lib64/
parts/
sdist/
var/
wheels/
*.egg-info/
.installed.cfg
*.egg

# Testing
.tox/
.coverage
.coverage.*
.cache
.pytest_cache/
htmlcov/

# Environment
.env
*.env

# IDE
.vscode/
.idea/
*.swp
*.swo

# Jupyter
.ipynb_checkpoints
EOF
            ;;
        go|golang)
            cat > "$gitignore_file" << 'EOF'
# Binaries
*.exe
*.exe~
*.dll
*.so
*.dylib

# Test binary
*.test

# Output
*.out

# Dependency directories
vendor/

# Go workspace
go.work

# Environment
.env
*.env

# IDE
.vscode/
.idea/
*.swp
EOF
            ;;
        rust)
            cat > "$gitignore_file" << 'EOF'
# Rust build
/target/
**/*.rs.bk
*.pdb

# Cargo
Cargo.lock

# IDE
.vscode/
.idea/
*.swp

# Environment
.env
EOF
            ;;
        *)
            cat > "$gitignore_file" << 'EOF'
# Build outputs
build/
dist/
out/
target/

# Dependencies
node_modules/
vendor/

# Environment
.env
.env.*
*.env

# IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
Thumbs.db

# Logs
*.log
logs/

# Temporary
tmp/
temp/
.cache/
EOF
            ;;
    esac

    print_success ".gitignore created for $project_type project"
}

# Function to initialize Git repository
init_git_repo() {
    local project_dir="${1:-.}"
    local project_type="${2:-general}"

    cd "$project_dir" || return 1

    if [[ -d ".git" ]]; then
        print_warning "Git repository already initialized"
        return 0
    fi

    print_status "Initializing Git repository..."

    # Initialize repository
    git init

    # Create gitignore
    create_gitignore "$project_type"

    # Create initial commit
    git add .gitignore
    git commit -m "chore: initial commit"

    # Setup hooks
    setup_git_hooks "$project_dir"

    print_success "Git repository initialized"
}

# Function to setup fork remotes
setup_fork_remotes() {
    local upstream_url=""

    # The upstream remote should already be set by gh repo fork
    # But we'll verify and configure if needed
    if ! git remote get-url upstream >/dev/null 2>&1; then
        print_warning "Upstream remote not configured. Fork may not have been set up correctly."
    else
        upstream_url=$(git remote get-url upstream)
        print_success "Fork configured with upstream: $upstream_url"
    fi
}

# Function to setup fork-specific Git aliases
setup_fork_aliases() {
    print_status "Setting up fork management aliases..."

    # Sync with upstream
    git config alias.sync-upstream '!git fetch upstream && git checkout main && git merge upstream/main'

    # Push to fork's origin
    git config alias.push-fork 'push origin HEAD'

    # Update all branches from upstream
    git config alias.update-from-upstream '!git fetch upstream && git rebase upstream/main'

    # Create PR-ready branch
    git config alias.pr-branch '!f() { git checkout -b "$1" upstream/main; }; f'

    # Show fork status
    git config alias.fork-status '!echo "=== Remotes ===" && git remote -v && echo && echo "=== Branch Tracking ===" && git branch -vv'

    print_success "Fork aliases configured:"
    echo "   • git sync-upstream    - Fetch and merge upstream changes"
    echo "   • git push-fork        - Push current branch to your fork"
    echo "   • git update-from-upstream - Rebase current branch on upstream/main"
    echo "   • git pr-branch <name> - Create new branch from upstream/main"
    echo "   • git fork-status      - Show fork remotes and branch tracking"
}

# Function to apply per-project Git config overrides
apply_git_config_overrides() {
    local git_name=""
    local git_email=""

    while [[ $# -gt 0 ]]; do
        case $1 in
            --name)
                git_name="$2"
                shift 2
                ;;
            --email)
                git_email="$2"
                shift 2
                ;;
            *)
                print_error "Unknown option: $1"
                return 1
                ;;
        esac
    done

    if [[ -z "$git_name" ]] && [[ -z "$git_email" ]]; then
        print_debug "No Git config overrides to apply"
        return 0
    fi

    print_status "Configuring Git for this project..."

    if [[ -n "$git_name" ]]; then
        if git config user.name "$git_name"; then
            print_success "Git user name set to: $git_name"
        else
            print_error "Failed to set Git user name"
            return 1
        fi
    fi

    if [[ -n "$git_email" ]]; then
        if git config user.email "$git_email"; then
            print_success "Git user email set to: $git_email"
        else
            print_error "Failed to set Git user email"
            return 1
        fi
    fi

    return 0
}

# Export functions
export -f setup_git_hooks create_gitignore init_git_repo
export -f setup_fork_remotes setup_fork_aliases apply_git_config_overrides
