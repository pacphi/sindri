# Zed Editor - Remote Development Setup

Zed is a modern, high-performance code editor with built-in SSH remote development capabilities. While dev container support is still evolving, Zed offers excellent performance and seamless remote editing over SSH.

## Prerequisites

- [Zed Editor](https://zed.dev/) v0.159 or later
- SSH access configured to remote servers
- Basic understanding of SSH key-based authentication

## Connection Methods

### Method 1: SSH Remote Development (Fly.io VM)

Zed's native remote development runs the UI locally at 120 FPS while language servers, tasks, and terminals run on the remote server.

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

2. **Connect from Zed:**
   - **Method A - Command Palette:**
     - Open Command Palette (`Cmd/Ctrl+Shift+P`)
     - Type "remote" and select "Projects: Open Remote"
     - Enter SSH connection string: `ssh://developer@your-app.fly.dev:22/alt/home/developer/workspace`

   - **Method B - Terminal Command:**
     ```bash
     zed ssh://developer@your-app.fly.dev:22/alt/home/developer/workspace
     ```

3. **First connection:**
   - Zed downloads and installs headless server on remote machine
   - Connection establishes automatically
   - Your project opens in Zed UI

**Architecture:**

- **Local:** Zed UI running at 120 FPS
- **Remote:** Headless Zed server, language servers, terminals, tasks
- **Communication:** Secure SSH tunnel
- **Resilience:** Language servers keep running if connection drops; Zed reconnects automatically

**Best Practices:**

- Use SSH key-based authentication (required)
- Configure SSH keepalive to maintain connection stability
- Ensure stable network connection for best experience
- Close remote sessions when finished to free server resources

### Method 2: Docker via SSH (Workaround for DevPod/Containers)

Zed does not natively support dev containers yet, but you can use SSH to connect to containers running SSH servers.

**Setup Steps:**

1. **Create Dockerfile with SSH server:**

   ```dockerfile
   FROM your-sindri-image:latest

   # Install OpenSSH server
   RUN apt-get update && apt-get install -y openssh-server

   # Configure SSH
   RUN mkdir /var/run/sshd
   RUN echo 'developer:password' | chpasswd
   RUN sed -i 's/#PermitRootLogin prohibit-password/PermitRootLogin yes/' /etc/ssh/sshd_config

   # SSH login fix for SSH clients
   RUN sed 's@session\s*required\s*pam_loginuid.so@session optional pam_loginuid.so@g' -i /etc/pam.d/sshd

   EXPOSE 22

   CMD ["/usr/sbin/sshd", "-D"]
   ```

2. **Run container with SSH:**

   ```bash
   docker run -d -p 2222:22 --name sindri-dev your-sindri-image:latest
   ```

3. **Configure SSH connection:**

   ```bash
   # Add to ~/.ssh/config
   Host sindri-container
     HostName localhost
     User developer
     Port 2222
   ```

4. **Connect via Zed:**
   ```bash
   zed ssh://developer@localhost:2222/workspace
   ```

**Limitations:**

- Not true dev container support
- Requires manual SSH server setup in container
- Extensions not managed separately for containers
- Workaround until native dev container support arrives

**Best Practices:**

- Use this only as a temporary workaround
- Consider VS Code or IntelliJ for native dev container support
- Monitor [Zed dev containers roadmap](https://github.com/zed-industries/zed/issues/5347) for updates

### Method 3: Local Docker (File Editing Only)

For local Docker containers, you can mount volumes and edit files directly.

**Setup Steps:**

1. **Mount project directory as volume:**

   ```bash
   docker run -v $(pwd):/workspace your-sindri-image:latest
   ```

2. **Edit files locally with Zed:**

   ```bash
   zed /path/to/mounted/project
   ```

3. **Execute commands in container:**
   ```bash
   docker exec -it container_name bash
   ```

**Limitations:**

- No integrated terminal in container
- No language server running in container context
- Manual command execution required
- Not a true remote development experience

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

### SSH Key Setup

**Generate SSH key:**

```bash
ssh-keygen -t ed25519 -C "your_email@example.com"
```

**Copy to remote server:**

```bash
ssh-copy-id -i ~/.ssh/id_ed25519.pub developer@your-app.fly.dev
```

**Add to SSH agent:**

```bash
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_ed25519
```

## Remote Development Features

### What Works Over SSH

- **Language servers:** Run on remote server, full IntelliSense
- **Terminals:** Execute commands on remote server
- **Tasks:** Build and run tasks on remote server
- **File operations:** Full read/write access to remote files
- **Auto-reconnection:** Seamless reconnection after network interruptions

### Current Limitations

- **No native dev container support** (as of v0.159)
- **Extensions not container-aware** (when using SSH-to-container workaround)
- **No devcontainer.json parsing** (use SSH configuration instead)

## Troubleshooting

### Connection Issues

- **"Connection refused":** Verify SSH server is running on remote host
- **"Permission denied":** Check SSH key is added to authorized_keys on remote
- **"Headless server install fails":** Ensure write permissions in remote home directory
- **Connection drops frequently:** Configure SSH keepalive settings

### Performance Issues

- **Laggy UI:** Check network latency (Zed performs best with <100ms latency)
- **Slow file operations:** Verify network bandwidth is sufficient
- **High CPU on remote:** Check language server processes, restart if needed

### SSH Container Workaround Issues

- **SSH server won't start:** Check container logs with `docker logs container_name`
- **Can't connect to container:** Verify port mapping with `docker port container_name`
- **Authentication fails:** Check user credentials in Dockerfile

## Resources

- [Zed Remote Development Documentation](https://zed.dev/docs/remote-development)
- [SSH Remoting Launch Blog Post](https://zed.dev/blog/remote-development)
- [Dev Containers in Zed (Experimental)](https://zed.dev/docs/dev-containers)
- [Zed with devcontainer SSH Workaround](https://www.tay-tec.de/en/blog/zed-devcontainer-ssh/index.html)
- [Remote Development Architecture](https://deepwiki.com/zed-industries/zed/5.1-remote-development-and-ssh)

## Future Development

Zed is actively working on native dev container support. Track progress:

- [Dev Containers Support Issue](https://github.com/zed-industries/zed/issues/5347)
- [Remote Development in Docker Discussion](https://github.com/zed-industries/zed/discussions/15787)

## Next Steps

After connecting to your Sindri environment:

1. Configure language servers for your project
2. Set up tasks for building and testing
3. Customize keybindings for remote workflows
4. Explore Zed's collaboration features
5. Monitor Zed's dev container roadmap for updates

---

**Related Documentation:**

- [VS Code Setup](VSCODE.md) - For full dev container support
- [IntelliJ IDEA Setup](INTELLIJ.md) - For JetBrains dev container support
- [Eclipse Setup](ECLIPSE.md)
- [Warp Terminal Setup](WARP.md)
