#!/bin/zsh
clear
echo "\033[1;94m╔════════════════════════════════════════════════════════════════════╗\033[0m"
echo "\033[1;94m║              🧠 OPENAI USER SHELL (UID 1002)                       ║\033[0m"
echo "\033[1;94m╚════════════════════════════════════════════════════════════════════╝\033[0m"
echo ""
echo "\033[1;32m📂 Working Directory:\033[0m /home/openai-user/workspace"
echo "\033[1;32m👤 User:\033[0m openai-user (UID 1002)"
echo "\033[1;32m🎯 Purpose:\033[0m Isolated OpenAI API operations"
echo "\033[1;32m🔐 Credentials:\033[0m ~/.config/openai/config.json"
echo ""
echo "\033[1;33m💡 OpenAI Tools:\033[0m"
echo "  \033[0;36mPython, Node.js, Rust\033[0m available"
echo "  \033[0;36mIsolated from other users\033[0m"
echo "  \033[0;36mDedicated workspace volume\033[0m"
echo ""
exec zsh
