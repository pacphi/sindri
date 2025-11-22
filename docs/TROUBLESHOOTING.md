# Troubleshooting Guide

Common issues and solutions for Sindri.

## Quick Diagnostics

```bash
# Check system status
./cli/sindri status

# View logs
./cli/sindri logs

# Validate configuration
./cli/sindri config validate

# Check extension status
extension-manager status <extension-name>
```

## Installation Issues

### yq not found

**Error:**

```text
Error: yq command not found
```

**Solution:**

```bash
# macOS
brew install yq

# Ubuntu/Debian
sudo apt install yq

# Manual install
sudo wget -qO /usr/local/bin/yq https://github.com/mikefarah/yq/releases/latest/download/yq_linux_amd64
sudo chmod +x /usr/local/bin/yq
```

### Docker not running

**Error:**

```text
Cannot connect to Docker daemon
```

**Solution:**

```bash
# Check Docker status
docker info

# Start Docker (macOS)
open /Applications/Docker.app

# Start Docker (Linux)
sudo systemctl start docker
```

### Permission denied (Docker)

**Error:**

```text
permission denied while trying to connect to the Docker daemon socket
```

**Solution:**

```bash
# Add user to docker group
sudo usermod -aG docker $USER

# Logout and login again, or:
newgrp docker

# Verify
docker ps
```

## Extension Issues

### Extension install fails

**Error:**

```text
Error installing extension: nodejs
```

**Diagnosis:**

```bash
# Check extension definition
./cli/extension-manager validate nodejs

# Check logs
cat /workspace/.system/logs/nodejs-install.log

# Check disk space
df -h /workspace
```

**Common causes:**

1. **Insufficient disk space:**

   ```bash
   # Check available space
   df -h /workspace

   # Clean up
   docker system prune -a
   ```

2. **Missing dependencies:**

   ```bash
   # Check dependencies
   ./cli/extension-manager show nodejs

   # Install dependencies first
   extension-manager install workspace-structure
   ```

3. **Network issues:**

   ```bash
   # Test DNS
   ping registry.npmjs.org

   # Check proxy settings
   echo $HTTP_PROXY
   ```

### Command not found after install

**Error:**

```text
node: command not found
```

**Solution:**

```bash
# Source bashrc to reload PATH
source ~/.bashrc

# Or logout and login again
exit
ssh developer@...
```

### Extension validation fails

**Error:**

```text
Validation failed for extension: nodejs
```

**Diagnosis:**

```bash
# Run validation manually
extension-manager validate nodejs

# Check if tools are in PATH
which node
which npm

# Check mise installations
mise list
mise doctor
```

**Solution:**

```bash
# Reinstall extension
extension-manager remove nodejs
extension-manager install nodejs

# Verify mise configuration
cat ~/.config/mise/conf.d/nodejs.toml
```

## Deployment Issues

### Docker deployment fails

**Error:**

```text
Error deploying to Docker
```

**Diagnosis:**

```bash
# Validate sindri.yaml
./cli/sindri config validate

# Check Docker
docker ps
docker images

# Check docker-compose
docker-compose config
```

**Solution:**

```bash
# Rebuild image
pnpm build

# Clean up old containers
docker-compose down -v

# Redeploy
./cli/sindri deploy --provider docker
```

### Fly.io deployment fails

**Error:**

```text
Error: failed to create app
```

**Diagnosis:**

```bash
# Check flyctl authentication
flyctl auth whoami

# Check app status
flyctl status -a my-app

# View logs
flyctl logs -a my-app
```

**Common solutions:**

1. **Not authenticated:**

   ```bash
   flyctl auth login
   ```

2. **App name taken:**

   ```bash
   # Use different name in sindri.yaml
   name: my-unique-app-name
   ```

3. **Region not available:**

   ```bash
   # List available regions
   flyctl platform regions

   # Update sindri.yaml
   providers:
     fly:
       region: sjc  # Use available region
   ```

### Kubernetes deployment fails

**Error:**

```text
Error: namespace not found
```

**Solution:**

```bash
# Create namespace
kubectl create namespace dev-envs

# Or update sindri.yaml
providers:
  kubernetes:
    namespace: default
```

## Connection Issues

### SSH connection refused (Fly.io)

**Error:**

```text
ssh: connect to host my-app.fly.dev port 10022: Connection refused
```

