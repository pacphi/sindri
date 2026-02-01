# OCI (Oracle Cloud Infrastructure) Packer Provider

> **Version:** 3.x
> **Last Updated:** 2026-02

Oracle Cloud Infrastructure VM image building using HashiCorp Packer.

## Overview

The OCI Packer provider enables building custom VM images on Oracle Cloud Infrastructure with:

- **Flexible Shapes** - Configurable OCPU and memory (1-64 OCPUs, 1-1024GB memory)
- **ARM and x86 Support** - Build for both Ampere A1 and AMD/Intel shapes
- **Compartment Organization** - Proper isolation and governance
- **Cross-Tenancy Distribution** - Export/import for partner sharing
- **OCI Marketplace Publishing** - Community and commercial distribution

**Best for:** Enterprise Oracle Cloud deployments, Always Free tier builds, ARM workloads, Oracle Database integrations

## Prerequisites

### Required Tools

| Requirement      | Version | Check Command      | Install                                                         |
| ---------------- | ------- | ------------------ | --------------------------------------------------------------- |
| HashiCorp Packer | 1.9+    | `packer --version` | https://developer.hashicorp.com/packer/install                  |
| OCI CLI          | 3.0+    | `oci --version`    | https://docs.oracle.com/iaas/Content/API/SDKDocs/cliinstall.htm |

### OCI CLI Installation

**Linux/macOS:**

```bash
bash -c "$(curl -L https://raw.githubusercontent.com/oracle/oci-cli/master/scripts/install/install.sh)"
```

**Windows (PowerShell):**

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "iex ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/oracle/oci-cli/master/scripts/install/install.ps1'))"
```

**Homebrew (macOS):**

```bash
brew install oci-cli
```

**Verify installation:**

```bash
oci --version
# Oracle Cloud Infrastructure CLI 3.x.x
```

### Tenancy Setup

Before building images, ensure you have:

1. **OCI Account** - Sign up at https://cloud.oracle.com/
2. **Compartment** - A compartment to store images (not recommended to use root)
3. **VCN and Subnet** - A virtual cloud network with a public subnet for build instances
4. **API Signing Key** - RSA key pair for CLI authentication

## Authentication Methods

OCI supports multiple authentication methods. Choose based on your environment.

### Method 1: Interactive Setup (Recommended for Local Development)

```bash
oci setup config
```

This interactive wizard will:

1. Prompt for User OCID, Tenancy OCID, and Region
2. Generate an API signing key pair
3. Display the public key to add to your OCI user
4. Create `~/.oci/config`

**Add the public key to OCI:**

1. Navigate to OCI Console > Identity > Users > Your User
2. Click "API Keys" > "Add API Key"
3. Paste the public key shown by the setup wizard

### Method 2: Config File (~/.oci/config)

Create or edit `~/.oci/config`:

```ini
[DEFAULT]
user=ocid1.user.oc1..aaaaaaaaexampleuserocid
fingerprint=aa:bb:cc:dd:ee:ff:00:11:22:33:44:55:66:77:88:99
tenancy=ocid1.tenancy.oc1..aaaaaaaaexampletenancyocid
region=us-phoenix-1
key_file=~/.oci/oci_api_key.pem
```

**Required fields:**

| Field         | Description                                | Example                                           |
| ------------- | ------------------------------------------ | ------------------------------------------------- |
| `user`        | User OCID from Identity > Users            | `ocid1.user.oc1..aaaaaa...`                       |
| `fingerprint` | API key fingerprint                        | `aa:bb:cc:dd:ee:ff:00:11:22:33:44:55:66:77:88:99` |
| `tenancy`     | Tenancy OCID from Administration > Tenancy | `ocid1.tenancy.oc1..aaaaaa...`                    |
| `region`      | Home region identifier                     | `us-phoenix-1`, `eu-frankfurt-1`                  |
| `key_file`    | Path to private key file                   | `~/.oci/oci_api_key.pem`                          |

**Generate API signing key manually:**

```bash
# Generate key pair
openssl genrsa -out ~/.oci/oci_api_key.pem 2048
chmod 600 ~/.oci/oci_api_key.pem

