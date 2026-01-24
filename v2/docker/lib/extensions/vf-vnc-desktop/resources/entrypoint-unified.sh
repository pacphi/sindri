#!/bin/bash
# Unified Container Entrypoint - Enhanced Edition
# Handles multi-user setup, credential distribution, service initialization, and CLAUDE.md enhancement

set -e

echo "========================================"
echo "  TURBO FLOW UNIFIED CONTAINER"
echo "========================================"
echo ""

# ============================================================================
# Phase 1: Directory Setup & Docker Socket Configuration
# ============================================================================

echo "[1/10] Setting up directories and Docker socket..."

# Ensure all required directories exist
mkdir -p /home/devuser/{workspace,models,agents,.claude/skills,.config,.cache,logs,.local/share}
mkdir -p /home/gemini-user/{workspace,.config,.cache,.gemini-flow}
mkdir -p /home/openai-user/{workspace,.config,.cache}
mkdir -p /home/zai-user/{workspace,.config,.cache}
mkdir -p /var/log /var/log/supervisor /run/dbus /run/user/1000 /tmp/.X11-unix /tmp/.ICE-unix
chmod 1777 /tmp/.X11-unix /tmp/.ICE-unix
chmod 700 /run/user/1000
chown devuser:devuser /run/user/1000

# Set permissions (skip read-only mounts like .ssh and .claude)
# Only chown known writable directories, skip .ssh and .claude which may be read-only mounts
set +e
chown -R devuser:devuser /home/devuser/workspace 2>/dev/null
chown -R devuser:devuser /home/devuser/models 2>/dev/null
chown -R devuser:devuser /home/devuser/agents 2>/dev/null
chown -R devuser:devuser /home/devuser/logs 2>/dev/null
chown -R devuser:devuser /home/devuser/.config 2>/dev/null
chown -R devuser:devuser /home/devuser/.cache 2>/dev/null
chown -R devuser:devuser /home/devuser/.local 2>/dev/null
chown -R gemini-user:gemini-user /home/gemini-user 2>/dev/null
chown -R openai-user:openai-user /home/openai-user 2>/dev/null
chown -R zai-user:zai-user /home/zai-user 2>/dev/null
set -e

# Configure Docker socket permissions for docker-manager skill
# Security fix H-4: Use group membership instead of chmod 666
if [ -S /var/run/docker.sock ]; then
    # Check if docker group exists, create if not
    if ! getent group docker >/dev/null 2>&1; then
        groupadd docker 2>/dev/null || echo "⚠️  Could not create docker group"
    fi

    # Add devuser to docker group
    if getent group docker >/dev/null 2>&1; then
        usermod -aG docker devuser 2>/dev/null || echo "⚠️  Could not add devuser to docker group"
        # Set secure permissions: owner + group only (660)
        chmod 660 /var/run/docker.sock
        # Ensure socket is owned by root:docker
        chown root:docker /var/run/docker.sock 2>/dev/null || true
        echo "✓ Docker socket permissions configured securely (group-based access)"
    else
        echo "⚠️  Docker group unavailable, skipping socket configuration"
    fi
else
    echo "ℹ️  Docker socket not found (this is normal if not mounting host socket)"
fi

echo "✓ Directories created and permissions set"

# ============================================================================
# Phase 2: Credential Distribution from Environment
# ============================================================================

echo "[2/10] Distributing credentials to users..."

# devuser - Claude Code configuration
if [ -n "$ANTHROPIC_API_KEY" ]; then
    sudo -u devuser bash -c "mkdir -p ~/.config/claude && cat > ~/.config/claude/config.json" <<EOF
{
  "apiKey": "$ANTHROPIC_API_KEY",
  "defaultModel": "claude-sonnet-4"
}
EOF
    echo "✓ Claude API key configured for devuser"
fi

