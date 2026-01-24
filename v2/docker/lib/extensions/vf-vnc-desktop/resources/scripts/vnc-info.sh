#!/bin/bash
# Display VNC connection information

echo "=== VNC Server Information ==="
echo ""

# Get container IP
CONTAINER_IP=$(hostname -I | awk '{print $1}')

echo "🖥️  VNC Display: :1"
echo "🔌 VNC Port: 5901"
echo "🔑 VNC Password: turboflow"
echo ""

echo "📡 Connection URLs:"
echo "   Direct VNC:     vnc://$CONTAINER_IP:5901"
echo "   VNC Viewer:     $CONTAINER_IP:5901"
echo ""

echo "📋 VNC Process Status:"
ps aux | grep -i vnc | grep -v grep || echo "   ⚠️  No VNC processes found"
echo ""

echo "🗂️  X11 Display Files:"
ls -la /tmp/.X11-unix/ 2>/dev/null || echo "   ⚠️  No X11 sockets found"
echo ""

echo "🔧 Control Commands:"
echo "   Restart VNC:    sudo /opt/venv/bin/supervisorctl restart x11vnc"
echo "   Stop VNC:       sudo /opt/venv/bin/supervisorctl stop x11vnc"
echo "   Start VNC:      sudo /opt/venv/bin/supervisorctl start x11vnc"
echo "   View logs:      tail -f /var/log/x11vnc.log"
