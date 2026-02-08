# Azure Packer Provider

> **Version:** 3.x
> **Last Updated:** 2026-02

Azure-specific guide for building Sindri VM images using HashiCorp Packer.

## Overview

The Azure Packer provider builds managed VM images and publishes them to Azure Compute Gallery (formerly Shared Image Gallery) for enterprise distribution. This guide covers Azure-specific setup, authentication, RBAC permissions, networking, and best practices.

**Key Features:**

- **Managed Images** - Store images in Azure-managed storage
- **Azure Compute Gallery** - Version, replicate, and share images across subscriptions/tenants
- **Community Gallery** - Publish images publicly for the Azure community
- **Trusted Launch** - Security-enhanced VM images with Secure Boot and vTPM
- **Cross-Region Replication** - Automatically replicate images to multiple regions

**Best for:** Enterprise deployments, multi-region infrastructure, Azure-native organizations

## Prerequisites

### Required Tools

| Requirement      | Version | Check Command      | Install                                                |
| ---------------- | ------- | ------------------ | ------------------------------------------------------ |
| HashiCorp Packer | 1.9+    | `packer --version` | https://developer.hashicorp.com/packer/install         |
| Azure CLI        | 2.50+   | `az --version`     | https://docs.microsoft.com/cli/azure/install-azure-cli |

### Install Azure CLI

**macOS:**

```bash
brew update && brew install azure-cli
```

**Ubuntu/Debian:**

```bash
curl -sL https://aka.ms/InstallAzureCLIDeb | sudo bash
```

**Windows:**

```powershell
winget install -e --id Microsoft.AzureCLI
```

**Verify installation:**

```bash
az --version
```

### Azure Subscription Setup

1. **Create or access an Azure subscription:**

   ```bash
   # List available subscriptions
   az account list --output table

   # Set active subscription
   az account set --subscription "<subscription-id-or-name>"

   # Verify current subscription
   az account show
   ```

2. **Register required resource providers:**

   ```bash
   # Register Compute provider (for VMs and images)
   az provider register --namespace Microsoft.Compute

   # Register Network provider (for VNet/NSG)
   az provider register --namespace Microsoft.Network

   # Check registration status
   az provider show --namespace Microsoft.Compute --query "registrationState"
   ```

## RBAC Roles and Permissions

### Required Azure RBAC Roles

For building Packer images, the following roles are required:

| Role                            | Scope          | Purpose                         |
| ------------------------------- | -------------- | ------------------------------- |
| **Contributor**                 | Resource Group | Create VMs, images, disks, NICs |
| **Virtual Machine Contributor** | Resource Group | Alternative to full Contributor |

For Azure Compute Gallery operations:

| Role            | Scope            | Purpose                               |
| --------------- | ---------------- | ------------------------------------- |
| **Contributor** | Gallery Resource | Create image definitions and versions |
| **Reader**      | Gallery Resource | Read gallery metadata                 |

### Minimum Custom Role Permissions

For security-conscious environments, create a custom role with minimum permissions:

