#!/bin/zsh
clear
echo "\033[1;34m╔════════════════════════════════════════════════════════════════════╗\033[0m"
echo "\033[1;34m║              🔀 GIT VERSION CONTROL                                ║\033[0m"
echo "\033[1;34m╚════════════════════════════════════════════════════════════════════╝\033[0m"
echo ""
echo "\033[1;32m📂 Working Directory:\033[0m /home/devuser/workspace/project"
echo "\033[1;32m👤 User:\033[0m devuser (UID 1000)"
echo "\033[1;32m🎯 Purpose:\033[0m Git operations and version control"
echo ""
echo "\033[1;33m💡 Quick Commands:\033[0m"
echo "  \033[0;36mgit status\033[0m             - Check repository status"
echo "  \033[0;36mgit log --oneline -10\033[0m  - Recent commits"
echo "  \033[0;36mgit branch -a\033[0m          - List all branches"
echo "  \033[0;36mgh pr list\033[0m             - GitHub PR list"
echo ""
cd /home/devuser/workspace/project 2>/dev/null || cd /home/devuser/workspace
if [ -d .git ]; then
  echo "\033[1;34m🔍 Repository Status:\033[0m"
  git branch --show-current
  git status -s | head -5
  echo ""
fi
exec zsh
