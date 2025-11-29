# Cloud Tools

Cloud provider CLI tools for AWS, Azure, GCP, OCI, Alibaba, DigitalOcean, and IBM Cloud.

## Overview

| Property         | Value          |
| ---------------- | -------------- |
| **Category**     | infrastructure |
| **Version**      | 2.0.0          |
| **Installation** | script         |
| **Disk Space**   | 2500 MB        |
| **Dependencies** | None           |

## Description

Cloud provider CLI tools (AWS, Azure, GCP, OCI, Alibaba, DO, IBM) - provides command-line interfaces for all major cloud providers.

## Installed Tools

| Tool       | Type     | Description             |
| ---------- | -------- | ----------------------- |
| `aws`      | cli-tool | Amazon Web Services CLI |
| `az`       | cli-tool | Microsoft Azure CLI     |
| `gcloud`   | cli-tool | Google Cloud SDK        |
| `aliyun`   | cli-tool | Alibaba Cloud CLI       |
| `doctl`    | cli-tool | DigitalOcean CLI        |
| `ibmcloud` | cli-tool | IBM Cloud CLI           |

## Configuration

### Templates

| Template                   | Destination                             | Mode      | Description     |
| -------------------------- | --------------------------------------- | --------- | --------------- |
| `ssh-environment.template` | `/etc/profile.d/00-ssh-environment.sh`  | append    | SSH environment |
| `bashrc.template`          | `~/.bashrc`                             | append    | Cloud aliases   |
| `readme.template`          | `/workspace/extensions/cloud/README.md` | overwrite | Usage guide     |

## Secrets (Optional)

| Secret                  | Provider | Description       |
| ----------------------- | -------- | ----------------- |
| `aws_access_key_id`     | AWS      | Access key ID     |
| `aws_secret_access_key` | AWS      | Secret access key |
| `aws_session_token`     | AWS      | Session token     |
| `azure_client_id`       | Azure    | Client ID         |
| `azure_client_secret`   | Azure    | Client secret     |
| `azure_tenant_id`       | Azure    | Tenant ID         |

## Network Requirements

- `awscli.amazonaws.com` - AWS CLI
- `aka.ms` - Azure CLI
- `packages.cloud.google.com` - Google Cloud SDK
- `github.com` - GitHub
- `api.github.com` - GitHub API
- `raw.githubusercontent.com` - GitHub raw
- `aliyuncli.alicdn.com` - Alibaba Cloud CLI
- `clis.cloud.ibm.com` - IBM Cloud CLI

## Installation

```bash
extension-manager install cloud-tools
```

## Usage

### AWS

```bash
aws configure
aws s3 ls
aws ec2 describe-instances
```

### Azure

```bash
az login
az account list
az vm list
```

### Google Cloud

```bash
gcloud auth login
gcloud projects list
gcloud compute instances list
```

### DigitalOcean

```bash
doctl auth init
doctl compute droplet list
```

### Alibaba Cloud

```bash
aliyun configure
aliyun ecs DescribeInstances
```

### IBM Cloud

```bash
ibmcloud login
ibmcloud resource groups
```

## Validation

```bash
aws --version       # Expected: aws-cli/X.X.X
az --version
gcloud --version
aliyun --version
doctl version
ibmcloud --version
```

## Upgrade

**Strategy:** manual

```bash
extension-manager upgrade cloud-tools
```

## Removal

```bash
extension-manager remove cloud-tools
```

Removes all cloud CLI tools and configuration.

## Related Extensions

- [infra-tools](INFRA-TOOLS.md) - Infrastructure tools
- [docker](DOCKER.md) - Containerization
