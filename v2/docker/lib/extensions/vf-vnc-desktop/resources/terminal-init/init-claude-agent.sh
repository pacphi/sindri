#!/bin/zsh
clear
echo "\033[1;35m╔════════════════════════════════════════════════════════════════════╗\033[0m"
echo "\033[1;35m║              🤖 CLAUDE CODE - AGENT EXECUTION                      ║\033[0m"
echo "\033[1;35m╚════════════════════════════════════════════════════════════════════╝\033[0m"
echo ""
echo "\033[1;32m📂 Working Directory:\033[0m /home/devuser/agents"
echo "\033[1;32m👤 User:\033[0m devuser (UID 1000)"
echo "\033[1;32m🎯 Purpose:\033[0m Agent testing and execution environment"
echo ""
echo "\033[1;33m💡 Available Agents:\033[0m"
echo "  \033[0;36mls *.md | wc -l\033[0m        - Count available agents"
echo "  \033[0;36mfind . -name '*github*'\033[0m - Find GitHub-specific agents"
echo "  \033[0;36mcf-swarm \"task\"\033[0m       - Launch claude-flow swarm"
echo ""
exec zsh
