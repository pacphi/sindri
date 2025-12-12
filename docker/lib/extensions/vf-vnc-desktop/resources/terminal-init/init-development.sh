#!/bin/zsh
clear
echo "\033[1;32m╔════════════════════════════════════════════════════════════════════╗\033[0m"
echo "\033[1;32m║              💻 DEVELOPMENT - AR-AI-KNOWLEDGE-GRAPH               ║\033[0m"
echo "\033[1;32m╚════════════════════════════════════════════════════════════════════╝\033[0m"
echo ""
echo "\033[1;32m📂 Working Directory:\033[0m /home/devuser/workspace/project"
echo "\033[1;32m👤 User:\033[0m devuser (UID 1000)"
echo "\033[1;32m🎯 Purpose:\033[0m External project development (mounted from host)"
echo "\033[1;32m🔗 Host Path:\033[0m /mnt/mldata/githubs/AR-AI-Knowledge-Graph"
echo ""
echo "\033[1;33m💡 Development Tools:\033[0m"
echo "  \033[0;36mpython, rust, node.js, docker\033[0m - All available"
echo "  \033[0;36mGPU acceleration enabled\033[0m - CUDA toolkit available"
echo "  \033[0;36mChanges persist to host\033[0m - Read-write mount"
echo ""
if [ -f ".git/config" ]; then
  echo "\033[1;34m📊 Git Status:\033[0m"
  git branch --show-current 2>/dev/null | sed 's/^/  Current branch: /'
  git status -s 2>/dev/null | head -5
fi
echo ""
exec zsh
