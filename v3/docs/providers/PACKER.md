# Packer Provider

> **Version:** 3.x
> **Last Updated:** 2026-01

Multi-cloud VM image building using HashiCorp Packer.

## Overview

The Packer provider enables building golden VM images across multiple cloud platforms using a unified configuration. Built images include Sindri pre-installed with your specified extensions and profiles, enabling fast instance launches without runtime provisioning.

**Supported Clouds:**

- **AWS** - EC2 AMI images via `amazon-ebs` builder
- **Azure** - Managed images with Shared Image Gallery support
- **GCP** - Compute Engine images via `googlecompute` builder
- **OCI** - Oracle Cloud Infrastructure custom images
- **Alibaba** - Alibaba Cloud ECS custom images

**Best for:** Enterprise deployments, golden image pipelines, multi-cloud infrastructure, pre-baked development environments

## Quick Start

```bash
# 1. Check prerequisites
sindri packer doctor --cloud aws

# 2. Build an AWS AMI
sindri packer build --cloud aws --name my-sindri-image --profile fullstack

# 3. List your images
sindri packer list --cloud aws

# 4. Deploy an instance from the image
sindri packer deploy --cloud aws ami-0123456789abcdef0
```

**Build time:** 10-20 minutes depending on profile and extensions

## Prerequisites

### Required

| Requirement      | Version | Check Command      | Install                                        |
| ---------------- | ------- | ------------------ | ---------------------------------------------- |
| HashiCorp Packer | 1.9+    | `packer --version` | https://developer.hashicorp.com/packer/install |

### Cloud-Specific CLIs

| Cloud   | CLI Tool | Check Command      | Install                                                         |
| ------- | -------- | ------------------ | --------------------------------------------------------------- |
| AWS     | aws      | `aws --version`    | https://aws.amazon.com/cli/                                     |
| Azure   | az       | `az --version`     | https://docs.microsoft.com/cli/azure/install-azure-cli          |
| GCP     | gcloud   | `gcloud --version` | https://cloud.google.com/sdk/docs/install                       |
| OCI     | oci      | `oci --version`    | https://docs.oracle.com/iaas/Content/API/SDKDocs/cliinstall.htm |
| Alibaba | aliyun   | `aliyun --version` | https://github.com/aliyun/aliyun-cli                            |

### Authentication Setup

**AWS:**

```bash
aws configure
# Or use environment variables:
export AWS_ACCESS_KEY_ID=<key>
export AWS_SECRET_ACCESS_KEY=<secret>
export AWS_REGION=us-west-2
```

**Azure:**

```bash
az login
export AZURE_SUBSCRIPTION_ID=<subscription-id>
export AZURE_RESOURCE_GROUP=sindri-packer
```

**GCP:**

```bash
gcloud auth application-default login
export GCP_PROJECT_ID=<project-id>
```

**OCI:**

```bash
oci setup config
export OCI_COMPARTMENT_OCID=<compartment-ocid>
export OCI_SUBNET_OCID=<subnet-ocid>
```

**Alibaba:**

```bash
aliyun configure
export ALICLOUD_ACCESS_KEY=<key>
export ALICLOUD_SECRET_KEY=<secret>
```

## Supported Clouds

### Feature Comparison

| Feature              | AWS | Azure   | GCP    | OCI | Alibaba |
| -------------------- | --- | ------- | ------ | --- | ------- |
| Image Building       | Yes | Yes     | Yes    | Yes | Yes     |
| Multi-Region Copy    | Yes | Gallery | Family | No  | Yes     |
| Encryption           | Yes | Yes     | Yes    | Yes | Yes     |
| Shared Image Gallery | No  | Yes     | No     | No  | No      |
| Image Family         | No  | No      | Yes    | No  | No      |
| Shielded VM          | No  | No      | Yes    | No  | No      |
| Flex Shape           | No  | No      | No     | Yes | No      |

### Default Instance Types

