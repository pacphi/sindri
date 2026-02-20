# sindri-clusters

Local Kubernetes cluster lifecycle management for Sindri. Provides a unified interface for creating and managing clusters using Kind (Kubernetes IN Docker) and K3d (K3s in Docker).

## Features

- Create, destroy, and list local Kubernetes clusters
- Multi-node cluster support (control-plane + worker nodes)
- K3d local Docker registry integration
- Auto-detection of installed cluster tools (kind, k3d)
- Cross-platform installation assistance (macOS, Linux)
- Provider pattern with `ClusterProvider` trait

## Modules

- `config` - Cluster configuration types (`ClusterConfig`, `ClusterInfo`, `K3dConfig`, `KindConfig`)
- `installer` - Tool installation helpers for kind and k3d
- `k3d` - K3d provider implementation with registry support
- `kind` - Kind provider implementation
- `platform` - Platform detection for installation guidance
- `traits` - `ClusterProvider` trait defining the cluster lifecycle interface

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
sindri-clusters = { path = "../sindri-clusters" }
```

## Part of [Sindri](../../)