# Extract public key
openssl rsa -pubout -in ~/.oci/oci_api_key.pem -out ~/.oci/oci_api_key_public.pem

# Get fingerprint
openssl rsa -pubout -outform DER -in ~/.oci/oci_api_key.pem | openssl md5 -c
```

### Method 3: Environment Variables

```bash
export OCI_USER_OCID="ocid1.user.oc1..aaaaaaaaexampleuserocid"
export OCI_TENANCY_OCID="ocid1.tenancy.oc1..aaaaaaaaexampletenancyocid"
export OCI_FINGERPRINT="aa:bb:cc:dd:ee:ff:00:11:22:33:44:55:66:77:88:99"
export OCI_REGION="us-phoenix-1"
export OCI_PRIVATE_KEY_PATH="~/.oci/oci_api_key.pem"
# Or embed key directly:
# export OCI_PRIVATE_KEY_CONTENTS="-----BEGIN RSA PRIVATE KEY-----\n..."
```

**Environment variables for Packer:**

| Variable                   | Description                        |
| -------------------------- | ---------------------------------- |
| `OCI_USER_OCID`            | User OCID                          |
| `OCI_TENANCY_OCID`         | Tenancy OCID                       |
| `OCI_FINGERPRINT`          | API key fingerprint                |
| `OCI_REGION`               | Default region                     |
| `OCI_PRIVATE_KEY_PATH`     | Path to private key file           |
| `OCI_PRIVATE_KEY_CONTENTS` | Private key contents (alternative) |
| `OCI_COMPARTMENT_OCID`     | Default compartment OCID           |
| `OCI_SUBNET_OCID`          | Default subnet OCID                |

### Method 4: Instance Principal (For OCI Compute)

When running on OCI Compute instances, use Instance Principal authentication:

```bash
# No configuration needed - uses instance metadata
export OCI_CLI_AUTH=instance_principal
```

**Requirements:**

1. Instance must be in a Dynamic Group
2. Dynamic Group must have IAM policies granting required permissions

**Create Dynamic Group:**

```
All {instance.compartment.id = 'ocid1.compartment.oc1..aaaaaaaaexample'}
```

**Grant Policies to Dynamic Group:**

```
Allow dynamic-group SindriBuilders to manage instances in compartment sindri-images
Allow dynamic-group SindriBuilders to manage instance-images in compartment sindri-images
Allow dynamic-group SindriBuilders to use virtual-network-family in compartment sindri-images
```

### Verify Authentication

```bash
# List regions (quick auth check)
oci iam region list --output table