| Cloud   | Default Instance    | vCPUs | Memory | Notes                     |
| ------- | ------------------- | ----- | ------ | ------------------------- |
| AWS     | t3.large            | 2     | 8GB    | Burstable, cost-effective |
| Azure   | Standard_D4s_v4     | 4     | 16GB   | General purpose           |
| GCP     | e2-standard-4       | 4     | 16GB   | Efficient, balanced       |
| OCI     | VM.Standard.E4.Flex | Flex  | Flex   | Configurable OCPU/memory  |
| Alibaba | ecs.g6.xlarge       | 4     | 16GB   | General purpose           |

### Default Regions

| Cloud   | Default Region | Notes                            |
| ------- | -------------- | -------------------------------- |
| AWS     | us-west-2      | Oregon, US West                  |
| Azure   | westus2        | West US 2                        |
| GCP     | us-west1-a     | Oregon, Zone A                   |
| OCI     | (required)     | Must specify availability domain |
| Alibaba | cn-hangzhou    | Hangzhou, China                  |

## CLI Commands Reference

### sindri packer build

Build a VM image for a specific cloud provider.

```bash
sindri packer build --cloud <CLOUD> [OPTIONS]
```

**Arguments:**

| Flag               | Short | Description                                  | Default         |
| ------------------ | ----- | -------------------------------------------- | --------------- |
| `--cloud`          | `-c`  | Target cloud (aws, azure, gcp, oci, alibaba) | Required        |
| `--name`           | `-n`  | Image name prefix                            | sindri-dev      |
| `--sindri-version` |       | Sindri version to install                    | latest          |
| `--profile`        |       | Extension profile (minimal, fullstack, etc)  | base            |
| `--extensions`     |       | Additional extensions (comma-separated)      | (none)          |
| `--region`         | `-r`  | Cloud region/zone                            | (cloud default) |
| `--instance-type`  |       | Build instance type/size                     | (cloud default) |
| `--disk-size`      |       | Disk size in GB                              | 60              |
| `--cis-hardening`  |       | Enable CIS benchmark hardening               | false           |
| `--force`          | `-f`  | Force rebuild even if cached                 | false           |
| `--dry-run`        |       | Generate template without building           | false           |
| `--debug`          |       | Enable debug output (PACKER_LOG=1)           | false           |
| `--var-file`       |       | Path to variable file                        | (none)          |
| `--json`           |       | Output as JSON                               | false           |

**Examples:**

```bash
# Basic AWS build
sindri packer build --cloud aws --name my-dev-image

# Full-stack with specific extensions
sindri packer build --cloud aws \
  --name production-sindri \
  --profile fullstack \
  --extensions "docker,kubernetes" \
  --region us-east-1 \
  --instance-type t3.xlarge \
  --disk-size 100

# GCP with CIS hardening
sindri packer build --cloud gcp \
  --name hardened-sindri \
  --cis-hardening \
  --region us-central1-a

# Dry run to preview template
sindri packer build --cloud azure --dry-run

# Force rebuild
sindri packer build --cloud aws --name my-image --force
```

### sindri packer validate

Validate a Packer template without building.

```bash
sindri packer validate --cloud <CLOUD> [OPTIONS]
```

**Arguments:**

| Flag               | Short | Description           | Default    |
| ------------------ | ----- | --------------------- | ---------- |
| `--cloud`          | `-c`  | Target cloud provider | Required   |
| `--name`           | `-n`  | Image name prefix     | sindri-dev |
| `--sindri-version` |       | Sindri version        | latest     |
| `--syntax-only`    |       | Syntax check only     | false      |
| `--json`           |       | Output as JSON        | false      |

**Examples:**

```bash
# Validate AWS template
sindri packer validate --cloud aws

# Syntax check only
sindri packer validate --cloud gcp --syntax-only

# JSON output for CI/CD
sindri packer validate --cloud azure --json
```

### sindri packer list

List built images for a cloud provider.

```bash
sindri packer list --cloud <CLOUD> [OPTIONS]
```

**Arguments:**

| Flag       | Short | Description           | Default         |
| ---------- | ----- | --------------------- | --------------- |
| `--cloud`  | `-c`  | Target cloud provider | Required        |
| `--name`   | `-n`  | Filter by name prefix | (all)           |
| `--region` | `-r`  | Cloud region          | (cloud default) |
| `--json`   |       | Output as JSON        | false           |

