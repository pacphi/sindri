# sindri-providers

Provider adapters for deploying Sindri environments to different platforms. Each provider implements a common `Provider` trait for uniform lifecycle management across backends.

## Features

- Unified provider trait for deploy, connect, status, start, stop, and destroy operations
- Docker provider for local development environments
- Fly.io provider for cloud deployment with auto-suspend
- DevPod provider for multi-cloud development environments
- E2B provider for cloud sandboxes
- Kubernetes provider for container orchestration
- RunPod provider for GPU cloud workloads
- Northflank provider for Kubernetes PaaS
- Tera-based template rendering for provider-specific configurations
- Provider factory for dynamic provider instantiation

## Modules

- `docker` - Docker and Docker Compose provider
- `fly` - Fly.io cloud provider
- `devpod` - DevPod multi-cloud provider
- `e2b` - E2B sandbox provider
- `kubernetes` - Kubernetes provider
- `runpod` - RunPod GPU cloud provider
- `northflank` - Northflank Kubernetes PaaS provider
- `traits` - `Provider` trait and `ProviderFactory`
- `templates` - Shared Tera template utilities
- `utils` - Common provider helper functions

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
sindri-providers = { path = "../sindri-providers" }
```

## Part of [Sindri](../../)
