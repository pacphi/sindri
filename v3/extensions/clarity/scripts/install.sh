#!/usr/bin/env bash
set -euo pipefail

# Clarity Spec Generation Skill - Installation Script
# Clones the Clarity agent skill into ~/.claude/skills/clarity/

SKILL_DIR="${HOME}/.claude/skills/clarity"
REPO_URL="https://github.com/francyjglisboa/clarity.git"

echo "Installing Clarity spec generation skill..."

# Create parent directory
mkdir -p "${HOME}/.claude/skills"

if [ -d "${SKILL_DIR}/.git" ]; then
    echo "Clarity skill already installed, updating..."
    cd "${SKILL_DIR}"
    git pull --ff-only origin main 2>/dev/null || {
        echo "Warning: Could not update Clarity skill (upstream may have diverged)"
        echo "Existing installation preserved at ${SKILL_DIR}"
    }
else
    # Remove any partial installation
    if [ -d "${SKILL_DIR}" ]; then
        rm -rf "${SKILL_DIR}"
    fi

    echo "Cloning Clarity from ${REPO_URL}..."
    git clone --depth 1 "${REPO_URL}" "${SKILL_DIR}"
fi

echo "Clarity skill installed to ${SKILL_DIR}"
echo "Use '/clarity <references>' to generate specs"