**Examples:**

```bash
# List all AWS images
sindri packer list --cloud aws

# Filter by name
sindri packer list --cloud aws --name production-sindri

# JSON output
sindri packer list --cloud gcp --json
```

**Output:**

```
Found 3 image(s):

ID: ami-0123456789abcdef0
Name: sindri-dev-1706745600
State: Available
Sindri version: 3.0.0
Created: 2026-01-31T20:00:00Z
```

### sindri packer delete

Delete a VM image by ID.

```bash
sindri packer delete --cloud <CLOUD> <IMAGE_ID> [OPTIONS]
```

**Arguments:**

| Flag       | Short | Description           | Default         |
| ---------- | ----- | --------------------- | --------------- |
| `--cloud`  | `-c`  | Target cloud provider | Required        |
| `--region` | `-r`  | Cloud region          | (cloud default) |
| `--force`  | `-f`  | Skip confirmation     | false           |

**Examples:**

```bash
# Delete with confirmation
sindri packer delete --cloud aws ami-0123456789abcdef0

# Force delete
sindri packer delete --cloud aws ami-0123456789abcdef0 --force

# Specify region
sindri packer delete --cloud aws ami-0123456789abcdef0 --region us-east-1
```

### sindri packer doctor

Check Packer prerequisites for all or specific clouds.

```bash
sindri packer doctor [OPTIONS]
```

**Arguments:**

| Flag      | Short | Description                   | Default |
| --------- | ----- | ----------------------------- | ------- |
| `--cloud` | `-c`  | Check specific cloud or "all" | all     |
| `--json`  |       | Output as JSON                | false   |

**Examples:**

```bash
# Check all clouds
sindri packer doctor

# Check specific cloud
sindri packer doctor --cloud aws

# JSON output for scripting
sindri packer doctor --json
```

**Output:**

```
Packer Prerequisites Check

Packer installed: 1.10.0

AWS Prerequisites
  CLI installed: 2.15.0
  Credentials configured

AZURE Prerequisites
  CLI installed: 2.56.0
  Credentials not configured
  Run: az login

GCP Prerequisites
  CLI not installed
  Install: https://cloud.google.com/sdk/docs/install
```

### sindri packer init

Generate a Packer HCL template file.

```bash
sindri packer init --cloud <CLOUD> [OPTIONS]
```

**Arguments:**

| Flag       | Short | Description              | Default  |
| ---------- | ----- | ------------------------ | -------- |
| `--cloud`  | `-c`  | Target cloud provider    | Required |
| `--output` | `-o`  | Output directory         | .        |
| `--force`  | `-f`  | Overwrite existing files | false    |

**Examples:**

```bash
# Generate AWS template in current directory
sindri packer init --cloud aws

# Generate to specific directory
sindri packer init --cloud gcp --output ./packer-templates

# Overwrite existing
sindri packer init --cloud azure --force
```

**Output:**

```
Created: ./aws.pkr.hcl

Next steps:
  1. Edit ./aws.pkr.hcl as needed
  2. Run: packer init ./aws.pkr.hcl
  3. Run: packer build ./aws.pkr.hcl
```

### sindri packer deploy

Deploy an instance from a pre-built image.

```bash
sindri packer deploy --cloud <CLOUD> <IMAGE_ID> [OPTIONS]
```

**Arguments:**

| Flag              | Short | Description           | Default         |
| ----------------- | ----- | --------------------- | --------------- |
| `--cloud`         | `-c`  | Target cloud provider | Required        |
| `--region`        | `-r`  | Cloud region          | (cloud default) |
| `--instance-type` |       | Instance type/VM size | (cloud default) |
| `--json`          |       | Output as JSON        | false           |

**Examples:**

```bash
# Deploy AWS instance
sindri packer deploy --cloud aws ami-0123456789abcdef0

# Deploy with custom instance type
sindri packer deploy --cloud aws ami-0123456789abcdef0 \
  --instance-type t3.xlarge \
  --region us-east-1

# GCP deployment
sindri packer deploy --cloud gcp \
  projects/my-project/global/images/sindri-dev-1706745600

# JSON output
sindri packer deploy --cloud azure my-image-id --json
```