```json
{
  "Name": "Packer Image Builder",
  "Description": "Minimum permissions for building Packer images",
  "Actions": [
    "Microsoft.Compute/images/read",
    "Microsoft.Compute/images/write",
    "Microsoft.Compute/images/delete",
    "Microsoft.Compute/virtualMachines/read",
    "Microsoft.Compute/virtualMachines/write",
    "Microsoft.Compute/virtualMachines/delete",
    "Microsoft.Compute/virtualMachines/powerOff/action",
    "Microsoft.Compute/virtualMachines/deallocate/action",
    "Microsoft.Compute/virtualMachines/generalize/action",
    "Microsoft.Compute/disks/read",
    "Microsoft.Compute/disks/write",
    "Microsoft.Compute/disks/delete",
    "Microsoft.Compute/galleries/read",
    "Microsoft.Compute/galleries/images/read",
    "Microsoft.Compute/galleries/images/write",
    "Microsoft.Compute/galleries/images/versions/read",
    "Microsoft.Compute/galleries/images/versions/write",
    "Microsoft.Network/virtualNetworks/read",
    "Microsoft.Network/virtualNetworks/subnets/read",
    "Microsoft.Network/virtualNetworks/subnets/join/action",
    "Microsoft.Network/networkInterfaces/read",
    "Microsoft.Network/networkInterfaces/write",
    "Microsoft.Network/networkInterfaces/delete",
    "Microsoft.Network/networkInterfaces/join/action",
    "Microsoft.Network/publicIPAddresses/read",
    "Microsoft.Network/publicIPAddresses/write",
    "Microsoft.Network/publicIPAddresses/delete",
    "Microsoft.Network/publicIPAddresses/join/action",
    "Microsoft.Network/networkSecurityGroups/read",
    "Microsoft.Network/networkSecurityGroups/write",
    "Microsoft.Network/networkSecurityGroups/delete",
    "Microsoft.Network/networkSecurityGroups/securityRules/read",
    "Microsoft.Network/networkSecurityGroups/securityRules/write",
    "Microsoft.Network/networkSecurityGroups/securityRules/delete",
    "Microsoft.Resources/subscriptions/resourceGroups/read"
  ],
  "NotActions": [],
  "DataActions": [],
  "NotDataActions": [],
  "AssignableScopes": ["/subscriptions/<subscription-id>/resourceGroups/<resource-group-name>"]
}
```

**Create the custom role:**

```bash
# Save the JSON above to packer-role.json
az role definition create --role-definition packer-role.json

# Assign to a user or service principal
az role assignment create \
  --assignee "<user-or-sp-object-id>" \
  --role "Packer Image Builder" \
  --scope "/subscriptions/<subscription-id>/resourceGroups/<resource-group-name>"
```

## Authentication Methods

### Method 1: Interactive Browser Login (Development)

Best for local development and testing.

```bash
# Login via browser
az login

# Set subscription
az account set --subscription "<subscription-id>"

# Verify
az account show --query "{name:name, id:id, user:user.name}"
```

**Environment variables (optional):**

```bash
export AZURE_SUBSCRIPTION_ID="<subscription-id>"
```

### Method 2: Service Principal (CI/CD and Automation)

Best for automated pipelines and non-interactive scenarios.

**Create a Service Principal:**

```bash
# Create SP with Contributor role on resource group
az ad sp create-for-rbac \
  --name "sindri-packer-sp" \
  --role Contributor \
  --scopes /subscriptions/<subscription-id>/resourceGroups/<resource-group-name>

# Output:
# {
#   "appId": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",        <- AZURE_CLIENT_ID
#   "displayName": "sindri-packer-sp",
#   "password": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",     <- AZURE_CLIENT_SECRET
#   "tenant": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"        <- AZURE_TENANT_ID
# }
```

**Login with Service Principal:**

```bash
az login --service-principal \
  -u "<appId>" \
  -p "<password>" \
  --tenant "<tenant>"
```

**Environment variables for automation:**

```bash
export AZURE_CLIENT_ID="<appId>"
export AZURE_CLIENT_SECRET="<password>"
export AZURE_TENANT_ID="<tenant>"
export AZURE_SUBSCRIPTION_ID="<subscription-id>"
```

### Method 3: Managed Identity (Azure VMs and Services)

Best for workloads running on Azure VMs, AKS, or Azure Functions.

**System-assigned managed identity:**

```bash
# Enable on VM
az vm identity assign \
  --resource-group <rg-name> \
  --name <vm-name>

# Grant permissions
az role assignment create \
  --assignee-object-id $(az vm show -g <rg-name> -n <vm-name> --query identity.principalId -o tsv) \
  --role Contributor \
  --scope /subscriptions/<subscription-id>/resourceGroups/<packer-rg>
```

**User-assigned managed identity:**

```bash
# Create identity
az identity create \
  --resource-group <rg-name> \
  --name sindri-packer-identity

# Assign to VM
az vm identity assign \
  --resource-group <rg-name> \
  --name <vm-name> \
  --identities /subscriptions/<sub-id>/resourcegroups/<rg>/providers/Microsoft.ManagedIdentity/userAssignedIdentities/sindri-packer-identity

# Grant permissions
az role assignment create \
  --assignee-object-id $(az identity show -g <rg-name> -n sindri-packer-identity --query principalId -o tsv) \
  --role Contributor \
  --scope /subscriptions/<subscription-id>/resourceGroups/<packer-rg>
```

