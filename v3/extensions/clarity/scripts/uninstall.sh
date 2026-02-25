#!/usr/bin/env bash
set -euo pipefail

# Clarity Spec Generation Skill - Uninstallation Script

SKILL_DIR="${HOME}/.claude/skills/clarity"

echo "Removing Clarity spec generation skill..."

if [ -d "${SKILL_DIR}" ]; then
    rm -rf "${SKILL_DIR}"
    echo "Clarity skill removed from ${SKILL_DIR}"
else
    echo "Clarity skill not found at ${SKILL_DIR} (already removed)"
fi
