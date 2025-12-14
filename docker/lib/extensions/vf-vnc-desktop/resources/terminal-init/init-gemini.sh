#!/bin/zsh
clear
echo "\033[1;95m╔════════════════════════════════════════════════════════════════════╗\033[0m"
echo "\033[1;95m║              🔮 GEMINI USER SHELL (UID 1001)                       ║\033[0m"
echo "\033[1;95m╚════════════════════════════════════════════════════════════════════╝\033[0m"
echo ""
echo "\033[1;32m📂 Working Directory:\033[0m /home/gemini-user/workspace"
echo "\033[1;32m👤 User:\033[0m gemini-user (UID 1001)"
echo "\033[1;32m🎯 Purpose:\033[0m Isolated Google Gemini API operations"
echo "\033[1;32m🔐 Credentials:\033[0m ~/.config/gemini/config.json"
echo ""
echo "\033[1;33m💡 Gemini Flow Commands:\033[0m"
echo "  \033[0;36mgemini-flow --version\033[0m  - Check installation"
echo "  \033[0;36mgf-init\033[0m                - Initialize project"
echo "  \033[0;36mgf-swarm\033[0m               - Launch 66-agent swarm"
echo "  \033[0;36mgf-status\033[0m              - Check swarm status"
echo ""
exec zsh
