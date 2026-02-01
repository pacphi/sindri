# GCP Packer Provider

> **Version:** 3.x
> **Last Updated:** 2026-02

Build and deploy VM images on Google Cloud Platform using Sindri and HashiCorp Packer.

## Overview

The GCP Packer provider enables building golden VM images on Google Compute Engine with:

- **Image Families** - Semantic versioning with automatic latest-image resolution
- **Shielded VMs** - Secure Boot, vTPM, and integrity monitoring
- **Cross-Project Sharing** - IAM-based image access control
- **Fast Provisioning** - Pre-baked Sindri environments with extensions
- **Workload Identity** - Secure, keyless CI/CD authentication

**Best for:** GCP-native deployments, enterprise environments, Kubernetes-integrated workflows

## Prerequisites

### Required Software

| Requirement      | Version | Check Command      | Install                                        |
| ---------------- | ------- | ------------------ | ---------------------------------------------- |
| HashiCorp Packer | 1.9+    | `packer --version` | https://developer.hashicorp.com/packer/install |
| gcloud CLI       | Latest  | `gcloud --version` | https://cloud.google.com/sdk/docs/install      |

### GCP Project Setup

```bash
# 1. Install gcloud CLI (if not installed)
# macOS
brew install --cask google-cloud-sdk

# Linux (Debian/Ubuntu)
echo "deb [signed-by=/usr/share/keyrings/cloud.google.gpg] https://packages.cloud.google.com/apt cloud-sdk main" | \
  sudo tee -a /etc/apt/sources.list.d/google-cloud-sdk.list
curl https://packages.cloud.google.com/apt/doc/apt-key.gpg | \
  sudo apt-key --keyring /usr/share/keyrings/cloud.google.gpg add -
sudo apt-get update && sudo apt-get install google-cloud-cli

# 2. Initialize gcloud
gcloud init

# 3. Set default project
gcloud config set project YOUR_PROJECT_ID

# 4. Enable required APIs
gcloud services enable compute.googleapis.com
gcloud services enable iam.googleapis.com
gcloud services enable cloudresourcemanager.googleapis.com
```

## IAM Roles

### Predefined Roles (Recommended for Quick Setup)

Grant these roles to the user or service account building images:

| Role                             | Purpose                            |
| -------------------------------- | ---------------------------------- |
| `roles/compute.instanceAdmin.v1` | Create and manage build instances  |
| `roles/compute.imageUser`        | Use and create images              |
| `roles/iam.serviceAccountUser`   | Act as the compute service account |

```bash
# Grant roles to a user
gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="user:developer@example.com" \
  --role="roles/compute.instanceAdmin.v1"

gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="user:developer@example.com" \
  --role="roles/compute.imageUser"

gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="user:developer@example.com" \
  --role="roles/iam.serviceAccountUser"

# Grant roles to a service account
gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:packer-builder@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/compute.instanceAdmin.v1"

gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:packer-builder@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/compute.imageUser"

gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:packer-builder@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/iam.serviceAccountUser"
```

### Custom Role (Minimum Permissions)

For production environments with least-privilege requirements, create a custom role:

```bash
# Create custom role definition
cat > packer-builder-role.yaml << 'EOF'
title: "Packer Image Builder"
description: "Minimum permissions for building VM images with Packer"
stage: "GA"
includedPermissions:
  - compute.disks.create
  - compute.disks.delete
  - compute.disks.useReadOnly
  - compute.images.create
  - compute.images.delete
  - compute.images.get
  - compute.images.list
  - compute.images.setIamPolicy
  - compute.instances.create
  - compute.instances.delete
  - compute.instances.get
  - compute.instances.setMetadata
  - compute.instances.setServiceAccount
  - compute.machineTypes.get
  - compute.networks.get
  - compute.projects.get
  - compute.subnetworks.use
  - compute.subnetworks.useExternalIp
  - compute.zones.get
EOF

# Create the custom role
gcloud iam roles create packerImageBuilder \
  --project=YOUR_PROJECT_ID \
  --file=packer-builder-role.yaml

# Assign the custom role
gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="user:developer@example.com" \
  --role="projects/YOUR_PROJECT_ID/roles/packerImageBuilder"
```

### Additional Permissions for Image Sharing

If sharing images across projects or publicly:

```bash
# For cross-project sharing
gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:packer-builder@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/compute.imageAdmin"

# Permission to set IAM policies on images
# (included in compute.imageAdmin or add to custom role)
# compute.images.setIamPolicy
```