# devuser - Z.AI API key for web-summary skill
if [ -n "$ZAI_API_KEY" ]; then
    sudo -u devuser bash -c "mkdir -p ~/.config/zai && cat > ~/.config/zai/api.json" <<EOF
{
  "apiKey": "$ZAI_API_KEY"
}
EOF
    echo "✓ Z.AI API key configured for devuser (web-summary skill)"
fi

# gemini-user - Google Gemini configuration
if [ -n "$GOOGLE_GEMINI_API_KEY" ]; then
    sudo -u gemini-user bash -c "mkdir -p ~/.config/gemini && cat > ~/.config/gemini/config.json" <<EOF
{
  "apiKey": "$GOOGLE_GEMINI_API_KEY",
  "defaultModel": "gemini-2.0-flash"
}
EOF
    export GOOGLE_API_KEY="$GOOGLE_GEMINI_API_KEY"
    echo "✓ Gemini API key configured for gemini-user"
fi

# openai-user - OpenAI configuration
if [ -n "$OPENAI_API_KEY" ]; then
    sudo -u openai-user bash -c "mkdir -p ~/.config/openai && cat > ~/.config/openai/config.json" <<EOF
{
  "apiKey": "$OPENAI_API_KEY",
  "organization": "$OPENAI_ORG_ID"
}
EOF
    echo "✓ OpenAI API key configured for openai-user"
fi

# zai-user - Z.AI service configuration
if [ -n "$ANTHROPIC_API_KEY" ] && [ -n "$ANTHROPIC_BASE_URL" ]; then
    sudo -u zai-user bash -c "mkdir -p ~/.config/zai && cat > ~/.config/zai/config.json" <<EOF
{
  "apiKey": "$ANTHROPIC_API_KEY",
  "baseUrl": "$ANTHROPIC_BASE_URL",
  "port": 9600,
  "workerPoolSize": ${CLAUDE_WORKER_POOL_SIZE:-4},
  "maxQueueSize": ${CLAUDE_MAX_QUEUE_SIZE:-50}
}
EOF
    echo "✓ Z.AI configuration created for zai-user"
fi

# GitHub token for all users
if [ -n "$GITHUB_TOKEN" ]; then
    for user in devuser gemini-user openai-user; do
        sudo -u $user bash -c "mkdir -p ~/.config/gh && cat > ~/.config/gh/config.yml" <<EOF
git_protocol: https
editor: vim
prompt: enabled
pager:
oauth_token: $GITHUB_TOKEN
EOF
    done
    echo "✓ GitHub token configured for all users"
fi

# ============================================================================
# Phase 3: GPU Verification
# ============================================================================

echo "[3/10] Verifying GPU access..."

# Check nvidia-smi
if command -v nvidia-smi &> /dev/null; then
    if nvidia-smi &> /dev/null; then
        GPU_COUNT=$(nvidia-smi --query-gpu=name --format=csv,noheader 2>/dev/null | wc -l)
        echo "✓ NVIDIA driver accessible: $GPU_COUNT GPU(s) detected"
        nvidia-smi --query-gpu=index,name,memory.total --format=csv,noheader | \
            awk -F', ' '{printf "  GPU %s: %s (%s)\n", $1, $2, $3}'
    else
        echo "⚠️  nvidia-smi failed - GPU may not be accessible"
    fi
else
    echo "⚠️  nvidia-smi not found"
fi

