# Visual Studio Code - Remote Development Setup

Visual Studio Code offers industry-leading remote development capabilities through extensions, making it an excellent choice for connecting to Sindri environments.

## Prerequisites

- [Visual Studio Code](https://code.visualstudio.com/) (latest stable version)
- Install the following extensions:
  - [Remote - SSH](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-ssh)
  - [Dev Containers](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
  - [Docker](https://marketplace.visualstudio.com/items?itemName=ms-azuretools.vscode-docker) (optional, for Docker management)

## Connection Methods

### Method 1: Dev Containers (DevPod)

Dev Containers provide the best developer experience for containerized environments with full IDE integration.

**Setup Steps:**

1. **Install the Dev Containers extension** from the VS Code marketplace
2. **Open your project in a container:**
   - Open Command Palette (`Cmd/Ctrl+Shift+P`)
   - Select "Dev Containers: Open Folder in Container"
   - Choose your Sindri project directory
3. **Configure devcontainer.json** (if not already present):
   ```json
   {
     "name": "Sindri Development",
     "image": "your-sindri-image:latest",
     "customizations": {
       "vscode": {
         "extensions": ["dbaeumer.vscode-eslint", "esbenp.prettier-vscode"]
       }
     },
     "forwardPorts": [3000, 8080],
     "remoteUser": "developer"
   }
   ```

**Best Practices:**

- Use [Features](https://code.visualstudio.com/docs/devcontainers/containers#_dev-container-features) to modularize tool installation
- Leverage volume mounts for persistent user data
- Configure port forwarding for all services you'll access
- Use pre-built images to reduce startup time

### Method 2: Remote SSH (Fly.io VM)

Connect to remote VMs running on Fly.io using SSH.

**Setup Steps:**

1. **Configure SSH access** to your Fly.io VM:

   ```bash
   # Add to ~/.ssh/config
   Host sindri-flyio
     HostName your-app.fly.dev
     User developer
     Port 22
     IdentityFile ~/.ssh/your_key
   ```

2. **Connect via VS Code:**
   - Open Command Palette (`Cmd/Ctrl+Shift+P`)
   - Select "Remote-SSH: Connect to Host"
   - Choose "sindri-flyio" from the list
   - VS Code will install the VS Code Server on the remote machine

3. **Open your workspace:**
   - File > Open Folder
   - Navigate to `/alt/home/developer/workspace`

**Best Practices:**

- Enable SSH agent for key-based authentication
- Use SSH ControlMaster for faster reconnection
- Configure bandwidth and latency-appropriate settings
- Set up automatic reconnection in case of network drops

### Method 3: Docker (Local Development)

Connect directly to local Docker containers for development.

**Setup Steps:**

1. **Using Docker Desktop + Dev Containers:**
   - Ensure Docker Desktop is running
   - Follow the Dev Containers method above

2. **Using Remote - Containers for existing containers:**
   - Open Command Palette
   - Select "Dev Containers: Attach to Running Container"
   - Choose your Sindri container from the list

**Best Practices:**

- Use docker-compose.yml for multi-service applications
- Mount your source code as a volume for live editing
- Configure resource limits appropriate to your machine
- Use .dockerignore to exclude unnecessary files

## Advanced Configuration

### Remote Development on Remote Docker Host

For connecting to Docker running on a remote machine:

1. **Set up Docker context:**

   ```bash
   docker context create remote-sindri --docker "host=ssh://user@remote-host"
   docker context use remote-sindri
   ```

2. **Configure VS Code settings:**
   ```json
   {
     "dev.containers.dockerPath": "docker",
     "dev.containers.dockerComposePath": "docker-compose"
   }
   ```

### Custom Instructions for AI Copilot

Enhance GitHub Copilot's context awareness in dev containers:

```json
{
  "github.copilot.advanced": {
    "debug.overrideEngine": "gpt-4",
    "devcontainer.environment": {
      "description": "Sindri development environment with mise and Claude Code"
    }
  }
}
```

### Volume Management

Persist user profile and settings across container rebuilds:

```json
{
  "mounts": [
    "source=vscode-server,target=/home/developer/.vscode-server,type=volume",
    "source=sindri-home,target=/alt/home/developer,type=volume"
  ]
}
```

## Troubleshooting

### Connection Issues

- **Server installation fails:** Check SSH permissions and ensure you have write access to `~/.vscode-server`
- **Port forwarding not working:** Verify firewall rules and ensure ports are exposed in your container/VM
- **Extensions not loading:** Reinstall extensions in the remote environment using the Extensions panel

### Performance Optimization

- **Slow file watching:** Add workspace to watcherExclude settings
- **High CPU usage:** Disable unnecessary extensions in remote environment
- **Slow initial connection:** Use pre-built images with VS Code Server pre-installed

## Resources

- [Developing inside a Container - VS Code Docs](https://code.visualstudio.com/docs/devcontainers/containers)
- [Dev Containers Tips and Tricks](https://code.visualstudio.com/docs/devcontainers/tips-and-tricks)
- [Remote Development using SSH](https://code.visualstudio.com/docs/remote/ssh)
- [Develop on a remote Docker host](https://code.visualstudio.com/remote/advancedcontainers/develop-remote-host)
- [Ultimate Guide to Dev Containers - Daytona](https://www.daytona.io/dotfiles/ultimate-guide-to-dev-containers)
- [Mastering Dev Containers in VS Code - Rost Glukhov](https://www.glukhov.org/post/2025/10/vs-code-dev-containers/)

## Next Steps

After connecting to your Sindri environment:

1. Install project-specific extensions
2. Configure integrated terminal settings
3. Set up debugging configurations
4. Explore workspace trust settings for security

---

**Related Documentation:**

- [IntelliJ IDEA Setup](INTELLIJ.md)
- [Zed Editor Setup](ZED.md)
- [Eclipse Setup](ECLIPSE.md)
- [Warp Terminal Setup](WARP.md)