## Authentication Methods

### Method 1: Interactive Login (Development)

Best for local development and testing:

```bash
# Interactive browser login
gcloud auth login

# Set application default credentials (required for Packer)
gcloud auth application-default login

# Verify authentication
gcloud auth print-access-token
```

### Method 2: Application Default Credentials

For applications and scripts:

```bash
# Login and set ADC
gcloud auth application-default login

# Credentials stored at:
# Linux/macOS: ~/.config/gcloud/application_default_credentials.json
# Windows: %APPDATA%/gcloud/application_default_credentials.json
```

### Method 3: Service Account Key (CI/CD - Legacy)

For older CI/CD systems without OIDC support:

```bash
# Create service account
gcloud iam service-accounts create packer-builder \
  --display-name="Packer Image Builder"

# Grant roles
gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:packer-builder@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/compute.instanceAdmin.v1"

gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:packer-builder@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/compute.imageUser"

gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:packer-builder@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/iam.serviceAccountUser"

# Create and download key
gcloud iam service-accounts keys create ~/packer-key.json \
  --iam-account=packer-builder@YOUR_PROJECT_ID.iam.gserviceaccount.com

# Set environment variable
export GOOGLE_APPLICATION_CREDENTIALS=~/packer-key.json
```

**Security Warning:** Service account keys are long-lived credentials. Rotate them regularly and prefer Workload Identity for CI/CD.

### Method 4: Workload Identity Federation (Recommended for CI/CD)

The most secure option for GitHub Actions and other CI/CD platforms:

```bash
# 1. Create Workload Identity Pool
gcloud iam workload-identity-pools create github-pool \
  --location=global \
  --display-name="GitHub Actions Pool"

# 2. Create OIDC Provider
gcloud iam workload-identity-pools providers create-oidc github-provider \
  --location=global \
  --workload-identity-pool=github-pool \
  --issuer-uri=https://token.actions.githubusercontent.com \
  --attribute-mapping="google.subject=assertion.sub,attribute.repository=assertion.repository" \
  --attribute-condition="assertion.repository=='YOUR_ORG/YOUR_REPO'"

# 3. Create service account for Packer builds
gcloud iam service-accounts create packer-github \
  --display-name="Packer Builder for GitHub Actions"

# 4. Grant service account permissions
gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:packer-github@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/compute.instanceAdmin.v1"

gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:packer-github@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/compute.imageUser"

gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="serviceAccount:packer-github@YOUR_PROJECT_ID.iam.gserviceaccount.com" \
  --role="roles/iam.serviceAccountUser"

# 5. Allow GitHub Actions to impersonate service account
gcloud iam service-accounts add-iam-policy-binding \
  packer-github@YOUR_PROJECT_ID.iam.gserviceaccount.com \
  --role=roles/iam.workloadIdentityUser \
  --member="principalSet://iam.googleapis.com/projects/PROJECT_NUMBER/locations/global/workloadIdentityPools/github-pool/attribute.repository/YOUR_ORG/YOUR_REPO"
```

**GitHub Actions Usage:**

```yaml
permissions:
  id-token: write
  contents: read

- uses: google-github-actions/auth@v2
  with:
    workload_identity_provider: projects/PROJECT_NUMBER/locations/global/workloadIdentityPools/github-pool/providers/github-provider
    service_account: packer-github@YOUR_PROJECT_ID.iam.gserviceaccount.com
```

### Method 5: Workload Identity (On GKE)

For builds running inside Google Kubernetes Engine:

```bash
# 1. Enable Workload Identity on cluster
gcloud container clusters update CLUSTER_NAME \
  --zone=ZONE \
  --workload-pool=YOUR_PROJECT_ID.svc.id.goog

# 2. Create Kubernetes service account
kubectl create serviceaccount packer-builder -n default

# 3. Link to GCP service account
gcloud iam service-accounts add-iam-policy-binding \
  packer-builder@YOUR_PROJECT_ID.iam.gserviceaccount.com \
  --role=roles/iam.workloadIdentityUser \
  --member="serviceAccount:YOUR_PROJECT_ID.svc.id.goog[default/packer-builder]"

# 4. Annotate Kubernetes service account
kubectl annotate serviceaccount packer-builder \
  --namespace=default \
  iam.gke.io/gcp-service-account=packer-builder@YOUR_PROJECT_ID.iam.gserviceaccount.com
```

## Project Configuration