# Get current user
oci iam user get --user-id $(oci iam user list --query 'data[0].id' --raw-output)
```

## Required Config Values

### Finding Your OCIDs

**User OCID:**

1. OCI Console > Identity & Security > Users
2. Click your username
3. Copy OCID from "User Information"

**Tenancy OCID:**

1. OCI Console > Profile icon (top right) > Tenancy: [name]
2. Copy OCID from "Tenancy Information"

**Compartment OCID:**

1. OCI Console > Identity & Security > Compartments
2. Click target compartment
3. Copy OCID

**Subnet OCID:**

1. OCI Console > Networking > Virtual Cloud Networks
2. Click VCN > Subnets
3. Click subnet > Copy OCID

### Required Values Summary

| Value               | Source                           | Format                                      |
| ------------------- | -------------------------------- | ------------------------------------------- |
| User OCID           | Identity > Users > [user]        | `ocid1.user.oc1..<unique_id>`               |
| Tenancy OCID        | Profile > Tenancy                | `ocid1.tenancy.oc1..<unique_id>`            |
| Compartment OCID    | Identity > Compartments > [comp] | `ocid1.compartment.oc1..<unique_id>`        |
| API Key Fingerprint | Identity > Users > API Keys      | `aa:bb:cc:...:99` (32 hex chars)            |
| Private Key Path    | Local filesystem                 | `~/.oci/oci_api_key.pem`                    |
| Subnet OCID         | Networking > VCNs > Subnets      | `ocid1.subnet.oc1..<unique_id>`             |
| Availability Domain | Compute > Availability Domains   | `Uocm:PHX-AD-1`, `Uocm:EU-FRANKFURT-1-AD-1` |

## IAM Policies

OCI uses policy statements to grant permissions. Create these policies for Packer image building.

### Basic Image Building Policy

**For User-Based Authentication:**

```
# Allow user to build images
Allow group SindriPackerBuilders to manage instances in compartment sindri-images
Allow group SindriPackerBuilders to manage instance-images in compartment sindri-images
Allow group SindriPackerBuilders to use virtual-network-family in compartment sindri-images
Allow group SindriPackerBuilders to manage volume-family in compartment sindri-images
```

### Detailed Policy Breakdown

**Instance Management (Required):**

```
Allow group SindriPackerBuilders to manage instances in compartment sindri-images
```

Permits: `CreateInstance`, `TerminateInstance`, `GetInstance`, `ListInstances`, `InstanceAction`

**Image Management (Required):**

```
Allow group SindriPackerBuilders to manage instance-images in compartment sindri-images
```

Permits: `CreateImage`, `DeleteImage`, `GetImage`, `ListImages`, `UpdateImage`, `ExportImage`

**Network Access (Required):**

```
Allow group SindriPackerBuilders to use virtual-network-family in compartment sindri-images
```

Permits: `GetVcn`, `GetSubnet`, `UseVnic`, `GetVnic`

**Volume Management (Required for boot volumes):**

```
Allow group SindriPackerBuilders to manage volume-family in compartment sindri-images
```

Permits: `CreateVolume`, `DeleteVolume`, `GetBootVolume`, `UpdateBootVolume`

### Cross-Compartment Policies

If your VCN is in a different compartment than your images:

```
# Images compartment
Allow group SindriPackerBuilders to manage instances in compartment sindri-images
Allow group SindriPackerBuilders to manage instance-images in compartment sindri-images
Allow group SindriPackerBuilders to manage volume-family in compartment sindri-images

# Network compartment
Allow group SindriPackerBuilders to use virtual-network-family in compartment networking
```

### Instance Principal Policies

For CI/CD running on OCI Compute:

```
# Create dynamic group first
# Matching rule: All {instance.compartment.id = 'ocid1.compartment.oc1..aaaaaaa...'}

Allow dynamic-group SindriCIBuilders to manage instances in compartment sindri-images
Allow dynamic-group SindriCIBuilders to manage instance-images in compartment sindri-images
Allow dynamic-group SindriCIBuilders to use virtual-network-family in compartment sindri-images
Allow dynamic-group SindriCIBuilders to manage volume-family in compartment sindri-images
```

### Minimal Permission Set

For security-conscious environments, here are the exact verb/resource combinations:

```
Allow group SindriPackerBuilders to {
  INSTANCE_CREATE,
  INSTANCE_DELETE,
  INSTANCE_READ,
  INSTANCE_UPDATE,
  INSTANCE_ATTACH_SECONDARY_VNIC,
  INSTANCE_DETACH_SECONDARY_VNIC
} in compartment sindri-images

Allow group SindriPackerBuilders to {
  IMAGE_CREATE,
  IMAGE_DELETE,
  IMAGE_READ,
  IMAGE_UPDATE
} in compartment sindri-images

Allow group SindriPackerBuilders to use vnics in compartment sindri-images
Allow group SindriPackerBuilders to use subnets in compartment sindri-images
Allow group SindriPackerBuilders to use network-security-groups in compartment sindri-images

Allow group SindriPackerBuilders to {
  BOOT_VOLUME_ATTACHMENT_CREATE,
  BOOT_VOLUME_ATTACHMENT_DELETE,
  BOOT_VOLUME_READ
} in compartment sindri-images
```

## Quick Start

```bash
# 1. Verify prerequisites
sindri packer doctor --cloud oci

