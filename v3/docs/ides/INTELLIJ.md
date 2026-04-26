# IntelliJ IDEA - Remote Development Setup

JetBrains IntelliJ IDEA provides powerful remote development capabilities through JetBrains Gateway and the Dev Containers plugin, enabling seamless work with Sindri environments.

## Prerequisites

- [IntelliJ IDEA](https://www.jetbrains.com/idea/) (2023.3 or later recommended)
- [JetBrains Gateway](https://www.jetbrains.com/remote-development/gateway/) (standalone or built-in)
- [Dev Containers plugin](https://plugins.jetbrains.com/plugin/21962-dev-containers) (for container support)
- Docker CLI installed locally (for remote Docker connections)
- Java 17 or later on remote servers

## Connection Methods

### Method 1: Dev Containers (DevPod)

IntelliJ IDEA supports the devcontainer.json specification through the Dev Containers plugin.

**Setup Steps:**

1. **Install the Dev Containers plugin:**
   - Settings > Plugins > Marketplace
   - Search for "Dev Containers"
   - Install and restart

2. **Create a Dev Container:**
   - Welcome Screen > Remote Development
   - Select "Dev Containers"
   - Click "Create Dev Container"
   - Choose your Sindri project directory

3. **Configure devcontainer.json:**

   ```json
   {
     "name": "Sindri Development",
     "image": "your-sindri-image:latest",
     "customizations": {
       "jetbrains": {
         "backend": "IntelliJ",
         "plugins": ["com.intellij.java", "org.jetbrains.plugins.yaml"]
       }
     },
     "forwardPorts": [3000, 8080],
     "remoteUser": "developer"
   }
   ```

4. **Connect to the container:**
   - IntelliJ reads the configuration
   - Builds the container automatically
   - Connects as a remote development environment

**Best Practices:**

- Use the Dev Containers plugin for seamless devcontainer.json support
- Ensure Docker is installed on both local and remote machines
- Configure JVM options for remote IDE backend performance
- Use persistent volumes for workspace and IDE settings

### Method 2: Remote SSH (Fly.io VM)

Connect to remote VMs using JetBrains Gateway with SSH.

**Setup Steps:**

1. **Configure SSH access:**

   ```bash
   # Add to ~/.ssh/config
   Host sindri-flyio
     HostName your-app.fly.dev
     User developer
     Port 22
     IdentityFile ~/.ssh/your_key
     ServerAliveInterval 60
     ServerAliveCountMax 3
   ```

2. **Connect via JetBrains Gateway:**
   - Launch JetBrains Gateway
   - Select "SSH Connection"
   - Click "New Connection"
   - Enter connection details:
     - Host: `your-app.fly.dev`
     - User: `developer`
     - Port: `22`
   - Choose authentication method (SSH key recommended)

3. **Select IDE and project:**
   - Gateway downloads and starts IntelliJ backend on remote server
   - Choose the IDE version to use
   - Select project directory: `/alt/home/developer/workspace`

**System Requirements for Remote Server:**

- Linux AMD64 distribution (Ubuntu 16.04+, RHEL/CentOS 7+)
- 2+ CPU cores
- 4GB+ RAM
- 5GB+ disk space
- OpenSSH server 7.9p1 or later
- Java 17 or later

**Network Requirements:**

- Minimum 20 Mbps bandwidth
- Maximum 200ms latency
- SSH port forwarding enabled

**Best Practices:**

- Use SSH key-based authentication (not password)
- Run an SSH agent for seamless key management
- Ensure correct permissions on SSH key files (chmod 600)
- Configure SSH keepalive to maintain connection stability
- Close unused remote connections to free server resources

### Method 3: Docker (Local Development)

Connect to local Docker containers for development.

**Setup Steps:**

1. **Install Docker Desktop or Docker CLI:**
   - Ensure Docker daemon is running
   - Verify with: `docker ps`

2. **Connect via Dev Containers plugin:**
   - Install Dev Containers plugin (see Method 1)
   - Use "Attach to Running Container" if container is already running
   - Or create new container from devcontainer.json

3. **Alternative: Docker plugin for management:**
   - Install Docker plugin from marketplace
   - Open Docker tool window
   - View and manage containers
   - Connect to containers for file browsing

**Best Practices:**

- Use Docker CLI (minimum requirement) or Docker Desktop
- Configure Docker socket access for plugin integration
- Use multi-stage builds for optimized images
- Mount source code as volumes for live development

## Advanced Configuration

### Remote Docker via SSH

Connect to Docker running on a remote machine:

1. **Configure SSH tunnel to remote Docker:**

   ```bash
   ssh -NfL localhost:2375:/var/run/docker.sock user@remote-host
   ```

2. **Set Docker host environment variable:**

   ```bash
   export DOCKER_HOST=tcp://localhost:2375
   ```

3. **Use Dev Containers plugin with remote Docker:**
   - Plugin will use DOCKER_HOST environment variable
   - Create or attach to containers on remote Docker daemon

### SSH Authentication Best Practices

**Key Generation:**

```bash
# Generate SSH key pair (if needed)
ssh-keygen -t ed25519 -C "your_email@example.com"

# Copy public key to remote server
ssh-copy-id -i ~/.ssh/id_ed25519.pub developer@your-app.fly.dev
```

**SSH Agent Setup:**

```bash
# Start SSH agent
eval "$(ssh-agent -s)"

# Add key to agent
ssh-add ~/.ssh/id_ed25519
```

**Common Authentication Issues:**

- **Incorrect key permissions:** Run `chmod 600 ~/.ssh/id_ed25519`
- **Key not in agent:** Run `ssh-add ~/.ssh/id_ed25519`
- **Public key not on server:** Verify `~/.ssh/authorized_keys` on remote

### Performance Tuning

**Increase JVM memory for IDE backend:**

```bash
# Add to remote server's ~/.bashrc or IDE settings
export IDEA_VM_OPTIONS="-Xmx4g -Xms2g"
```

**Optimize indexing:**

- Exclude build directories from indexing
- Use "Power Save Mode" for resource-constrained environments
- Configure IDE heap size based on project size

## Troubleshooting

### Connection Issues

- **Connection timeout:** Check network latency and bandwidth requirements
- **SSH key rejected:** Verify key permissions and authorized_keys on server
- **Port forwarding fails:** Ensure SSH config allows port forwarding
- **Gateway can't start backend:** Verify Java 17+ is installed on remote server

### Dev Container Issues

- **Container build fails:** Check Docker daemon is running and accessible
- **Plugin not detecting devcontainer.json:** Ensure file is in .devcontainer/ or project root
- **Extensions not loading:** Add JetBrains-specific plugin IDs to customizations

### Performance Issues

- **High latency UI:** Check network connection meets minimum requirements (20 Mbps, <200ms latency)
- **Slow indexing:** Exclude unnecessary directories, increase IDE heap size
- **Connection drops:** Configure SSH keepalive settings

## Resources

- [Remote Development Overview - JetBrains](https://www.jetbrains.com/help/idea/remote-development-overview.html)
- [JetBrains Gateway Documentation](https://www.jetbrains.com/help/idea/remote-development-a.html)
- [Dev Containers in IntelliJ IDEA](https://www.jetbrains.com/help/idea/start-dev-container-for-a-remote-project.html)
- [System Requirements for Remote Development](https://www.jetbrains.com/help/idea/prerequisites.html)
- [FAQ about Dev Containers](https://www.jetbrains.com/help/idea/faq-about-dev-containers.html)
- [Dev Containers Setup Guide 2025 - BytePlus](https://www.byteplus.com/en/topic/504503)

## Next Steps

After connecting to your Sindri environment:

1. Configure code style and inspections
2. Set up run/debug configurations
3. Install additional plugins as needed
4. Configure version control integration
5. Set up database tools if using databases

---

**Related Documentation:**

- [VS Code Setup](VSCODE.md)
- [Zed Editor Setup](ZED.md)
- [Eclipse Setup](ECLIPSE.md)
- [Warp Terminal Setup](WARP.md)
