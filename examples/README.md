# Sindri Configuration Examples

Ready-to-use configuration files for deploying Sindri to various providers. **67 examples** covering all profiles, providers, and configuration permutations.

## Quick Start

1. Browse the examples below
2. Copy one that matches your needs: `cp examples/fly/minimal.sindri.yaml my-sindri.yaml`
3. Customize with your settings
4. Deploy: `./v2/cli/sindri deploy --config my-sindri.yaml`

## Overview

### Example Types

| Type              | Count | Description                         |
| ----------------- | ----- | ----------------------------------- |
| **Profile-based** | 51    | Use curated extension profiles      |
| **Custom**        | 7     | Hand-picked extensions (no profile) |
| **Reference**     | 9     | Profile reference configurations    |

### Coverage Matrix (Profiles x Providers)

| Profile       | Fly | Docker | E2B | DevPod | K8s | Custom | Profiles | Total |
| ------------- | --- | ------ | --- | ------ | --- | ------ | -------- | ----- |
| minimal       | 1   | 1      | 1   | 6      | 2   | -      | 1        | 12    |
| fullstack     | 1   | 1      | 1   | 2      | -   | -      | 1        | 6     |
| ai-dev        | -   | 1      | 1   | 3      | -   | -      | 1        | 6     |
| anthropic-dev | 1   | 1      | -   | -      | -   | -      | -        | 2     |
| systems       | 1   | 1      | -   | 1      | -   | -      | -        | 3     |
| devops        | -   | 1      | -   | 1      | -   | -      | 1        | 3     |
| enterprise    | -   | 1      | -   | -      | -   | -      | 1        | 2     |
| mobile        | 1   | 1      | -   | -      | -   | -      | -        | 2     |
| **Other**     | 5   | 7      | 3   | 5      | -   | 7      | 5        | 32    |
| **Total**     | 10  | 15     | 6   | 18     | 2   | 7      | 9        | 67    |

**Note:** "Other" includes region-specific configs, GPU examples, specialized setups (DinD, vision-flow, etc.), and custom extension combinations.

## Directory Structure

### By Provider

| Directory              | Provider       | Examples | Description                               |
| ---------------------- | -------------- | -------- | ----------------------------------------- |
| `fly/`                 | Fly.io         | 10       | Deploy to Fly.io's global edge network    |
| `docker/`              | Docker Compose | 15       | Local development with Docker             |
| `e2b/`                 | E2B            | 6        | Ultra-fast cloud sandboxes (~150ms start) |
| `devpod/aws/`          | DevPod + AWS   | 5        | EC2-based development environments        |
| `devpod/gcp/`          | DevPod + GCP   | 5        | GCE-based development environments        |
| `devpod/azure/`        | DevPod + Azure | 2        | Azure VM-based development environments   |
| `devpod/digitalocean/` | DevPod + DO    | 2        | DigitalOcean droplet environments         |
| `devpod/kubernetes/`   | DevPod + K8s   | 4        | Kubernetes pod-based environments         |
| `k8s/`                 | Kind/K3d       | 2        | Local cluster creation + deployment       |
| `custom/`              | Mixed          | 7        | Custom extension combinations             |
| `profiles/`            | Reference      | 9        | Profile reference configurations          |

### By Extension Profile

| File                              | Profile    | Extensions (count)                                                |
| --------------------------------- | ---------- | ----------------------------------------------------------------- |
| `profiles/minimal.sindri.yaml`    | minimal    | nodejs, python (2)                                                |
| `profiles/fullstack.sindri.yaml`  | fullstack  | nodejs, python, docker, nodejs-devtools (4)                       |
| `profiles/ai-dev.sindri.yaml`     | ai-dev     | nodejs, python, ai-toolkit, openskills, monitoring (5)            |
| `profiles/devops.sindri.yaml`     | devops     | docker, infra-tools, cloud-tools, monitoring (4)                  |
| `profiles/enterprise.sindri.yaml` | enterprise | nodejs, python, golang, rust, ruby, jvm, dotnet, docker, etc. (9) |

**Note:** Provider-specific examples for `anthropic-dev`, `systems`, and `mobile` profiles are in `fly/` and `docker/` directories.

## Example Files

Each example includes:

