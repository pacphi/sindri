#!/bin/bash
# Create welcome script in /etc/skel so it gets copied to the persistent home
set -e

cat > /etc/skel/welcome.sh << 'EOF'
#!/bin/bash
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  Welcome to Sindri - Your AI-Powered Development Forge!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "  Getting Started:"
echo ""
echo "  1. Install development tools (optional):"
echo "     extension-manager --interactive     # Interactive setup"
echo "     extension-manager install-all       # Install all active extensions"
echo "     extension-manager list              # View available extensions"
echo ""
echo "  2. Start coding with Claude:"
echo "     claude                              # Launch Claude Code"
echo ""
echo "  Pre-installed: Claude Code, mise, Git, jq, yq, curl"
echo ""
echo "  Workspace: ~/workspace (persistent volume)"
echo "  Projects:  ~/workspace/projects"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
EOF

chmod +x /etc/skel/welcome.sh