### Setting Default Project and Region

```bash
# Set default project
gcloud config set project YOUR_PROJECT_ID

# Set default region and zone
gcloud config set compute/region us-west1
gcloud config set compute/zone us-west1-a

# Verify configuration
gcloud config list
```

### Enabling Required APIs

```bash
# Enable all required APIs at once
gcloud services enable \
  compute.googleapis.com \
  iam.googleapis.com \
  cloudresourcemanager.googleapis.com \
  iamcredentials.googleapis.com \
  sts.googleapis.com

# Verify APIs are enabled
gcloud services list --enabled --filter="NAME:(compute|iam|cloudresourcemanager)"
```

## Configuration Examples

### Basic sindri.yaml Configuration

```yaml
version: "1.0"
name: sindri-gcp

deployment:
  provider: packer

providers:
  packer:
    cloud: gcp
    image_name: sindri-dev
    description: "Sindri development environment"

    gcp:
      project_id: $GCP_PROJECT_ID
      zone: us-west1-a
      machine_type: e2-standard-4
      disk_size: 80
      disk_type: pd-ssd
```

### Production Configuration with Image Families

```yaml
version: "1.0"
name: sindri-production

deployment:
  provider: packer

extensions:
  profile: fullstack
  additional:
    - docker
    - kubernetes

providers:
  packer:
    cloud: gcp
    image_name: sindri-production
    description: "Production Sindri environment with CIS hardening"

    build:
      sindri_version: "3.0.0"
      cache: true
      ssh_timeout: "20m"
      security:
        cis_hardening: true
        clean_sensitive_data: true
        remove_ssh_keys: true

    gcp:
      project_id: $GCP_PROJECT_ID
      zone: us-central1-a
      machine_type: n2-standard-4
      disk_size: 100
      disk_type: pd-ssd
      network: default
      subnetwork: default

      # Image family for versioning
      image_family: sindri-production

      # Shielded VM options
      enable_secure_boot: true
      enable_vtpm: true
      enable_integrity_monitoring: true

    tags:
      Environment: production
      Team: platform
      ManagedBy: sindri
```

### Custom Network Configuration

```yaml
providers:
  packer:
    cloud: gcp
    gcp:
      project_id: my-project
      zone: us-west1-a
      machine_type: e2-standard-4
      disk_size: 80
      disk_type: pd-ssd

      # Custom VPC configuration
      network: projects/my-project/global/networks/custom-vpc
      subnetwork: projects/my-project/regions/us-west1/subnetworks/build-subnet

      # Use internal IP only (requires Cloud NAT for internet access)
      use_internal_ip: true
      omit_external_ip: true

      # Custom service account for the build instance
      service_account_email: packer-instance@my-project.iam.gserviceaccount.com
```

## Image Families

### Understanding Image Families

Image families provide semantic versioning for GCP images:

- The `--image-family` flag always resolves to the **newest non-deprecated** image in the family
- Enables rolling updates without changing instance templates
- Supports deprecation workflows for phased rollouts

### Creating Images with Families

```bash
# Build image with family
sindri packer build --cloud gcp \
  --name sindri-dev \
  --profile fullstack

# The generated template includes:
# image_family = "sindri-dev"
```

**sindri.yaml configuration:**

```yaml
providers:
  packer:
    cloud: gcp
    gcp:
      image_family: sindri-dev # Images auto-join this family
```

### Versioning Strategy

**Recommended naming convention:**

```
sindri-{profile}-{date}-{build_number}
```

Example image names in the `sindri-production` family:

- `sindri-production-20260201-001`
- `sindri-production-20260201-002`
- `sindri-production-20260215-001`

### Best Practices for Rolling Updates

**1. Test before production:**

```bash
# Build to staging family first
sindri packer build --cloud gcp \
  --name sindri-staging \
  --profile fullstack

# Test the image
gcloud compute instances create test-vm \
  --image-family=sindri-staging \
  --image-project=YOUR_PROJECT_ID

# After validation, rebuild with production family
sindri packer build --cloud gcp \
  --name sindri-production \
  --profile fullstack
```

**2. Deprecate old images gracefully:**

```bash
# Mark old image as deprecated (instances can still launch)
gcloud compute images deprecate sindri-production-20260115-001 \
  --state=DEPRECATED \
  --replacement=sindri-production-20260201-001

# After 30 days, mark as obsolete (new instances blocked)
gcloud compute images deprecate sindri-production-20260115-001 \
  --state=OBSOLETE

# Delete when no longer needed
gcloud compute images delete sindri-production-20260115-001
```