# 2. Build an OCI image
sindri packer build --cloud oci \
  --name sindri-dev \
  --profile fullstack

# 3. List built images
sindri packer list --cloud oci

# 4. Deploy an instance from the image
sindri packer deploy --cloud oci <image-ocid>
```

## Configuration Examples

### Basic sindri.yaml Configuration

```yaml
version: "1.0"
name: sindri-oci

deployment:
  provider: packer

extensions:
  profile: fullstack

providers:
  packer:
    cloud: oci
    image_name: sindri-dev
    description: "Sindri development environment for OCI"

    oci:
      compartment_ocid: $OCI_COMPARTMENT_OCID
      availability_domain: Uocm:PHX-AD-1
      shape: VM.Standard.E4.Flex
      shape_config:
        ocpus: 2
        memory_in_gbs: 16
      subnet_ocid: $OCI_SUBNET_OCID
      boot_volume_size_gb: 100
```

### Flexible Shape Configuration

```yaml
providers:
  packer:
    cloud: oci
    oci:
      compartment_ocid: ocid1.compartment.oc1..aaaaaaaaexample
      availability_domain: Uocm:PHX-AD-1
      subnet_ocid: ocid1.subnet.oc1.phx.aaaaaaaaexample

      # x86 Flexible Shape (AMD EPYC)
      shape: VM.Standard.E4.Flex
      shape_config:
        ocpus: 4 # 1-64 OCPUs
        memory_in_gbs: 32 # 1-64 GB per OCPU (max 1024GB)

      boot_volume_size_gb: 100
```

### ARM (Ampere A1) Configuration

```yaml
providers:
  packer:
    cloud: oci
    oci:
      compartment_ocid: $OCI_COMPARTMENT_OCID
      availability_domain: Uocm:PHX-AD-1
      subnet_ocid: $OCI_SUBNET_OCID

      # ARM Flexible Shape (Ampere A1)
      shape: VM.Standard.A1.Flex
      shape_config:
        ocpus: 4 # 1-80 OCPUs
        memory_in_gbs: 24 # 1-64 GB per OCPU (max 512GB)

      # Use ARM-compatible base image
      base_image_ocid: ocid1.image.oc1.phx.aaaaaaaaarm...
      boot_volume_size_gb: 100

    tags:
      Architecture: arm64
```

### Production Configuration with Security

```yaml
version: "1.0"
name: sindri-production

deployment:
  provider: packer

extensions:
  profile: enterprise

providers:
  packer:
    cloud: oci
    image_name: sindri-prod
    description: "Production Sindri environment - CIS hardened"

    build:
      sindri_version: "3.0.0"
      cache: true
      ssh_timeout: "20m"
      security:
        cis_hardening: true
        clean_sensitive_data: true
        remove_ssh_keys: true

    oci:
      compartment_ocid: $OCI_COMPARTMENT_OCID
      availability_domain: Uocm:PHX-AD-1
      shape: VM.Standard.E4.Flex
      shape_config:
        ocpus: 4
        memory_in_gbs: 32
      subnet_ocid: $OCI_SUBNET_OCID
      boot_volume_size_gb: 150

      # Security settings
      use_private_ip: false # Use bastion for private builds
      ssh_username: opc # Default OCI Linux user

    tags:
      Environment: production
      Compliance: CIS-L1
      ManagedBy: sindri-packer
```

### Multi-AD Configuration

```yaml
providers:
  packer:
    cloud: oci
    oci:
      compartment_ocid: $OCI_COMPARTMENT_OCID

      # Build in AD-1, image available across all ADs
      availability_domain: Uocm:PHX-AD-1
      shape: VM.Standard.E4.Flex
      shape_config:
        ocpus: 2
        memory_in_gbs: 16
      subnet_ocid: $OCI_SUBNET_OCID

    # Image is regional - automatically available in all ADs