**Output:**

```
Deploying AWS instance from image

Image ID: ami-0123456789abcdef0
Region: us-west-2
Instance type: t3.large

Instance launched successfully
Instance ID: i-0abc123def456789
Public IP: 54.123.45.67
Private IP: 10.0.1.100
SSH: ssh ubuntu@54.123.45.67
```

## Configuration

### sindri.yaml Configuration

Configure Packer settings in your `sindri.yaml`:

```yaml
version: "1.0"
name: my-sindri-image

deployment:
  provider: packer

extensions:
  profile: fullstack
  additional:
    - docker
    - kubernetes

providers:
  packer:
    cloud: aws
    image_name: sindri-production
    description: "Production Sindri environment"

    build:
      sindri_version: "3.0.0"
      cache: true
      ssh_timeout: "20m"
      security:
        cis_hardening: true
        openscap_scan: false
        clean_sensitive_data: true
        remove_ssh_keys: true

    aws:
      region: us-west-2
      instance_type: t3.large
      volume_size: 80
      volume_type: gp3
      encrypt_boot: true
      ami_regions:
        - us-east-1
        - eu-west-1
      ami_users:
        - "123456789012"

    tags:
      Environment: production
      Team: platform
```

### AWS Configuration

```yaml
providers:
  packer:
    cloud: aws
    aws:
      region: us-west-2
      instance_type: t3.large
      volume_size: 80
      volume_type: gp3 # gp2, gp3, io1, io2
      encrypt_boot: true
      vpc_id: vpc-0123456789 # Optional
      subnet_id: subnet-0123456 # Optional
      ami_regions: # Copy to additional regions
        - us-east-1
        - eu-west-1
      ami_users: # Share with AWS accounts
        - "123456789012"
      ami_groups: # Share with groups
        - "all" # Makes AMI public
```

### Azure Configuration

```yaml
providers:
  packer:
    cloud: azure
    azure:
      subscription_id: $AZURE_SUBSCRIPTION_ID
      resource_group: sindri-packer
      location: westus2
      vm_size: Standard_D4s_v4
      os_disk_size_gb: 80
      storage_account_type: Premium_LRS
      gallery: # Shared Image Gallery
        gallery_name: sindri_gallery
        image_name: sindri-dev
        image_version: "1.0.0"
        replication_regions:
          - eastus
          - westeurope
```

### GCP Configuration

```yaml
providers:
  packer:
    cloud: gcp
    gcp:
      project_id: $GCP_PROJECT_ID
      zone: us-west1-a
      machine_type: e2-standard-4
      disk_size: 80
      disk_type: pd-ssd
      enable_secure_boot: true
      image_family: sindri-dev # Enable versioning
      network: default
      subnetwork: default
```

### OCI Configuration

```yaml
providers:
  packer:
    cloud: oci
    oci:
      compartment_ocid: $OCI_COMPARTMENT_OCID
      availability_domain: Uocm:PHX-AD-1
      shape: VM.Standard.E4.Flex
      shape_config:
        ocpus: 4
        memory_in_gbs: 32
      subnet_ocid: $OCI_SUBNET_OCID
      boot_volume_size_gb: 80
```

### Alibaba Configuration

```yaml
providers:
  packer:
    cloud: alibaba
    alibaba:
      region: cn-hangzhou
      instance_type: ecs.g6.xlarge
      system_disk_size_gb: 80
      system_disk_category: cloud_essd
      vswitch_id: vsw-0123456789 # Optional
```

### Build Configuration

