# Cloud Tools Extension

> Version: 2.0.0 | Category: devops | Last Updated: 2026-01-29

## Overview

Cloud provider CLI tools for AWS, Azure, GCP, Fly.io, OCI, Alibaba Cloud, DigitalOcean, and IBM Cloud. Provides unified access to major cloud platforms.

## What It Provides

| Tool     | Type     | License    | Description       |
| -------- | -------- | ---------- | ----------------- |
| aws      | cli-tool | Apache-2.0 | AWS CLI v2        |
| az       | cli-tool | MIT        | Azure CLI         |
| gcloud   | cli-tool | Apache-2.0 | Google Cloud SDK  |
| aliyun   | cli-tool | Apache-2.0 | Alibaba Cloud CLI |
| doctl    | cli-tool | Apache-2.0 | DigitalOcean CLI  |
| flyctl   | cli-tool | Apache-2.0 | Fly.io CLI        |
| ibmcloud | cli-tool | Apache-2.0 | IBM Cloud CLI     |

## Requirements

- **Disk Space**: 2500 MB
- **Memory**: 256 MB
- **Install Time**: ~180 seconds
- **Dependencies**: python (required for Azure CLI pip installation)

### Network Domains

- amazonaws.com, awscli.amazonaws.com
- pypi.org, files.pythonhosted.org (Azure CLI pip installation)
- google.com, dl.google.com (Google Cloud SDK tarball)
- fly.io
- github.com, api.github.com, raw.githubusercontent.com
- alicdn.com, aliyuncli.alicdn.com
- ibm.com, clis.cloud.ibm.com, download.clis.cloud.ibm.com

### Secrets (Optional)

- `aws_access_key_id`, `aws_secret_access_key`, `aws_session_token`
- `azure_client_id`, `azure_client_secret`, `azure_tenant_id`
- `fly_api_token`

## Installation

```bash
sindri extension install cloud-tools
```

### Security-Hardened Installation

This extension uses sudo-free, user-local installation methods for all cloud CLIs to ensure compatibility with security-hardened containers (`no-new-privileges` flag and `/tmp` mounted `noexec`).

**Installation Methods:**

- **AWS CLI**: User-local installer to `~/.local/bin/aws`
- **Azure CLI**: pip installation to `~/.local/bin/az` (requires Python 3.10+)
- **Google Cloud SDK**: Tarball extraction to `~/google-cloud-sdk/bin/gcloud`
- **Fly.io CLI**: User install script to `~/.fly/bin/flyctl`
- **Oracle CLI**: User install script to `~/bin/oci`
- **Alibaba CLI**: Binary download to `~/.local/bin/aliyun`
- **DigitalOcean CLI**: Binary download to `~/.local/bin/doctl`
- **IBM Cloud CLI**: Tarball extraction to `~/.local/bin/ibmcloud`

All tools install to user directories without requiring sudo or root access, ensuring compatibility with production security policies.

**Python Requirement:**

Azure CLI requires Python 3.10 or higher for pip installation. The python extension is automatically installed as a dependency. If Python is unavailable or below version 3.10, Azure CLI installation will be skipped with a warning, but all other cloud CLIs will still install successfully.

**PATH Configuration:**

All cloud CLI tools are automatically added to your PATH via the bashrc template:

```bash
export PATH="$HOME/.local/bin:$HOME/google-cloud-sdk/bin:$HOME/.fly/bin:$HOME/bin:$PATH"
```

## Configuration

### Templates

- ssh-environment.template - SSH environment setup
- bashrc.template - Shell configuration
- readme.template - Documentation at ~/extensions/cloud/README.md

### Install Method

Uses a custom installation script with 900 second timeout.

### Upgrade Strategy

Manual - run upgrade.sh script.

## Usage Examples

### AWS CLI

```bash
# Configure credentials
aws configure

# List S3 buckets
aws s3 ls

# Deploy with CloudFormation
aws cloudformation deploy --template-file template.yaml --stack-name my-stack

# ECS task operations
aws ecs list-tasks --cluster my-cluster
```

### Azure CLI

```bash
# Login
az login

# List resource groups
az group list

# Create a web app
az webapp create --name myapp --resource-group mygroup --plan myplan

# AKS cluster operations
az aks get-credentials --resource-group mygroup --name mycluster
```

### Google Cloud

```bash
# Authenticate
gcloud auth login

# Set project
gcloud config set project my-project

# List compute instances
gcloud compute instances list

# Deploy to App Engine
gcloud app deploy
```

### Fly.io

```bash
# Login
flyctl auth login

# Launch an app
flyctl launch

# Deploy
flyctl deploy

# Check status
flyctl status
```

### DigitalOcean

```bash
# Authenticate
doctl auth init

# List droplets
doctl compute droplet list

# Create a droplet
doctl compute droplet create myserver --region nyc1 --image ubuntu-22-04-x64 --size s-1vcpu-1gb
```

### Alibaba Cloud

```bash
# Configure
aliyun configure

# List ECS instances
aliyun ecs DescribeInstances

# List OSS buckets
aliyun oss ls
```

### IBM Cloud

```bash
# Login
ibmcloud login

# List resources
ibmcloud resource service-instances

# Kubernetes operations
ibmcloud ks cluster ls
```

## Validation

The extension validates the following commands:

- `aws` - Must match pattern `aws-cli`
- `az` - Must be available
- `gcloud` - Must be available
- `flyctl` - Must match pattern `flyctl`

## Removal

```bash
sindri extension remove cloud-tools
```

This removes CLI tools and configurations.

## Related Extensions

- [infra-tools](INFRA-TOOLS.md) - Infrastructure tools (Terraform, K8s)
