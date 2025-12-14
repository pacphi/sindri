#!/bin/zsh
clear
echo "\033[1;93m╔════════════════════════════════════════════════════════════════════╗\033[0m"
echo "\033[1;93m║              ⚡ Z.AI USER SHELL (UID 1003)                         ║\033[0m"
echo "\033[1;93m╚════════════════════════════════════════════════════════════════════╝\033[0m"
echo ""
echo "\033[1;32m📂 Working Directory:\033[0m /home/zai-user"
echo "\033[1;32m👤 User:\033[0m zai-user (UID 1003)"
echo "\033[1;32m🎯 Purpose:\033[0m Z.AI service management (cost-effective Claude API)"
echo "\033[1;32m🔐 Credentials:\033[0m ~/.config/zai/config.json"
echo "\033[1;32m🌐 Service:\033[0m http://localhost:9600 (internal only)"
echo ""
echo "\033[1;33m💡 Z.AI Service:\033[0m"
echo "  \033[0;36mcurl http://localhost:9600/health\033[0m - Check service health"
echo "  \033[0;36m4-worker pool\033[0m with 50-request queue"
echo "  \033[0;36mUsed by web-summary skill\033[0m internally"
echo ""
exec zsh
