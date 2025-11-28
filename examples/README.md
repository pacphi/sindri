# Sindri Configuration Examples

Ready-to-use configuration files for deploying Sindri to various providers. **47 examples** covering all profiles, providers, and configuration permutations.

## Quick Start

1. Browse the examples below
2. Copy one that matches your needs: `cp examples/fly/minimal.sindri.yaml my-sindri.yaml`
3. Customize with your settings
4. Deploy: `./cli/sindri deploy --config my-sindri.yaml`

## Overview

### Example Types

| Type                 | Count | Description                            |
| -------------------- | ----- | -------------------------------------- |
| **Profile-based**    | 40    | Use curated extension profiles         |
| **Custom**           | 7     | Hand-picked extensions (no profile)    |
| **Provider-focused** | 24    | Provider-specific features and regions |

### Coverage Matrix (Profiles × Providers)

| Profile       | Fly | Docker | DevPod | Custom | Total |
| ------------- | --- | ------ | ------ | ------ | ----- |
| minimal       | 4   | 1      | 11     | -      | 16    |
| fullstack     | 2   | 1      | 2      | -      | 5     |
| ai-dev        | 1   | 1      | 2      | -      | 4     |
| anthropic-dev | 2   | 1      | -      | -      | 3     |
| systems       | 1   | 1      | 1      | -      | 3     |
| devops        | 1   | 1      | 1      | -      | 3     |
| data-science  | 2   | 1      | 1      | -      | 4     |
| enterprise    | 1   | 1      | -      | -      | 2     |
| mobile        | 1   | 1      | -      | -      | 2     |
| **Custom**    | 2   | 2      | 1      | 2      | 7     |
| **Total**     | 17  | 11     | 19     | 2      | 47    |

## Directory Structure

### By Provider

| Directory              | Provider       | Examples | Description                             |
| ---------------------- | -------------- | -------- | --------------------------------------- |
| `fly/`                 | Fly.io         | 17       | Deploy to Fly.io's global edge network  |
| `docker/`              | Docker Compose | 11       | Local development with Docker           |
| `devpod/aws/`          | DevPod + AWS   | 5        | EC2-based development environments      |
| `devpod/gcp/`          | DevPod + GCP   | 4        | GCE-based development environments      |
| `devpod/azure/`        | DevPod + Azure | 3        | Azure VM-based development environments |
| `devpod/digitalocean/` | DevPod + DO    | 2        | DigitalOcean droplet environments       |
| `devpod/kubernetes/`   | DevPod + K8s   | 3        | Kubernetes pod-based environments       |
| `custom/`              | Mixed          | 7        | Custom extension combinations           |

### By Extension Profile

| File                                 | Profile       | Extensions (count)                                                 |
| ------------------------------------ | ------------- | ------------------------------------------------------------------ |
| `profiles/minimal.sindri.yaml`       | minimal       | nodejs, python (2)                                                 |
| `profiles/fullstack.sindri.yaml`     | fullstack     | nodejs, python, docker, nodejs-devtools (4)                        |
| `profiles/ai-dev.sindri.yaml`        | ai-dev        | nodejs, python, ai-toolkit, openskills, monitoring (5)             |
| `profiles/anthropic-dev.sindri.yaml` | anthropic-dev | agent-manager, ai-toolkit, claude-code-mux, cloud-tools, etc. (12) |
| `profiles/systems.sindri.yaml`       | systems       | rust, golang, docker, infra-tools (4)                              |
| `profiles/devops.sindri.yaml`        | devops        | docker, infra-tools, cloud-tools, monitoring (4)                   |
| `profiles/data-science.sindri.yaml`  | data-science  | python, monitoring (2)                                             |
| `profiles/enterprise.sindri.yaml`    | enterprise    | nodejs, python, golang, rust, ruby, jvm, dotnet, docker, etc. (9)  |
| `profiles/mobile.sindri.yaml`        | mobile        | nodejs (1+)                                                        |

## Example Files

Each example includes:

- Complete, valid configuration
- Comments explaining each option
- Tested in CI (if it's here, it works)

### Fly.io Examples (17 examples)

Production-ready deployments to Fly.io's global edge network.

```text
fly/
├── minimal.sindri.yaml               # Basic (1GB RAM, shared CPU)
├── fullstack.sindri.yaml             # Full dev environment (4GB RAM, dedicated CPU)
├── production.sindri.yaml            # Production-ready with secrets
├── ai-dev.sindri.yaml                # AI/ML development (4GB RAM)
├── anthropic-dev.sindri.yaml         # Full Anthropic toolset (8GB RAM, always-on)
├── systems.sindri.yaml               # Rust/Go systems programming (4GB RAM)
├── mobile.sindri.yaml                # Mobile development backend (6GB RAM)
├── devops.sindri.yaml                # DevOps tools (4GB RAM)
├── data-science.sindri.yaml          # Data science (4GB RAM)
├── data-science-high-mem.sindri.yaml # Data science with 16GB RAM
├── enterprise.sindri.yaml            # All languages (8GB RAM)
└── regions/                          # Region-specific deployments
    ├── ord.sindri.yaml               # Chicago region
    ├── ams.sindri.yaml               # Amsterdam region
    └── iad.sindri.yaml               # Virginia region
```

### Docker Examples (11 examples)

Local development with Docker Compose.

```text
docker/
├── minimal.sindri.yaml        # Basic local setup (2GB RAM)
├── fullstack.sindri.yaml      # Full-stack dev (4GB RAM)
├── ai-dev.sindri.yaml         # AI/ML development (12GB RAM, Jupyter ports)
├── anthropic-dev.sindri.yaml  # Full Anthropic toolset (16GB RAM)
├── systems.sindri.yaml        # Rust/Go (6GB RAM)
├── devops.sindri.yaml         # DevOps tools with privileged mode (8GB RAM)
├── data-science.sindri.yaml   # Data science with Jupyter (10GB RAM)
├── enterprise.sindri.yaml     # All languages (16GB RAM, multi-port)
└── mobile.sindri.yaml         # Mobile development (8GB RAM, Expo ports)
```

### DevPod Examples (19 examples)

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
│   └── regions/
│       └── europe-west1.sindri.yaml # Belgium region
├── azure/
│   ├── minimal.sindri.yaml         # Standard_B2s basic setup
│   ├── data-science.sindri.yaml    # Standard_D4s_v3 (16GB RAM)
│   └── regions/
│       └── westeurope.sindri.yaml  # Netherlands region
├── digitalocean/
│   ├── minimal.sindri.yaml         # 2GB droplet
│   └── regions/
│       └── sfo3.sindri.yaml        # San Francisco region
└── kubernetes/
    ├── minimal.sindri.yaml         # Basic K8s pod
    ├── devops.sindri.yaml          # DevOps in K8s (standard storage)
    └── systems.sindri.yaml         # Rust/Go in K8s (fast-ssd storage)
```

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
./cli/sindri deploy --config my-sindri.yaml

# Or deploy an example directly (for testing)
./cli/sindri deploy --config examples/fly/minimal.sindri.yaml
```

### Teardown

```bash
# Teardown using the same config
./cli/sindri destroy --config my-sindri.yaml
```

### Validate (before deploy)

```bash
# Validate your config against the schema
./cli/sindri config validate --config my-sindri.yaml
```

### Test (for contributors)

All examples are tested automatically in CI using the `test-sindri-config.yml` workflow.

#### Test a Single Example

```bash
# Test a single example locally
./cli/sindri test --config examples/fly/minimal.sindri.yaml --suite smoke

# Test with validation only
./cli/sindri config validate --config examples/docker/ai-dev.sindri.yaml
```

#### Test Multiple Examples

```bash
# Test all examples for a provider
./cli/sindri test --config examples/fly/ --suite smoke

# Test all examples in a directory
./cli/sindri test --config examples/profiles/ --suite smoke

# Test all custom extension examples
./cli/sindri test --config examples/custom/ --suite smoke
```

#### Test via GitHub Actions

You can manually trigger tests using GitHub Actions:

1. Go to **Actions** → **Test Sindri Configuration**
2. Click **Run workflow**
3. Select a config path from the dropdown (52 individual examples or directories)
4. Choose test suite: `smoke`, `integration`, or `full`
5. Optionally skip cleanup for debugging

**Available test targets:**

- Individual files: All 52 examples are available
- Directories: `examples/fly/`, `examples/docker/`, `examples/devpod/aws/`, etc.
- All examples: `examples/` (tests all 52 configurations)

## See Also

- [Schema Documentation](../docs/SCHEMA.md) for all options
- [CLI Reference](../docs/CLI.md) for command details
- [Extension Registry](../docker/lib/registry.yaml) for available extensions