```

## Compartment Organization

### Recommended Hierarchy

```
Root Compartment (Tenancy)
├── Production
│   ├── production-compute      # Production instances
│   ├── production-images       # Production golden images
│   └── production-network      # VCNs, subnets, security lists
├── Development
│   ├── dev-compute             # Dev instances
│   ├── dev-images              # Dev/test images
│   └── dev-network             # Dev networking
├── Shared
│   ├── shared-images           # Cross-environment images
│   └── shared-network          # Shared services network
└── CI-CD
    └── build-resources         # Packer build instances
```

### Compartment Best Practices

1. **Separate Build Compartment** - Isolate Packer builds from production workloads
2. **Image Compartment per Environment** - Keep prod/dev images separate
3. **Cross-Compartment Policies** - Allow builds to use shared networking
4. **Tag-Based Cost Tracking** - Use tags to track Packer build costs
5. **Quotas** - Set compute quotas to prevent runaway builds

### Create Compartment Structure

```bash
# Create image compartment
oci iam compartment create \
  --compartment-id $OCI_TENANCY_OCID \
  --name "sindri-images" \
  --description "Sindri VM images built by Packer"

# Create build compartment
oci iam compartment create \
  --compartment-id $OCI_TENANCY_OCID \
  --name "sindri-builds" \
  --description "Temporary Packer build instances"
```

## Availability Domains

### Understanding ADs

OCI regions have 1-3 Availability Domains (ADs). Images are regional but instances launch in specific ADs.

| Region Type      | AD Count | Example                    |
| ---------------- | -------- | -------------------------- |
| Multi-AD Region  | 3        | us-phoenix-1, us-ashburn-1 |
| Single-AD Region | 1        | uk-london-1, ap-tokyo-1    |

### Listing Availability Domains

```bash
# List ADs in current region
oci iam availability-domain list \
  --compartment-id $OCI_TENANCY_OCID \
  --query 'data[].name' \
  --output table
```

### AD Selection Strategy

**For Building Images:**

- Choose any AD - images are regional
- Consider AD with best shape availability
- Check capacity before building

**For Deployments:**

- Spread across ADs for high availability
- Use AD-specific subnets for isolation
- Consider data locality requirements

### Check Shape Availability per AD

```bash
# Check available shapes in an AD
oci compute shape list \
  --compartment-id $OCI_COMPARTMENT_OCID \
  --availability-domain "Uocm:PHX-AD-1" \
  --query 'data[?starts_with(shape, `VM.Standard.E4`)].shape' \
  --output table
```

## Shape Selection

### Flexible Shapes (Recommended)

| Shape               | Architecture | OCPU Range | Memory/OCPU | Best For           |
| ------------------- | ------------ | ---------- | ----------- | ------------------ |
| VM.Standard.E4.Flex | x86 (AMD)    | 1-64       | 1-64 GB     | General purpose    |
| VM.Standard.E5.Flex | x86 (AMD)    | 1-94       | 1-64 GB     | Latest AMD, memory |
| VM.Standard3.Flex   | x86 (Intel)  | 1-32       | 1-64 GB     | Intel workloads    |
| VM.Standard.A1.Flex | ARM          | 1-80       | 1-64 GB     | ARM builds, cost   |
| VM.Optimized3.Flex  | x86 (Intel)  | 1-18       | 1-64 GB     | High-frequency     |

### Fixed Shapes

| Shape            | OCPUs | Memory | Use Case         |
| ---------------- | ----- | ------ | ---------------- |
| VM.Standard.E2.1 | 1     | 8 GB   | Minimal builds   |
| VM.Standard.E2.2 | 2     | 16 GB  | Standard builds  |
| VM.Standard2.4   | 4     | 60 GB  | Memory-intensive |

### Shape Selection for Packer Builds

**Development/Testing:**

```yaml
shape: VM.Standard.E4.Flex
shape_config:
  ocpus: 2
  memory_in_gbs: 16
```

**Production Builds (faster):**

```yaml
shape: VM.Standard.E4.Flex
shape_config:
  ocpus: 4
  memory_in_gbs: 32
