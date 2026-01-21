#!/bin/zsh
clear
echo "\033[1;35m╔════════════════════════════════════════════════════════════════════╗\033[0m"
echo "\033[1;35m║              🐳 DOCKER MANAGER                                     ║\033[0m"
echo "\033[1;35m╚════════════════════════════════════════════════════════════════════╝\033[0m"
echo ""
echo "\033[1;32m📂 Working Directory:\033[0m /home/devuser/workspace"
echo "\033[1;32m👤 User:\033[0m devuser (UID 1000)"
echo "\033[1;32m🎯 Purpose:\033[0m Container and image management"
echo ""
echo "\033[1;33m💡 Quick Commands:\033[0m"
echo "  \033[0;36mdocker ps\033[0m              - List running containers"
echo "  \033[0;36mdocker images\033[0m          - List images"
echo "  \033[0;36mdocker compose ps\033[0m      - Show compose services"
echo "  \033[0;36mdocker stats\033[0m           - Resource usage"
echo ""
exec zsh