### Method 4: Environment Variables (Portable)

Set these environment variables for any authentication scenario:

```bash
# Required for Service Principal auth
export AZURE_CLIENT_ID="<appId>"
export AZURE_CLIENT_SECRET="<password>"
export AZURE_TENANT_ID="<tenant>"
export AZURE_SUBSCRIPTION_ID="<subscription-id>"

# Optional: Resource group for Packer builds
export AZURE_RESOURCE_GROUP="sindri-packer"
```

**Verify credentials:**

```bash
# Using Azure CLI
az account show

# Sindri verification
sindri vm doctor --cloud azure
```

## Resource Group Setup

### Create Resource Groups

Organize Azure resources with dedicated resource groups:

```bash
# Create resource group for Packer builds
az group create \
  --name sindri-packer \
  --location westus2 \
  --tags Environment=Production Team=Platform Purpose=PackerBuilds

# Create resource group for galleries (optional, can use same RG)
az group create \
  --name sindri-images \
  --location westus2 \
  --tags Environment=Production Team=Platform Purpose=ImageGallery
```

### Resource Group Organization Strategy

| Resource Group   | Purpose          | Resources                                        |
| ---------------- | ---------------- | ------------------------------------------------ |
| `sindri-packer`  | Build operations | Temporary VMs, NICs, Public IPs during build     |
| `sindri-images`  | Image storage    | Managed Images, Azure Compute Gallery            |
| `sindri-network` | Networking       | VNet, Subnets, NSGs (if using custom networking) |

### Lock Critical Resource Groups

Prevent accidental deletion of image resources:

```bash
az lock create \
  --name PreventDelete \
  --resource-group sindri-images \
  --lock-type CanNotDelete \
  --notes "Protect Sindri VM images from accidental deletion"
```

## Configuration Examples

### Basic Azure Configuration

```yaml
version: "1.0"
name: sindri-azure-image

deployment:
  provider: packer

extensions:
  profile: fullstack

providers:
  packer:
    cloud: azure
    image_name: sindri-dev
    description: "Sindri development environment"

    azure:
      subscription_id: $AZURE_SUBSCRIPTION_ID
      resource_group: sindri-packer
      location: westus2
      vm_size: Standard_D4s_v4
      os_disk_size_gb: 80
```

### Production Configuration with Gallery

```yaml
version: "1.0"
name: sindri-production

deployment:
  provider: packer

extensions:
  profile: enterprise
  additional:
    - docker
    - kubernetes

providers:
  packer:
    cloud: azure
    image_name: sindri-production
    description: "Production Sindri environment with enterprise extensions"

    build:
      sindri_version: "3.0.0"
      ssh_timeout: "25m"
      security:
        cis_hardening: true
        clean_sensitive_data: true
        remove_ssh_keys: true

    azure:
      subscription_id: $AZURE_SUBSCRIPTION_ID
      resource_group: sindri-packer
      location: westus2
      vm_size: Standard_D4s_v4
      os_disk_size_gb: 100
      storage_account_type: Premium_LRS

      # Azure Compute Gallery configuration
      gallery:
        gallery_name: sindri_gallery
        image_name: sindri-enterprise
        image_version: "1.0.0"
        replication_regions:
          - eastus
          - westeurope
          - southeastasia

    tags:
      Environment: production
      Team: platform
      CostCenter: engineering
      ManagedBy: sindri-packer
```

### Configuration with Custom Networking

```yaml
version: "1.0"
name: sindri-enterprise

deployment:
  provider: packer

providers:
  packer:
    cloud: azure

    azure:
      subscription_id: $AZURE_SUBSCRIPTION_ID
      resource_group: sindri-packer
      location: eastus2
      vm_size: Standard_D8s_v4
      os_disk_size_gb: 120

      # Custom networking
      virtual_network_name: sindri-vnet
      virtual_network_subnet_name: packer-subnet
      virtual_network_resource_group_name: sindri-network

      # Private build (no public IP)
      private_virtual_network_with_public_ip: false

      # Or use existing NSG
      # azure_nsg_name: packer-nsg
      # azure_nsg_resource_group_name: sindri-network
```

