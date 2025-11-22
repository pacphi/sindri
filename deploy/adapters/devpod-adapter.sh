#!/bin/bash
# DevPod adapter - DevContainer deployment

set -e

# shellcheck disable=SC2034  # May be used in future adapter implementations
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SINDRI_YAML="${1:-sindri.yaml}"

if [[ ! -f "$SINDRI_YAML" ]]; then
    echo "Error: $SINDRI_YAML not found"
    exit 1
fi

# Parse sindri.yaml
NAME=$(yq '.name' "$SINDRI_YAML")
PROFILE=$(yq '.extensions.profile // "minimal"' "$SINDRI_YAML")

# Create .devcontainer directory
mkdir -p .devcontainer

# Generate devcontainer.json
cat > .devcontainer/devcontainer.json << EODC
{
  "name": "${NAME}",
  "dockerFile": "../docker/Dockerfile",
  "workspaceFolder": "/workspace",
  "workspaceMount": "source=\${localWorkspaceFolder},target=/workspace,type=bind",

  "customizations": {
    "vscode": {
      "extensions": [
        "ms-vscode.vscode-typescript-next",
        "dbaeumer.vscode-eslint",
        "esbenp.prettier-vscode",
        "ms-python.python",
        "ms-python.vscode-pylance",
        "golang.go",
        "rust-lang.rust-analyzer"
      ],
      "settings": {
        "terminal.integrated.defaultProfile.linux": "bash",
        "terminal.integrated.profiles.linux": {
          "bash": {
            "path": "/bin/bash",
            "icon": "terminal-bash"
          }
        }
      }
    }
  },

  "features": {
    "ghcr.io/devcontainers/features/github-cli:1": {},
    "ghcr.io/devcontainers/features/docker-in-docker:2": {}
  },

  "postCreateCommand": "/docker/lib/cli/extension-manager install-profile ${PROFILE}",
  "postStartCommand": "echo 'Welcome to Sindri DevContainer!'",

  "remoteUser": "developer",
  "containerUser": "developer",

  "mounts": [
    "source=sindri-workspace,target=/workspace,type=volume"
  ],

  "runArgs": [
    "--cap-add=SYS_PTRACE",
    "--security-opt", "seccomp=unconfined"
  ],

  "forwardPorts": [3000, 8080],

  "portsAttributes": {
    "3000": {
      "label": "Application",
      "onAutoForward": "notify"
    },
    "8080": {
      "label": "API",
      "onAutoForward": "silent"
    }
  }
}
EODC

# Generate provider.yaml for DevPod
cat > .devcontainer/provider.yaml << EOPY
name: sindri-provider
version: v1.0.0
description: Sindri development environment provider

agent:
  path: \${DEVPOD}
  driver: docker
  docker:
    buildRepository: sindri-devpod

options:
  PROFILE:
    description: Extension profile to install
    default: "${PROFILE}"
    enum: ["minimal", "fullstack", "ai-dev", "systems", "enterprise"]

  WORKSPACE_SIZE:
    description: Workspace volume size
    default: "10GB"

exec:
  command: |-
    docker exec -it \${CONTAINER_ID} \${COMMAND}

  init: |-
    echo "Initializing Sindri workspace..."
    /docker/lib/cli/extension-manager install-profile \${PROFILE}

  shutdown: |-
    echo "Shutting down Sindri workspace..."
EOPY

echo "==> DevPod configuration created"
echo ""
echo "To use with DevPod:"
echo "  1. Install DevPod: https://devpod.sh/docs/getting-started/install"
echo "  2. Create workspace: devpod up . --provider docker"
echo ""
echo "To use with VS Code:"
echo "  1. Install 'Dev Containers' extension"
echo "  2. Open folder in container: Ctrl+Shift+P -> 'Dev Containers: Open Folder in Container'"
echo ""
echo "To use with GitHub Codespaces:"
echo "  1. Push to GitHub repository"
echo "  2. Create codespace from repository"