# Test PyTorch CUDA detection
echo "Testing PyTorch CUDA support..."
PYTORCH_TEST=$(/opt/venv/bin/python3 -c "
import torch
print(f'PyTorch: {torch.__version__}')
print(f'CUDA available: {torch.cuda.is_available()}')
if torch.cuda.is_available():
    print(f'CUDA version: {torch.version.cuda}')
    print(f'GPU count: {torch.cuda.device_count()}')
    for i in range(torch.cuda.device_count()):
        print(f'  GPU {i}: {torch.cuda.get_device_name(i)}')
else:
    print('WARNING: PyTorch cannot access CUDA')
" 2>&1)

echo "$PYTORCH_TEST"

if echo "$PYTORCH_TEST" | grep -q "CUDA available: True"; then
    echo "✓ PyTorch GPU acceleration ready"
else
    echo "⚠️  PyTorch GPU acceleration not available - will fallback to CPU"
    echo "   This may significantly impact performance for AI workloads"
fi

# ============================================================================
# Phase 4: Verify Host Claude Configuration Mount
# ============================================================================

echo "[4/10] Verifying host Claude configuration..."

if [ -d "/home/devuser/.claude" ]; then
    # Ensure proper ownership (host mount may have different UID)
    # Only change ownership on writable files to avoid errors on read-only mounts
    set +e
    find /home/devuser/.claude -writable -exec chown devuser:devuser {} \; 2>/dev/null
    find /home/devuser/.claude -writable -exec chmod u+rw {} \; 2>/dev/null
    set -e
    echo "✓ Host Claude configuration mounted at /home/devuser/.claude (read-write)"
else
    # Create directory if mount failed
    mkdir -p /home/devuser/.claude/skills
    chown -R devuser:devuser /home/devuser/.claude
    echo "⚠️  Claude config directory created (host mount not detected)"
fi

# ============================================================================
# Phase 5: Initialize DBus
# ============================================================================

echo "[5/10] Initializing DBus..."

# Clean up any stale PID files from previous runs
rm -f /run/dbus/pid /var/run/dbus/pid

# DBus will be started by supervisord
echo "✓ DBus configured (supervisord will start)"

# ============================================================================
# Phase 6: Setup Claude Skills
# ============================================================================

echo "[6/10] Setting up Claude Code skills..."

# Make skill tools executable
find /home/devuser/.claude/skills -name "*.py" -exec chmod +x {} \;
find /home/devuser/.claude/skills -name "*.js" -exec chmod +x {} \;
find /home/devuser/.claude/skills -name "*.sh" -exec chmod +x {} \;

# Count skills
SKILL_COUNT=$(find /home/devuser/.claude/skills -name "SKILL.md" | wc -l)
echo "✓ $SKILL_COUNT Claude Code skills available"

# ============================================================================
# Phase 6: Setup Agents
# ============================================================================

echo "[7/10] Setting up Claude agents..."

AGENT_COUNT=$(find /home/devuser/agents -name "*.md" 2>/dev/null | wc -l)
if [ "$AGENT_COUNT" -gt 0 ]; then
    echo "✓ $AGENT_COUNT agent templates available"
else
    echo "ℹ️  No agent templates found"
fi

# ============================================================================
# Phase 6.5: Initialize Claude Flow & Clean NPX Cache
# ============================================================================

echo "[6.5/10] Initializing Claude Flow..."

# Clean any stale NPX caches from all users to prevent corruption
rm -rf /home/devuser/.npm/_npx/* 2>/dev/null || true
rm -rf /home/gemini-user/.npm/_npx/* 2>/dev/null || true
rm -rf /home/openai-user/.npm/_npx/* 2>/dev/null || true
rm -rf /home/zai-user/.npm/_npx/* 2>/dev/null || true
rm -rf /root/.npm/_npx/* 2>/dev/null || true

# Run claude-flow init --force as devuser
sudo -u devuser bash -c "cd /home/devuser && claude-flow init --force" 2>/dev/null || echo "ℹ️  Claude Flow init skipped (not critical)"

# Fix hooks to use global claude-flow instead of npx (prevents cache corruption)
if [ -f /home/devuser/.claude/settings.json ]; then
    sed -i 's|npx claude-flow@alpha|claude-flow|g' /home/devuser/.claude/settings.json
    chown devuser:devuser /home/devuser/.claude/settings.json
    echo "✓ Hooks updated to use global claude-flow"
fi

echo "✓ Claude Flow initialized and NPX cache cleared"

# ============================================================================
# Phase 6.7: Configure Cross-User Service Access & Dynamic MCP Discovery
# ============================================================================

echo "[6.7/10] Configuring cross-user service access..."

# Create shared directory for inter-service sockets
mkdir -p /var/run/agentic-services
chmod 755 /var/run/agentic-services

# Create symlinks for devuser to access isolated services
mkdir -p /home/devuser/.local/share/agentic-sockets
ln -sf /var/run/agentic-services/gemini-mcp.sock /home/devuser/.local/share/agentic-sockets/gemini-mcp.sock 2>/dev/null || true
ln -sf http://localhost:9600 /home/devuser/.local/share/agentic-sockets/zai-api.txt 2>/dev/null || true

# Add environment variable exports to devuser's zshrc for service discovery
sudo -u devuser bash -c 'cat >> ~/.zshrc' <<'ENV_EXPORTS'

# Cross-user service access (auto-configured)
export GEMINI_MCP_SOCKET="/var/run/agentic-services/gemini-mcp.sock"
export ZAI_API_URL="http://localhost:9600"
export ZAI_CONTAINER_URL="http://localhost:9600"
export OPENAI_CODEX_SOCKET="/var/run/agentic-services/openai-codex.sock"

# Display and supervisorctl configuration
export DISPLAY=:1
alias supervisorctl="/opt/venv/bin/supervisorctl"
ENV_EXPORTS

# ============================================================================
# Dynamic MCP Settings Generation
# Discovers skills with mcp_server: true in SKILL.md frontmatter
# ============================================================================

echo "  Discovering MCP-enabled skills..."

mkdir -p /home/devuser/.config/claude

# Use generate-mcp-settings.sh if available, otherwise inline discovery
if [ -x /usr/local/bin/generate-mcp-settings.sh ]; then
    sudo -u devuser SKILLS_DIR=/home/devuser/.claude/skills \
        OUTPUT_FILE=/home/devuser/.config/claude/mcp_settings.json \
        /usr/local/bin/generate-mcp-settings.sh
else
    # Inline dynamic discovery (fallback)
    sudo -u devuser bash -c '
        SKILLS_DIR="/home/devuser/.claude/skills"
        OUTPUT_FILE="/home/devuser/.config/claude/mcp_settings.json"

        # Start JSON
        echo "{" > "$OUTPUT_FILE"
        echo "  \"mcpServers\": {" >> "$OUTPUT_FILE"

        first=true
        skill_count=0

        for skill_md in "$SKILLS_DIR"/*/SKILL.md; do
            [ -f "$skill_md" ] || continue
            skill_dir=$(dirname "$skill_md")
            skill_name=$(basename "$skill_dir")

            # Parse frontmatter for mcp_server: true
            mcp_server=$(awk "/^---$/,/^---$/" "$skill_md" | grep "^mcp_server:" | sed "s/mcp_server:[[:space:]]*//" | tr -d " ")
            [ "$mcp_server" != "true" ] && continue

            # Get entry_point and protocol
            entry_point=$(awk "/^---$/,/^---$/" "$skill_md" | grep "^entry_point:" | sed "s/entry_point:[[:space:]]*//" | tr -d " ")
            protocol=$(awk "/^---$/,/^---$/" "$skill_md" | grep "^protocol:" | sed "s/protocol:[[:space:]]*//" | tr -d " ")

            [ -z "$entry_point" ] && continue

            full_path="$skill_dir/$entry_point"
            [ ! -f "$full_path" ] && continue

            # Determine command
            case "$entry_point" in
                *.py) cmd="python3"; args="[\"-u\", \"$full_path\"]" ;;
                *.js) cmd="node"; args="[\"$full_path\"]" ;;
                *) continue ;;
            esac

            # Comma handling
            [ "$first" = "true" ] && first=false || echo "," >> "$OUTPUT_FILE"

            # Build skill entry with env vars based on skill name
            echo -n "    \"$skill_name\": {\"command\": \"$cmd\", \"args\": $args" >> "$OUTPUT_FILE"

            case "$skill_name" in
                web-summary)
                    echo -n ", \"env\": {\"ZAI_URL\": \"http://localhost:9600/chat\", \"ZAI_TIMEOUT\": \"60\"}" >> "$OUTPUT_FILE"
                    ;;
                qgis)
                    echo -n ", \"env\": {\"QGIS_HOST\": \"localhost\", \"QGIS_PORT\": \"9877\"}" >> "$OUTPUT_FILE"
                    ;;
                blender)
                    echo -n ", \"env\": {\"BLENDER_HOST\": \"localhost\", \"BLENDER_PORT\": \"9876\"}" >> "$OUTPUT_FILE"
                    ;;
                playwright)
                    echo -n ", \"env\": {\"DISPLAY\": \":1\", \"CHROMIUM_PATH\": \"/usr/bin/chromium\"}" >> "$OUTPUT_FILE"
                    ;;
                comfyui)
                    echo -n ", \"env\": {\"COMFYUI_URL\": \"http://localhost:8188\"}" >> "$OUTPUT_FILE"
                    ;;
                perplexity)
                    echo -n ", \"env\": {\"PERPLEXITY_API_KEY\": \"\$PERPLEXITY_API_KEY\"}" >> "$OUTPUT_FILE"
                    ;;
                deepseek-reasoning)
                    echo -n ", \"env\": {\"DEEPSEEK_API_KEY\": \"\$DEEPSEEK_API_KEY\"}" >> "$OUTPUT_FILE"
                    ;;
            esac

            echo -n "}" >> "$OUTPUT_FILE"
            skill_count=$((skill_count + 1))
        done

        # Close mcpServers and add VisionFlow config
        echo "" >> "$OUTPUT_FILE"
        cat >> "$OUTPUT_FILE" <<VISIONFLOW
  },
  "visionflow": {
    "tcp_bridge": {"host": "localhost", "port": 9500},
    "discovery": {"resource_pattern": "{skill}://capabilities", "refresh_interval": 300}
  },
  "metadata": {
    "generated_at": "$(date -Iseconds)",
    "skills_count": $skill_count,
    "generator": "entrypoint-unified.sh v2.0.0"
  }
}
VISIONFLOW

        echo "  Found $skill_count MCP-enabled skills"
    '