```yaml
providers:
  packer:
    build:
      # Sindri version
      sindri_version: "3.0.0"

      # Extensions to pre-install
      extensions:
        - python
        - node
        - docker

      # Extension profile
      profile: fullstack

      # Cache behavior
      cache: true # Reuse existing image if config matches

      # Build timeout
      ssh_timeout: "20m"

      # Custom provisioning scripts
      scripts:
        - ./scripts/custom-setup.sh
        - ./scripts/install-deps.sh

      # Ansible playbook
      ansible_playbook: ./playbooks/provision.yml

      # Environment variables for provisioning
      environment:
        CUSTOM_VAR: "value"
        FEATURE_FLAG: "enabled"

      # File uploads
      file_uploads:
        - source: ./configs/app.conf
          destination: /etc/app/app.conf
        - source: ./certs/
          destination: /etc/ssl/certs/

      # Parallel builds (0 = unlimited)
      parallel_builds: 2
```

### Security Configuration

```yaml
providers:
  packer:
    build:
      security:
        # CIS Benchmark hardening (Level 1)
        cis_hardening: true

        # Run OpenSCAP security scan
        openscap_scan: false

        # Clean sensitive data before capture
        clean_sensitive_data: true

        # Remove SSH host keys
        remove_ssh_keys: true
```

## Examples

### AWS Production Image

```bash
# Build production AWS AMI with full security
sindri packer build --cloud aws \
  --name production-sindri \
  --profile fullstack \
  --extensions "docker,kubernetes,monitoring" \
  --region us-west-2 \
  --instance-type t3.xlarge \
  --disk-size 100 \
  --cis-hardening

# Copy to multiple regions for DR
# Configure ami_regions in sindri.yaml for automatic replication
```

### Azure Enterprise Image

```bash
# Build Azure image with Shared Image Gallery
sindri packer build --cloud azure \
  --name enterprise-sindri \
  --profile enterprise \
  --region eastus

# Image will be published to configured gallery
# with automatic replication to specified regions
```

### GCP with Image Family

```bash
# Build GCP image with family for easy versioning
sindri packer build --cloud gcp \
  --name sindri-dev \
  --profile fullstack \
  --region us-central1-a

# Deploy latest from family:
# gcloud compute instances create my-vm \
#   --image-family=sindri-dev \
#   --image-project=my-project
```

### Multi-Cloud Pipeline

```bash
#!/bin/bash
# Build images for all major clouds in parallel

clouds=("aws" "azure" "gcp")

for cloud in "${clouds[@]}"; do
  sindri packer build \
    --cloud "$cloud" \
    --name "sindri-$(date +%Y%m%d)" \
    --profile fullstack \
    --json > "build-${cloud}.json" &
done

wait
echo "All builds completed"
```

### CI/CD Integration

```yaml
# .github/workflows/packer-build.yml
name: Build VM Images

on:
  push:
    branches: [main]
    paths:
      - "packer/**"
      - "sindri.yaml"

jobs:
  build:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        cloud: [aws, azure, gcp]

    steps:
      - uses: actions/checkout@v4

      - name: Setup Packer
        uses: hashicorp/setup-packer@main
        with:
          version: "1.10.0"

      - name: Configure ${{ matrix.cloud }} credentials
        run: |
          # Cloud-specific credential setup

      - name: Build Image
        run: |
          sindri packer build \
            --cloud ${{ matrix.cloud }} \
            --name "sindri-${{ github.sha }}" \
            --profile fullstack \
            --json > build-result.json

      - name: Upload Build Result
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.cloud }}-build
          path: build-result.json
```

## Template System

### Tera Templates