**Diagnosis:**

```bash
# Check machine status
flyctl status -a my-app

# Check if suspended
flyctl machine list -a my-app
```

**Solution:**

```bash
# Start machine
flyctl machine start <machine-id> -a my-app

# Or wait for auto-start (if enabled)
# Try connecting again after 10-20 seconds
```

### SSH authentication fails

**Error:**

```text
Permission denied (publickey)
```

**Solution:**

```bash
# Check SSH key exists
ls -la ~/.ssh/id_*.pub

# Generate SSH key if needed
ssh-keygen -t ed25519 -C "your@email.com"

# Add key to ssh-agent
ssh-add ~/.ssh/id_ed25519

# Upload key to Fly.io
flyctl ssh issue --agent -a my-app
```

### Cannot connect to Docker container

**Error:**

```text
Error: No such container
```

**Solution:**

```bash
# List containers
docker ps -a

# Check container name
docker-compose ps

# Start container
docker-compose start

# Or recreate
docker-compose up -d
```

## Volume Issues

### Volume mount fails

**Error:**

```text
Error: failed to create volume
```

**Solution (Docker):**

```bash
# Remove old volume
docker volume rm sindri-workspace

# Recreate
docker volume create sindri-workspace

# Redeploy
./cli/sindri deploy
```

**Solution (Fly.io):**

```bash
# List volumes
flyctl volumes list -a my-app

# Delete volume
flyctl volumes delete <volume-id>

# Redeploy (creates new volume)
./cli/sindri deploy
```

### Volume out of space

**Error:**

```text
No space left on device
```

**Diagnosis:**

```bash
# Check usage
df -h /workspace

# Find large files
du -sh /workspace/* | sort -h
```

**Solution:**

```bash
# Clean up
rm -rf /workspace/projects/old-project

# Clean Docker cache
docker system prune -a

# Or increase volume size in sindri.yaml
deployment:
  volumes:
    workspace:
      size: 30GB  # Increase from 10GB
```

### Permission denied in /workspace

**Error:**

```text
Permission denied: /workspace/projects
```

**Diagnosis:**

```bash
# Check ownership
ls -la /workspace

# Check current user
whoami
id
```

**Solution:**

```bash
# Fix ownership (run as root in container)
chown -R developer:developer /workspace

# Or rebuild volume
docker volume rm sindri-workspace
./cli/sindri deploy
```

## Resource Issues

### Out of memory

**Error:**

```text
Killed (OOM)
```

**Diagnosis:**

```bash
# Check memory usage
free -h

# Check container limits
docker stats
```

**Solution:**

```bash
# Increase memory in sindri.yaml
deployment:
  resources:
    memory: 4GB  # Increase from 2GB

# Redeploy
./cli/sindri deploy
```

### High CPU usage

**Diagnosis:**

```bash
# Check processes
htop
top

# Check container CPU
docker stats
```

**Solution:**

```bash
# Kill runaway process
pkill -f <process-name>

# Increase CPUs in sindri.yaml
deployment:
  resources:
    cpus: 2  # Increase from 1

# Redeploy
./cli/sindri deploy
```

## Build Issues

### Docker build fails

**Error:**

```text
ERROR: failed to solve
```

**Diagnosis:**

```bash
# Check Docker version
docker version

# Check available space
df -h

# View build logs
pnpm build 2>&1 | tee build.log
```

**Solution:**

```bash
# Clean Docker cache
docker system prune -a

# Rebuild without cache
docker build --no-cache -t sindri:local -f docker/Dockerfile .

# Check for syntax errors in Dockerfile
docker build --dry-run -f docker/Dockerfile .
```

### Extension build fails in Dockerfile

**Error:**

```text
Error: mise install failed
```

**Solution:**

```bash
# Check mise.toml syntax
mise ls

# Validate extension.yaml
./cli/extension-manager validate <extension>

# Test locally first
pnpm build
docker run -it sindri:local bash
extension-manager install <extension>
```

## Configuration Issues

### Invalid sindri.yaml

**Error:**

```text
Schema validation failed
```

**Solution:**

```bash
# Validate configuration
./cli/sindri config validate

# Check syntax
yq . sindri.yaml

# Compare with examples
diff sindri.yaml examples/fly-minimal.sindri.yaml
```

### Invalid extension.yaml

**Error:**