fi

# Count registered skills
MCP_SKILL_COUNT=$(grep -c '"command":' /home/devuser/.config/claude/mcp_settings.json 2>/dev/null || echo "0")

chown -R devuser:devuser /home/devuser/.local/share/agentic-sockets
chown -R devuser:devuser /home/devuser/.config/claude

echo "✓ Cross-user service access configured"
echo "  - Gemini MCP socket: /var/run/agentic-services/gemini-mcp.sock"
echo "  - Z.AI API: http://localhost:9600"
echo "  - MCP Servers: $MCP_SKILL_COUNT skills auto-discovered from SKILL.md frontmatter"
echo "  - VisionFlow TCP bridge: localhost:9500"
echo "  - Environment variables added to devuser's .zshrc"

# ============================================================================
# Phase 7: Generate SSH Host Keys
# ============================================================================

echo "[8/10] Generating SSH host keys..."

if [ ! -f /etc/ssh/ssh_host_rsa_key ]; then
    ssh-keygen -A
    echo "✓ SSH host keys generated"
else
    echo "ℹ️  SSH host keys already exist"
fi

# ============================================================================
# Phase 7.3: Configure SSH Credentials (Host Mount)
# ============================================================================

echo "[7.3/10] Configuring SSH credentials..."

# Check if SSH mount exists (host's ~/.ssh mounted read-only)
if [ -d "/home/devuser/.ssh" ] && [ "$(ls -A /home/devuser/.ssh 2>/dev/null)" ]; then
    echo "✓ SSH credentials detected from host mount"

    # Since mount is read-only, SSH will work directly - no ownership changes needed
    # Skip any chown operations on SSH directory (mounted read-only from host)

    # Verify key files
    KEY_COUNT=$(find /home/devuser/.ssh -type f -name "id_*" ! -name "*.pub" 2>/dev/null | wc -l)
    PUB_COUNT=$(find /home/devuser/.ssh -type f -name "*.pub" 2>/dev/null | wc -l)

    echo "  - Private keys: $KEY_COUNT"
    echo "  - Public keys: $PUB_COUNT"
    echo "  - Mount: read-only (secure)"

    # Add SSH environment setup to devuser's zshrc if not already present
    if ! grep -q "SSH_AUTH_SOCK" /home/devuser/.zshrc 2>/dev/null; then
        sudo -u devuser bash -c 'cat >> ~/.zshrc' <<'SSH_ENV'