Sindri uses [Tera](https://tera.netlify.app/) templates to generate HCL2 Packer configurations. Templates are embedded in the `sindri-packer` crate and rendered at build time.

**Template Location:** `v3/crates/sindri-packer/src/templates/hcl/`

Available templates:

- `aws.pkr.hcl.tera`
- `azure.pkr.hcl.tera`
- `gcp.pkr.hcl.tera`
- `oci.pkr.hcl.tera`
- `alibaba.pkr.hcl.tera`

### Template Variables

Templates receive these variables from configuration:

| Variable               | Description               | Source                      |
| ---------------------- | ------------------------- | --------------------------- |
| `image_name`           | Image name prefix         | config.image_name           |
| `sindri_version`       | Sindri version to install | config.build.sindri_version |
| `description`          | Image description         | config.description          |
| `profile`              | Extension profile         | config.build.profile        |
| `extensions`           | Extension list            | config.build.extensions     |
| `region`               | Cloud region              | Cloud-specific config       |
| `instance_type`        | Build instance type       | Cloud-specific config       |
| `volume_size`          | Disk size in GB           | Cloud-specific config       |
| `ssh_timeout`          | SSH connection timeout    | config.build.ssh_timeout    |
| `cis_hardening`        | Enable CIS hardening      | config.build.security       |
| `clean_sensitive_data` | Clean sensitive data      | config.build.security       |
| `remove_ssh_keys`      | Remove SSH host keys      | config.build.security       |
| `tags`                 | Resource tags             | config.tags                 |
| `environment`          | Environment variables     | config.build.environment    |
| `file_uploads`         | Files to upload           | config.build.file_uploads   |

### Preview Generated Template

```bash
# View generated HCL without building
sindri packer build --cloud aws --dry-run

# Or initialize a template file
sindri packer init --cloud aws
cat aws.pkr.hcl
```

### Custom Template Sections

Generated templates include these provisioning phases:

1. **File Upload** - Upload scripts and custom files
2. **Initialization** - System updates, base packages
3. **Sindri Installation** - Install Sindri with profile/extensions
4. **Security Hardening** - CIS benchmarks (if enabled)
5. **Ansible Provisioning** - Custom playbooks (if configured)
6. **Cleanup** - Remove sensitive data, SSH keys

## Troubleshooting

### Packer Not Found

**Symptom:** `Packer is not installed`

**Solution:**

```bash
# Install Packer
# macOS
brew tap hashicorp/tap
brew install hashicorp/tap/packer

# Linux
wget -O - https://apt.releases.hashicorp.com/gpg | sudo gpg --dearmor -o /usr/share/keyrings/hashicorp-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] https://apt.releases.hashicorp.com $(lsb_release -cs) main" | sudo tee /etc/apt/sources.list.d/hashicorp.list
sudo apt update && sudo apt install packer

# Verify
packer --version
```

### Cloud Credentials Not Configured

**Symptom:** `Credentials not configured` in doctor output

**Solutions by cloud:**

```bash
# AWS
aws configure
# Or use environment variables / IAM role

# Azure
az login
az account set --subscription <subscription-id>

# GCP
gcloud auth application-default login
gcloud config set project <project-id>

# OCI
oci setup config

# Alibaba
aliyun configure
```

### Build Timeout

**Symptom:** Build fails with SSH timeout

**Solution:**

```yaml
providers:
  packer:
    build:
      ssh_timeout: "30m" # Increase timeout
```

Or via CLI:

```bash
# Use a larger instance for faster builds
sindri packer build --cloud aws \
  --instance-type t3.xlarge
```

### Insufficient Permissions

**Symptom:** Access denied errors during build

**AWS Required Permissions:**

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "ec2:CreateImage",
        "ec2:RegisterImage",
        "ec2:DeregisterImage",
        "ec2:DescribeImages",
        "ec2:DescribeInstances",
        "ec2:RunInstances",
        "ec2:TerminateInstances",
        "ec2:CreateTags",
        "ec2:CreateSecurityGroup",
        "ec2:DeleteSecurityGroup",
        "ec2:AuthorizeSecurityGroupIngress",
        "ec2:DescribeSecurityGroups",
        "ec2:CreateKeyPair",
        "ec2:DeleteKeyPair",
        "ec2:DescribeKeyPairs",
        "ec2:DescribeSubnets",
        "ec2:DescribeVpcs"
      ],
      "Resource": "*"
    }
  ]
}
```

### Image Build Failed

**Symptom:** Build fails during provisioning

**Debug steps:**

```bash
# Enable debug mode
sindri packer build --cloud aws --debug

# Review generated template
sindri packer build --cloud aws --dry-run

# Check Packer logs
PACKER_LOG=1 packer build aws.pkr.hcl
```

### Cached Image Not Found

**Symptom:** Build runs despite cache enabled

**Cause:** Image config hash doesn't match existing images

**Solution:**

```bash
# Force use specific image
sindri packer deploy --cloud aws <image-id>

