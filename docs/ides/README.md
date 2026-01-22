# IDE Integration Guide

Connect to your Sindri development environment using your favorite IDE or editor. Whether you prefer a full-featured IDE, a modern lightweight editor, or a powerful terminal, we've got you covered.

## Quick Start

Sindri supports three primary connection methods:

- **Dev Containers (DevPod):** Full IDE integration with containerized environments
- **Remote SSH (Fly.io VM):** Connect to cloud-hosted development VMs
- **Docker (Local):** Develop in local Docker containers

Choose your IDE below to get started with step-by-step setup instructions.

## Supported IDEs and Editors

### Visual Studio Code

**Best for:** Modern web development, cross-platform teams, devcontainer workflows

VS Code offers industry-leading remote development with the Remote-SSH and Dev Containers extensions, making it the most mature option for all Sindri connection methods.

**Key Features:**

- Native devcontainer.json support
- Seamless SSH remote development
- Extensive extension ecosystem
- Free and open-source

[**→ VS Code Setup Guide**](VSCODE.md)

---

### JetBrains IntelliJ IDEA

**Best for:** Java/Kotlin development, JVM ecosystems, enterprise projects

IntelliJ IDEA provides powerful remote development through JetBrains Gateway with excellent dev container support via the Dev Containers plugin.

**Key Features:**

- JetBrains Gateway for SSH connections
- Dev Containers plugin for devcontainer.json
- Advanced Java/JVM tooling
- Professional refactoring capabilities

[**→ IntelliJ IDEA Setup Guide**](INTELLIJ.md)

---

### Zed Editor

**Best for:** Performance-focused developers, minimalists, SSH-based workflows

Zed is a modern, high-performance editor with native SSH remote development running at 120 FPS. Dev container support is in development.

**Key Features:**

- Ultra-fast SSH remote development
- Native Rust performance
- Auto-reconnection on network drops
- Modern collaborative features

[**→ Zed Editor Setup Guide**](ZED.md)

---

### Eclipse IDE

**Best for:** Java/JEE development, legacy projects, educational settings

Eclipse offers remote development through Remote System Explorer (RSE) and Docker Tools plugin, suitable for traditional enterprise Java development.

**Key Features:**

- Remote System Explorer for SSH
- Docker Tools for container management
- Mature plugin ecosystem
- Strong Java/JEE support

[**→ Eclipse Setup Guide**](ECLIPSE.md)

---

### Warp Terminal

**Best for:** Terminal-focused developers, DevOps workflows, Docker power users

Warp is a modern terminal with AI assistance and Docker integration. While not a full IDE, it excels at terminal-based development workflows.

**Key Features:**

- AI-powered command search
- Docker Desktop integration (macOS)
- Workflows for common operations
- Modern terminal features (blocks, sharing)

[**→ Warp Terminal Setup Guide**](WARP.md)

---

## Feature Comparison

| Feature                | VS Code         | IntelliJ             | Zed               | Eclipse         | Warp                  |
| ---------------------- | --------------- | -------------------- | ----------------- | --------------- | --------------------- |
| **Dev Containers**     | ✅ Native       | ✅ Plugin            | 🚧 In Development | ❌ Manual Setup | ❌ No                 |
| **Remote SSH**         | ✅ Excellent    | ✅ Gateway           | ✅ Native         | ✅ RSE Plugin   | ✅ Standard SSH       |
| **Docker Integration** | ✅ Excellent    | ✅ Good              | ⚠️ Via SSH        | ✅ Plugin       | ✅ Extension (macOS)  |
| **Language Servers**   | ✅ Full Support | ✅ Full Support      | ✅ Remote         | ✅ Full Support | ❌ N/A                |
| **Free/Open Source**   | ✅ Yes          | ⚠️ Community Edition | ✅ Yes            | ✅ Yes          | ⚠️ Freemium           |
| **Platform Support**   | All             | All                  | All               | All             | macOS, Linux, Windows |
| **Performance**        | Good            | Good                 | Excellent         | Moderate        | Excellent             |
| **Learning Curve**     | Low             | Medium               | Low               | Medium          | Low                   |

**Legend:**

- ✅ Full support
- ⚠️ Limited or partial support
- 🚧 In development
- ❌ Not supported

## Choosing the Right IDE

### For Web Development

**Recommended:** VS Code

VS Code has the best support for modern web frameworks, TypeScript, and JavaScript ecosystems, with excellent dev container integration.

### For Java/JVM Development

**Recommended:** IntelliJ IDEA or Eclipse

Both offer mature Java tooling. IntelliJ provides a more modern experience, while Eclipse is well-suited for traditional enterprise projects.

