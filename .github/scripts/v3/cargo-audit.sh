#!/usr/bin/env bash
# Run the v3 dependency audit exactly as CI's "Dependency Audit" job does.
# Ignored advisories live only in v3/.cargo/audit.toml (read by cargo audit itself).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

cd "$REPO_ROOT/v3"
exec cargo audit --deny warnings "$@"
