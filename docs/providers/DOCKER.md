# Docker Provider

Local development and testing using Docker Compose.

## Overview

The Docker provider generates `docker-compose.yml` for local development. It's the fastest way to test Sindri configurations without cloud costs.

**Best for:** Local development, testing, offline work, CI/CD pipelines

## Prerequisites

- Docker Engine 20.10+
- Docker Compose v2+

## Configuration

### Basic Configuration

```yaml
# sindri.yaml
version: 1.0
name: sindri-docker

deployment:
  provider: docker
  resources:
    memory: 2GB
    cpus: 1

extensions:
  profile: minimal
```

### Advanced Configuration

```yaml
version: 1.0
name: sindri-docker

deployment:
  provider: docker
  resources:
    memory: 4GB
    cpus: 2

extensions:
  profile: fullstack

providers:
  docker:
    ports:
      - "3000:3000"
      - "8080:8080"
      - "5432:5432"
    volumes:
      - "./projects:/workspace/projects"
    network: sindri-network
```

**Generated:** `docker-compose.yml`

## Deployment

### Deploy

```bash
./cli/sindri deploy --provider docker
```

This will:

- Parse sindri.yaml
- Generate docker-compose.yml
- Build/pull the Sindri image
- Create volumes
- Start containers

### Connect

```bash
# Interactive shell
docker exec -it sindri-docker bash

# As developer user
docker exec -it -u developer sindri-docker bash
```

### Lifecycle Management

```bash
# Start
docker compose up -d

# Stop
docker compose stop

# Restart
docker compose restart

# View logs
docker compose logs -f

# Teardown (preserves volumes)
docker compose down

# Teardown (removes volumes)
docker compose down -v
```

## Secrets Management

Use a `.env` file (not committed to git):

```bash
# .env
ANTHROPIC_API_KEY=sk-ant-...
GITHUB_TOKEN=ghp_...
GIT_USER_NAME=Your Name
GIT_USER_EMAIL=you@example.com
```

Reference in generated `docker-compose.yml`:

```yaml
services:
  sindri:
    env_file: .env
```

Or pass directly:

```bash
ANTHROPIC_API_KEY=sk-ant-... docker compose up -d
```

## Volume Management

### Default Volumes

The adapter creates a persistent volume for `/workspace`:

```yaml
volumes:
  sindri-workspace:
    driver: local
```

### Custom Volume Mounts

```yaml
providers:
  docker:
    volumes:
      - "./projects:/workspace/projects"
      - "~/.ssh:/home/developer/.ssh:ro"
      - "~/.gitconfig:/home/developer/.gitconfig:ro"
```

### Volume Operations

```bash
# List volumes
docker volume ls | grep sindri

# Inspect volume
docker volume inspect sindri-workspace

# Backup volume
docker run --rm -v sindri-workspace:/data -v $(pwd):/backup \
  alpine tar czf /backup/workspace-backup.tar.gz -C /data .

# Restore volume
docker run --rm -v sindri-workspace:/data -v $(pwd):/backup \
  alpine tar xzf /backup/workspace-backup.tar.gz -C /data
```

## Port Forwarding

### Configure Ports

```yaml
providers:
  docker:
    ports:
      - "3000:3000" # Web app
      - "8080:8080" # API
      - "5432:5432" # PostgreSQL
      - "6379:6379" # Redis
```

### Dynamic Port Assignment

```yaml
providers:
  docker:
    ports:
      - "3000" # Assigns random host port
```

Check assigned port:

```bash
docker compose port sindri 3000
```

## Resource Limits

```yaml
deployment:
  resources:
    memory: 4GB
    cpus: 2
```

Generated in docker-compose.yml:

```yaml
services:
  sindri:
    deploy:
      resources:
        limits:
          cpus: "2"
          memory: 4G
        reservations:
          cpus: "1"
          memory: 2G
```

## Networking

### Default Network

```yaml
networks:
  sindri-network:
    driver: bridge
```

### Connect to Existing Network

```yaml
providers:
  docker:
    network: my-existing-network
    networkExternal: true
```

### Multiple Networks

```yaml
providers:
  docker:
    networks:
      - sindri-network
      - database-network
```

## Troubleshooting

### Container Won't Start

```bash
# Check logs
docker compose logs sindri

# Check container status
docker compose ps

# Inspect container
docker inspect sindri-docker
```

### Permission Issues

```bash
# Check volume permissions
docker exec sindri-docker ls -la /workspace

# Fix ownership
docker exec -u root sindri-docker chown -R developer:developer /workspace
```

### Out of Memory

Increase memory limit:

```yaml
deployment:
  resources:
    memory: 8GB
```

Or check what's using memory:

```bash
docker stats sindri-docker
```

### Port Already in Use

```bash
# Find what's using the port
lsof -i :3000

# Use different port
providers:
  docker:
    ports:
      - "3001:3000"  # Map to different host port
```

## Best Practices

1. **Use .env files** - Never hardcode secrets in sindri.yaml
2. **Mount SSH keys read-only** - `~/.ssh:/home/developer/.ssh:ro`
3. **Set resource limits** - Prevent runaway containers
4. **Use named volumes** - For persistent data
5. **Test locally first** - Before deploying to cloud providers

## Cost

**$0** - Uses local machine resources only.

**Resource Requirements:**

- 2-8 GB RAM recommended
- 10-50 GB disk space
- 1-2 CPU cores

## Related Documentation

- [Deployment Overview](../DEPLOYMENT.md)
- [Configuration Reference](../CONFIGURATION.md)
- [Troubleshooting](../TROUBLESHOOTING.md)