# SSH Agent Configuration (auto-configured)
# Start ssh-agent if not running
if [ -z "$SSH_AUTH_SOCK" ]; then
    eval "$(ssh-agent -s)" > /dev/null 2>&1
    # Auto-add keys on first shell
    find ~/.ssh -type f -name "id_*" ! -name "*.pub" -exec ssh-add {} \; 2>/dev/null
fi
SSH_ENV
        echo "  - SSH agent auto-start configured in .zshrc"
    fi

    echo "✓ SSH credentials configured successfully"
else
    echo "ℹ️  SSH credentials not mounted (mount ~/.ssh to container for SSH key access)"
fi

# ============================================================================
# Phase 7.5: Install Management API Health Check Script
# ============================================================================

echo "[7.5/10] Installing Management API health check script..."

# Create scripts directory
mkdir -p /opt/scripts

# Copy verification script if available in unified-config
if [ -f "/unified-config/scripts/verify-management-api.sh" ]; then
    cp /unified-config/scripts/verify-management-api.sh /opt/scripts/
    chmod +x /opt/scripts/verify-management-api.sh
    echo "✓ Management API health check script installed"
else
    # Create inline if not available (fallback)
    cat > /opt/scripts/verify-management-api.sh <<'HEALTHCHECK_SCRIPT'