### Trusted Launch Configuration

```yaml
version: "1.0"
name: sindri-secure

deployment:
  provider: packer

providers:
  packer:
    cloud: azure

    azure:
      subscription_id: $AZURE_SUBSCRIPTION_ID
      resource_group: sindri-packer
      location: westus2
      vm_size: Standard_D4s_v5

      # Trusted Launch settings
      secure_boot_enabled: true
      vtpm_enabled: true
      security_type: TrustedLaunch

      gallery:
        gallery_name: sindri_gallery
        image_name: sindri-trusted
        image_version: "1.0.0"
        # TrustedLaunch images require compatible VM sizes
```

## Azure Compute Gallery

### Creating a Gallery

Azure Compute Gallery (formerly Shared Image Gallery) provides:

- Versioned image definitions
- Cross-region replication
- RBAC-controlled sharing
- Community publishing

**Create gallery via CLI:**

```bash
# Create gallery
az sig create \
  --resource-group sindri-images \
  --gallery-name sindri_gallery \
  --description "Sindri VM Images"

# Create image definition
az sig image-definition create \
  --resource-group sindri-images \
  --gallery-name sindri_gallery \
  --gallery-image-definition sindri-dev \
  --publisher Sindri \
  --offer SindriEnvironment \
  --sku Dev \
  --os-type Linux \
  --os-state Generalized \
  --hyper-v-generation V2 \
  --features SecurityType=TrustedLaunch
```

### Image Definitions and Versions

**sindri.yaml configuration:**

```yaml
providers:
  packer:
    cloud: azure
    azure:
      gallery:
        gallery_name: sindri_gallery
        image_name: sindri-dev # Image definition name
        image_version: "1.0.0" # Semantic version
        replication_regions:
          - eastus
          - westus2
          - westeurope
```

**List gallery contents:**

```bash
# List image definitions
az sig image-definition list \
  --resource-group sindri-images \
  --gallery-name sindri_gallery \
  --output table

# List image versions
az sig image-version list \
  --resource-group sindri-images \
  --gallery-name sindri_gallery \
  --gallery-image-definition sindri-dev \
  --output table
```

### Cross-Region Replication

Configure automatic replication to multiple regions:

```yaml
providers:
  packer:
    azure:
      gallery:
        gallery_name: sindri_gallery
        image_name: sindri-global
        image_version: "1.0.0"
        replication_regions:
          - eastus # Primary
          - westus2 # US West
          - westeurope # Europe
          - southeastasia # Asia Pacific
          - australiaeast # Australia
        replica_count: 2 # Replicas per region (default: 1)
```

**Replication status:**

```bash
az sig image-version show \
  --resource-group sindri-images \
  --gallery-name sindri_gallery \
  --gallery-image-definition sindri-dev \
  --gallery-image-version 1.0.0 \
  --query "publishingProfile.replicaCount" -o tsv
```

### Community Gallery for Public Sharing

Community Galleries allow public image distribution to all Azure users.

**Create a community gallery:**

```bash
# Create gallery with community sharing
az sig create \
  --resource-group sindri-images \
  --gallery-name sindri_community_gallery \
  --permissions Community

# Configure community settings
az sig share enable-community \
  --resource-group sindri-images \
  --gallery-name sindri_community_gallery \
  --publisher-uri "https://sindri.dev" \
  --publisher-email "support@sindri.dev" \
  --eula "https://sindri.dev/eula" \
  --public-name-prefix "sindri"
```

**Requirements for community galleries:**

- Valid publisher URL
- Valid contact email
- EULA URL
- Microsoft review for public listing

**2025+ API Changes:**
Starting with API version 2025-03-03, image definitions default to TrustedLaunch validation for enhanced security.

### Sharing with Specific Subscriptions/Tenants

For enterprise sharing without public access:

```bash
# Share with specific subscription
az sig share add \
  --resource-group sindri-images \
  --gallery-name sindri_gallery \
  --subscription-ids "<target-subscription-id>"

# Share with specific tenant
az sig share add \
  --resource-group sindri-images \
  --gallery-name sindri_gallery \
  --tenant-ids "<target-tenant-id>"
```

**Limits:**

- Direct sharing: Up to 30 subscriptions, 5 tenants
- For broader sharing: Use Community Gallery

## Networking

### Default Networking

By default, Packer creates temporary networking resources:

- Public IP address
- Network interface
- Network security group with SSH (port 22) allowed

These are automatically cleaned up after the build.

### Custom VNet Configuration

Use existing VNet/Subnet for builds (required for private networks):

```yaml
providers:
  packer:
    azure:
      # Reference existing VNet
      virtual_network_name: sindri-vnet
      virtual_network_subnet_name: packer-subnet
      virtual_network_resource_group_name: sindri-network
```

**Create networking resources:**

```bash
# Create VNet
az network vnet create \
  --resource-group sindri-network \
  --name sindri-vnet \
  --address-prefix 10.0.0.0/16

# Create subnet for Packer builds
az network vnet subnet create \
  --resource-group sindri-network \
  --vnet-name sindri-vnet \
  --name packer-subnet \
  --address-prefix 10.0.1.0/24

# Create NSG
az network nsg create \
  --resource-group sindri-network \
  --name packer-nsg

# Allow SSH from known IPs only
az network nsg rule create \
  --resource-group sindri-network \
  --nsg-name packer-nsg \
  --name AllowSSH \
  --priority 100 \
  --source-address-prefixes "YOUR_IP/32" \
  --destination-port-ranges 22 \
  --protocol Tcp \
  --access Allow

# Associate NSG with subnet
az network vnet subnet update \
  --resource-group sindri-network \
  --vnet-name sindri-vnet \
  --name packer-subnet \
  --network-security-group packer-nsg
```

### Private Builds (No Public IP)

For security-sensitive environments:

```yaml
providers:
  packer:
    azure:
      virtual_network_name: sindri-vnet
      virtual_network_subnet_name: packer-subnet
      virtual_network_resource_group_name: sindri-network
      private_virtual_network_with_public_ip: false
```

**Requirements for private builds:**

- Azure Bastion or VPN for SSH access
- NAT Gateway for outbound internet (package downloads)
- Or fully private with Azure Private Endpoint for container registries

## Cost Optimization

### VM Size Selection

Choose appropriate VM sizes for builds:

| Use Case         | Recommended VM Size | vCPUs | Memory | Cost/Hour\* |
| ---------------- | ------------------- | ----- | ------ | ----------- |
| Minimal builds   | Standard_B2s        | 2     | 4 GB   | ~$0.04      |
| Standard builds  | Standard_D4s_v4     | 4     | 16 GB  | ~$0.19      |
| Fast builds      | Standard_D8s_v4     | 8     | 32 GB  | ~$0.38      |
| Large extensions | Standard_D16s_v4    | 16    | 64 GB  | ~$0.77      |

\*Prices are approximate and vary by region.

```yaml
providers:
  packer:
    azure:
      # Cost-effective for simple builds
      vm_size: Standard_B2s

      # Or performance-optimized
      vm_size: Standard_D8s_v4
```

### Spot VMs for Cost Savings

Use Azure Spot VMs for up to 90% cost savings:

```yaml
providers:
  packer:
    azure:
      vm_size: Standard_D4s_v4

      # Enable Spot VM (not yet supported in Sindri, configure in raw HCL)
      # spot:
      #   enabled: true
      #   max_price: 0.05  # Max price per hour, or -1 for market rate
```

**Manual spot configuration (raw Packer HCL):**

```hcl
source "azure-arm" "sindri" {
  # ... other config ...

  spot {
    eviction_policy = "Delete"
    max_price       = -1  # Use current spot price
  }
}
```

**Note:** Spot VMs can be evicted. Enable retries in CI/CD:

```yaml
# GitHub Actions example
- name: Build with retry
  uses: nick-fields/retry@v2
  with:
    timeout_minutes: 45
    max_attempts: 3
    command: sindri vm build --cloud azure
```

### Storage Optimization