```

**ARM Builds:**

```yaml
shape: VM.Standard.A1.Flex
shape_config:
  ocpus: 4
  memory_in_gbs: 24
```

### Always Free Tier

OCI offers Always Free resources including:

- **VM.Standard.E2.1.Micro** - 1 OCPU, 1 GB memory (AMD x86)
- **VM.Standard.A1.Flex** - Up to 4 OCPUs, 24 GB total (ARM)

**Always Free Build Configuration:**

```yaml
providers:
  packer:
    cloud: oci
    oci:
      # ARM Always Free shape
      shape: VM.Standard.A1.Flex
      shape_config:
        ocpus: 4
        memory_in_gbs: 24
      boot_volume_size_gb: 47 # Max for Always Free
```

## Image Distribution

### Cross-Tenancy Export/Import

Export an image for sharing with other OCI tenancies:

**1. Create Object Storage Bucket:**

```bash
oci os bucket create \
  --compartment-id $OCI_COMPARTMENT_OCID \
  --name sindri-image-exports \
  --public-access-type NoPublicAccess
```

**2. Export Image to Object Storage:**

```bash
oci compute image export to-object \
  --image-id ocid1.image.oc1.phx.aaaaaaaaexample \
  --bucket-name sindri-image-exports \
  --name sindri-dev-v3.0.0.oci \
  --export-format QCOW2
```

**3. Create Pre-Authenticated Request (PAR) for Sharing:**

```bash
oci os preauth-request create \
  --bucket-name sindri-image-exports \
  --name "sindri-image-share" \
  --access-type ObjectRead \
  --object-name sindri-dev-v3.0.0.oci \
  --time-expires "2026-12-31T00:00:00Z"
```

**4. Import in Target Tenancy:**

```bash
oci compute image import from-object-uri \
  --compartment-id $TARGET_COMPARTMENT_OCID \
  --display-name sindri-dev-imported \
  --uri "https://objectstorage.us-phoenix-1.oraclecloud.com/p/<par-id>/n/<namespace>/b/sindri-image-exports/o/sindri-dev-v3.0.0.oci" \
  --source-image-type QCOW2 \
  --launch-mode PARAVIRTUALIZED
```

### OCI Marketplace Publishing

**Publishing to OCI Marketplace:**

1. **Register as Partner:**
   - Go to https://cloudmarketplace.oracle.com/
   - Complete partner registration
   - Accept terms and conditions

2. **Prepare Image:**

   ```yaml
   providers:
     packer:
       build:
         security:
           cis_hardening: true
           clean_sensitive_data: true
           remove_ssh_keys: true
   ```

3. **Create Marketplace Listing:**
   - Navigate to Partner Portal > Create Listing
   - Select "Virtual Machine Image"
   - Upload image or reference by OCID
   - Complete metadata (description, pricing, support)

4. **Submit for Review:**
   - Oracle reviews security and functionality
   - Typical review: 5-10 business days
   - Address any feedback

**Marketplace Listing Types:**

| Type | Description                   | Revenue Model         |
| ---- | ----------------------------- | --------------------- |
| Free | Community images, open source | No charge             |
| BYOL | Bring Your Own License        | User provides license |
| Paid | Oracle handles billing        | Per-hour or per-OCPU  |

### Image Sharing Within Tenancy

**Share image with another compartment:**

```bash
# Images are compartment-scoped but can be used across compartments
# Grant policy to allow usage
```

**Policy for cross-compartment image usage:**

```
Allow group DevTeam to use instance-images in compartment sindri-images
Allow group DevTeam to read instance-images in compartment sindri-images
```

## CLI Commands Reference

### Build Image

```bash
sindri packer build --cloud oci [OPTIONS]
```

**Options:**

| Flag               | Description                        | Default    |
| ------------------ | ---------------------------------- | ---------- |
| `--name`           | Image name prefix                  | sindri-dev |
| `--sindri-version` | Sindri version to install          | latest     |
| `--profile`        | Extension profile                  | base       |
| `--cis-hardening`  | Enable CIS benchmark hardening     | false      |
| `--dry-run`        | Generate template without building | false      |
| `--debug`          | Enable debug output                | false      |
| `--json`           | Output as JSON                     | false      |

**Example:**

```bash
sindri packer build --cloud oci \
  --name production-sindri \
  --profile enterprise \
  --cis-hardening \
  --json > build-result.json