#!/bin/bash
# Management API Health Check Script
set -e
MANAGEMENT_API_HOST="${MANAGEMENT_API_HOST:-localhost}"
MANAGEMENT_API_PORT="${MANAGEMENT_API_PORT:-9090}"
MAX_RETRIES=30
RETRY_DELAY=2
echo "=== Management API Health Check ==="
echo "Target: http://${MANAGEMENT_API_HOST}:${MANAGEMENT_API_PORT}/health"
for i in $(seq 1 $MAX_RETRIES); do
    if curl -s -f "http://${MANAGEMENT_API_HOST}:${MANAGEMENT_API_PORT}/health" > /dev/null 2>&1; then
        RESPONSE=$(curl -s "http://${MANAGEMENT_API_HOST}:${MANAGEMENT_API_PORT}/health")
        echo "✅ Management API is healthy (attempt $i/$MAX_RETRIES)"
        echo "   Response: $RESPONSE"
        exit 0
    else
        echo "⏳ Attempt $i/$MAX_RETRIES: Management API not ready..."
        if /opt/venv/bin/supervisorctl status management-api | grep -q "RUNNING"; then
            echo "   Process status: RUNNING"
        else
            echo "   ⚠️  Process not running! Restarting..."
            /opt/venv/bin/supervisorctl restart management-api
        fi
        sleep $RETRY_DELAY
    fi
done
echo "❌ Management API health check FAILED"
/opt/venv/bin/supervisorctl status management-api
exit 1
HEALTHCHECK_SCRIPT
    chmod +x /opt/scripts/verify-management-api.sh
    echo "✓ Management API health check script created inline"
