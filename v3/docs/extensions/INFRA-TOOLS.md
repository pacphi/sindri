# Infrastructure Tools Extension

> Version: 2.0.0 | Category: devops | Last Updated: 2026-01-26

## Overview

Infrastructure and DevOps tooling including Terraform, Kubernetes tools, and Configuration Management. Comprehensive IaC toolkit.

## What It Provides

| Tool           | Type     | License    | Description                   |
| -------------- | -------- | ---------- | ----------------------------- |
| terraform      | cli-tool | BUSL-1.1   | Infrastructure as Code        |
| kubectl        | cli-tool | Apache-2.0 | Kubernetes CLI                |
| helm           | cli-tool | Apache-2.0 | Kubernetes package manager    |
| k9s            | cli-tool | Apache-2.0 | Kubernetes TUI                |
| ansible        | cli-tool | GPL-3.0    | Configuration management      |
| pulumi         | cli-tool | Apache-2.0 | Modern IaC                    |
| crossplane     | cli-tool | Apache-2.0 | Kubernetes-native IaC         |
| kubectx/kubens | cli-tool | Apache-2.0 | Context/namespace switching   |
| kapp           | cli-tool | Apache-2.0 | Carvel application management |
| ytt            | cli-tool | Apache-2.0 | YAML templating               |
| kbld           | cli-tool | Apache-2.0 | Image building                |
| vendir         | cli-tool | Apache-2.0 | Dependency management         |
| imgpkg         | cli-tool | Apache-2.0 | OCI packaging                 |

## Requirements

- **Disk Space**: 2500 MB
- **Memory**: 256 MB
- **Install Time**: ~120 seconds
- **Dependencies**: None

### Network Domains

- releases.hashicorp.com, apt.releases.hashicorp.com
- dl.k8s.io
- get.helm.sh
- raw.githubusercontent.com, github.com, api.github.com
- carvel.dev
- pulumi.com, www.pulumi.com

## Installation

```bash
extension-manager install infra-tools
```

## Configuration

### Environment Variables

| Variable                    | Value              | Description                   |
| --------------------------- | ------------------ | ----------------------------- |
| `KUBECONFIG`                | $HOME/.kube/config | Kubernetes config path        |
| `ANSIBLE_HOST_KEY_CHECKING` | False              | Disable SSH host key checking |

### Templates

- infra-tools.bashrc.template - Shell configuration
- infra-tools.readme.template - Documentation at ~/infrastructure/README.md

### Install Method

Hybrid installation with mise, apt packages, and custom scripts.

### Upgrade Strategy

Automatic via mise upgrade.

## Usage Examples

### Terraform

```bash
# Initialize
terraform init

# Plan changes
terraform plan

# Apply changes
terraform apply

# Destroy infrastructure
terraform destroy

# Format code
terraform fmt
```

### Kubernetes (kubectl)

```bash
# Get resources
kubectl get pods
kubectl get services
kubectl get deployments

# Apply manifests
kubectl apply -f deployment.yaml

# Logs and exec
kubectl logs pod-name
kubectl exec -it pod-name -- bash

# Context management
kubectx my-cluster
kubens my-namespace
```

### Helm

```bash
# Add repository
helm repo add bitnami https://charts.bitnami.com/bitnami

# Search charts
helm search repo nginx

# Install a chart
helm install my-release bitnami/nginx

# Upgrade
helm upgrade my-release bitnami/nginx

# List releases
helm list
```

### K9s

```bash
# Launch TUI
k9s

# With specific context
k9s --context my-cluster

# With namespace filter
k9s -n my-namespace
```

### Ansible

```bash
# Run playbook
ansible-playbook playbook.yml

# Inventory check
ansible-inventory --list

# Ad-hoc commands
ansible all -m ping

# Lint playbook
ansible-lint playbook.yml
```

### Pulumi

```bash
# Create project
pulumi new aws-python

# Preview changes
pulumi preview

# Deploy
pulumi up

# Destroy
pulumi destroy
```

### Carvel Tools

```bash
# ytt - Template YAML
ytt -f config/ -f values.yaml

# kapp - Deploy application
kapp deploy -a my-app -f config/

# kbld - Build images
kbld -f config/

# vendir - Sync dependencies
vendir sync
```

## Validation

The extension validates the following commands:

- `terraform version` - Must match pattern `Terraform v\d+\.\d+\.\d+`
- `ansible --version` - Must match pattern `ansible \[core \d+\.\d+\.\d+\]`
- `kubectl version --client` - Must match pattern `Client Version`
- `helm version --short` - Must match pattern `v\d+\.\d+\.\d+`
- `packer version` - Must match pattern `Packer v\d+\.\d+\.\d+`
- `kustomize version` - Must match pattern `v\d+\.\d+\.\d+`

## Removal

```bash
extension-manager remove infra-tools
```

**Requires confirmation.** Removes mise tools and installed binaries.

## Related Extensions

- [cloud-tools](CLOUD-TOOLS.md) - Cloud provider CLIs
- [docker](DOCKER.md) - Container runtime
