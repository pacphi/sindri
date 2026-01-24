#!/bin/bash
# Wait for desktop to fully start
sleep 5

# Ensure DISPLAY is set (required for xfce4-terminal)
export DISPLAY="${DISPLAY:-:1}"

# Launch 9 xfce4 terminals with colorful init scripts (3x3 grid)

# Row 1 (3 terminals) - Claude workspace
xfce4-terminal --title="🤖 Claude-Main" --geometry=80x24 -e "/home/devuser/.config/init-claude-main.sh" &
sleep 0.5
xfce4-terminal --title="🤖 Claude-Agent" --geometry=80x24 -e "/home/devuser/.config/init-claude-agent.sh" &
sleep 0.5
xfce4-terminal --title="⚙️  Services" --geometry=80x24 -e "/home/devuser/.config/init-services.sh" &

# Row 2 (3 terminals) - Development
sleep 0.5
xfce4-terminal --title="💻 Development" --geometry=80x24 -e "/home/devuser/.config/init-development.sh" &
sleep 0.5
xfce4-terminal --title="🐳 Docker" --geometry=80x24 -e "/home/devuser/.config/init-docker.sh" &
sleep 0.5
xfce4-terminal --title="🔀 Git" --geometry=80x24 -e "/home/devuser/.config/init-git.sh" &

# Row 3 (3 terminals) - User shells
sleep 0.5
xfce4-terminal --title="🔮 Gemini-Shell" --geometry=80x24 -e "bash -c 'sudo -u gemini-user /home/devuser/.config/init-gemini.sh'" &
sleep 0.5
xfce4-terminal --title="🧠 OpenAI-Shell" --geometry=80x24 -e "bash -c 'sudo -u openai-user /home/devuser/.config/init-openai.sh'" &
sleep 0.5
xfce4-terminal --title="⚡ Z.AI-Shell" --geometry=80x24 -e "bash -c 'sudo -u zai-user /home/devuser/.config/init-zai.sh'" &

# Row 4 (1 terminal) - DeepSeek
sleep 0.5
xfce4-terminal --title="🧠 DeepSeek-Shell" --geometry=80x24 -e "bash -c 'sudo -u deepseek-user /home/devuser/.config/init-deepseek.sh'" &

# Launch Chromium with DevTools
sleep 2
chromium --remote-debugging-port=9222 --user-data-dir=/home/devuser/.config/chromium-mcp &
