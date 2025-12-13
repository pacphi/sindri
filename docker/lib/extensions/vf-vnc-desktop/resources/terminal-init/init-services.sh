#!/bin/zsh
clear
echo "\033[1;33m╔════════════════════════════════════════════════════════════════════╗\033[0m"
echo "\033[1;33m║              ⚙️  SERVICE MONITORING & MANAGEMENT                   ║\033[0m"
echo "\033[1;33m╚════════════════════════════════════════════════════════════════════╝\033[0m"
echo ""
echo "\033[1;32m📂 Working Directory:\033[0m /home/devuser"
echo "\033[1;32m👤 User:\033[0m devuser (UID 1000) with sudo"
echo "\033[1;32m🎯 Purpose:\033[0m Monitor and manage system services"
echo ""
echo "\033[1;33m💡 Service Commands:\033[0m"
echo "  \033[0;36msudo /opt/venv/bin/supervisorctl status\033[0m         - Check all services"
echo "  \033[0;36msudo /opt/venv/bin/supervisorctl tail -f <name>\033[0m - View service logs"
echo "  \033[0;36msudo /opt/venv/bin/supervisorctl restart <name>\033[0m - Restart service"
echo "  \033[0;36mcurl http://localhost:9090/health\033[0m - Management API health"
echo ""
echo "\033[1;34m🔍 Running service status check...\033[0m"
sudo /opt/venv/bin/supervisorctl status | head -10
echo ""
exec zsh