```text
Extension validation failed
```

**Solution:**

```bash
# Validate against schema
./cli/extension-manager validate <extension>

# Check YAML syntax
yamllint docker/lib/extensions/<extension>/extension.yaml

# Check required fields
yq '.metadata.name' docker/lib/extensions/<extension>/extension.yaml
```

## Network Issues

### Cannot reach package registries

**Error:**

```text
Failed to fetch package
```

**Diagnosis:**

```bash
# Test DNS
nslookup registry.npmjs.org
nslookup pypi.org

# Test connectivity
curl -I https://registry.npmjs.org

# Check proxy
echo $HTTP_PROXY
echo $HTTPS_PROXY
```

**Solution:**

```bash
# Configure proxy (if needed)
export HTTP_PROXY=http://proxy:port
export HTTPS_PROXY=http://proxy:port

# Add to ~/.bashrc for persistence
echo 'export HTTP_PROXY=http://proxy:port' >> ~/.bashrc
```

### Fly.io network issues

**Error:**

```text
Cannot connect to Fly.io
```

**Solution:**

```bash
# Check Fly.io status
curl -I https://fly.io

# Check firewall
# Ensure ports 443, 22 are open

# Try different region
providers:
  fly:
    region: iad  # Try different region
```

## Secrets Issues

### Secret not available in container

**Error:**

```text
ANTHROPIC_API_KEY not set
```

**Diagnosis:**

```bash
# Check if secret is set (Fly.io)
flyctl secrets list -a my-app

# Check environment in container
printenv | grep ANTHROPIC
```

**Solution (Fly.io):**

```bash
# Set secret
flyctl secrets set ANTHROPIC_API_KEY=sk-ant-... -a my-app

# Restart machine
flyctl machine restart <machine-id> -a my-app
```

**Solution (Docker):**

```bash
# Create .env file
echo "ANTHROPIC_API_KEY=sk-ant-..." > .env

# Ensure .env in .gitignore
echo ".env" >> .gitignore

# Restart container
docker-compose restart
```

## Mise Issues

### Mise tool not found

**Error:**

```text
Error: tool not found: node
```

**Diagnosis:**

```bash
# Check mise configuration
mise ls
mise doctor

# Check installed tools
mise list

# Check PATH
echo $PATH | grep mise
```

**Solution:**

```bash
# Activate mise
eval "$(mise activate bash)"

# Add to ~/.bashrc
echo 'eval "$(mise activate bash)"' >> ~/.bashrc

# Reinstall tool
mise install node@lts

# Reshim
mise reshim
```

### Mise install hangs

**Error:**

```text
Installing node... (hangs)
```

**Solution:**

```bash
# Kill hung process
pkill -f mise

# Clear cache
rm -rf ~/.local/share/mise

# Reinstall
mise install node@lts
```

## Performance Issues

### Slow container startup

**Diagnosis:**

```bash
# Time startup
time docker run --rm sindri:local echo "ready"

# Check image size
docker images sindri
```

**Solution:**

```bash
# Use minimal profile
extensions:
  profile: minimal

# Disable unnecessary extensions
# Remove from sindri.yaml

# Use cached base image
# Ensure layer caching in Dockerfile
```

### Slow extension install

**Diagnosis:**

```bash
# Time installation
time extension-manager install nodejs
```

**Solution:**

```bash
# Use mise (faster than manual downloads)
install:
  method: mise

# Parallelize installations
# Install multiple extensions at once
extension-manager install nodejs python golang
```

## Getting More Help

### Enable Debug Mode

```bash
export DEBUG=true
./cli/sindri deploy
```

### Collect Diagnostic Info

```bash
# System info
uname -a
docker version
yq --version

# Sindri config
cat sindri.yaml

# Extension status
extension-manager list
extension-manager validate-all

# Logs
cat /workspace/.system/logs/*.log
```

### Report Issues

When reporting issues, include:

1. Error message (full output)
2. sindri.yaml (sanitized)
3. Deployment provider
4. Docker/Fly.io version
5. Steps to reproduce

**GitHub Issues:** https://github.com/pacphi/sindri/issues

## Related Documentation

- [Configuration Reference](CONFIGURATION.md)
- [Extension Catalog](EXTENSIONS.md)
- [Deployment Guide](DEPLOYMENT.md)
- [Contributing](CONTRIBUTING.md)