fi

# ============================================================================
# Phase 8: Enhance CLAUDE.md with Project Context
# ============================================================================

echo "[9/10] Enhancing CLAUDE.md with project-specific context..."

# Append compact project documentation to system CLAUDE.md
sudo -u devuser bash -c 'cat >> /home/devuser/CLAUDE.md' <<'CLAUDE_APPEND'

---

## 🚀 Project-Specific: Turbo Flow Claude

### 610 Claude Sub-Agents
- **Repository**: https://github.com/ChrisRoyse/610ClaudeSubagents
- **Location**: `/home/devuser/agents/*.md` (610+ templates)
- **Usage**: Load specific agents with `cat agents/<agent-name>.md`
- **Key Agents**: doc-planner, microtask-breakdown, github-pr-manager, tdd-london-swarm

### Z.AI Service (Cost-Effective Claude API)
**Port**: 9600 (internal only) | **User**: zai-user | **Worker Pool**: 4 concurrent
```bash
# Health check
curl http://localhost:9600/health

# Chat request
curl -X POST http://localhost:9600/chat \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Your prompt here", "timeout": 30000}'

# Switch to zai-user
as-zai
```

### Gemini Flow Commands
```bash
gf-init        # Initialize (protocols: a2a,mcp, topology: hierarchical)
gf-swarm       # 66 agents with intelligent coordination
gf-architect   # 5 system architects
gf-coder       # 12 master coders
gf-status      # Swarm status
gf-monitor     # Protocols and performance
gf-health      # Health check
```

### Multi-User System
| User | UID | Purpose | Switch |
|------|-----|---------|--------|
| devuser | 1000 | Claude Code, primary dev | - |
| gemini-user | 1001 | Google Gemini, gemini-flow | `as-gemini` |
| openai-user | 1002 | OpenAI Codex | `as-openai` |
| zai-user | 1003 | Z.AI service (port 9600) | `as-zai` |

### tmux Workspace (8 Windows)
**Attach**: `tmux attach -t workspace`
| Win | Name | Purpose |
|-----|------|---------|
| 0 | Claude-Main | Primary workspace |
| 1 | Claude-Agent | Agent execution |
| 2 | Services | supervisord monitoring |
| 3 | Development | Python/Rust/CUDA dev |
| 4 | Logs | Service logs (split) |
| 5 | System | htop monitoring |
| 6 | VNC-Status | VNC info |
| 7 | SSH-Shell | General shell |

### Management API
**Base**: http://localhost:9090 | **Auth**: `X-API-Key: <MANAGEMENT_API_KEY>`
```bash
GET  /health              # Health (no auth)
GET  /api/status          # System status
POST /api/tasks           # Create task
GET  /api/tasks/:id       # Task status
GET  /metrics             # Prometheus metrics
GET  /documentation       # Swagger UI
```

### Diagnostic Commands
```bash
# Service status
sudo supervisorctl status

# Container diagnostics
docker exec turbo-flow-unified supervisorctl status
docker stats turbo-flow-unified

# Logs
sudo supervisorctl tail -f management-api
sudo supervisorctl tail -f claude-zai
tail -f /var/log/supervisord.log

# User switching test
as-gemini whoami  # Should output: gemini-user
```

### Service Ports
| Port | Service | Access |
|------|---------|--------|
| 22 | SSH | Public (mapped to 2222) |
| 5901 | VNC | Public |
| 8080 | code-server | Public |
| 9090 | Management API | Public |
| 9600 | Z.AI | Internal only |

**Security**: Default creds are DEVELOPMENT ONLY. Change before production:
- SSH: `devuser:turboflow`
- VNC: `turboflow`
- Management API: `X-API-Key: change-this-secret-key`

### Development Environment Notes

