#!/bin/zsh
clear
echo "\033[1;36m╔════════════════════════════════════════════════════════════════════╗\033[0m"
echo "\033[1;36m║              🤖 CLAUDE CODE - MAIN WORKSPACE                       ║\033[0m"
echo "\033[1;36m╚════════════════════════════════════════════════════════════════════╝\033[0m"
echo ""
echo "\033[1;32m📂 Working Directory:\033[0m /home/devuser/workspace"
echo "\033[1;32m👤 User:\033[0m devuser (UID 1000)"
echo "\033[1;32m🎯 Purpose:\033[0m Primary Claude Code development workspace"
echo ""
echo "\033[1;33m💡 Quick Commands:\033[0m"
echo "  \033[0;36mdsp\033[0m                    - Start Claude Code (dangerously skip permissions)"
echo "  \033[0;36mcd project\033[0m             - Go to external project mount"
echo "  \033[0;36mtmux attach -t workspace\033[0m - Connect to SSH tmux session"
echo ""
exec zsh