```

### List Images

```bash
sindri packer list --cloud oci [OPTIONS]
```

**Example:**

```bash
sindri packer list --cloud oci --name sindri --json
```

### Delete Image

```bash
sindri packer delete --cloud oci <IMAGE_OCID>
```

**Example:**

```bash
sindri packer delete --cloud oci ocid1.image.oc1.phx.aaaaaaaaexample
```

### Deploy Instance

```bash
sindri packer deploy --cloud oci <IMAGE_OCID> [OPTIONS]
```

**Example:**

```bash
sindri packer deploy --cloud oci ocid1.image.oc1.phx.aaaaaaaaexample
```

### Check Prerequisites

```bash
sindri packer doctor --cloud oci
```

## Troubleshooting

### Authentication Errors

**Error:** `ServiceError: NotAuthenticated`

**Causes and Solutions:**

1. **Invalid API Key:**

   ```bash
   # Verify key fingerprint matches OCI Console
   openssl rsa -pubout -outform DER -in ~/.oci/oci_api_key.pem | openssl md5 -c
   # Compare with fingerprint in OCI Console > Users > API Keys
   ```

2. **Clock Skew:**

   ```bash
   # OCI requires time sync within 5 minutes
   sudo ntpdate pool.ntp.org
   # Or check NTP sync
   timedatectl status
   ```

3. **Wrong Tenancy/User OCID:**
   ```bash
   # Verify config file OCIDs
   cat ~/.oci/config
   # Cross-reference with OCI Console
   ```

### Permission Errors

**Error:** `ServiceError: NotAuthorized`

**Solution:** Verify IAM policies are correctly configured:

```bash
# Check policies in compartment
oci iam policy list \
  --compartment-id $OCI_COMPARTMENT_OCID \
  --query 'data[].statements' \
  --output table
```

Ensure these statements exist:

- `manage instances in compartment <compartment>`
- `manage instance-images in compartment <compartment>`
- `use virtual-network-family in compartment <compartment>`

### Shape Availability Errors

**Error:** `ServiceError: LimitExceeded` or shape not available

**Solutions:**

1. **Check service limits:**

   ```bash
   oci limits resource-availability get \
     --compartment-id $OCI_COMPARTMENT_OCID \
     --service-name compute \
     --limit-name standard-e4-core-count \
     --availability-domain "Uocm:PHX-AD-1"
   ```

2. **Try different AD:**

   ```yaml
   oci:
     availability_domain: Uocm:PHX-AD-2 # Try AD-2 or AD-3
   ```

3. **Use different shape:**

   ```yaml
   oci:
     shape: VM.Standard.E5.Flex # Try newer shape
   ```

4. **Request limit increase:**
   - OCI Console > Governance > Limits, Quotas and Usage
   - Create support request for limit increase

### Subnet/Network Errors

**Error:** `ServiceError: InvalidParameter` for subnet

**Solutions:**

1. **Verify subnet OCID:**

   ```bash
   oci network subnet get --subnet-id $OCI_SUBNET_OCID
   ```

2. **Check subnet is in correct AD (for regional subnets, use any AD):**

   ```bash
   oci network subnet list \
     --compartment-id $OCI_COMPARTMENT_OCID \
     --query 'data[].{Name:display_name,AD:availability_domain}' \
     --output table
   ```

3. **Ensure subnet has internet access for Packer provisioning:**
   - Attach Internet Gateway to VCN
   - Update route table with 0.0.0.0/0 -> IGW
   - Security list allows outbound HTTPS (443)

### Build Timeout

**Error:** SSH connection timeout during build

**Solutions:**

1. **Increase timeout:**

   ```yaml
   providers:
     packer:
       build:
         ssh_timeout: "30m"
   ```

2. **Use larger shape for faster provisioning:**

   ```yaml
   oci:
     shape_config:
       ocpus: 4
       memory_in_gbs: 32
   ```

3. **Check security list allows SSH:**
   ```bash
   # Ensure ingress rule for SSH (port 22)
   oci network security-list list \
     --compartment-id $OCI_COMPARTMENT_OCID \
     --query 'data[].ingress-security-rules[?destination-port-range.min==`22`]'
   ```

### Image Creation Errors

**Error:** `ServiceError: InternalError` during image creation

**Solutions:**

1. **Ensure instance is stopped cleanly:**
   - Packer should stop instance before capture
   - Check for stuck processes

2. **Check boot volume health:**

   ```bash
   oci bv boot-volume list \
     --compartment-id $OCI_COMPARTMENT_OCID \
     --availability-domain "Uocm:PHX-AD-1"
   ```

3. **Retry with clean state:**
   ```bash
   sindri packer build --cloud oci --force
   ```

### Debug Mode

Enable comprehensive logging:

```bash
# Enable Packer debug
export PACKER_LOG=1
export PACKER_LOG_PATH=/tmp/packer-oci.log