- Complete, valid configuration
- Comments explaining each option
- Tested in CI (if it's here, it works)

### Fly.io Examples (10 examples)

Production-ready deployments to Fly.io's global edge network.

```text
fly/
├── minimal.sindri.yaml               # Basic (1GB RAM, shared CPU)
├── fullstack.sindri.yaml             # Full dev environment (4GB RAM, dedicated CPU)
├── production.sindri.yaml            # Production-ready with secrets
├── anthropic-dev.sindri.yaml         # Full Anthropic toolset (8GB RAM, always-on)
├── systems.sindri.yaml               # Rust/Go systems programming (4GB RAM)
├── mobile.sindri.yaml                # Mobile development backend (6GB RAM)
├── gpu-ml-training.sindri.yaml       # GPU-accelerated ML training
└── regions/                          # Region-specific deployments
    ├── ord.sindri.yaml               # Chicago region
    ├── ams.sindri.yaml               # Amsterdam region
    └── iad.sindri.yaml               # Virginia region
```

### Docker Examples (15 examples)

Local development with Docker Compose.

```text
docker/
├── minimal.sindri.yaml        # Basic local setup (2GB RAM)
├── fullstack.sindri.yaml      # Full-stack dev (4GB RAM)
├── ai-dev.sindri.yaml         # AI/ML development (12GB RAM, Jupyter ports)
├── anthropic-dev.sindri.yaml  # Full Anthropic toolset (16GB RAM)
├── systems.sindri.yaml        # Rust/Go (6GB RAM)
├── devops.sindri.yaml         # DevOps tools with privileged mode (8GB RAM)
├── enterprise.sindri.yaml     # All languages (16GB RAM, multi-port)
├── mobile.sindri.yaml         # Mobile development (8GB RAM, Expo ports)
├── gpu-ai-dev.sindri.yaml     # GPU-accelerated AI development
├── claude-codepro.sindri.yaml # Claude Code Pro setup
├── agent-browser.sindri.yaml  # Browser automation agent
├── pal-mcp-server.sindri.yaml # PAL MCP server setup
├── dind-privileged.sindri.yaml # Docker-in-Docker (privileged mode)
├── dind-socket.sindri.yaml    # Docker-in-Docker (socket mount)
└── dind-sysbox.sindri.yaml    # Docker-in-Docker (Sysbox runtime)
```

### E2B Examples (6 examples)

Ultra-fast cloud sandboxes with ~150ms startup. Perfect for AI agents and rapid prototyping.

```text
e2b/
├── minimal.sindri.yaml        # Quick start with defaults (2GB RAM)
├── ai-dev.sindri.yaml         # Full AI development (4GB RAM, Claude Code)
├── fullstack.sindri.yaml      # Web development with public access (4GB RAM)
├── ephemeral.sindri.yaml      # Throwaway sandbox (no persistence)
├── cost-optimized.sindri.yaml # Maximum cost savings (short timeout)
└── secure.sindri.yaml         # Network-restricted (domain allowlist)
```

**E2B-specific features:**

- ~150ms startup (snapshot-based boot)
- `sindri pause` - Pause sandbox (preserves state, stops billing)
- WebSocket PTY terminal (no SSH required)
- Per-second billing (~$0.13/hr for 2 vCPU, 2GB)

### DevPod Examples (18 examples)

Cloud-based development environments via DevPod.

```text
devpod/
├── aws/
│   ├── minimal.sindri.yaml         # t3.small basic setup
│   ├── fullstack.sindri.yaml       # t3.xlarge (8GB RAM, 4 CPU)
│   ├── ai-dev-gpu.sindri.yaml      # g4dn.2xlarge GPU instance (16GB RAM)
│   └── regions/
│       ├── us-east-1.sindri.yaml   # Virginia region
│       └── eu-west-1.sindri.yaml   # Ireland region
├── gcp/
│   ├── minimal.sindri.yaml         # n2-standard-2 basic setup
│   ├── fullstack.sindri.yaml       # n2-standard-4 (8GB RAM, 4 CPU)
│   ├── ai-dev.sindri.yaml          # n2-highmem-8 (16GB RAM)
│   ├── gpu-inference.sindri.yaml   # GPU inference workloads
│   └── regions/
│       └── europe-west1.sindri.yaml # Belgium region
├── azure/
│   ├── minimal.sindri.yaml         # Standard_B2s basic setup
│   └── regions/
│       └── westeurope.sindri.yaml  # Netherlands region
├── digitalocean/
│   ├── minimal.sindri.yaml         # 2GB droplet
│   └── regions/
│       └── sfo3.sindri.yaml        # San Francisco region
└── kubernetes/
    ├── minimal.sindri.yaml         # Basic K8s pod
    ├── devops.sindri.yaml          # DevOps in K8s (standard storage)
    ├── systems.sindri.yaml         # Rust/Go in K8s (fast-ssd storage)
    └── gpu-workload.sindri.yaml    # GPU workloads in K8s
```

### Local Kubernetes Examples (2 examples)

These configs **create local Kubernetes clusters** AND deploy Sindri (all-in-one for local dev):

```text
k8s/
├── kind-minimal.sindri.yaml       # Creates kind cluster + deploys via DevPod
└── k3d-with-registry.sindri.yaml  # Creates k3d cluster with local registry
```

**Key difference from `devpod/kubernetes/`:**

| Directory            | Purpose                    | When to Use           |
| -------------------- | -------------------------- | --------------------- |
| `devpod/kubernetes/` | Deploy to EXISTING cluster | CI, external clusters |
| `k8s/`               | CREATE cluster + deploy    | Local development     |

**Usage:**

```bash
# Option 1: Use k8s/ configs (creates cluster + deploys)
./v2/cli/sindri deploy --config examples/k8s/kind-minimal.sindri.yaml

# Option 2: Create cluster separately, then use devpod/kubernetes
kind create cluster --name my-cluster
./v2/cli/sindri deploy --config examples/devpod/kubernetes/minimal.sindri.yaml
```

**CI Testing Note:** The CI workflow uses `devpod/kubernetes/` configs because it handles
cluster creation separately (auto-creates kind if no KUBECONFIG secret is provided).

### Custom Extension Examples (7 examples)

Hand-picked extensions without profiles, showing flexible combinations.

```text
custom/
├── python-ml.sindri.yaml       # Python + AI toolkit only (no Node.js)
├── rust-wasm.sindri.yaml       # Rust + Node.js for WebAssembly
├── cloud-native.sindri.yaml    # Docker + infra + cloud tools (no languages)
├── polyglot-backend.sindri.yaml # Python + Go + Rust backend
├── frontend-only.sindri.yaml   # Node.js + devtools + Playwright
├── minimal-golang.sindri.yaml  # Go-only (2GB RAM, 1 CPU)
└── security-focused.sindri.yaml # Security and compliance tooling
```

## Configuration Variation Dimensions

Our examples cover multiple configuration dimensions to show real-world permutations:

### 1. Resource Configurations

Examples vary across three resource tiers:

- **Low**: 1-2GB RAM, 1-2 CPU (minimal, lightweight workloads)
- **Medium**: 4-8GB RAM, 2-4 CPU (development, AI/ML)
- **High**: 8-16GB RAM, 4-8 CPU (enterprise, GPU workloads)

### 2. Provider-Specific Features

Each provider showcases unique capabilities:

- **Fly.io**: Regions (ord/sjc/ams), CPU types (shared/performance), auto-stop/start
- **Docker**: Port mappings, privileged mode, volume mounting
- **DevPod**: Cloud providers (AWS/GCP/Azure/DO/K8s), instance types, regions

### 3. Deployment Scenarios

Examples demonstrate different use cases:

- **Development**: Auto-stop enabled, cost-optimized
- **Production**: Always-on, high availability, larger resources
- **Testing**: Ephemeral, CI-friendly configurations

### 4. Extension Strategies

Two approaches to extension management:

- **Profile-based** (`extensions.profile`): Use curated extension bundles
- **Custom** (`extensions.active`): Hand-pick specific extensions

## Customization

Common customizations:

- `name`: Change the deployment name
- `deployment.resources`: Adjust memory/CPU
- `extensions.profile` or `extensions.active`: Choose your tools
- `providers.<provider>.*`: Provider-specific options

### Example: Custom Extensions

Instead of using a profile, you can specify individual extensions:

```yaml
extensions:
  active:
    - nodejs
    - python
    - rust
    - docker
    - ai-toolkit
```

### Example: Profile + Additional Extensions

Combine a profile with extra extensions:

```yaml
extensions:
  profile: fullstack
  active:
    - ai-toolkit # Add AI tools to fullstack
    - monitoring # Add monitoring tools
```

### Example: Adding Secrets

```yaml
secrets:
  - name: ANTHROPIC_API_KEY
    source: env
  - name: GITHUB_TOKEN
    source: env
```

### Example: Resource Tuning

Adjust resources for your workload:

```yaml
deployment:
  resources:
    memory: 8GB # Increase for memory-intensive tasks
    cpus: 4 # Increase for parallel workloads
  volumes:
    workspace:
      size: 100GB # Increase for large datasets
```

## Usage

### Deploy

```bash
# Deploy using your config
./v2/cli/sindri deploy --config my-sindri.yaml

# Or deploy an example directly (for testing)
./v2/cli/sindri deploy --config examples/fly/minimal.sindri.yaml
```

### Teardown

```bash
# Teardown using the same config
./v2/cli/sindri destroy --config my-sindri.yaml
```

### Validate (before deploy)

```bash
# Validate your config against the schema
./v2/cli/sindri config validate --config my-sindri.yaml
```

### Test (for contributors)

All examples are tested automatically in CI using the `test-sindri-config.yml` workflow.

#### Test a Single Example

```bash
# Test a single example locally
./v2/cli/sindri test --config examples/fly/minimal.sindri.yaml --suite smoke

# Test with validation only
./v2/cli/sindri config validate --config examples/docker/ai-dev.sindri.yaml
```

#### Test Multiple Examples

```bash
# Test all examples for a provider
./v2/cli/sindri test --config examples/fly/ --suite smoke

# Test all examples in a directory
./v2/cli/sindri test --config examples/profiles/ --suite smoke

# Test all custom extension examples
./v2/cli/sindri test --config examples/custom/ --suite smoke
```

#### Test via GitHub Actions

You can manually trigger tests using GitHub Actions:

1. Go to **Actions** → **Test Sindri Configuration**
2. Click **Run workflow**
3. Select a config path from the dropdown (67 individual examples or directories)
4. Choose test suite: `smoke`, `integration`, or `full`
5. Optionally skip cleanup for debugging

**Available test targets:**

- Individual files: All 67 examples are available
- Directories: `examples/fly/`, `examples/docker/`, `examples/devpod/aws/`, etc.
- All examples: `examples/` (tests all 67 configurations)

## See Also

- [Schema Documentation](../v2/docs/SCHEMA.md) for all options
- [CLI Reference](../v2/docs/CLI.md) for command details
- [Extension Registry](../v2/docker/lib/registry.yaml) for available extensions
