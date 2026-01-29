# Infrastructure Tools

Infrastructure and DevOps tooling (Terraform, Kubernetes, Config Management).

## Overview

| Property         | Value                        |
| ---------------- | ---------------------------- |
| **Category**     | infrastructure               |
| **Version**      | 2.0.0                        |
| **Installation** | hybrid (mise + apt + script) |
| **Disk Space**   | 2500 MB                      |
| **Dependencies** | None                         |

## Description

Infrastructure and DevOps tooling (Terraform, K8s, Config Mgmt) - provides a comprehensive suite of infrastructure as code and Kubernetes management tools.

## Installed Tools

| Tool           | Type     | Source | Description                                |
| -------------- | -------- | ------ | ------------------------------------------ |
| `terraform`    | cli-tool | mise   | Infrastructure provisioning                |
| `kubectl`      | cli-tool | mise   | Kubernetes CLI                             |
| `helm`         | cli-tool | mise   | Kubernetes package manager                 |
| `k9s`          | cli-tool | mise   | Kubernetes TUI                             |
| `ansible`      | cli-tool | apt    | Configuration management                   |
| `ansible-lint` | cli-tool | apt    | Ansible linter                             |
| `pulumi`       | cli-tool | script | Infrastructure as code                     |
| `crossplane`   | cli-tool | script | Cloud-native control planes                |
| `kubectx`      | cli-tool | script | Kubernetes context switcher                |
| `kubens`       | cli-tool | script | Kubernetes namespace switcher              |
| `kapp`         | cli-tool | script | Carvel - Kubernetes application deployment |
| `ytt`          | cli-tool | script | Carvel - YAML templating                   |
| `kbld`         | cli-tool | script | Carvel - Image resolution                  |
| `vendir`       | cli-tool | script | Carvel - Vendor dependencies               |
| `imgpkg`       | cli-tool | script | Carvel - Image packaging                   |

## Configuration

### Environment Variables

| Variable                    | Value                | Scope  |
| --------------------------- | -------------------- | ------ |
| `KUBECONFIG`                | `$HOME/.kube/config` | bashrc |
| `ANSIBLE_HOST_KEY_CHECKING` | `False`              | bashrc |

### Templates

| Template                      | Destination                           | Description        |
| ----------------------------- | ------------------------------------- | ------------------ |
| `infra-tools.bashrc.template` | `~/.bashrc.d/infra-tools.sh`          | Tool configuration |
| `infra-tools.readme.template` | `/workspace/infrastructure/README.md` | Usage guide        |

## Network Requirements

- `releases.hashicorp.com` - Terraform
- `apt.releases.hashicorp.com` - HashiCorp apt
- `dl.k8s.io` - Kubernetes
- `get.helm.sh` - Helm
- `raw.githubusercontent.com` - GitHub raw
- `github.com` - GitHub
- `carvel.dev` - Carvel tools
- `www.pulumi.com` - Pulumi

## Installation

```bash
extension-manager install infra-tools
```

## Validation

```bash
terraform version     # Expected: Terraform vX.X.X
ansible --version     # Expected: ansible [core X.X.X]
kubectl version --client
helm version --short  # Expected: vX.X.X
k9s version
pulumi version
crossplane --version
```

### Mise Validation

The extension validates that at least 4 of these tools are installed via mise:

- terraform
- kubectl
- helm
- ubi:derailed/k9s

## Upgrade

**Strategy:** automatic

Automatically upgrades mise-managed tools and apt packages.

## Removal

Requires confirmation before removal.

```bash
extension-manager remove infra-tools
```

Removes mise configuration and all installed tools.

## Related Extensions

- [docker](DOCKER.md) - Containerization
- [cloud-tools](CLOUD-TOOLS.md) - Cloud CLIs