**Container Modification Best Practices**:
- ✅ **DO**: Modify Dockerfile and entrypoint scripts DIRECTLY in the project
- ❌ **DON'T**: Create patching scripts or temporary fixes
- ✅ **DO**: Edit /home/devuser/workspace/project/multi-agent-docker/ files
- ❌ **DON'T**: Use workarounds - fix the root cause

**Isolated Docker Environment**:
- This container is isolated from external build systems
- Only these validation tools work:
  - \`cargo test\` - Rust project testing
  - \`npm run check\` / \`npm test\` - Node.js validation
  - \`pytest\` - Python testing
- **DO NOT** attempt to:
  - Build external projects directly
  - Run production builds inside container
  - Execute deployment scripts
  - Access external build infrastructure
- **Instead**: Test, validate, and export artifacts

**File Organization**:
- Never save working files to root (/)
- Use appropriate subdirectories:
  - /docs - Documentation
  - /scripts - Helper scripts
  - /tests - Test files
  - /config - Configuration
CLAUDE_APPEND

echo "✓ CLAUDE.md enhanced with project context"

# ============================================================================
# Phase 9: Display Connection Information
# ============================================================================

echo "[10/10] Container ready! Connection information:"
echo ""
echo "+-------------------------------------------------------------+"
echo "│                   CONNECTION DETAILS                        │"
echo "+-------------------------------------------------------------│"
echo "│ SSH:             ssh devuser@<container-ip> -p 22           │"
echo "│                  Password: turboflow                        │"
echo "│                                                             │"
echo "│ VNC:             vnc://<container-ip>:5901                  │"
echo "│                  Password: turboflow                        │"
echo "│                  Display: :1                                │"
echo "│                                                             │"
echo "│ code-server:     http://<container-ip>:8080                 │"
echo "│                  (No authentication required)              │"
echo "│                                                             │"
echo "│ Management API:  http://<container-ip>:9090                 │"
echo "│                  Health: /health                            │"
echo "│                  Status: /api/v1/status                     │"
echo "│                                                             │"
echo "│ Z.AI Service:    http://localhost:9600 (internal only)      │"
echo "│                  Accessible via ragflow network            │"
echo "+-------------------------------------------------------------│"
echo "│ Users:                                                      │"
echo "│   devuser (1000)      - Claude Code, development           │"
echo "│   gemini-user (1001)  - Google Gemini CLI, gemini-flow     │"
echo "│   openai-user (1002)  - OpenAI Codex                       │"
echo "│   zai-user (1003)     - Z.AI service                       │"
echo "+-------------------------------------------------------------│"
echo "│ Skills:           $SKILL_COUNT custom Claude Code skills             │"
echo "│ Agents:           $AGENT_COUNT agent templates                       │"
echo "+-------------------------------------------------------------│"
echo "│ tmux Session:     workspace (8 windows)                     │"
echo "│   Attach with:    tmux attach-session -t workspace         │"
echo "+-------------------------------------------------------------+"
echo ""

# ============================================================================
# Phase 10: Start Supervisord
# ============================================================================

echo "[11/11] Starting supervisord (all services)..."
echo ""

# Display what will start
echo "Starting services:"
echo "  ✓ DBus daemon"
echo "  ✓ SSH server (port 22)"
echo "  ✓ VNC server (port 5901)"
echo "  ✓ XFCE4 desktop"
echo "  ✓ Management API (port 9090)"
echo "  ✓ code-server (port 8080)"
echo "  ✓ Claude Z.AI service (port 9600)"
echo "  ✓ ComfyUI server (port 8188)"
echo "  ✓ MCP servers (web-summary, qgis, blender, imagemagick, playwright)"
echo "  ✓ Gemini-flow daemon"
echo "  ✓ tmux workspace auto-start"
echo ""
echo "========================================"
echo "  ALL SYSTEMS READY - STARTING NOW"
echo "========================================"
echo ""

# Start supervisord (will run in foreground)
exec /opt/venv/bin/supervisord -n -c /etc/supervisord.conf
