#!/bin/bash
set -e

echo "🔧 Setting up deepseek-user..."

# Create deepseek user if doesn't exist
if ! id -u deepseek-user >/dev/null 2>&1; then
    useradd -m -u 1004 -s /usr/bin/zsh deepseek-user
    echo "✓ Created deepseek-user (UID 1004)"
else
    echo "✓ deepseek-user already exists"
fi

# Create directory structure
mkdir -p /home/deepseek-user/{workspace,agentic-flow,.config/deepseek,.cache}
chown -R deepseek-user:deepseek-user /home/deepseek-user

# Configure sudo access
if ! grep -q "devuser ALL=(deepseek-user)" /etc/sudoers; then
    echo "devuser ALL=(deepseek-user) NOPASSWD: ALL" >> /etc/sudoers
    echo "✓ Configured sudo access"
fi

# Create DeepSeek API config
cat > /home/deepseek-user/.config/deepseek/config.json <<'EOF'
{
  "apiKey": "sk-[your deepseek api key]",
  "baseUrl": "https://api.deepseek.com/v3.2_speciale_expires_on_20251215",
  "model": "deepseek-chat",
  "maxTokens": 4096,
  "temperature": 0.7
}
EOF
chown deepseek-user:deepseek-user /home/deepseek-user/.config/deepseek/config.json
chmod 600 /home/deepseek-user/.config/deepseek/config.json

# Clone and configure agentic-flow
if [ ! -d /home/deepseek-user/agentic-flow ]; then
    echo "📥 Cloning agentic-flow..."
    sudo -u deepseek-user git clone https://github.com/ruvnet/agentic-flow /home/deepseek-user/agentic-flow
    echo "✓ Cloned agentic-flow"
fi

cd /home/deepseek-user/agentic-flow

# Install dependencies
echo "📦 Installing agentic-flow dependencies..."
sudo -u deepseek-user npm install

# Create .env file for agentic-flow
cat > /home/deepseek-user/agentic-flow/.env <<'EOF'
# DeepSeek API Configuration
DEEPSEEK_API_KEY=sk-[your deepseek api key]
DEEPSEEK_BASE_URL=https://api.deepseek.com/v3.2_speciale_expires_on_20251215
DEEPSEEK_MODEL=deepseek-chat
DEEPSEEK_MAX_TOKENS=4096
DEEPSEEK_TEMPERATURE=0.7

# Default AI Provider
AI_PROVIDER=deepseek
API_KEY=sk-[your deepseek api key]
API_BASE_URL=https://api.deepseek.com/v3.2_speciale_expires_on_20251215
MODEL=deepseek-chat
EOF
chown deepseek-user:deepseek-user /home/deepseek-user/agentic-flow/.env
chmod 600 /home/deepseek-user/agentic-flow/.env

echo "✅ DeepSeek user setup complete!"
echo ""
echo "🧪 Test with:"
echo "  sudo -u deepseek-user bash -c 'cd ~/agentic-flow && npx agentic-flow --help'"
