# Warp Terminal - Remote Development Setup

Warp is a modern, Rust-based terminal with AI assistance and Docker integration. While not a full IDE, Warp provides excellent terminal-based development workflows for Sindri environments.

## Prerequisites

- [Warp Terminal](https://www.warp.dev/) (latest version)
- macOS, Linux, or Windows (with WSL2)
- Docker Desktop (for Docker integration features)
- SSH client configured

## Connection Methods

### Method 1: SSH to Remote VM (Fly.io)

Warp provides a standard SSH experience with modern terminal features.

**Setup Steps:**

1. **Configure SSH in ~/.ssh/config:**

   ```bash
   Host sindri-flyio
     HostName your-app.fly.dev
     User developer
     Port 22
     IdentityFile ~/.ssh/your_key
     ServerAliveInterval 60
     ServerAliveCountMax 3
   ```

2. **Connect via Warp:**
   - Open Warp terminal
   - Run: `ssh sindri-flyio`
   - Or use full connection string: `ssh developer@your-app.fly.dev`

3. **Navigate to workspace:**
   ```bash
   cd /alt/home/developer/workspace
   ```

**Warp-Specific Features:**

- **AI Command Search:** Use Cmd+P to search for commands with natural language
- **Blocks:** Each command output is a block you can select, copy, or share
- **Workflows:** Save commonly used command sequences
- **Command History:** Smart search across all command history

**Best Practices:**

- Use SSH key-based authentication
- Configure SSH multiplexing for faster reconnection
- Save frequently used SSH commands as Warp workflows
- Use Warp's AI to discover new commands and tools

### Method 2: Docker Integration (Local Containers)

Warp's Docker extension simplifies container access on macOS.

**Setup Steps:**

1. **Install Warp Docker Extension (macOS only):**
   - Visit [Docker Hub - Warp Extension](https://hub.docker.com/extensions/warpdotdev/warp)
   - Install via Docker Desktop Extensions
   - Or install from Warp's integrations panel

2. **Access containers via Warp:**
   - Extension lists all running Docker containers
   - Click to open container in Warpified subshell
   - No need to manually type `docker exec` or container IDs

3. **Manual container access (all platforms):**

   ```bash
   # List running containers
   docker ps

   # Execute shell in container
   docker exec -it container_name bash

   # Or for Sindri containers
   docker exec -it sindri-dev bash
   ```

**Docker Extension Features (macOS only):**

- One-click container access
- Automatic container ID completion
- Quick container switching
- Warp features work inside container shells

**Best Practices:**

- Use named containers for easier identification
- Leverage Warp's command completion inside containers
- Create workflows for common container commands
- Use Warp AI to learn Docker commands

### Method 3: SSH to Docker Container (DevPod Workaround)

Since Warp doesn't have native devcontainer support, use SSH to access containers.

**Setup Steps:**

1. **Create Dockerfile with SSH server:**

   ```dockerfile
   FROM your-sindri-image:latest

   # Install OpenSSH server
   RUN apt-get update && apt-get install -y openssh-server

   # Configure SSH
   RUN mkdir /var/run/sshd
   RUN echo 'developer:password' | chpasswd

   # Expose SSH port
   EXPOSE 22

   CMD ["/usr/sbin/sshd", "-D"]
   ```

2. **Run container with SSH:**

   ```bash
   docker run -d -p 2222:22 --name sindri-ssh your-sindri-image:latest
   ```

3. **Configure SSH connection:**

   ```bash
   # Add to ~/.ssh/config
   Host sindri-container
     HostName localhost
     User developer
     Port 2222
   ```

4. **Connect via Warp:**
   ```bash
   ssh sindri-container
   ```

**Limitations:**

- Not native devcontainer support
- Manual SSH server setup required
- Additional container complexity

## Warp Features for Remote Development

### Workflows

Save common command sequences for quick access.

**Create a workflow:**

1. Open Warp Settings > Workflows
2. Click "New Workflow"
3. Example workflow for Sindri:
   ```yaml
   name: "Connect to Sindri Fly.io"
   command: "ssh sindri-flyio && cd /alt/home/developer/workspace"
   ```

**Access workflows:**

- Cmd+Shift+R to open workflows panel
- Search and execute saved workflows

### AI Command Search

Use natural language to find commands.

**Examples:**

- "How do I check disk usage" → `df -h`
- "List files by size" → `ls -lhS`
- "Find large files" → `find . -type f -size +100M`

**Access AI:**

- Cmd+P or click sparkle icon
- Type natural language query
- Execute suggested command

### Blocks

Each command and its output is a block you can manipulate.

**Block features:**

- Click to select entire block
- Cmd+C to copy block content
- Share blocks with team members
- Navigate between blocks with keyboard

### Command History

Smart search across all command history.

**Access:**

- Cmd+R for history search
- Type to filter commands
- Arrow keys to navigate
- Enter to execute

## Advanced Configuration

### SSH Connection Optimization

**Multiplexing for faster reconnection:**

```bash
# Add to ~/.ssh/config
Host *
  ControlMaster auto
  ControlPath ~/.ssh/control-%r@%h:%p
  ControlPersist 10m
```

**Connection keepalive:**

```bash
# Add to ~/.ssh/config
Host sindri-flyio
  ServerAliveInterval 60
  ServerAliveCountMax 3
  TCPKeepAlive yes
```

### Custom SSH Scripts

Create shell scripts for common operations:

```bash
#!/bin/bash
# ~/bin/sindri-connect

ssh sindri-flyio << 'EOF'
  cd /alt/home/developer/workspace
  mise install
  exec $SHELL
EOF
```

Make executable and use:

```bash
chmod +x ~/bin/sindri-connect
~/bin/sindri-connect
```

### Warp Drive (Shared Context)

Share terminal sessions with team members:

1. Click "Share" button in Warp
2. Select blocks to share
3. Generate shareable link
4. Team members can view and copy commands

## Docker-Specific Features

### Container Management Commands

**Quick reference for Warp:**

```bash
# List containers
docker ps -a

# Start/stop containers
docker start sindri-dev
docker stop sindri-dev

# View logs
docker logs -f sindri-dev

# Execute commands
docker exec -it sindri-dev bash

# Remove containers
docker rm sindri-dev

# View container resource usage
docker stats sindri-dev
```

### Warp AI for Docker

Use Warp AI to learn Docker commands:

- "How do I see running containers" → `docker ps`
- "How do I enter a container" → `docker exec -it container_name bash`
- "How do I view container logs" → `docker logs container_name`

## Limitations

### No Native Remote Development Features

Warp is a terminal, not an IDE:

- No language server support
- No integrated file browser
- No code completion (beyond shell completion)
- No debugging capabilities
- Limited devcontainer support

### Workarounds

**For full remote development experience, combine Warp with:**

- **VS Code:** Use Warp for terminal, VS Code for editing
- **tmux/screen:** Persistent sessions on remote servers
- **vim/neovim:** Terminal-based code editing
- **Language-specific REPLs:** Interactive development

### macOS-Only Features

Some features only work on macOS:

- Docker Desktop extension
- Native macOS integration
- Some performance optimizations

## Troubleshooting

### SSH Connection Issues

- **"Connection refused":** Verify SSH server is running
- **"Permission denied":** Check SSH key configuration
- **Timeout errors:** Check network and firewall settings

### Docker Integration Issues

- **Extension not available:** Ensure you're on macOS with Docker Desktop
- **Can't connect to container:** Verify container is running with `docker ps`
- **Permission errors:** Add user to docker group (Linux)

### Performance Issues

- **Slow rendering:** Check GPU acceleration in Warp settings
- **High memory usage:** Restart Warp or limit scroll buffer
- **Laggy SSH sessions:** Check network latency and bandwidth

## Resources

- [Warp Terminal Documentation](https://docs.warp.dev/)
- [Warp Docker Extension](https://hub.docker.com/extensions/warpdotdev/warp)
- [SSH in Docker Container - Warp Guide](https://www.warp.dev/terminus/ssh-docker-container)
- [Warp Integrations and Plugins](https://docs.warp.dev/terminal/integrations-and-plugins)
- [Remote connection management discussion](https://github.com/warpdotdev/warp/discussions/442)

## Recommended Workflows

### Daily Development

1. Use Warp for terminal operations and SSH connections
2. Use VS Code or IntelliJ for code editing and IDE features
3. Use Warp's AI for discovering commands
4. Save common operations as Warp workflows

### Container Development

1. Use Docker Desktop for container management
2. Use Warp Docker extension for quick container access (macOS)
3. Use `docker exec` commands for container interaction
4. Combine with IDE remote features for full development experience

### Remote Server Development

1. Configure SSH connections in ~/.ssh/config
2. Use Warp for all terminal operations
3. Use tmux/screen for persistent sessions
4. Combine with VS Code Remote-SSH for editing

## Next Steps

After setting up Warp for Sindri development:

1. Configure SSH keys for all remote connections
2. Install useful terminal tools (tmux, vim, fzf)
3. Create workflows for common operations
4. Explore Warp AI for learning new commands
5. Consider combining Warp with a full-featured IDE

---

**Related Documentation:**

- [VS Code Setup](VSCODE.md) - For full IDE integration
- [IntelliJ IDEA Setup](INTELLIJ.md) - For JetBrains development
- [Zed Editor Setup](ZED.md) - For modern editing
- [Eclipse Setup](ECLIPSE.md) - For Java development

**Note:** Warp is excellent for terminal operations but should be combined with a full IDE (VS Code, IntelliJ, etc.) for comprehensive remote development capabilities.
