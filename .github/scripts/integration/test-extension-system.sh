#!/bin/bash
# Test extension system with nodejs installation
set -e

cd /workspace/scripts/lib

echo "=== Testing Extension System ==="

# Test extension-manager availability
echo ""
echo "Testing extension-manager list..."
bash extension-manager.sh list

echo ""
echo "✅ Extension manager available"

# Test extension installation (mise-config already installed from protected extensions)
echo ""
echo "Installing nodejs extension..."
if bash extension-manager.sh install nodejs 2>&1; then
  echo "✅ nodejs extension installed"
else
  echo "⚠️  Installation failed, checking mise status..."
  if command -v mise >/dev/null 2>&1; then
    echo "Running mise doctor for diagnostics:"
    mise doctor || true
  else
    echo "mise not available (this is expected for non-mise extensions)"
  fi
  exit 1
fi

# Verify nodejs installation via mise
echo ""
echo "Verifying nodejs via mise..."
eval "$(mise activate bash)"
if command -v node >/dev/null 2>&1; then
  echo "✅ nodejs available via mise"
  node --version
else
  echo "❌ nodejs not found after installation"
  exit 1
fi

# Verify mise is managing nodejs
echo ""
echo "Checking mise management of nodejs..."
if command -v mise >/dev/null 2>&1; then
  echo "✅ mise is available, running diagnostics..."
  mise doctor || echo "⚠️  mise doctor check completed with warnings"
else
  echo "ℹ️  mise not available (this is expected for current extension set)"
fi

echo ""
echo "✅ Extension system test passed"