**3. Automate deprecation in CI/CD:**

```yaml
# GitHub Actions example
- name: Deprecate previous image
  run: |
    PREVIOUS=$(gcloud compute images list \
      --filter="family=sindri-production AND status=READY" \
      --sort-by="~creationTimestamp" \
      --limit=2 \
      --format="value(name)" | tail -1)

    if [ -n "$PREVIOUS" ]; then
      gcloud compute images deprecate $PREVIOUS \
        --state=DEPRECATED \
        --replacement=${{ steps.build.outputs.image_name }}
    fi
```

### Using Image Families

**Launch instance from family:**

```bash
# Always gets the latest image
gcloud compute instances create my-vm \
  --image-family=sindri-dev \
  --image-project=YOUR_PROJECT_ID \
  --zone=us-west1-a
```

**Instance template with family:**

```bash
gcloud compute instance-templates create sindri-template \
  --image-family=sindri-production \
  --image-project=YOUR_PROJECT_ID \
  --machine-type=e2-standard-4
```

## Cross-Project Sharing

### IAM Bindings for Specific Users

```bash
# Grant a specific user access to use the image
gcloud compute images add-iam-policy-binding sindri-dev-20260201-001 \
  --member='user:developer@example.com' \
  --role='roles/compute.imageUser'

# Grant access to a service account
gcloud compute images add-iam-policy-binding sindri-dev-20260201-001 \
  --member='serviceAccount:ci-runner@other-project.iam.gserviceaccount.com' \
  --role='roles/compute.imageUser'
```

### Sharing with Specific Projects

```bash
# Grant an entire project access
gcloud compute images add-iam-policy-binding sindri-production-20260201-001 \
  --member='serviceAccount:PROJECT_NUMBER-compute@developer.gserviceaccount.com' \
  --role='roles/compute.imageUser'

# Or use a Google Group for easier management
gcloud compute images add-iam-policy-binding sindri-production-20260201-001 \
  --member='group:image-consumers@example.com' \
  --role='roles/compute.imageUser'
```

### Making Images Public

**Warning:** Public images can be used by any Google account. Only make images public if intended for community/open-source distribution.

```bash
# Make image accessible to all authenticated Google users
gcloud compute images add-iam-policy-binding sindri-community-20260201-001 \
  --member='allAuthenticatedUsers' \
  --role='roles/compute.imageUser'
```

**Using from another project:**

```bash
# Reference public image by project and name
gcloud compute instances create my-vm \
  --image=sindri-community-20260201-001 \
  --image-project=sindri-public-images
```

### Sharing Image Families

To share all images in a family, set IAM at the project level:

```bash
# Grant project-level image access (applies to all images)
gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member='serviceAccount:ci-runner@other-project.iam.gserviceaccount.com' \
  --role='roles/compute.imageUser' \
  --condition='expression=resource.type == "compute.googleapis.com/Image" && resource.name.startsWith("sindri-"),title=sindri-images-only'
```

## Networking

### Default Network Configuration

By default, Packer uses:

- Network: `default`
- Subnetwork: `default`
- External IP: Assigned (required for provisioning)

### Custom VPC Configuration

```yaml
providers:
  packer:
    cloud: gcp
    gcp:
      network: my-custom-vpc
      subnetwork: build-subnet
```

### Private Build (No External IP)

For security-hardened environments, build without external IP:

```yaml
providers:
  packer:
    cloud: gcp
    gcp:
      network: my-vpc
      subnetwork: private-subnet
      use_internal_ip: true
      omit_external_ip: true
```

**Requirements for private builds:**

- Cloud NAT configured for internet access (package downloads)
- IAP (Identity-Aware Proxy) tunnel for SSH, OR
- VPN/Interconnect to your network

**Cloud NAT setup:**

```bash
# Create Cloud Router
gcloud compute routers create nat-router \
  --network=my-vpc \
  --region=us-west1

# Create NAT gateway
gcloud compute routers nats create nat-gateway \
  --router=nat-router \
  --region=us-west1 \
  --nat-all-subnet-ip-ranges \
  --auto-allocate-nat-external-ips
```

### Firewall Rules

Packer requires SSH access to the build instance:

```bash
# Allow SSH from Packer (external builds)
gcloud compute firewall-rules create allow-packer-ssh \
  --network=my-vpc \
  --allow=tcp:22 \
  --source-ranges=0.0.0.0/0 \
  --target-tags=packer-build

# For IAP-based SSH (private builds)
gcloud compute firewall-rules create allow-iap-ssh \
  --network=my-vpc \
  --allow=tcp:22 \
  --source-ranges=35.235.240.0/20 \
  --target-tags=packer-build
```

**sindri.yaml with firewall tags:**

```yaml
providers:
  packer:
    cloud: gcp
    gcp:
      network: my-vpc
      subnetwork: build-subnet
      tags:
        - packer-build
```

## Troubleshooting

### Authentication Errors

**Symptom:** `Error 401: Request had invalid authentication credentials`

**Solutions:**

```bash
# Re-authenticate
gcloud auth login
gcloud auth application-default login

# Verify credentials
gcloud auth print-access-token

# Check active account
gcloud config get-value account
```

### Permission Denied

**Symptom:** `Error 403: Required 'compute.instances.create' permission`

**Solutions:**

```bash
# Verify current permissions
gcloud projects get-iam-policy YOUR_PROJECT_ID \
  --flatten="bindings[].members" \
  --filter="bindings.members:$(gcloud config get-value account)"

# Grant missing role
gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="user:$(gcloud config get-value account)" \
  --role="roles/compute.instanceAdmin.v1"
```

### API Not Enabled

**Symptom:** `Error 403: Compute Engine API has not been used in project`

**Solution:**

```bash
gcloud services enable compute.googleapis.com
```

### Quota Exceeded

**Symptom:** `Error 403: Quota exceeded for resource`

**Solutions:**

```bash
# Check current quotas
gcloud compute project-info describe --project=YOUR_PROJECT_ID

# Request quota increase via Console:
# https://console.cloud.google.com/iam-admin/quotas
```

### Network/Firewall Issues

**Symptom:** `Timeout waiting for SSH`

**Solutions:**

```bash
# Check if firewall allows SSH
gcloud compute firewall-rules list --filter="allowed[].ports:22"

# Verify network configuration
gcloud compute networks describe default

# Test SSH manually
gcloud compute ssh INSTANCE_NAME --zone=ZONE
```

### Image Creation Failed

**Symptom:** `Error creating image: The disk resource is already being used`

**Solution:**

```bash
# The build instance may not have been deleted
gcloud compute instances list --filter="name~packer"

# Force delete stuck instance
gcloud compute instances delete INSTANCE_NAME --zone=ZONE

# Retry build
sindri packer build --cloud gcp --force
```

### Service Account Issues

**Symptom:** `Error 403: The caller does not have permission to act as service account`

**Solution:**

```bash
# Grant service account user role
gcloud projects add-iam-policy-binding YOUR_PROJECT_ID \
  --member="user:$(gcloud config get-value account)" \
  --role="roles/iam.serviceAccountUser"
```

### Debug Mode

For detailed debugging:

```bash
# Enable Packer debug logging
sindri packer build --cloud gcp --debug

# Or set environment variable
export PACKER_LOG=1
sindri packer build --cloud gcp
```

## Quick Reference

### Common Commands

```bash
# Check prerequisites
sindri packer doctor --cloud gcp

# Validate template
sindri packer validate --cloud gcp

# Build image
sindri packer build --cloud gcp --name my-image --profile fullstack

# List images
sindri packer list --cloud gcp

# Delete image
sindri packer delete --cloud gcp IMAGE_NAME

# Deploy from image
sindri packer deploy --cloud gcp projects/PROJECT/global/images/IMAGE_NAME
```

### Environment Variables

| Variable                         | Description                      |
| -------------------------------- | -------------------------------- |
| `GCP_PROJECT_ID`                 | Default GCP project              |
| `GOOGLE_APPLICATION_CREDENTIALS` | Path to service account key file |
| `CLOUDSDK_COMPUTE_ZONE`          | Default compute zone             |
| `CLOUDSDK_COMPUTE_REGION`        | Default compute region           |

## Related Documentation

- [Packer Provider Overview](PACKER.md)
- [Configuration Reference](../CONFIGURATION.md)
- [Secrets Management](../SECRETS_MANAGEMENT.md)
- [CLI Reference](../CLI.md)
- [GCP Compute Engine Images](https://cloud.google.com/compute/docs/images)
- [GCP IAM Roles](https://cloud.google.com/compute/docs/access/iam)
- [Workload Identity Federation](https://cloud.google.com/iam/docs/workload-identity-federation)
