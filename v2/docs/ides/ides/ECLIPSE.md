# Eclipse IDE - Remote Development Setup

Eclipse IDE offers remote development capabilities through the Remote System Explorer (RSE) plugin and Docker integration, enabling connection to Sindri environments for Java and other language development.

## Prerequisites

- [Eclipse IDE](https://www.eclipse.org/downloads/) (2023-12 or later recommended)
- Install the following components:
  - **Remote System Explorer (RSE)** - For SSH connections
  - **Docker Tools** - For Docker container integration
  - **CDT (C/C++ Development Tools)** - For enhanced Docker container support (optional)
- Java 17 or later

## Connection Methods

### Method 1: Remote System Explorer (SSH to Fly.io VM)

RSE provides SSH-based remote file access, terminal access, and remote debugging capabilities.

**Setup Steps:**

1. **Install Remote System Explorer:**
   - Help > Eclipse Marketplace
   - Search for "Remote System Explorer"
   - Install and restart Eclipse

2. **Configure SSH connection:**
   - Window > Perspective > Open Perspective > Remote System Explorer
   - In "Remote Systems" view, right-click > New > Connection
   - Select "SSH Only"
   - Enter connection details:
     - Host name: `your-app.fly.dev`
     - Connection name: `Sindri Fly.io`
     - Description: `Sindri development environment`

3. **Configure authentication:**
   - Expand connection in Remote Systems view
   - Right-click "Sftp Files" > Properties
   - Configure SSH key or password authentication
   - Recommended: Use SSH key-based authentication

4. **Connect and browse files:**
   - Right-click connection > Connect
   - Navigate to `/alt/home/developer/workspace`
   - Double-click files to edit remotely

**Best Practices:**

- Use SSH key-based authentication for security
- Configure SSH agent for seamless key management
- Create filters to show only relevant directories
- Use "Show in Remote Shell" for terminal access

### Method 2: Docker Integration (Local Containers)

Eclipse's Docker Tools plugin provides container management and development capabilities.

**Setup Steps:**

1. **Install Docker Tools plugin:**
   - Help > Eclipse Marketplace
   - Search for "Docker Tooling"
   - Install and restart Eclipse

2. **Configure Docker connection:**
   - Window > Show View > Other > Docker > Docker Explorer
   - Click "Connect to Docker daemon"
   - Select connection type:
     - **Unix socket:** `unix:///var/run/docker.sock` (macOS/Linux)
     - **TCP:** `tcp://localhost:2375` (if Docker daemon exposed)

3. **View and manage containers:**
   - Docker Explorer shows images, containers, networks, volumes
   - Right-click container > Start/Stop/Remove
   - Right-click running container > Display Log

4. **Develop in container:**
   - Create container with Sindri image
   - Mount workspace as volume:
     ```bash
     docker run -d -v /path/to/workspace:/workspace \
       --name sindri-dev your-sindri-image:latest
     ```
   - Use RSE to connect to container via exposed SSH port

**Best Practices:**

- Use docker-compose.yml for multi-service applications
- Configure volume mounts for persistent data
- Use .dockerignore to exclude build artifacts
- Monitor container resource usage

### Method 3: Remote Docker Host

Connect to Docker running on a remote machine for container development.

**Setup Steps:**

1. **Set up Docker context on remote host:**

   ```bash
   # On your local machine
   docker context create remote-sindri \
     --docker "host=ssh://user@remote-host"
   docker context use remote-sindri
   ```

2. **Configure Eclipse Docker Tools:**
   - Open Docker Explorer
   - Add new connection
   - Connection name: `Remote Sindri Docker`
   - TCP Connection: Point to SSH-tunneled Docker socket

3. **SSH tunnel to remote Docker (alternative):**

   ```bash
   ssh -NfL localhost:2375:/var/run/docker.sock user@remote-host
   ```

4. **Connect Eclipse to tunneled Docker:**
   - Docker Explorer > New Connection
   - TCP Connection: `tcp://localhost:2375`

**Best Practices:**

- Use SSH tunneling for secure remote Docker access
- Don't expose Docker daemon over plain TCP (security risk)
- Use Docker contexts for managing multiple remote hosts
- Implement proper authentication and authorization

## Advanced Configuration

### SSH Configuration for RSE

**Configure SSH key authentication:**

1. **Generate SSH key (if needed):**

   ```bash
   ssh-keygen -t ed25519 -C "your_email@example.com"
   ```

2. **Copy public key to remote server:**

   ```bash
   ssh-copy-id -i ~/.ssh/id_ed25519.pub developer@your-app.fly.dev
   ```

3. **Configure RSE to use key:**
   - Right-click SSH connection > Properties
   - SSH Settings > Authentication
   - Select "Public Key" authentication
   - Browse to private key file: `~/.ssh/id_ed25519`

### Remote Debugging

**Debug Java applications running in containers:**

1. **Start application with debug port exposed:**

   ```bash
   docker run -p 8000:8000 -e JAVA_TOOL_OPTIONS="-agentlib:jdwp=transport=dt_socket,server=y,suspend=n,address=*:8000" \
     your-sindri-image:latest
   ```

2. **Configure debug configuration in Eclipse:**
   - Run > Debug Configurations
   - Create new "Remote Java Application"
   - Set connection properties:
     - Host: `localhost` (or remote host)
     - Port: `8000`

3. **Start debugging:**
   - Click "Debug"
   - Set breakpoints in your code
   - Trigger application logic

### Docker Container with Development Tools

**Create enhanced container for Eclipse development:**

```dockerfile
FROM your-sindri-image:latest

# Install Java and development tools
RUN apt-get update && apt-get install -y \
    openjdk-17-jdk \
    maven \
    gradle

# Configure SSH for RSE access
RUN apt-get install -y openssh-server
RUN mkdir /var/run/sshd
RUN echo 'developer:password' | chpasswd

EXPOSE 22 8000

CMD ["/usr/sbin/sshd", "-D"]
```

## Eclipse-Specific Features

### Project Synchronization

**Synchronize local and remote projects:**

1. Create local project in Eclipse
2. Use RSE to upload/download files to/from remote
3. Right-click project > Synchronize > Remote System

### Terminal Access

**Open terminal to remote system:**

1. Right-click connection in Remote Systems view
2. Select "Launch Terminal"
3. Choose terminal type (SSH)
4. Execute commands on remote system

### File Transfer

**Efficient file transfer with RSE:**

- Drag and drop files between local and remote
- Right-click files > Export/Import for batch operations
- Configure filters to exclude unnecessary files

## Troubleshooting

### SSH Connection Issues

- **"Connection refused":** Verify SSH server is running on remote host
- **"Permission denied":** Check SSH key permissions (chmod 600)
- **"Host key verification failed":** Remove old host key from ~/.ssh/known_hosts
- **Timeout errors:** Check firewall rules and network connectivity

### Docker Integration Issues

- **"Cannot connect to Docker daemon":** Ensure Docker is running
- **"Permission denied" on socket:** Add user to docker group (Linux)
- **Container not starting:** Check Docker logs with `docker logs container_name`
- **Volume mount issues:** Verify paths and permissions

### Performance Issues

- **Slow file operations:** Check network latency and bandwidth
- **High memory usage:** Increase Eclipse heap size in eclipse.ini
- **Indexing problems:** Exclude large directories from indexing

## Security Best Practices

### SSH Security

- Never store passwords in Eclipse - use SSH keys
- Use strong passphrases for SSH keys
- Keep private keys secure (chmod 600)
- Rotate SSH keys periodically

### Docker Security

- Don't expose Docker daemon over unencrypted TCP
- Use SSH tunneling for remote Docker access
- Don't store secrets in Docker images
- Follow principle of least privilege for container access
- Implement audit logging for container access

## Resources

- [Eclipse Remote System Explorer](https://www.eclipse.org/community/eclipse_newsletter/2017/april/article1.php)
- [Docker Tools for Eclipse](https://www.eclipse.org/community/eclipse_newsletter/2015/june/article3.php)
- [Advanced Remote Development - Eclipse](https://www.swiftorial.com/tutorials/development_tools/eclipse/remote_development/advanced_remote_development)
- [Docker Best Practices 2025 - ThinkSys](https://thinksys.com/devops/docker-best-practices/)
- [Remote Debugging with Eclipse](https://medium.com/@ravi.ajmera/connecting-eclipse-to-docker-container-for-remote-debugging-d459b5d53249)

## Limitations

- No native devcontainer.json support
- Remote development features less mature than VS Code/IntelliJ
- Limited container-aware language server support
- Manual setup required for most remote workflows

## Next Steps

After connecting to your Sindri environment:

1. Import existing projects or create new ones
2. Configure build tools (Maven, Gradle)
3. Set up run and debug configurations
4. Install additional plugins as needed
5. Configure code formatters and style checkers

---

**Related Documentation:**

- [VS Code Setup](VSCODE.md) - For modern dev container support
- [IntelliJ IDEA Setup](INTELLIJ.md) - For JetBrains remote development
- [Zed Editor Setup](ZED.md)
- [Warp Terminal Setup](WARP.md)