# Run build with debug
sindri packer build --cloud oci --debug

# Review logs
cat /tmp/packer-oci.log
```

### Common OCI-Specific Issues

| Issue                       | Cause                   | Solution                              |
| --------------------------- | ----------------------- | ------------------------------------- |
| `NotAuthenticated`          | Invalid/expired API key | Regenerate and upload API key         |
| `NotAuthorized`             | Missing IAM policy      | Add required policy statements        |
| `LimitExceeded`             | Quota reached           | Request increase or use smaller shape |
| `InvalidParameter (subnet)` | Wrong AD or OCID        | Verify subnet OCID and AD match       |
| `Timeout`                   | No internet access      | Check IGW and route table             |
| `ShapeNotFound`             | Shape unavailable in AD | Try different AD or shape             |
| `CompartmentNotFound`       | Deleted or wrong OCID   | Verify compartment exists             |

## Cost Estimates

### Build Costs (per build)

| Shape                  | OCPUs | Memory | Hourly Rate | 15-min Build |
| ---------------------- | ----- | ------ | ----------- | ------------ |
| VM.Standard.E4.Flex    | 2     | 16 GB  | ~$0.05      | ~$0.01       |
| VM.Standard.E4.Flex    | 4     | 32 GB  | ~$0.10      | ~$0.025      |
| VM.Standard.A1.Flex    | 4     | 24 GB  | ~$0.04      | ~$0.01       |
| VM.Standard.E2.1.Micro | 1     | 1 GB   | Free        | Free         |

### Storage Costs (monthly)

| Storage Type   | Cost per GB | 100 GB Image |
| -------------- | ----------- | ------------ |
| Custom Image   | $0.025      | ~$2.50       |
| Object Storage | $0.0255     | ~$2.55       |
| Boot Volume    | $0.017      | ~$1.70       |

### Cost Optimization Tips

1. **Use Always Free shapes** - ARM A1.Flex or E2.1.Micro for builds
2. **Clean up old images** - Delete unused images monthly
3. **Use caching** - Don't rebuild if config unchanged
4. **Regional builds** - Build in one region, copy only if needed
5. **Off-peak builds** - No cost difference, but may have better availability

## Related Documentation

- [Packer Provider Overview](PACKER.md)
- [Provider Overview](README.md)
- [Configuration Reference](../CONFIGURATION.md)
- [Secrets Management](../SECRETS_MANAGEMENT.md)
- [CLI Reference](../CLI.md)
- [OCI Documentation](https://docs.oracle.com/en-us/iaas/Content/home.htm)
- [Packer OCI Builder](https://developer.hashicorp.com/packer/integrations/hashicorp/oracle/latest/components/builder/oci)
