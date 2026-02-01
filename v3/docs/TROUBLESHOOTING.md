# Sindri V3 Troubleshooting Guide

> **Version:** 3.0.0
> **Last Updated:** January 2026

This guide helps you diagnose and resolve common issues with Sindri V3.

## Table of Contents

- [Overview](#overview)
- [Using sindri doctor](#using-sindri-doctor)
- [Common Issues](#common-issues)
  - [Installation Problems](#installation-problems)
  - [Configuration Issues](#configuration-issues)
  - [Extension Problems](#extension-problems)
  - [Provider-Specific Issues](#provider-specific-issues)
  - [Permission Problems](#permission-problems)
- [Packer Image Building Issues](#packer-image-building-issues)
  - [Prerequisites Issues](#prerequisites-issues)
  - [Build Failures](#build-failures)
  - [Cloud-Specific Issues](#cloud-specific-issues)
  - [Debugging Packer Builds](#debugging-packer-builds)
  - [CI/CD Issues](#cicd-issues)
- [Debugging](#debugging)
- [Getting Help](#getting-help)

---

## Overview

When troubleshooting Sindri V3, follow this general approach:

1. **Run diagnostics** - Use `sindri doctor` to identify missing tools or configuration problems
2. **Check configuration** - Validate your `sindri.yaml` with `sindri config validate`
3. **Enable verbose output** - Add `-v`, `-vv`, or `-vvv` flags for detailed logging
4. **Review exit codes** - Check the command exit code for error categorization
5. **Consult logs** - Review provider-specific logs for deployment issues

### Exit Codes Reference

| Code | Description          |
| ---- | -------------------- |
| 0    | Success              |
| 1    | General error        |
| 2    | Configuration error  |
| 3    | Provider error       |
| 4    | Network error        |
| 5    | Authentication error |

---

## Using sindri doctor

The `sindri doctor` command is your primary diagnostic tool. It performs comprehensive system checks and provides actionable remediation steps.

For complete documentation, see [DOCTOR.md](./DOCTOR.md).

### Quick Diagnostics

```bash
# Check all required tools
sindri doctor

# Check for a specific provider
sindri doctor --provider docker
sindri doctor --provider fly
sindri doctor --provider k8s

# Check with authentication status
sindri doctor --check-auth

# Verbose output with timing
sindri doctor --all --verbose-output

# JSON output for scripting
sindri doctor --ci --format json
```

### Auto-Fix Mode

The doctor can automatically install missing tools:

```bash
# Preview what would be installed
sindri doctor --fix --dry-run

# Install missing tools interactively
sindri doctor --fix

# Non-interactive installation
sindri doctor --fix --yes
```

### CI/CD Integration

```bash
# Exit with non-zero code if required tools are missing
sindri doctor --ci --provider docker
```

Exit codes for CI mode:

| Code | Meaning                                                      |
| ---- | ------------------------------------------------------------ |
| 0    | All required tools available (optional tools may be missing) |
| 1    | Missing required tools                                       |
| 2    | Tools present but version too old                            |
| 3    | Tools present but not authenticated (when auth is required)  |

---

## Common Issues

### Installation Problems

#### CLI Binary Not Found

**Symptom:**

```text
sindri: command not found
```

**Solution:**

Ensure the binary is in your PATH:

```bash
# Check installation location
which sindri

# Add to PATH if needed (add to ~/.bashrc or ~/.zshrc)
export PATH="$PATH:/usr/local/bin"

# Verify
sindri --version
```

#### Permission Denied on Binary

**Symptom:**

```text
bash: /usr/local/bin/sindri: Permission denied
```

**Solution:**

```bash
# Make executable
chmod +x /usr/local/bin/sindri

# Or reinstall with correct permissions
sudo install -m 755 sindri /usr/local/bin/
```

#### macOS Security Block

**Symptom:**

```text
"sindri" cannot be opened because it is from an unidentified developer.
```

**Solution:**

```bash
# Remove quarantine attribute
xattr -d com.apple.quarantine /usr/local/bin/sindri

# Or allow in System Preferences > Security & Privacy
```

---

### Configuration Issues

#### Invalid sindri.yaml

**Symptom:**

```text
Error: Schema validation failed
```

**Diagnosis:**

```bash
# Validate configuration
sindri config validate

# Show resolved configuration
sindri config show

# Check YAML syntax
sindri config validate --file ./sindri.yaml
```

**Common causes:**

1. **Invalid version format:**

   ```yaml
   # Wrong
   version: 3.0

   # Correct
   version: "3.0"
   ```

2. **Invalid name format:**

   ```yaml
   # Wrong - uppercase, underscore
   name: My_Project

   # Correct - lowercase, hyphens only
   name: my-project
   ```

3. **Both profile and active specified:**

   ```yaml
   # Wrong - mutually exclusive
   extensions:
     profile: minimal
     active:
       - nodejs

   # Correct - use one or the other
   extensions:
     profile: minimal
   ```

4. **Unknown provider:**

   ```yaml
   # Valid providers: docker, docker-compose, fly, devpod, e2b, kubernetes
   deployment:
     provider: docker
   ```

#### Configuration Not Found

**Symptom:**

```text
Error: Configuration file not found
```

**Solution:**

Sindri looks for `sindri.yaml` in these locations (in order):

1. `./sindri.yaml` (current directory)
2. `~/.config/sindri/sindri.yaml`
3. `/etc/sindri/sindri.yaml`

Create a configuration or specify the path:

```bash
# Initialize new configuration
sindri config init

# Or specify path
sindri deploy --config /path/to/sindri.yaml
```

#### Image Configuration Errors

**Symptom:**

```text
Error: Failed to resolve image version
```

**Diagnosis:**

```bash
# List available images
sindri image list

# Check version compatibility
sindri image versions --cli-version 3.0.0
```

**Common fixes:**

```yaml
# Use explicit version
image_config:
  registry: ghcr.io/pacphi/sindri
  version: "^3.0.0"
  # Or pin to specific version
  tag_override: v3.0.0
  resolution_strategy: explicit
```

---

### Extension Problems

#### Extension Install Fails

**Symptom:**

```text
Error: Extension 'nodejs' installation failed
```

**Diagnosis:**

```bash
# Validate extension definition
sindri extension validate nodejs

# Check extension info
sindri extension info nodejs

# List installed extensions
sindri extension list --installed
```

**Common causes:**

1. **Missing dependencies:**

   ```bash
   # Check dependencies
   sindri extension info nodejs --json | jq '.dependencies'

   # Install dependencies first
   sindri extension install mise
   ```

2. **Network issues:**

   ```bash
   # Test connectivity
   curl -I https://registry.npmjs.org

   # Check proxy settings
   echo $HTTP_PROXY
   echo $HTTPS_PROXY
   ```

3. **Insufficient disk space:**

   ```bash
   df -h
   ```

#### Extension Validation Fails

**Symptom:**

```text
Error: Extension 'my-extension' validation failed
```

**Solution:**

```bash
# Validate with verbose output
sindri extension validate my-extension -v

# Check extension file syntax
sindri extension validate --file ./extension.yaml
```

#### Extension Not Found After Install

**Symptom:**

Tools installed by extension not available.

**Solution:**

```bash
# Restart shell or source profile
source ~/.bashrc
# or
source ~/.zshrc

# Check mise shims
mise list
mise doctor
```

---

### Provider-Specific Issues

#### Docker Provider

**Docker not running:**

```text
Error: Docker is not running. Please start Docker and try again.
```

**Solution:**

```bash
# Check Docker status
docker info

# Start Docker (macOS)
open /Applications/Docker.app

# Start Docker (Linux)
sudo systemctl start docker
```

**Permission denied:**

```text
Error: permission denied while trying to connect to the Docker daemon socket
```

**Solution:**

```bash
# Add user to docker group (Linux)
sudo usermod -aG docker $USER

# Apply group change
newgrp docker

# Verify
docker ps
```

**Container build fails:**

```bash
# Check Docker disk space
docker system df

# Clean up unused resources
docker system prune -a

# Rebuild without cache
docker build --no-cache .
```

#### Fly.io Provider

**Not authenticated:**

```text
Error: Not authenticated with Fly.io
```

**Solution:**

```bash
# Check authentication
flyctl auth whoami

# Login
flyctl auth login
```

**App name taken:**

```text
Error: App name 'my-app' is already taken
```

**Solution:**

Update the name in `sindri.yaml`:

```yaml
name: my-unique-app-name
```

**Region not available:**

```bash
# List available regions
flyctl platform regions

# Update sindri.yaml
providers:
  fly:
    region: iad  # Use available region
```

**SSH connection refused:**

```text
Error: ssh: connect to host my-app.fly.dev port 10022: Connection refused
```

**Diagnosis:**

```bash
# Check machine status
flyctl status -a my-app

# Check if suspended
flyctl machine list -a my-app

# Start machine if needed
flyctl machine start <machine-id> -a my-app
```

#### Kubernetes Provider

**kubectl not configured:**

```text
Error: kubectl is not configured
```

**Solution:**

```bash
# Check kubectl configuration
kubectl config current-context

# Set context
kubectl config use-context my-cluster

# Test connectivity
kubectl cluster-info
```

**Namespace not found:**

```text
Error: namespace 'sindri' not found
```

**Solution:**

```bash
# Create namespace
kubectl create namespace sindri

# Or update sindri.yaml
providers:
  kubernetes:
    namespace: default
```

**Image pull fails:**

```bash
# Check image pull secrets
kubectl get secrets

# Create registry secret if needed
kubectl create secret docker-registry ghcr-secret \
  --docker-server=ghcr.io \
  --docker-username=$GITHUB_USER \
  --docker-password=$GITHUB_TOKEN
```

#### DevPod Provider

**Provider not installed:**

```text
Error: DevPod provider 'aws' is not installed
```

**Solution:**

```bash
# List available providers
devpod provider list

# Install provider
devpod provider add aws
```

**Build repository required:**

```text
Error: buildRepository is required for cloud deployments
```

**Solution:**

Update `sindri.yaml`:

```yaml
providers:
  devpod:
    type: aws
    buildRepository: ghcr.io/myorg/sindri
```

#### E2B Provider

**Not authenticated:**

```text
Error: E2B authentication required
```

**Solution:**

```bash
# Check authentication
e2b auth status

# Login
e2b auth login
```

**GPU not supported:**

```text
Error: GPU configuration not supported by E2B provider
```

E2B does not support GPU. Remove GPU configuration:

```yaml
deployment:
  resources:
    memory: 4GB
    cpus: 2
    # Remove gpu section for E2B
```

---

### Permission Problems

#### Volume Permission Denied

**Symptom:**

```text
Error: Permission denied: /home/developer/workspace
```

**Solution (Docker):**

```bash
# Fix ownership (run inside container as root)
docker exec -u root <container> chown -R developer:developer /home/developer/workspace

# Or recreate volume
docker volume rm sindri-workspace
sindri deploy
```

**Solution (Kubernetes):**

Check security context in deployment:

```yaml
securityContext:
  runAsUser: 1000
  runAsGroup: 1000
  fsGroup: 1000
```

#### SSH Key Permission Denied

**Symptom:**

```text
Error: Permission denied (publickey)
```

**Solution:**

```bash
# Check SSH key exists
ls -la ~/.ssh/id_*.pub

# Generate if needed
ssh-keygen -t ed25519 -C "your@email.com"

# Add to ssh-agent
eval "$(ssh-agent -s)"
ssh-add ~/.ssh/id_ed25519

# For Fly.io
flyctl ssh issue --agent -a my-app
```

#### Sudo/Apt Installation Fails in Container

**Symptom:**

```text
sudo: effective uid is not 0, is /usr/bin/sudo on a file system with the 'nosuid' option set or an NFS file system without root privileges?
```

or:

```text
sudo: PERM_SUDOERS: setresuid(-1, 1, -1): Operation not permitted
```

**Cause:**

The container is running with `no-new-privileges` security flag, which blocks sudo. This happens in **socket** mode (when sharing the host Docker daemon).

**Solution:**

Check the DinD mode in your deployment:

```bash
# Check current DinD mode
grep SINDRI_DIND_MODE docker-compose.yml
```

| Mode         | sudo Works? | Description                         |
| ------------ | ----------- | ----------------------------------- |
| `none`       | YES         | Default development mode            |
| `sysbox`     | YES         | User namespace isolation            |
| `privileged` | YES         | Legacy DinD                         |
| `socket`     | NO          | Production security (shared daemon) |

**Options if sudo is blocked:**

1. **Use a different DinD mode** (if you need sudo):

   ```yaml
   # sindri.yaml
   providers:
     docker:
       dind:
         mode: none # or sysbox
   ```

2. **Use sudo-free installation methods** (recommended for production):
   - Extensions like `cloud-tools` use pip and tarball extraction
   - See [ADR-041](architecture/adr/041-security-hardened-extension-installation.md) for patterns

3. **Pre-install at build time**:
   - Add apt packages to your Dockerfile
   - Use `extensions.profile` in sindri.yaml for build-time installation

**Security Note:**

Socket mode applies `no-new-privileges` intentionally - when sharing the host Docker daemon, preventing privilege escalation is important. For development, use `none` or `sysbox` mode to allow sudo.

---

## Packer Image Building Issues

This section covers troubleshooting for Packer-based image builds used by Sindri V3 for multi-cloud deployments.

### Prerequisites Issues

#### Packer Not Installed

**Symptom:**

```text
Error: Packer is not installed or not in PATH
```

**Solution:**

```bash
# Check if installed
packer version

# Install via package manager (macOS)
brew tap hashicorp/tap
brew install hashicorp/tap/packer

# Install via package manager (Ubuntu/Debian)
curl -fsSL https://apt.releases.hashicorp.com/gpg | sudo gpg --dearmor -o /usr/share/keyrings/hashicorp-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] https://apt.releases.hashicorp.com $(lsb_release -cs) main" | sudo tee /etc/apt/sources.list.d/hashicorp.list
sudo apt update && sudo apt install packer

# Verify installation
packer version
```

#### Cloud CLIs Not Installed

**Symptom:**

```text
Error: AWS CLI not found
Error: Azure CLI not found
Error: gcloud CLI not found
```

**Solution:**

```bash
# AWS CLI
curl "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip" -o "awscliv2.zip"
unzip awscliv2.zip
sudo ./aws/install
aws --version

# Azure CLI
curl -sL https://aka.ms/InstallAzureCLIDeb | sudo bash
az --version

# Google Cloud CLI
curl https://sdk.cloud.google.com | bash
exec -l $SHELL
gcloud --version

# OCI CLI
bash -c "$(curl -L https://raw.githubusercontent.com/oracle/oci-cli/master/scripts/install/install.sh)"
oci --version

# Alibaba Cloud CLI
curl -sSL https://aliyuncli.alicdn.com/aliyun-cli-linux-latest-amd64.tgz | tar -xz
sudo mv aliyun /usr/local/bin/
aliyun --version
```

#### Cloud Credentials Not Configured

**Symptom:**

```text
Error: No valid credential sources found
Error: Unable to locate credentials
```

**Solution:**

```bash
# AWS - Configure credentials
aws configure
# Or use environment variables
export AWS_ACCESS_KEY_ID="your-access-key"
export AWS_SECRET_ACCESS_KEY="your-secret-key"
export AWS_DEFAULT_REGION="us-east-1"

# Azure - Login
az login
az account set --subscription "your-subscription-id"

# GCP - Authenticate
gcloud auth login
gcloud auth application-default login
gcloud config set project your-project-id

# OCI - Configure
oci setup config
# Or set environment variable
export OCI_CLI_CONFIG_FILE="~/.oci/config"

# Alibaba - Configure
aliyun configure
# Or use environment variables
export ALICLOUD_ACCESS_KEY="your-access-key"
export ALICLOUD_SECRET_KEY="your-secret-key"
export ALICLOUD_REGION="cn-hangzhou"
```

---

### Build Failures

#### Template Validation Errors

**Symptom:**

```text
Error: Failed to parse template
Error: Unknown variable: source_ami
```

**Diagnosis:**

```bash
# Validate the template
packer validate -var-file=variables.pkrvars.hcl template.pkr.hcl

# Format check
packer fmt -check template.pkr.hcl

# Initialize plugins
packer init template.pkr.hcl
```

**Common causes:**

1. **Missing required variables:**

   ```hcl
   # Ensure all required variables are defined
   variable "source_ami" {
     type        = string
     description = "Source AMI ID"
   }
   ```

2. **Plugin not installed:**

   ```bash
   # Initialize plugins
   packer init .

   # Or manually install
   packer plugins install github.com/hashicorp/amazon
   ```

3. **Syntax errors:**

   ```bash
   # Auto-format and check
   packer fmt -diff template.pkr.hcl
   ```

#### SSH Connection Timeouts

**Symptom:**

```text
Error: Timeout waiting for SSH
Error: ssh: handshake failed: ssh: unable to authenticate
```

**Solution:**

1. **Increase timeout:**

   ```hcl
   source "amazon-ebs" "example" {
     ssh_timeout = "30m"
     ssh_handshake_attempts = 100
   }
   ```

2. **Check security group rules:**

   ```bash
   # AWS - Ensure SSH is allowed
   aws ec2 describe-security-groups --group-ids sg-xxxxx

   # Packer creates temporary security groups - ensure VPC allows this
   ```

3. **Verify SSH username:**

   ```hcl
   # Different AMIs use different usernames
   source "amazon-ebs" "ubuntu" {
     ssh_username = "ubuntu"  # Ubuntu
   }
   source "amazon-ebs" "amazon-linux" {
     ssh_username = "ec2-user"  # Amazon Linux
   }
   source "amazon-ebs" "debian" {
     ssh_username = "admin"  # Debian
   }
   ```

4. **Check network connectivity:**

   ```hcl
   # Use public IP for SSH
   source "amazon-ebs" "example" {
     associate_public_ip_address = true
   }
   ```

#### Provisioner Script Failures

**Symptom:**

```text
Error: Script exited with non-zero exit status: 1
Error: Command not found
```

**Diagnosis:**

```bash
# Test script locally first
bash -x ./scripts/setup.sh

# Check script permissions
ls -la ./scripts/

# Ensure script has correct line endings
file ./scripts/setup.sh
# Should show: ASCII text executable
# NOT: ASCII text executable, with CRLF line terminators

# Fix line endings if needed
sed -i 's/\r$//' ./scripts/setup.sh
```

**Common fixes:**

1. **Add error handling to scripts:**

   ```bash
   #!/bin/bash
   set -euo pipefail

   # Your provisioning commands
   ```

2. **Wait for cloud-init:**

   ```hcl
   provisioner "shell" {
     inline = [
       "cloud-init status --wait",
       "sudo apt-get update"
     ]
   }
   ```

3. **Handle apt lock:**

   ```bash
   # Wait for apt lock
   while sudo fuser /var/lib/apt/lists/lock >/dev/null 2>&1; do
     sleep 1
   done
   sudo apt-get update
   ```

#### Cloud API Rate Limiting

**Symptom:**

```text
Error: Rate exceeded
Error: RequestLimitExceeded
Error: Too many requests
```

**Solution:**

1. **Add retry configuration:**

   ```hcl
   # AWS
   source "amazon-ebs" "example" {
     aws_polling {
       delay_seconds = 30
       max_attempts  = 50
     }
   }
   ```

2. **Stagger parallel builds:**

   ```bash
   # Run builds sequentially instead of parallel
   packer build -parallel-builds=1 .
   ```

3. **Implement backoff in scripts:**

   ```bash
   # Exponential backoff for API calls
   retry_command() {
     local max_attempts=5
     local timeout=1
     local attempt=0
     while [[ $attempt -lt $max_attempts ]]; do
       if "$@"; then
         return 0
       fi
       attempt=$((attempt + 1))
       sleep $timeout
       timeout=$((timeout * 2))
     done
     return 1
   }
   ```

---

### Cloud-Specific Issues

#### AWS Issues

**IAM Permissions Missing:**

```text
Error: UnauthorizedOperation: You are not authorized to perform this operation
```

**Required IAM permissions:**

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "ec2:AttachVolume",
        "ec2:AuthorizeSecurityGroupIngress",
        "ec2:CopyImage",
        "ec2:CreateImage",
        "ec2:CreateKeypair",
        "ec2:CreateSecurityGroup",
        "ec2:CreateSnapshot",
        "ec2:CreateTags",
        "ec2:CreateVolume",
        "ec2:DeleteKeyPair",
        "ec2:DeleteSecurityGroup",
        "ec2:DeleteSnapshot",
        "ec2:DeleteVolume",
        "ec2:DeregisterImage",
        "ec2:DescribeImageAttribute",
        "ec2:DescribeImages",
        "ec2:DescribeInstances",
        "ec2:DescribeInstanceStatus",
        "ec2:DescribeRegions",
        "ec2:DescribeSecurityGroups",
        "ec2:DescribeSnapshots",
        "ec2:DescribeSubnets",
        "ec2:DescribeTags",
        "ec2:DescribeVolumes",
        "ec2:DetachVolume",
        "ec2:GetPasswordData",
        "ec2:ModifyImageAttribute",
        "ec2:ModifyInstanceAttribute",
        "ec2:ModifySnapshotAttribute",
        "ec2:RegisterImage",
        "ec2:RunInstances",
        "ec2:StopInstances",
        "ec2:TerminateInstances"
      ],
      "Resource": "*"
    }
  ]
}
```

**VPC/Subnet Issues:**

```text
Error: No default VPC found
Error: Subnet not found
```

**Solution:**

```hcl
# Explicitly specify VPC and subnet
source "amazon-ebs" "example" {
  vpc_id    = "vpc-xxxxxxxx"
  subnet_id = "subnet-xxxxxxxx"

  # Or use filters
  subnet_filter {
    filters = {
      "tag:Name" = "packer-subnet"
    }
    most_free = true
    random    = false
  }
}
```

**AMI Quota Exceeded:**

```text
Error: AMI quota exceeded
```

**Solution:**

```bash
# List existing AMIs
aws ec2 describe-images --owners self --query 'Images[*].[ImageId,Name,CreationDate]' --output table

# Deregister old AMIs
aws ec2 deregister-image --image-id ami-xxxxxxxx

# Delete associated snapshots
aws ec2 delete-snapshot --snapshot-id snap-xxxxxxxx

# Request quota increase
aws service-quotas request-service-quota-increase \
  --service-code ec2 \
  --quota-code L-0E3CBAB9 \
  --desired-value 100
```

#### Azure Issues

**RBAC Roles Missing:**

```text
Error: AuthorizationFailed
Error: The client does not have authorization to perform action
```

**Required role assignments:**

```bash
# Assign Contributor role
az role assignment create \
  --assignee <service-principal-id> \
  --role "Contributor" \
  --scope /subscriptions/<subscription-id>

# For Shared Image Gallery, also assign:
az role assignment create \
  --assignee <service-principal-id> \
  --role "Compute Gallery Sharing Admin" \
  --scope /subscriptions/<subscription-id>
```

**Resource Group Access:**

```text
Error: Resource group not found
```

**Solution:**

```bash
# Create resource group if needed
az group create --name packer-rg --location eastus

# Verify access
az group show --name packer-rg
```

```hcl
# Specify in Packer template
source "azure-arm" "example" {
  managed_image_resource_group_name = "packer-rg"
  build_resource_group_name         = "packer-build-rg"
}
```

**Shared Image Gallery Issues:**

```text
Error: Could not find gallery
Error: Image version already exists
```

**Solution:**

```bash
# Create gallery if needed
az sig create \
  --resource-group packer-rg \
  --gallery-name myGallery

# Create image definition
az sig image-definition create \
  --resource-group packer-rg \
  --gallery-name myGallery \
  --gallery-image-definition sindri-v3 \
  --publisher sindri \
  --offer sindri \
  --sku v3 \
  --os-type Linux

# List existing versions
az sig image-version list \
  --resource-group packer-rg \
  --gallery-name myGallery \
  --gallery-image-definition sindri-v3
```

#### GCP Issues

**API Not Enabled:**

```text
Error: googleapi: Error 403: Compute Engine API has not been used in project
```

**Solution:**

```bash
# Enable required APIs
gcloud services enable compute.googleapis.com
gcloud services enable storage.googleapis.com
gcloud services enable iam.googleapis.com

# List enabled APIs
gcloud services list --enabled
```

**Service Account Issues:**

```text
Error: Could not fetch access token
Error: Permission denied on resource project
```

**Solution:**

```bash
# Create service account
gcloud iam service-accounts create packer-builder \
  --display-name="Packer Builder"

# Grant required roles
gcloud projects add-iam-policy-binding PROJECT_ID \
  --member="serviceAccount:packer-builder@PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/compute.instanceAdmin.v1"

gcloud projects add-iam-policy-binding PROJECT_ID \
  --member="serviceAccount:packer-builder@PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/iam.serviceAccountUser"

# Create and download key
gcloud iam service-accounts keys create packer-sa-key.json \
  --iam-account=packer-builder@PROJECT_ID.iam.gserviceaccount.com

# Set environment variable
export GOOGLE_APPLICATION_CREDENTIALS="$(pwd)/packer-sa-key.json"
```

**Quota Exceeded:**

```text
Error: Quota 'CPUS' exceeded
Error: Quota 'IMAGES' exceeded
```

**Solution:**

```bash
# Check current quotas
gcloud compute project-info describe --project PROJECT_ID

# Request quota increase via console or
gcloud compute project-info update PROJECT_ID \
  --set-limits=compute.googleapis.com/cpus=100
```

#### OCI Issues

**Compartment Permissions:**

```text
Error: Authorization failed or requested resource not found
```

**Solution:**

```bash
# Verify compartment OCID
oci iam compartment list --compartment-id-in-subtree true

# Create policy for Packer
cat > packer-policy.json << 'EOF'
Allow group PackerBuilders to manage instance-family in compartment MyCompartment
Allow group PackerBuilders to manage image-family in compartment MyCompartment
Allow group PackerBuilders to use virtual-network-family in compartment MyCompartment
Allow group PackerBuilders to manage volume-family in compartment MyCompartment
EOF
```

```hcl
# Specify compartment in template
source "oracle-oci" "example" {
  compartment_ocid = "ocid1.compartment.oc1..xxxxx"
  base_image_ocid  = "ocid1.image.oc1..xxxxx"
}
```

**Shape Availability:**

```text
Error: Shape not available in availability domain
```

**Solution:**

```bash
# List available shapes in region
oci compute shape list --compartment-id <compartment-ocid>

# Check availability domain capacity
oci compute shape list \
  --compartment-id <compartment-ocid> \
  --availability-domain <ad-name>
```

```hcl
# Try different availability domains
source "oracle-oci" "example" {
  availability_domain = "AD-2"  # Try different AD
  shape               = "VM.Standard.E4.Flex"
  shape_config {
    ocpus         = 2
    memory_in_gbs = 8
  }
}
```

#### Alibaba Cloud Issues

**RAM Policies Missing:**

```text
Error: Forbidden.RAM
Error: You are not authorized to operate on the specified resource
```

**Required RAM policy:**

```json
{
  "Version": "1",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "ecs:CreateInstance",
        "ecs:DeleteInstance",
        "ecs:StartInstance",
        "ecs:StopInstance",
        "ecs:DescribeInstances",
        "ecs:CreateImage",
        "ecs:DeleteImage",
        "ecs:DescribeImages",
        "ecs:CreateSecurityGroup",
        "ecs:DeleteSecurityGroup",
        "ecs:AuthorizeSecurityGroup",
        "ecs:DescribeSecurityGroups",
        "ecs:CreateKeyPair",
        "ecs:DeleteKeyPair",
        "ecs:DescribeKeyPairs",
        "vpc:DescribeVpcs",
        "vpc:DescribeVSwitches"
      ],
      "Resource": "*"
    }
  ]
}
```

**Region Restrictions:**

```text
Error: Region not supported
Error: The specified region does not exist
```

**Solution:**

```bash
# List available regions
aliyun ecs DescribeRegions

# Use valid region in template
```

```hcl
source "alicloud-ecs" "example" {
  region = "cn-hangzhou"  # Use valid region
}
```

**EIP Quota Exceeded:**

```text
Error: QuotaExceeded.Eip
```

**Solution:**

```bash
# Check EIP quota
aliyun vpc DescribeEipAddresses --RegionId cn-hangzhou

# Release unused EIPs or request quota increase

# Or use VPC without public IP
```

```hcl
source "alicloud-ecs" "example" {
  associate_public_ip_address = false
  # Use private network with NAT gateway
}
```

---

### Debugging Packer Builds

#### Using the Debug Flag

```bash
# Enable debug mode (step-by-step with pauses)
packer build -debug template.pkr.hcl

# Debug mode features:
# - Pauses between each step
# - Saves SSH private key for manual inspection
# - Press Enter to continue to next step
```

#### Reading Packer Logs

```bash
# Enable detailed logging
export PACKER_LOG=1
export PACKER_LOG_PATH="packer.log"

packer build template.pkr.hcl

# View logs
tail -f packer.log

# Search for errors
grep -i error packer.log
grep -i failed packer.log
```

#### Log Levels

```bash
# Trace level for maximum detail
export PACKER_LOG=1

# Machine-readable output
packer build -machine-readable template.pkr.hcl | tee build.log
```

#### Common Error Messages and Solutions

| Error Message               | Cause                        | Solution                                        |
| --------------------------- | ---------------------------- | ----------------------------------------------- |
| `VPCIdNotSpecified`         | No default VPC               | Add `vpc_id` to template                        |
| `InvalidAMIID.NotFound`     | AMI doesn't exist in region  | Verify AMI ID and region                        |
| `UnauthorizedAccess`        | Missing IAM permissions      | Add required IAM policies                       |
| `ssh: handshake failed`     | SSH key or username issue    | Check `ssh_username` and security groups        |
| `context deadline exceeded` | Timeout waiting for resource | Increase timeout values                         |
| `provisioner script failed` | Script error                 | Test script locally, check exit codes           |
| `quota exceeded`            | Resource limit reached       | Request quota increase or cleanup old resources |
| `InvalidParameterValue`     | Invalid configuration value  | Validate template with `packer validate`        |

#### Inspecting Failed Builds

```bash
# Keep instance running on failure (AWS example)
source "amazon-ebs" "debug" {
  # ... other config ...
  skip_create_ami = true  # Don't create AMI, just run provisioners

  # SSH for manual inspection
  ssh_keep_alive_interval = "5s"
}
```

```bash
# Connect manually after failure
ssh -i /path/to/debug-key.pem ubuntu@<instance-ip>

# Check provisioning logs
sudo cat /var/log/cloud-init-output.log
sudo journalctl -xe
```

---

### CI/CD Issues

#### OIDC Authentication Failures

**Symptom:**

```text
Error: Unable to assume role with OIDC
Error: Not authorized to perform sts:AssumeRoleWithWebIdentity
```

**AWS OIDC Setup:**

```bash
# Create OIDC provider (one-time)
aws iam create-open-id-connect-provider \
  --url https://token.actions.githubusercontent.com \
  --client-id-list sts.amazonaws.com \
  --thumbprint-list 6938fd4d98bab03faadb97b34396831e3780aea1

# Create trust policy
cat > trust-policy.json << 'EOF'
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "Federated": "arn:aws:iam::ACCOUNT_ID:oidc-provider/token.actions.githubusercontent.com"
      },
      "Action": "sts:AssumeRoleWithWebIdentity",
      "Condition": {
        "StringEquals": {
          "token.actions.githubusercontent.com:aud": "sts.amazonaws.com"
        },
        "StringLike": {
          "token.actions.githubusercontent.com:sub": "repo:ORG/REPO:*"
        }
      }
    }
  ]
}
EOF

# Create role
aws iam create-role \
  --role-name GitHubActionsPacker \
  --assume-role-policy-document file://trust-policy.json
```

**Azure OIDC Setup:**

```bash
# Create federated credential
az ad app federated-credential create \
  --id <application-id> \
  --parameters '{
    "name": "GitHubActionsFederated",
    "issuer": "https://token.actions.githubusercontent.com",
    "subject": "repo:ORG/REPO:ref:refs/heads/main",
    "audiences": ["api://AzureADTokenExchange"]
  }'
```

**GCP OIDC Setup:**

```bash
# Create workload identity pool
gcloud iam workload-identity-pools create "github-pool" \
  --location="global" \
  --display-name="GitHub Actions Pool"

# Create provider
gcloud iam workload-identity-pools providers create-oidc "github-provider" \
  --location="global" \
  --workload-identity-pool="github-pool" \
  --display-name="GitHub Provider" \
  --attribute-mapping="google.subject=assertion.sub,attribute.repository=assertion.repository" \
  --issuer-uri="https://token.actions.githubusercontent.com"

# Grant access to service account
gcloud iam service-accounts add-iam-policy-binding packer-builder@PROJECT_ID.iam.gserviceaccount.com \
  --role="roles/iam.workloadIdentityUser" \
  --member="principalSet://iam.googleapis.com/projects/PROJECT_NUMBER/locations/global/workloadIdentityPools/github-pool/attribute.repository/ORG/REPO"
```

#### GitHub Actions Secrets Configuration

**Required secrets for multi-cloud builds:**

```yaml
# AWS
AWS_ROLE_ARN: arn:aws:iam::123456789:role/GitHubActionsPacker

# Azure
AZURE_CLIENT_ID: <app-client-id>
AZURE_TENANT_ID: <tenant-id>
AZURE_SUBSCRIPTION_ID: <subscription-id>

# GCP
GCP_WORKLOAD_IDENTITY_PROVIDER: projects/123456/locations/global/workloadIdentityPools/github-pool/providers/github-provider
GCP_SERVICE_ACCOUNT: packer-builder@project-id.iam.gserviceaccount.com

# OCI
OCI_CLI_USER: <user-ocid>
OCI_CLI_TENANCY: <tenancy-ocid>
OCI_CLI_FINGERPRINT: <key-fingerprint>
OCI_CLI_KEY_CONTENT: <private-key-base64>
OCI_CLI_REGION: us-ashburn-1

# Alibaba
ALICLOUD_ACCESS_KEY: <access-key>
ALICLOUD_SECRET_KEY: <secret-key>
ALICLOUD_REGION: cn-hangzhou
```

**Verify secrets are set:**

```bash
# Check repository secrets (requires admin access)
gh secret list --repo ORG/REPO
```

#### Workflow Trigger Problems

**Workflow not running:**

```text
Workflow not triggered on push/PR
```

**Check workflow triggers:**

```yaml
# .github/workflows/packer-build.yml
on:
  push:
    branches: [main]
    paths:
      - "packer/**"
      - ".github/workflows/packer-build.yml"
  pull_request:
    branches: [main]
    paths:
      - "packer/**"
  workflow_dispatch: # Manual trigger


# Ensure branch protection allows workflow
```

**Debugging workflow issues:**

```bash
# Check workflow runs
gh run list --repo ORG/REPO --workflow=packer-build.yml

# View specific run logs
gh run view <run-id> --log

# Re-run failed workflow
gh run rerun <run-id>
```

**Concurrency issues:**

```yaml
# Prevent concurrent builds
concurrency:
  group: packer-${{ github.ref }}
  cancel-in-progress: true
```

**Timeout issues:**

```yaml
jobs:
  build:
    timeout-minutes: 120 # Increase for long builds
    steps:
      - name: Build images
        timeout-minutes: 90 # Per-step timeout
        run: packer build .
```

---

## Debugging

### Enable Verbose Output

Use verbosity flags for detailed output:

```bash
# Minimal verbosity
sindri deploy -v

# Medium verbosity
sindri deploy -vv

# Maximum verbosity (debug level)
sindri deploy -vvv
```

### Environment Variables

Set logging level via environment:

```bash
# Set log level
export SINDRI_LOG_LEVEL=debug

# Trace level for maximum detail
export SINDRI_LOG_LEVEL=trace

sindri deploy
```

Available log levels: `trace`, `debug`, `info`, `warn`, `error`

### Dry Run Mode

Preview changes without executing:

```bash
# Deploy dry run
sindri deploy --dry-run

# Backup dry run
sindri backup --dry-run

# Restore dry run
sindri restore backup.tar.gz --dry-run
```

### Collect Diagnostic Information

When reporting issues, collect this information:

```bash
# System information
sindri version --json
uname -a
docker version 2>/dev/null || echo "Docker not installed"
kubectl version --client 2>/dev/null || echo "kubectl not installed"

# Doctor output
sindri doctor --all --format json

# Configuration (sanitize secrets!)
sindri config show

# Extension status
sindri extension status --json
```

### Provider-Specific Logs

**Docker:**

```bash
docker logs <container-name>
docker logs -f <container-name>  # Follow logs
```

**Fly.io:**

```bash
flyctl logs -a my-app
flyctl logs -a my-app --region sjc
```

**Kubernetes:**

```bash
kubectl logs deployment/my-app
kubectl logs -f deployment/my-app
kubectl describe pod <pod-name>
```

---

## Getting Help

### Check Documentation

- [CLI Reference](./CLI.md) - Complete command documentation
- [Configuration Reference](./CONFIGURATION.md) - sindri.yaml schema
- [Doctor Guide](./DOCTOR.md) - Diagnostic command details
- [Getting Started](./GETTING_STARTED.md) - Installation and first steps
- [Image Management](./IMAGE_MANAGEMENT.md) - Container image security
- [Secrets Management](./SECRETS_MANAGEMENT.md) - Secrets configuration

### Report Issues

When reporting issues on GitHub, include:

1. **Sindri version:** `sindri version --json`
2. **Error message:** Full error output
3. **Configuration:** `sindri.yaml` (remove secrets!)
4. **Provider:** Which provider you are using
5. **Doctor output:** `sindri doctor --all --format json`
6. **Steps to reproduce:** Minimal steps to trigger the issue

**GitHub Issues:** https://github.com/pacphi/sindri/issues

### Community Resources

- **GitHub Discussions:** https://github.com/pacphi/sindri/discussions
- **FAQ:** https://sindri-faq.fly.dev

### Search Existing Issues

Before creating a new issue, search for existing solutions:

```bash
# Search GitHub issues
gh issue list --repo pacphi/sindri --search "your error message"
```

---

## Related Documentation

- [CLI Reference](./CLI.md)
- [Configuration Reference](./CONFIGURATION.md)
- [Doctor Guide](./DOCTOR.md)
- [Getting Started](./GETTING_STARTED.md)
- [Image Management](./IMAGE_MANAGEMENT.md)
- [Secrets Management](./SECRETS_MANAGEMENT.md)
