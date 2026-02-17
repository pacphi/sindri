# Provider Architecture Comparison

> **Version:** 3.x
> **Last Updated:** 2026-02

This document explains how Sindri providers translate your declarative `sindri.yaml` configuration into provider-specific deployments. Understanding these patterns helps you reason about what happens under the hood when you run `sindri deploy`.

## The Declarative Contract

All providers consume the same `sindri.yaml` configuration. Common fields like `deployment.resources`, `deployment.volumes`, `extensions`, and `secrets` are portable across providers. Provider-specific tuning lives under the `providers.<name>` block.

```yaml
# These fields are consumed by EVERY provider
deployment:
  provider: <name>
  resources:
    memory: 8GB
    cpus: 4
    gpu:
      enabled: true
      type: nvidia
      count: 1
  volumes:
    workspace:
      path: /alt/home/developer/workspace
      size: 50GB

extensions:
  profile: anthropic-dev

# Provider-specific configuration
providers:
  <name>:
    # ... provider-specific fields
```

## Two Deployment Architecture Patterns

Sindri providers fall into two categories based on how they translate configuration into deployments.

### Pattern 1: Template-Based (Config File Generation)

**Providers:** Docker, Fly.io, DevPod, E2B, Kubernetes

These providers generate a **local configuration file** on disk, then invoke a CLI tool that reads that file.

```
sindri.yaml
  → SindriConfig (parsed Rust struct)
    → TemplateContext (provider-neutral intermediate representation)
      → Tera template rendering
        → Local config file (docker-compose.yml, fly.toml, etc.)
          → CLI tool (docker compose, flyctl, kubectl, etc.)
```

| Provider    | Template                       | Generated File        | CLI Tool         |
| ----------- | ------------------------------ | --------------------- | ---------------- |
| Docker      | `docker-compose.yml.tera`      | `docker-compose.yml`  | `docker compose` |
| Docker DinD | `docker-compose.dind.yml.tera` | `docker-compose.yml`  | `docker compose` |
| Fly.io      | `fly.toml.tera`                | `fly.toml`            | `flyctl`         |
| DevPod      | `devcontainer.json.tera`       | `devcontainer.json`   | `devpod`         |
| E2B         | `e2b.toml.tera`                | `e2b.toml`            | `e2b`            |
| Kubernetes  | `k8s-deployment.yaml.tera`     | `k8s-deployment.yaml` | `kubectl`        |

**Key benefit:** The generated file is an inspectable artifact. You can review `docker-compose.yml` or `fly.toml` before deployment, customize it, or version-control it. The `TemplateContext` struct serves as a provider-neutral "what I want" declaration.

### Pattern 2: API-Direct (Programmatic Payload Construction)

**Providers:** RunPod, Northflank

These providers construct API payloads programmatically in Rust and send them directly to the provider's API -- no intermediate config file is written to disk.

```
sindri.yaml
  → SindriConfig (parsed Rust struct)
    → Provider-specific config extraction (get_runpod_config / get_northflank_config)
      → API request struct / JSON payload
        → REST API call (RunPod) or CLI with --input JSON (Northflank)
```

| Provider   | Config Method             | Payload Type              | Delivery           |
| ---------- | ------------------------- | ------------------------- | ------------------ |
| RunPod     | `get_runpod_config()`     | `CreatePodRequest` struct | REST API (reqwest) |
| Northflank | `get_northflank_config()` | JSON via `serde_json`     | CLI `--input` flag |

**Why no templates?** These providers don't use CLI tools that read config files from disk. RunPod's API is purely REST-based (no CLI required), and Northflank's CLI accepts inline JSON rather than reading from a file. Generating a config file would be an unnecessary intermediate step.

**Trade-off:** There is no inspectable local artifact to review before deployment. Use `sindri deploy --dry-run` to preview what will be sent to the provider.

## Declarative Config Flow Comparison

Despite the architectural difference, both patterns read from the same declarative source. Here is how common `sindri.yaml` fields map through each pattern:

| `sindri.yaml` Field            | Template-Based                               | API-Direct                                           |
| ------------------------------ | -------------------------------------------- | ---------------------------------------------------- |
| `deployment.resources.memory`  | `TemplateContext.memory` → template variable | `get_*_config()` → parsed to MB for API payload      |
| `deployment.resources.cpus`    | `TemplateContext.cpus` → template variable   | `get_*_config()` → API field                         |
| `deployment.resources.gpu`     | `TemplateContext.gpu_*` → template variable  | `get_*_config()` → GPU type mapping + API field      |
| `deployment.volumes.workspace` | `TemplateContext.volume_size` → template     | `get_*_config()` → parsed to GB for API payload      |
| `extensions.profile`           | `TemplateContext.profile` → template         | Handled at image build time (same for all providers) |
| `secrets`                      | `.env.secrets` file → template mount         | Resolved and injected as env vars via API            |
| `providers.<name>.*`           | Provider-specific template variables         | Provider-specific API fields                         |

## Inspecting Deployments

### Template-Based Providers

Generated config files are written to the working directory (or `output_dir`). You can inspect them directly:

```bash
# After sindri deploy with Docker provider
cat docker-compose.yml

# After sindri deploy with Fly.io provider
cat fly.toml
```

### API-Direct Providers

Use the planning and dry-run features:

```bash
# Preview the deployment plan (all providers)
sindri plan

# Dry-run deployment (shows what would be created without creating it)
sindri deploy --dry-run
```

## Start/Stop Behavior by Provider

All providers implement `start` and `stop` through the `Provider` trait, but the underlying mechanism varies. The user experience is uniform -- `sindri stop` to save costs, `sindri start` to resume -- while each provider uses the most appropriate lifecycle primitive.

| Provider       | `sindri stop`                     | `sindri start`                      | State Preserved                 | Cost While Stopped  |
| -------------- | --------------------------------- | ----------------------------------- | ------------------------------- | ------------------- |
| **Docker**     | `docker compose stop`             | `docker compose start`              | Container + volumes             | Free (local)        |
| **Fly.io**     | Stop machine (`machines stop`)    | Start machine (`machines start`)    | Volume persists                 | Storage only        |
| **E2B**        | Pause sandbox (snapshot state)    | Resume sandbox (from snapshot)      | Full memory + disk              | Storage only        |
| **DevPod**     | `devpod stop`                     | `devpod up`                         | Depends on backend              | Depends on backend  |
| **Kubernetes** | Scale deployment to 0 replicas    | Scale deployment to 1 replica       | PVC persists                    | PVC storage only    |
| **RunPod**     | Stop pod (`POST /pods/{id}/stop`) | Start pod (`POST /pods/{id}/start`) | Container disk + network volume | Network volume only |
| **Northflank** | Pause service (zero instances)    | Resume service                      | Volume persists                 | Volume storage only |

### Key Differences

- **Docker** and **DevPod** use native container stop/start -- the container process halts but the filesystem is retained.
- **Fly.io** and **RunPod** stop the underlying VM/pod. The instance is deallocated but persistent volumes survive.
- **E2B** takes a memory snapshot on pause, enabling sub-second resume with full state (including running processes).
- **Kubernetes** achieves stop/start by scaling replicas to 0/1. PersistentVolumeClaims survive independently of pods.
- **Northflank** has native pause/resume at the service level -- the service is deallocated but can be resumed with its original configuration and attached volumes.

## Adding New Providers

New providers implement the `Provider` trait from `sindri-providers/src/traits.rs`. Choose the pattern that matches the provider's interface:

- **Template-based** if the provider's CLI reads a config file (e.g., `terraform apply`, `helm install -f values.yaml`)
- **API-direct** if the provider exposes a REST/GraphQL API or a CLI that accepts inline payloads

Both patterns pull from the same `SindriConfig`, ensuring the user's declarative `sindri.yaml` remains the single source of truth regardless of deployment mechanism.

## Related Documentation

- [Provider README](README.md) -- Provider comparison and selection guide
- [Configuration Reference](../CONFIGURATION.md) -- Full `sindri.yaml` schema
- [ADR-002: Provider Abstraction Layer](../architecture/adr/002-provider-abstraction-layer.md) -- Design decisions
- [ADR-005: Provider-Specific Implementations](../architecture/adr/005-provider-specific-implementations.md) -- Implementation patterns