```yaml
providers:
  packer:
    azure:
      # Standard SSD (balanced)
      storage_account_type: StandardSSD_LRS

      # Premium SSD (faster builds)
      # storage_account_type: Premium_LRS

      # Standard HDD (cheapest, slowest)
      # storage_account_type: Standard_LRS

      # Right-size disk
      os_disk_size_gb: 64 # Minimum needed
```

### Image Storage Costs

| Storage Type         | Cost per GB/Month | 100GB Image      |
| -------------------- | ----------------- | ---------------- |
| Managed Image        | ~$0.05            | ~$5/month        |
| Gallery (per region) | ~$0.05            | ~$5/month/region |

**Cost optimization tips:**

1. Delete old image versions regularly
2. Minimize replication regions
3. Use lifecycle policies for automatic cleanup

```bash
# Delete old image versions
az sig image-version delete \
  --resource-group sindri-images \
  --gallery-name sindri_gallery \
  --gallery-image-definition sindri-dev \
  --gallery-image-version 0.9.0
```

## GitHub Actions Integration

### OIDC Authentication (Recommended)

Use OpenID Connect for secure, secretless authentication:

**1. Create App Registration with Federated Credentials:**

```bash
# Create app registration
az ad app create --display-name "github-actions-sindri"

# Get application ID
APP_ID=$(az ad app list --display-name "github-actions-sindri" --query "[0].appId" -o tsv)

# Create federated credential for main branch
az ad app federated-credential create \
  --id $APP_ID \
  --parameters '{
    "name": "github-main",
    "issuer": "https://token.actions.githubusercontent.com",
    "subject": "repo:<org>/<repo>:ref:refs/heads/main",
    "audiences": ["api://AzureADTokenExchange"]
  }'

# Create service principal
az ad sp create --id $APP_ID

# Assign Contributor role
az role assignment create \
  --assignee $APP_ID \
  --role Contributor \
  --scope /subscriptions/<subscription-id>/resourceGroups/sindri-packer
```

**2. Configure GitHub Secrets:**

- `AZURE_CLIENT_ID` - Application (client) ID
- `AZURE_TENANT_ID` - Directory (tenant) ID
- `AZURE_SUBSCRIPTION_ID` - Azure subscription ID

**3. Workflow Example:**

```yaml
name: Build Azure Image

on:
  workflow_dispatch:
    inputs:
      sindri_version:
        description: "Sindri version to install"
        required: true
        default: "latest"

permissions:
  id-token: write
  contents: read

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - name: Azure Login (OIDC)
        uses: azure/login@v2
        with:
          client-id: ${{ secrets.AZURE_CLIENT_ID }}
          tenant-id: ${{ secrets.AZURE_TENANT_ID }}
          subscription-id: ${{ secrets.AZURE_SUBSCRIPTION_ID }}

      - name: Setup Packer
        uses: hashicorp/setup-packer@main
        with:
          version: "1.10.0"

      - name: Build Sindri Image
        run: |
          sindri vm build --cloud azure \
            --name "sindri-${{ github.sha }}" \
            --profile fullstack \
            --json > build-result.json

      - name: Upload Build Result
        uses: actions/upload-artifact@v4
        with:
          name: azure-build
          path: build-result.json
```

## Troubleshooting

### Authentication Errors

**Symptom:** `AADSTS700016: Application not found`

**Cause:** Service principal not created or wrong client ID

**Solution:**

```bash
# Verify service principal exists
az ad sp show --id $AZURE_CLIENT_ID

# Recreate if needed
az ad sp create-for-rbac --name "sindri-packer-sp" --role Contributor --scopes /subscriptions/$AZURE_SUBSCRIPTION_ID/resourceGroups/sindri-packer
```

---

**Symptom:** `AuthorizationFailed: does not have authorization to perform action`

**Cause:** Insufficient RBAC permissions

**Solution:**

```bash
# Check current role assignments
az role assignment list --assignee $AZURE_CLIENT_ID --output table

# Add Contributor role
az role assignment create \
  --assignee $AZURE_CLIENT_ID \
  --role Contributor \
  --scope /subscriptions/$AZURE_SUBSCRIPTION_ID/resourceGroups/sindri-packer
```