# Or rebuild
sindri packer build --cloud aws --force
```

### VPC/Subnet Issues

**Symptom:** Cannot launch build instance

**Solution:**

```yaml
providers:
  packer:
    aws:
      vpc_id: vpc-0123456789abcdef
      subnet_id: subnet-0123456789abcdef
```

Ensure the subnet:

- Has internet gateway attached (for package downloads)
- Auto-assigns public IPs (or use NAT gateway)
- Security group allows SSH (port 22) from Packer

## Security

### Image Hardening

Enable CIS benchmark hardening for production images:

```yaml
providers:
  packer:
    build:
      security:
        cis_hardening: true
```

**CIS Level 1 hardening includes:**

- Filesystem configuration
- System file permissions
- User authentication settings
- Network configuration hardening
- Logging and auditing
- System access controls

### Pre-Capture Cleanup

Always enabled by default to remove sensitive data before image capture:

```yaml
providers:
  packer:
    build:
      security:
        clean_sensitive_data: true # Default: true
        remove_ssh_keys: true # Default: true
```

**Cleanup removes:**

- Bash history
- SSH authorized_keys
- SSH host keys
- Temporary files
- Package manager cache
- Cloud-init state
- Log files

### OpenSCAP Scanning

Enable security compliance scanning (generates report):

```yaml
providers:
  packer:
    build:
      security:
        openscap_scan: true
```

### Access Control

**AWS AMI Sharing:**

```yaml
providers:
  packer:
    aws:
      ami_users: # Share with specific accounts
        - "123456789012"
        - "987654321098"
      ami_groups: [] # Empty = private (default)
      # ami_groups: ["all"]   # Would make public - USE WITH CAUTION
```

**Azure Shared Image Gallery:**

```yaml
providers:
  packer:
    azure:
      gallery:
        gallery_name: sindri_gallery
        # RBAC controls access to gallery
```

**GCP Image Sharing:**

```yaml
# Managed through IAM roles on the image
# See: https://cloud.google.com/compute/docs/images/managing-access-custom-images
```

### Encryption

**AWS Boot Volume Encryption:**

```yaml
providers:
  packer:
    aws:
      encrypt_boot: true # Default: true
```

**GCP Secure Boot:**

```yaml
providers:
  packer:
    gcp:
      enable_secure_boot: true
```

## Cost Estimates

### Build Costs (per build)

| Cloud   | Instance Type   | Hourly Rate | Typical Build | Est. Cost |
| ------- | --------------- | ----------- | ------------- | --------- |
| AWS     | t3.large        | $0.0832     | 15 min        | ~$0.02    |
| AWS     | t3.xlarge       | $0.1664     | 10 min        | ~$0.03    |
| Azure   | Standard_D4s_v4 | $0.192      | 15 min        | ~$0.05    |
| GCP     | e2-standard-4   | $0.134      | 15 min        | ~$0.03    |
| OCI     | VM.Standard.E4  | $0.10       | 15 min        | ~$0.025   |
| Alibaba | ecs.g6.xlarge   | $0.15       | 15 min        | ~$0.04    |

### Storage Costs (per month, per region)

| Cloud   | Storage Type  | Cost per GB | 80GB Image |
| ------- | ------------- | ----------- | ---------- |
| AWS     | AMI (EBS)     | $0.05       | ~$4.00     |
| Azure   | Managed Image | $0.05       | ~$4.00     |
| GCP     | Image         | $0.050      | ~$4.00     |
| OCI     | Custom Image  | $0.025      | ~$2.00     |
| Alibaba | Custom Image  | $0.034      | ~$2.70     |

**Cost Optimization Tips:**

1. Use caching - don't rebuild if config unchanged
2. Clean up old images regularly
3. Use smaller instance types for builds
4. Build in one region, copy only where needed
5. Schedule builds during off-peak hours

## Related Documentation

- [Provider Overview](README.md)
- [Configuration Reference](../CONFIGURATION.md)
- [Secrets Management](../SECRETS_MANAGEMENT.md)
- [CLI Reference](../CLI.md)
- [HashiCorp Packer Documentation](https://developer.hashicorp.com/packer/docs)
