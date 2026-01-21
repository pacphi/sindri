#!/bin/bash
# Show status of all services

echo "=== Service Status Dashboard ==="
echo ""

# Supervisord status
echo "📊 Supervisord Services:"
sudo /opt/venv/bin/supervisorctl status
echo ""

# Port listeners
echo "🔌 Port Listeners:"
echo "   SSH (22):         $(ss -tlnp | grep ':22 ' | wc -l) listener(s)"
echo "   VNC (5901):       $(ss -tlnp | grep ':5901 ' | wc -l) listener(s)"
echo "   code-server (8080): $(ss -tlnp | grep ':8080 ' | wc -l) listener(s)"
echo "   Management API (9090): $(ss -tlnp | grep ':9090 ' | wc -l) listener(s)"
echo "   Z.AI (9600):      $(ss -tlnp | grep ':9600 ' | wc -l) listener(s)"
echo ""

# tmux sessions
echo "🖥️  tmux Sessions:"
sudo -u devuser tmux ls 2>/dev/null || echo "   No active sessions"
echo ""

# User processes
echo "👥 User Processes:"
echo "   devuser:      $(ps aux | grep -E '^devuser' | wc -l) processes"
echo "   gemini-user:  $(ps aux | grep -E '^gemini-user' | wc -l) processes"
echo "   openai-user:  $(ps aux | grep -E '^openai-user' | wc -l) processes"
echo "   zai-user:     $(ps aux | grep -E '^zai-user' | wc -l) processes"
echo ""

# Resource usage
echo "💾 Resource Usage:"
echo "   Memory: $(free -h | grep Mem | awk '{print $3 "/" $2}')"
echo "   CPU Load: $(uptime | awk -F'load average:' '{print $2}')"
echo ""

# Health check
echo "🏥 Health Check:"
if curl -sf http://localhost:9090/health >/dev/null 2>&1; then
    echo "   ✅ Management API is healthy"
else
    echo "   ❌ Management API is not responding"
fi