### Build Failures

**Symptom:** `Error waiting for SSH: timeout`

**Cause:** NSG blocking SSH or VM not starting

**Solution:**

```bash
# Check NSG rules
az network nsg rule list \
  --resource-group sindri-network \
  --nsg-name packer-nsg \
  --output table

# Ensure SSH is allowed
az network nsg rule create \
  --resource-group sindri-network \
  --nsg-name packer-nsg \
  --name AllowSSH \
  --priority 100 \
  --source-address-prefixes "*" \
  --destination-port-ranges 22 \
  --protocol Tcp \
  --access Allow
```

---

**Symptom:** `Quota exceeded for resource type`

**Cause:** Subscription quota limits

**Solution:**

```bash
# Check current usage
az vm list-usage --location westus2 --output table

# Request quota increase via Azure Portal
# Or use a different VM size/region
```

---

**Symptom:** `The requested VM size is not available in the current region`

**Cause:** VM size not available in selected region

**Solution:**

```bash
# List available sizes in region
az vm list-sizes --location westus2 --output table

# Use available size
providers:
  packer:
    azure:
      vm_size: Standard_D4s_v5  # Alternative size
```

### Gallery Errors

**Symptom:** `Gallery image version failed replication`

**Cause:** Replication to target region failed

**Solution:**

```bash
# Check replication status
az sig image-version show \
  --resource-group sindri-images \
  --gallery-name sindri_gallery \
  --gallery-image-definition sindri-dev \
  --gallery-image-version 1.0.0 \
  --query "publishingProfile.replicatedRegions"

# Retry replication
az sig image-version update \
  --resource-group sindri-images \
  --gallery-name sindri_gallery \
  --gallery-image-definition sindri-dev \
  --gallery-image-version 1.0.0 \
  --target-regions westus2 eastus
```

---

**Symptom:** `Image definition not compatible with security type`

**Cause:** Trusted Launch mismatch

**Solution:**

```bash
# Create compatible image definition
az sig image-definition create \
  --resource-group sindri-images \
  --gallery-name sindri_gallery \
  --gallery-image-definition sindri-trusted \
  --publisher Sindri \
  --offer SindriEnvironment \
  --sku Trusted \
  --os-type Linux \
  --os-state Generalized \
  --hyper-v-generation V2 \
  --features SecurityType=TrustedLaunch
```

### Networking Issues

**Symptom:** `Failed to download packages` during build

**Cause:** No outbound internet access

**Solution (NAT Gateway):**

```bash
# Create NAT gateway
az network nat gateway create \
  --resource-group sindri-network \
  --name sindri-nat \
  --public-ip-addresses sindri-nat-pip

# Associate with subnet
az network vnet subnet update \
  --resource-group sindri-network \
  --vnet-name sindri-vnet \
  --name packer-subnet \
  --nat-gateway sindri-nat
```

### Debugging

**Enable debug mode:**

```bash
# Sindri debug
sindri vm build --cloud azure --debug

# Or set Packer log level
export PACKER_LOG=1
sindri vm build --cloud azure

# View generated template
sindri vm build --cloud azure --dry-run
```

**Check Azure activity logs:**

```bash
az monitor activity-log list \
  --resource-group sindri-packer \
  --start-time $(date -u -d '1 hour ago' +%Y-%m-%dT%H:%M:%SZ) \
  --output table
```

## Related Documentation

- [Packer Provider Overview](../VM.md)
- [AWS Packer Guide](AWS.md)
- [GCP Packer Guide](GCP.md)
- [OCI Packer Guide](OCI.md)
- [Alibaba Packer Guide](ALIBABA.md)
- [Provider Overview](../README.md)
- [Configuration Reference](../../CONFIGURATION.md)
- [Secrets Management](../../SECRETS_MANAGEMENT.md)
- [Azure CLI Documentation](https://docs.microsoft.com/cli/azure/)
- [HashiCorp Packer Azure Builder](https://developer.hashicorp.com/packer/plugins/builders/azure)
- [Azure Compute Gallery](https://learn.microsoft.com/azure/virtual-machines/azure-compute-gallery)
