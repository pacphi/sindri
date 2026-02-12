# Kubernetes Cluster Example

Deploy Sindri as a pod in an existing Kubernetes cluster.

## Usage

```bash
sindri init --from examples/kubernetes-cluster
sindri deploy
```

## What This Configures

- Kubernetes provider targeting the `sindri-dev` namespace
- `gp3` storage class for persistent workspace volume
- Ingress enabled with an internal hostname
- `devops` profile (Docker, infra-tools, monitoring, cloud-tools)
- Kubeconfig pulled from HashiCorp Vault

## Prerequisites

- Access to a Kubernetes cluster with `kubectl` configured
- HashiCorp Vault accessible for secret retrieval
- `gp3` (or equivalent) StorageClass available in the cluster