### For Performance-Critical SSH Workflows

**Recommended:** Zed Editor

Zed's 120 FPS UI and native SSH support make it the fastest option for remote development over SSH.

### For Terminal-Centric Workflows

**Recommended:** Warp + VS Code

Use Warp for terminal operations and Docker management, combined with VS Code for editing and IDE features.

### For Cross-Platform Teams

**Recommended:** VS Code or IntelliJ IDEA

Both work consistently across macOS, Windows, and Linux, with strong remote development support on all platforms.

## Connection Method Details

### Dev Containers (DevPod)

Dev containers provide the most integrated experience, where your IDE runs locally but all tools, dependencies, and execution happen inside a Docker container.

**Supported IDEs:**

- ✅ VS Code (Native)
- ✅ IntelliJ IDEA (Dev Containers Plugin)
- 🚧 Zed (In Development)
- ⚠️ Eclipse (Manual Setup)
- ❌ Warp (Not Applicable)

**When to use:**

- You want reproducible development environments
- Your team uses the devcontainer.json standard
- You're developing containerized applications
- You need isolation between projects

### Remote SSH (Fly.io VM)

SSH remote development connects your local IDE to a remote server running on Fly.io, with all processing happening on the remote machine.

**Supported IDEs:**

- ✅ VS Code (Remote-SSH Extension)
- ✅ IntelliJ IDEA (JetBrains Gateway)
- ✅ Zed (Native SSH Support)
- ✅ Eclipse (Remote System Explorer)
- ✅ Warp (Standard SSH)

**When to use:**

- You need more computing power than your local machine
- Your team shares cloud-based development environments
- You want to develop from multiple devices
- You're working with large datasets or intensive workloads

### Docker (Local)

Local Docker development runs containers on your machine, with your IDE connecting to or managing those containers.

**Supported IDEs:**

- ✅ VS Code (Dev Containers + Docker Extension)
- ✅ IntelliJ IDEA (Docker Plugin + Dev Containers)
- ⚠️ Zed (SSH to Container Workaround)
- ✅ Eclipse (Docker Tools Plugin)
- ✅ Warp (Docker Extension - macOS only)

**When to use:**

- You're developing locally with Docker Desktop
- You need quick iteration cycles
- You want offline development capability
- You're testing containerized applications

## Prerequisites

### All Methods

- Git installed and configured
- SSH key-based authentication set up
- Basic understanding of your chosen IDE

### Dev Containers

- Docker Desktop or Docker CLI
- Dev container support in your IDE (see guides)

### Remote SSH

- SSH client configured
- Access to Fly.io VM or remote server
- Stable internet connection (20+ Mbps, <200ms latency recommended)

### Docker Local

- Docker Desktop or Docker daemon running
- Sufficient local system resources (varies by IDE)

## Getting Started

1. **Choose your IDE** from the list above
2. **Review the feature comparison** to ensure it meets your needs
3. **Follow the setup guide** for your chosen IDE
4. **Configure your connection** (Dev Container, SSH, or Docker)
5. **Start developing** in your Sindri environment

## Common Issues

### Can't Connect to Remote Server

- Verify SSH configuration in `~/.ssh/config`
- Check SSH key permissions (`chmod 600 ~/.ssh/id_ed25519`)
- Ensure remote server is running and accessible

### Dev Container Won't Start

- Verify Docker is running: `docker ps`
- Check devcontainer.json syntax
- Review Docker logs: `docker logs container_name`

### Performance Issues

- Check network latency and bandwidth
- Increase IDE heap size if needed
- Exclude large directories from indexing
- Use pre-built images to reduce build time

### Extension/Plugin Issues

- Reinstall extensions in remote environment
- Check IDE version compatibility
- Review extension logs for errors

## Additional Resources

- [Sindri Architecture](../../v2/docs/ARCHITECTURE.md)
- [Sindri Deployment Guide](../../v2/docs/DEPLOYMENT.md)
- [Sindri Configuration](../../v2/docs/CONFIGURATION.md)
- [Troubleshooting Guide](../../v2/docs/TROUBLESHOOTING.md)

## Contributing

Found an issue with these guides or want to add support for another IDE? Contributions are welcome:

1. Open an issue describing the problem or feature request
2. Submit a pull request with your improvements
3. Share your IDE setup tips with the community

## Need Help?

If you're having trouble connecting your IDE to Sindri:

1. Check the [Troubleshooting Guide](../../v2/docs/TROUBLESHOOTING.md)
2. Review your IDE's specific setup guide above
3. Check IDE-specific logs and error messages
4. Open an issue with details about your setup and error

---

**Happy coding with Sindri!**
