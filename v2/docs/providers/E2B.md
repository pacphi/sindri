# E2B Cloud Sandbox Provider

## Overview

[E2B](https://e2b.dev) (formerly CodeSandbox) is a cloud platform that provides secure, isolated sandboxes optimized for AI-generated code execution. Sindri's E2B provider enables ultra-fast development environments with ~150ms startup times using snapshot-based persistence.

### Why Use E2B?

| Feature                | Benefit                                     |
| ---------------------- | ------------------------------------------- |
| **Ultra-fast startup** | ~150ms sandbox boot from snapshots          |
| **Pause/Resume**       | Preserve full state (memory + filesystem)   |
| **Pay-per-second**     | Only pay for active compute time            |
| **WebSocket access**   | Works through corporate firewalls (no SSH)  |
| **AI-optimized**       | Built for AI agent code execution           |
| **Strong isolation**   | Lightweight VMs with full kernel separation |

### E2B vs Other Providers

| Aspect        | Docker    | Fly.io     | DevPod          | E2B           |
| ------------- | --------- | ---------- | --------------- | ------------- |
| Startup time  | 10-30s    | 10-60s     | 30-60s          | ~150ms        |
| Access method | SSH       | SSH        | SSH             | WebSocket PTY |
| Persistence   | Volumes   | Volumes    | Volumes         | Pause/Resume  |
| GPU support   | Yes       | Yes        | Yes             | No            |
| Offline       | Yes       | No         | Yes             | No            |
| Best for      | Local dev | Remote dev | IDE integration | AI sandboxes  |

## Prerequisites

### 1. E2B Account

Create a free E2B account at [e2b.dev](https://e2b.dev):

- Hobby tier: $100 free credits
- Pro tier: $150/month + usage
- Ultimate tier: Custom pricing

### 2. E2B API Key

Obtain your API key from the [E2B Dashboard](https://e2b.dev/dashboard):

```bash
# Set as environment variable
export E2B_API_KEY="e2b_..."

# Or add to .env file (not committed to git)
echo "E2B_API_KEY=e2b_..." >> .env
```

### 3. E2B CLI Installation

Install the E2B command-line interface:

```bash
# Using npm
npm install -g @e2b/cli

# Verify installation
e2b --version

# Authenticate
e2b auth login
```

## Quick Start

### 1. Configure sindri.yaml

```yaml
# sindri.yaml
version: "1.0"
name: my-e2b-dev

deployment:
  provider: e2b
  resources:
    memory: 2GB
    cpus: 2

extensions:
  profile: fullstack

providers:
  e2b:
    timeout: 3600 # 1 hour
    autoPause: true # Pause on timeout
    autoResume: true # Resume on connect
```

### 2. Deploy

```bash
# Deploy to E2B
./v2/cli/sindri deploy --provider e2b

# Check status
./v2/cli/sindri status

# Connect
./v2/cli/sindri connect
```

### 3. Work

```bash
# Inside the sandbox
claude  # Start Claude Code
cd /workspace
git clone https://github.com/your/project.git
```

### 4. Pause When Done

```bash
# Pause to preserve state (from host)
./v2/cli/sindri pause

# Or let it auto-pause after timeout
```

### 5. Resume Later

```bash
# Resume where you left off
./v2/cli/sindri connect  # Auto-resumes if paused
```

### 6. Cleanup

```bash
# Destroy sandbox (loses state)
./v2/cli/sindri destroy
```

## Configuration Reference

### Full Example

```yaml
version: "1.0"
name: sindri-e2b-dev

deployment:
  provider: e2b
  resources:
    memory: 4GB # 512MB - 8GB
    cpus: 4 # 1 - 8 vCPUs
  volumes:
    workspace:
      size: 20GB # Ephemeral storage

extensions:
  profile: fullstack
  additional:
    - docker
    - monitoring

providers:
  e2b:
    # Template configuration
    templateAlias: my-sindri-template # Custom template name
    reuseTemplate: true # Reuse existing template

    # Sandbox behavior
    timeout: 3600 # Timeout in seconds (1 hour)
    autoPause: true # Pause instead of kill on timeout
    autoResume: true # Resume paused sandbox on connect

    # Network configuration
    internetAccess: true # Outbound internet access
    allowedDomains: # Whitelist (empty = all allowed)
      - github.com
      - "*.github.com"
      - registry.npmjs.org
      - api.anthropic.com
    blockedDomains: [] # Blacklist
    publicAccess: false # Public URL access

    # Metadata
    metadata:
      project: my-project
      environment: development
      owner: developer@example.com

    # Advanced options
    # team: my-team                     # E2B team for billing
    # buildOnDeploy: false              # Force rebuild on every deploy

secrets:
  - name: E2B_API_KEY
    source: env
    required: true
```

### Provider Options

| Option           | Type    | Default  | Description                              |
| ---------------- | ------- | -------- | ---------------------------------------- |
| `templateAlias`  | string  | `{name}` | Template identifier for sandbox creation |
| `reuseTemplate`  | boolean | `true`   | Reuse existing template if available     |
| `timeout`        | integer | `300`    | Sandbox timeout in seconds (60-86400)    |
| `autoPause`      | boolean | `true`   | Pause on timeout instead of killing      |
| `autoResume`     | boolean | `true`   | Auto-resume paused sandbox on connect    |
| `internetAccess` | boolean | `true`   | Enable outbound internet                 |
| `allowedDomains` | array   | `[]`     | Whitelist domains (empty = all)          |
| `blockedDomains` | array   | `[]`     | Blacklist domains                        |
| `publicAccess`   | boolean | `false`  | Allow public URL access to services      |
| `metadata`       | object  | `{}`     | Custom key-value metadata                |
| `team`           | string  | -        | E2B team for billing                     |
| `buildOnDeploy`  | boolean | `false`  | Force template rebuild                   |

### Resource Limits

| Resource | Minimum | Maximum | Default               |
| -------- | ------- | ------- | --------------------- |
| Memory   | 512MB   | 8GB     | 2GB                   |
| vCPUs    | 1       | 8       | 2                     |
| Storage  | 10GB    | 20GB    | 10GB (tier-dependent) |

## Commands

### Standard Sindri Commands

```bash
# Deploy / create sandbox
./v2/cli/sindri deploy --provider e2b

# Check status
./v2/cli/sindri status

# Connect to sandbox
./v2/cli/sindri connect

# Show deployment plan
./v2/cli/sindri plan

# Destroy sandbox
./v2/cli/sindri destroy
./v2/cli/sindri destroy --force  # Skip confirmation
```

### E2B-Specific Commands

```bash
# Pause sandbox (preserve state)
./v2/cli/sindri pause

# Template management
./v2/cli/sindri template build     # Build template from Dockerfile
./v2/cli/sindri template list      # List available templates
./v2/cli/sindri template delete    # Delete a template
```

### Direct E2B CLI Usage

```bash
# List sandboxes
e2b sandbox list

# Connect to sandbox terminal
e2b sandbox terminal <sandbox-id>

# Pause sandbox
e2b sandbox pause <sandbox-id>

# Resume sandbox
e2b sandbox resume <sandbox-id>

# Kill sandbox
e2b sandbox kill <sandbox-id>

# File operations
e2b sandbox files list <id> /path
e2b sandbox files upload <id> local.txt /remote/path.txt
e2b sandbox files download <id> /remote/file.txt ./local.txt
```

## Persistence Model

### How E2B Persistence Works

Unlike traditional providers that use persistent volumes, E2B uses a **pause/resume** model:

```text
                        Work Session
                    ┌─────────────────┐
    sindri deploy   │                 │   sindri pause
         │          │   Sandbox       │        │
         ▼          │   Running       │        ▼
    ┌─────────┐     │                 │   ┌─────────┐
    │ Running │◄────┤   (all data     │───▶│ Paused  │
    └─────────┘     │    in memory)   │   └─────────┘
                    │                 │        │
                    └─────────────────┘        │  sindri connect
                                               │  (auto-resume)
                                               ▼
                                          ┌─────────┐
                                          │ Running │
                                          │(restored)│
                                          └─────────┘
```

### State Preservation

| State   | Data Retained    | Duration      | Billable          |
| ------- | ---------------- | ------------- | ----------------- |
| Running | All (RAM + disk) | Until timeout | Yes (compute)     |
| Paused  | All (snapshot)   | 30 days       | No (storage only) |
| Killed  | None             | -             | No                |

### Pause Timing

Pausing takes approximately **4 seconds per 1 GiB of RAM**:

- 2GB RAM: ~8 seconds
- 4GB RAM: ~16 seconds
- 8GB RAM: ~32 seconds

### Important Limitations

1. **30-day retention**: Paused sandboxes expire 30 days from initial creation
2. **No traditional volumes**: Cannot attach external storage
3. **Data on kill**: `sindri destroy` immediately loses all data

### Best Practices for Data Safety

```bash
# Always commit and push before ending session
git add .
git commit -m "Work in progress"
git push origin feature-branch

# Pause (don't destroy) to preserve state
./v2/cli/sindri pause

# Use GitHub for persistent storage
```

## Connection Strategy

### No SSH - WebSocket PTY

E2B sandboxes don't support SSH. Instead, Sindri provides terminal access via WebSocket PTY:

```bash
# Connect via Sindri CLI (recommended)
./v2/cli/sindri connect

# Or via E2B CLI directly
e2b sandbox terminal <sandbox-id>
```

### Connection Works Through Firewalls

Because E2B uses WebSocket over HTTPS (port 443), connections work through corporate firewalls that block SSH:

```text
┌─────────────┐     HTTPS/443     ┌─────────────┐
│ Your Machine│◄─────────────────▶│  E2B Cloud  │
│             │   WebSocket PTY   │  (Sandbox)  │
└─────────────┘                   └─────────────┘
```

### VS Code Integration

Since E2B doesn't support SSH, VS Code Remote SSH won't work directly. Alternatives:

1. **Web-based VS Code**: Run VS Code Server inside the sandbox
2. **File sync**: Use `e2b sandbox files` commands
3. **Local + deploy**: Edit locally, sync to sandbox

```yaml
# Future: VS Code Server in sandbox
providers:
  e2b:
    # Run VS Code Server on port 8080
    # Access via https://<port>-<sandbox-id>.e2b.app
```

## Cost Optimization

### Pricing Overview

**Compute (per-second billing):**

| vCPUs | Cost/second | Cost/hour |
| ----- | ----------- | --------- |
| 1     | $0.000014   | ~$0.05    |
| 2     | $0.000028   | ~$0.10    |
| 4     | $0.000056   | ~$0.20    |

**RAM:** $0.0000045/GiB/second (~$0.016/GiB/hour)

**Example: 2 vCPU, 2GB sandbox**

- Per second: $0.000028 + $0.000009 = $0.000037
- Per hour: ~$0.13
- 8 hours/day, 22 days: ~$23/month

### Cost Reduction Tips

1. **Use autoPause**

   ```yaml
   providers:
     e2b:
       autoPause: true # Pause instead of kill
       timeout: 300 # 5 min idle timeout
   ```

2. **Pause frequently**

   ```bash
   # Pause when stepping away
   ./cli/sindri pause
   ```

3. **Right-size resources**

   ```yaml
   resources:
     memory: 1GB # Start small
     cpus: 1 # Scale up if needed
   ```

4. **Use ephemeral for testing**

   ```bash
   # Disposable sandbox - no pause costs
   ./cli/sindri deploy --ephemeral
   ```

5. **Monitor usage**
   - Check [E2B Dashboard](https://e2b.dev/dashboard) regularly
   - Set billing alerts

### Cost Comparison

**8 hours active development per day, 22 days/month:**

| Provider             | Configuration          | Monthly Cost     |
| -------------------- | ---------------------- | ---------------- |
| E2B                  | 2 vCPU, 2GB, autoPause | ~$23             |
| E2B (with 50% pause) | 2 vCPU, 2GB            | ~$12             |
| Fly.io               | shared-cpu-2x, 2GB     | ~$15             |
| Docker               | Local                  | $0 (electricity) |

## Troubleshooting

### Sandbox Won't Start

**Symptom:** Deploy fails or times out

**Solutions:**

1. Check API key: `echo $E2B_API_KEY`
2. Verify E2B CLI: `e2b auth whoami`
3. Check E2B status: [status.e2b.dev](https://status.e2b.dev)
4. Rebuild template: `./cli/sindri template build --rebuild`

### Connection Fails

**Symptom:** `connect` command hangs or errors

**Solutions:**

1. Check sandbox status: `./cli/sindri status`
2. If paused, sandbox should auto-resume - wait a few seconds
3. Try direct E2B CLI: `e2b sandbox terminal <id>`
4. Check network connectivity to E2B

### Data Lost After Pause

**Symptom:** Data missing when resuming

**Solutions:**

1. Check 30-day retention limit
2. Verify pause completed: `./cli/sindri status` should show "paused"
3. Avoid using `destroy` - use `pause` instead

### Template Build Fails

**Symptom:** `template build` errors

**Solutions:**

1. Check Dockerfile syntax
2. Ensure base image is accessible
3. Check E2B build logs: `e2b template logs <alias>`
4. Try building with verbose: `./cli/sindri template build --verbose`

### Slow Sandbox Performance

**Symptom:** Commands run slowly in sandbox

**Solutions:**

1. Increase resources:
   ```yaml
   resources:
     memory: 4GB
     cpus: 4
   ```
2. Check if near storage limit
3. Consider network latency (sandbox is remote)

### GPU Not Supported Error

**Symptom:** Deploy fails with GPU configuration

**Solution:** E2B does not support GPU. Remove GPU configuration:

```yaml
resources:
  memory: 4GB
  cpus: 4
  # Remove gpu section entirely
```

Use Fly.io or DevPod for GPU workloads.

## Limitations

### Not Supported on E2B

| Feature            | Status        | Alternative                |
| ------------------ | ------------- | -------------------------- |
| GPU                | Not available | Use Fly.io or DevPod       |
| SSH                | Not supported | Use WebSocket PTY          |
| Persistent volumes | Not supported | Use pause/resume           |
| Offline access     | Not possible  | Use Docker locally         |
| Long-term storage  | 30-day limit  | Push to git frequently     |
| Custom kernels     | Not available | Use standard configuration |

### Resource Limits

- Maximum 8 vCPUs
- Maximum 8GB RAM
- Maximum 20GB storage (tier-dependent)
- Maximum 24-hour session (Pro tier)
- Maximum 100 concurrent sandboxes (Pro tier)

### Network Restrictions

- No inbound connections (unless publicAccess enabled)
- Domain filtering available but not granular
- WebSocket required for terminal access

## Use Cases

### Best For

- **AI agent sandboxes**: Fast, isolated code execution
- **Quick prototyping**: Sub-second startup for rapid iteration
- **CI/CD testing**: Ephemeral environments per PR
- **Remote work**: Works through corporate firewalls
- **Cost-sensitive**: Pay only for active time

### Not Recommended For

- **GPU workloads**: Use Fly.io or cloud DevPod
- **Long-running servers**: Use Fly.io
- **Offline development**: Use Docker
- **VS Code Remote SSH**: Use DevPod
- **Production hosting**: Use proper hosting platforms

## Related Documentation

- [Deployment Overview](../DEPLOYMENT.md)
- [Configuration Reference](../CONFIGURATION.md)
- [Secrets Management](../SECRETS_MANAGEMENT.md)
- [Docker Provider](DOCKER.md)
- [Fly.io Provider](FLY.md)
- [DevPod Provider](DEVPOD.md)

## External Resources

- [E2B Documentation](https://e2b.dev/docs)
- [E2B SDK Reference](https://e2b.dev/docs/sdk-reference)
- [E2B Pricing](https://e2b.dev/pricing)
- [E2B CLI Reference](https://e2b.dev/docs/cli)
- [E2B Status Page](https://status.e2b.dev)
