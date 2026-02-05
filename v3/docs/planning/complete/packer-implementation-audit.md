# Sindri V3 Packer Implementation Audit

**Date:** 2026-02-01
**Status:** Active Audit
**Author:** Engineering Audit

---

## Executive Summary

This audit examines the current state of Sindri V3's Packer support across CLI architecture, GitHub workflows/actions, cloud provider implementations, and documentation. Several critical issues were identified requiring immediate attention.

### Key Findings

| Area                     | Status               | Critical Issues                                 |
| ------------------------ | -------------------- | ----------------------------------------------- |
| CLI Architecture         | **Excellent**        | None - well-designed provider abstraction       |
| `v3-packer-build.yml`    | **Incomplete**       | Only 3/5 clouds (missing OCI, Alibaba)          |
| `v3-provider-packer.yml` | **BROKEN**           | Bypasses CLI, references non-existent templates |
| Cloud Provider Impl      | **Complete**         | Minor inconsistencies only                      |
| Documentation            | **Severely Lacking** | No PACKER.md, no cloud guides                   |

---

## Part 1: CLI Architecture Analysis

### 1.1 Architecture Overview

The V3 CLI uses a modern, well-designed Rust architecture with clean separation of concerns:

```
v3/crates/
├── sindri/              # Main CLI binary (commands, routing)
├── sindri-core/         # Configuration, types, validation
├── sindri-packer/       # VM image building (multi-cloud)
├── sindri-providers/    # Deployment backends (Docker, Fly, K8s)
├── sindri-extensions/   # Extension management system
└── ... (9 additional crates)
```

### 1.2 Packer Integration Architecture

**Entry Point:** `/v3/crates/sindri/src/commands/packer.rs` (652 lines)

The CLI provides 7 subcommands for Packer operations:

| Command              | Purpose               | Status      |
| -------------------- | --------------------- | ----------- |
| `sindri vm build`    | Build VM image        | Implemented |
| `sindri vm validate` | Validate template     | Implemented |
| `sindri vm list`     | List cloud images     | Implemented |
| `sindri vm delete`   | Delete image by ID    | Implemented |
| `sindri vm doctor`   | Check prerequisites   | Implemented |
| `sindri vm init`     | Generate HCL template | Implemented |
| `sindri vm deploy`   | Deploy from image     | Implemented |

### 1.3 Provider Abstraction Pattern

**Location:** `/v3/crates/sindri-packer/src/`

All cloud providers implement a unified `PackerProvider` trait:

```rust
pub trait PackerProvider: Send + Sync {
    fn cloud_name(&self) -> &'static str;
    async fn build_image(&self, config: &PackerConfig, opts: BuildOptions) -> Result<BuildResult>;
    async fn list_images(&self, config: &PackerConfig) -> Result<Vec<ImageInfo>>;
    async fn delete_image(&self, config: &PackerConfig, image_id: &str) -> Result<()>;
    async fn validate_template(&self, config: &PackerConfig) -> Result<ValidationResult>;
    fn check_cloud_prerequisites(&self) -> Result<CloudPrerequisiteStatus>;
    async fn find_cached_image(&self, config: &PackerConfig) -> Result<Option<String>>;
    async fn deploy_from_image(&self, image_id: &str, config: &PackerConfig) -> Result<DeployFromImageResult>;
    fn generate_template(&self, config: &PackerConfig) -> Result<String>;
}
```

### 1.4 Template System

Templates are **embedded in the Rust binary** using Tera templating:

**Location:** `/v3/crates/sindri-packer/src/templates/hcl/`

- `aws.pkr.hcl.tera`
- `azure.pkr.hcl.tera`
- `gcp.pkr.hcl.tera`
- `oci.pkr.hcl.tera`
- `alibaba.pkr.hcl.tera`

The CLI dynamically generates HCL2 templates at runtime based on user configuration. **There are NO standalone Packer template files in the repository.**

### 1.5 CLI Architecture Assessment

**Strengths:**

- Clean provider abstraction enabling multi-cloud support
- Unified interface across all 5 cloud providers
- Template generation handles complexity internally
- Proper async/await patterns throughout
- Comprehensive error handling

**Assessment:** The CLI architecture is well-designed and production-ready.

---

## Part 2: GitHub Workflows & Actions Analysis

### 2.1 Workflow Inventory

| Workflow                 | Purpose           | Uses CLI?   | Uses Custom Actions?      | Cloud Coverage             |
| ------------------------ | ----------------- | ----------- | ------------------------- | -------------------------- |
| `v3-packer-build.yml`    | Build VM images   | **Yes**     | No                        | 3/5 (missing OCI, Alibaba) |
| `v3-packer-test.yml`     | Test built images | **No**      | **No** (duplicates them!) | 3/5 (missing OCI, Alibaba) |
| `v3-provider-packer.yml` | Extension testing | **Partial** | **Yes**                   | 5/5 (but build broken)     |

### 2.2 v3-packer-build.yml Analysis

**Status: INCOMPLETE - Missing 2 of 5 Supported Clouds**

This workflow properly leverages the CLI but only supports 3 of 5 clouds:

```yaml
# Validation step
./v3/target/release/sindri vm validate --cloud aws \
  --sindri-version ${{ github.event.inputs.sindri_version || 'latest' }}

# Build step
./v3/target/release/sindri vm build --cloud aws \
  --sindri-version ${{ github.event.inputs.sindri_version || 'latest' }} \
  --profile ${{ github.event.inputs.profile || 'base' }} \
  --json > build-result.json
```

**Flow:**

1. Builds `sindri` binary from Rust source
2. Uses `sindri vm validate` for template validation
3. Uses `sindri vm build` for image building
4. Properly outputs structured JSON for downstream processing

**Supported Clouds:** AWS, Azure, GCP only

**Missing Clouds:** OCI and Alibaba - despite CLI supporting all 5 providers

**Gap Analysis:**

| Cloud   | CLI Support | Workflow Support | Status                |
| ------- | ----------- | ---------------- | --------------------- |
| AWS     | Yes         | Yes              | Complete              |
| Azure   | Yes         | Yes              | Complete              |
| GCP     | Yes         | Yes              | Complete              |
| OCI     | Yes         | **No**           | **Missing build job** |
| Alibaba | Yes         | **No**           | **Missing build job** |

The workflow needs `build-oci` and `build-alibaba` jobs added following the same pattern as the existing cloud jobs.

### 2.3 v3-provider-packer.yml Analysis

**Status: CRITICALLY BROKEN**

This workflow **bypasses the CLI** and makes false assumptions:

```yaml
# Lines 141-150 - BROKEN CODE
- name: Build Packer Image
  id: build
  if: steps.check.outputs.image_exists != 'true'
  working-directory: v3/packer # <-- PROBLEM: This directory has NO templates
  run: |
    packer init .
    packer build \
      -var "cloud=$CLOUD" \
      -var "region=${{ inputs.region }}" \
      -machine-readable \
      . | tee packer-output.log
```

**Critical Issues:**

1. **Non-existent Templates:** The `v3/packer/` directory contains only:

   ```
   v3/packer/
   └── scripts/
       └── openscap-scan.sh
   ```

   There are **NO Packer template files** (`.pkr.hcl`) in this directory.

2. **Bypasses CLI Abstraction:** Calls raw `packer init` and `packer build` instead of `sindri vm build`

3. **False Assumptions:** Assumes templates exist at `v3/packer/` when they are embedded in the Rust binary

4. **Caching Logic Incomplete:** Only implements AWS image caching check (lines 121-133), other clouds fall through

### 2.4 Custom Actions Analysis

**Location:** `.github/actions/packer/`

| Action                | Purpose                   | Status  |
| --------------------- | ------------------------- | ------- |
| `launch-instance/`    | Multi-cloud VM launcher   | Working |
| `terminate-instance/` | Multi-cloud VM terminator | Working |
| `providers/aws/`      | AWS EC2 management        | Working |
| `providers/azure/`    | Azure VM management       | Working |
| `providers/gcp/`      | GCP Compute management    | Working |
| `providers/oci/`      | OCI Compute management    | Working |
| `providers/alibaba/`  | Alibaba ECS management    | Working |

### 2.5 Custom Actions Deep Dive

**Location:** `.github/actions/packer/`

```
.github/actions/packer/
├── launch-instance/action.yml     # Multi-cloud dispatcher (170 lines)
├── terminate-instance/action.yml  # Multi-cloud dispatcher (89 lines)
└── providers/
    ├── aws/action.yml            # AWS EC2 impl (145 lines)
    ├── azure/action.yml          # Azure VM impl (137 lines)
    ├── gcp/action.yml            # GCP Compute impl (111 lines)
    ├── oci/action.yml            # OCI Compute impl (170 lines)
    └── alibaba/action.yml        # Alibaba ECS impl (236 lines)
```

#### Purpose

These actions were created for **testing VM images** - launching instances from Packer-built images to validate them, then terminating them. They are NOT used for image building.

#### Implementation Status by Provider

| Provider | Status       | SSH Key Handling             | CLI Install | Complexity   |
| -------- | ------------ | ---------------------------- | ----------- | ------------ |
| AWS      | **Basic**    | Pre-existing key required    | Assumed     | Simple       |
| Azure    | **Complete** | Auto-generates               | Assumed     | Medium       |
| GCP      | **Complete** | Generates fresh key          | Assumed     | Simple       |
| OCI      | **Complete** | Generates fresh key          | On-demand   | Complex      |
| Alibaba  | **Complete** | Generates + imports key pair | On-demand   | Most complex |

#### Per-Provider Analysis

**AWS (`providers/aws/action.yml`):**

- Relies on pre-existing SSH key pair (`sindri-test`)
- Expects `AWS_SSH_PRIVATE_KEY` env var or falls back to `~/.ssh/id_rsa`
- No key generation - will fail if key pair doesn't exist
- Basic implementation compared to others

**Azure (`providers/azure/action.yml`):**

- Uses `--generate-ssh-keys` flag (auto-creates)
- Creates resource group if not exists
- Comprehensive cleanup: VM, NIC, disk, public IP

**GCP (`providers/gcp/action.yml`):**

- **FIXED:** Now generates fresh SSH key pair per instance (like OCI/Alibaba)
- Uses direct SSH key injection via metadata (not OS Login)
- Compatible with external SSH tools like InSpec
- Uses `sindri` as the SSH username

**OCI (`providers/oci/action.yml`):**

- Installs OCI CLI on-demand
- Generates fresh SSH key per instance
- Complex VNIC lookup for public IP
- Proper cleanup of keys

**Alibaba (`providers/alibaba/action.yml`):**

- Installs Aliyun CLI on-demand
- Generates SSH key and imports as cloud key pair
- Allocates elastic IP and associates
- Most comprehensive cleanup (EIP, key pairs, instance)

#### Who Uses These Actions?

| Workflow                 | Uses Actions? | Notes                                |
| ------------------------ | ------------- | ------------------------------------ |
| `v3-provider-packer.yml` | **YES**       | Lines 206, 271                       |
| `v3-packer-build.yml`    | **NO**        | Builds images only                   |
| `v3-packer-test.yml`     | **NO**        | **DUPLICATES functionality inline!** |

#### Critical Issue: Duplicated Functionality

`v3-packer-test.yml` **reimplements** the same cloud VM launch/terminate logic inline rather than using the custom actions:

```yaml
# v3-packer-test.yml - test-aws job (lines 80-128)
- name: Launch Test Instance
  run: |
    INSTANCE_ID=$(aws ec2 run-instances ...)  # Duplicates providers/aws/action.yml
```

This creates:

1. **Code duplication** across two places
2. **Maintenance burden** - fixes must be applied twice
3. **Inconsistent behavior** - inline code differs from actions

---

### 2.6 v3-packer-test.yml Analysis

**Status: INCOMPLETE and INCONSISTENT**

#### Current State

| Aspect              | Status                | Issue                                                     |
| ------------------- | --------------------- | --------------------------------------------------------- |
| Cloud Coverage      | 3/5                   | Missing OCI, Alibaba test jobs                            |
| Uses Custom Actions | NO                    | Duplicates functionality inline                           |
| Uses CLI            | NO                    | Raw cloud CLIs throughout                                 |
| Trigger             | Manual + workflow_run | Only triggers from "Build Sindri VM Images" (wrong name?) |

#### Structure

```yaml
jobs:
  test-aws: # Inline AWS instance management - 93 lines
  test-azure: # Inline Azure instance management - 66 lines
  test-gcp: # Inline GCP instance management - 65 lines
  summary: # Aggregates results
```

#### Issues Identified

1. **Missing Clouds:** No `test-oci` or `test-alibaba` jobs despite:
   - CLI supporting all 5 clouds
   - Custom actions supporting all 5 clouds
   - `v3-packer-build.yml` workflow supporting (potentially) all 5 clouds

2. **Doesn't Use Custom Actions:** Each test job has ~60-90 lines of inline instance management code that duplicates `.github/actions/packer/providers/*`

3. **workflow_run Trigger Issue:**

   ```yaml
   workflow_run:
     workflows: ["Build Sindri VM Images"] # This name doesn't match v3-packer-build.yml
   ```

   The actual workflow name is `"v3: Build Sindri VM Images"` - this may cause trigger failures.

4. **Hardcoded Resource References:**
   - `--key-name sindri-test` (AWS)
   - `--resource-group sindri-test` (Azure)
   - `--security-group-ids ${{ secrets.AWS_SECURITY_GROUP_ID }}`
   - These assume specific infrastructure exists

5. **InSpec Profile:** `v3/test/integration/sindri/` exists with controls for:
   - `docker_installed.rb` - Docker installation verification
   - `extensions.rb` - Extension system checks
   - `mise.rb` - Mise installation
   - `security.rb` - Security hardening
   - `sindri_installed.rb` - Sindri CLI presence

6. **No CLI Usage:** Uses raw cloud CLIs instead of `sindri vm deploy`

#### Recommended Refactor

```yaml
test-aws:
  steps:
    - name: Launch Test Instance
      uses: ./.github/actions/packer/launch-instance
      with:
        cloud: aws
        image-id: ${{ steps.image.outputs.image_id }}
        region: ${{ inputs.region }}

    # ... run InSpec tests ...

    - name: Terminate Instance
      uses: ./.github/actions/packer/terminate-instance
      with:
        cloud: aws
        instance-id: ${{ steps.launch.outputs.instance-id }}
```

---

### 2.7 Architectural Inconsistencies Summary

| Issue                                          | Impact                               | Severity   |
| ---------------------------------------------- | ------------------------------------ | ---------- |
| `v3-packer-test.yml` duplicates custom actions | Code duplication, maintenance burden | **High**   |
| Test workflow missing OCI, Alibaba             | Incomplete test coverage             | **High**   |
| CLI not used for instance deploy               | Bypasses abstraction                 | **Medium** |
| SSH auth varies wildly by provider             | Confusing, inconsistent              | **Medium** |
| GCP OS Login mismatch                          | Potential SSH failures               | **Medium** |
| AWS requires pre-existing key pair             | Setup friction                       | **Low**    |
| Workflow trigger name mismatch                 | Tests may not auto-run               | **Medium** |

---

### 2.8 Workflow Architecture Assessment

**Root Cause of Issues:**

The workflows were partially migrated to use the CLI but the `v3-provider-packer.yml` workflow was left in an inconsistent state where it attempts to run raw Packer commands against a non-existent template directory.

**Recommended Fix:**

Replace the broken `v3-provider-packer.yml` build step with CLI invocation:

```yaml
- name: Build Packer Image
  run: |
    ./v3/target/release/sindri vm build --cloud ${{ inputs.cloud }} \
      --region ${{ inputs.region }} \
      --json > build-result.json
    IMAGE_ID=$(jq -r '.image_id' build-result.json)
    echo "image_id=$IMAGE_ID" >> $GITHUB_OUTPUT
```

---

## Part 3: Cloud Provider Implementation Status

### 3.1 Implementation Matrix

| Provider | Trait Impl | Templates              | Tests | Deploy | Status               |
| -------- | ---------- | ---------------------- | ----- | ------ | -------------------- |
| AWS      | Complete   | `aws.pkr.hcl.tera`     | Yes   | Yes    | **Production Ready** |
| Azure    | Complete   | `azure.pkr.hcl.tera`   | Yes   | Yes    | **Production Ready** |
| GCP      | Complete   | `gcp.pkr.hcl.tera`     | Yes   | Yes    | **Production Ready** |
| OCI      | Complete   | `oci.pkr.hcl.tera`     | Yes   | Yes    | **Production Ready** |
| Alibaba  | Complete   | `alibaba.pkr.hcl.tera` | Yes   | Yes    | **Production Ready** |

### 3.2 Configuration Comparison

| Config      | Required Fields                                    | Defaults                                                   |
| ----------- | -------------------------------------------------- | ---------------------------------------------------------- |
| **AWS**     | None                                               | region=us-west-2, instance_type=t3.large, volume_size=80GB |
| **Azure**   | subscription_id, resource_group                    | location=westus2, vm_size=Standard_D4s_v4                  |
| **GCP**     | project_id                                         | zone=us-west1-a, machine_type=e2-standard-4                |
| **OCI**     | compartment_ocid, availability_domain, subnet_ocid | shape=VM.Standard.E4.Flex                                  |
| **Alibaba** | None                                               | region=cn-hangzhou, instance_type=ecs.g6.xlarge            |

### 3.3 Minor Inconsistencies

| Aspect       | AWS       | Azure     | GCP       | OCI           | Alibaba         |
| ------------ | --------- | --------- | --------- | ------------- | --------------- |
| SSH User     | ubuntu    | sindri    | ubuntu    | ubuntu        | root            |
| IP Retrieval | Immediate | Immediate | Immediate | None          | 10s delay       |
| Tags Format  | Key/Value | Flat      | labels    | freeform-tags | TagKey/TagValue |

These are minor implementation details and do not affect functionality.

### 3.4 Prerequisite CLI Tools

| Provider | Required CLI | Installation                                                                  |
| -------- | ------------ | ----------------------------------------------------------------------------- |
| AWS      | `aws`        | https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html |
| Azure    | `az`         | https://learn.microsoft.com/en-us/cli/azure/install-azure-cli                 |
| GCP      | `gcloud`     | https://cloud.google.com/sdk/docs/install                                     |
| OCI      | `oci`        | https://docs.oracle.com/en-us/iaas/Content/API/SDKDocs/cliinstall.htm         |
| Alibaba  | `aliyun`     | https://www.alibabacloud.com/help/en/cli/install-cli                          |
| All      | `packer`     | https://developer.hashicorp.com/packer/install                                |

---

## Part 4: Credential & IAM Prerequisites

This section documents all authentication requirements for both local CLI operations and CI/CD workflows.

### 4.1 CLI Local Operations

For developers running `sindri vm` commands locally, the following credentials are required:

#### AWS

**Required IAM Permissions:**

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
        "ec2:DescribeInstanceStatus",
        "ec2:DescribeRegions",
        "ec2:DescribeSecurityGroups",
        "ec2:DescribeSnapshots",
        "ec2:DescribeSubnets",
        "ec2:DescribeVolumes",
        "ec2:DescribeVpcs",
        "ec2:CreateSnapshot",
        "ec2:DeleteSnapshot",
        "ec2:ModifyImageAttribute",
        "ec2:RunInstances",
        "ec2:StopInstances",
        "ec2:TerminateInstances",
        "ec2:CreateTags",
        "ec2:CreateKeyPair",
        "ec2:DeleteKeyPair",
        "ec2:GetPasswordData"
      ],
      "Resource": "*"
    },
    {
      "Effect": "Allow",
      "Action": ["sts:GetCallerIdentity"],
      "Resource": "*"
    }
  ]
}
```

**Authentication Methods:**

1. `aws configure` - Interactive setup with Access Key ID and Secret Access Key
2. Environment variables: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_DEFAULT_REGION`
3. AWS SSO: `aws sso login --profile <profile-name>`
4. IAM Instance Profile (when running on EC2)

**Credential Check:** CLI runs `aws sts get-caller-identity` to verify authentication.

#### Azure

**Required Azure RBAC Roles:**

- `Contributor` role on the target Resource Group (minimum)
- For Shared Image Gallery: `Contributor` on the gallery resource

**Required API Permissions:**

- Microsoft.Compute/images/\*
- Microsoft.Compute/virtualMachines/\*
- Microsoft.Compute/galleries/\* (for Shared Image Gallery)
- Microsoft.Network/virtualNetworks/read
- Microsoft.Network/publicIPAddresses/\*
- Microsoft.Network/networkInterfaces/\*
- Microsoft.Resources/subscriptions/resourceGroups/\*

**Authentication Methods:**

1. `az login` - Interactive browser login
2. Service Principal: `az login --service-principal -u <client-id> -p <secret> --tenant <tenant-id>`
3. Managed Identity (when running on Azure VMs)
4. Environment variables: `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID`

**Credential Check:** CLI runs `az account show` to verify authentication.

#### GCP

**Required IAM Roles:**

- `roles/compute.instanceAdmin.v1` - Create/manage instances
- `roles/compute.imageUser` - Use images
- `roles/iam.serviceAccountUser` - Act as service account

**Custom Role Permissions (minimum):**

```
compute.disks.create
compute.disks.delete
compute.disks.useReadOnly
compute.images.create
compute.images.delete
compute.images.get
compute.images.list
compute.images.setIamPolicy
compute.instances.create
compute.instances.delete
compute.instances.get
compute.instances.setMetadata
compute.instances.setServiceAccount
compute.machineTypes.get
compute.networks.get
compute.projects.get
compute.subnetworks.use
compute.subnetworks.useExternalIp
compute.zones.get
```

**Authentication Methods:**

1. `gcloud auth login` - Interactive browser login
2. `gcloud auth application-default login` - For applications
3. Service Account key: `GOOGLE_APPLICATION_CREDENTIALS=/path/to/key.json`
4. Workload Identity (on GKE)

**Credential Check:** CLI runs `gcloud auth print-access-token` to verify authentication.

#### OCI (Oracle Cloud Infrastructure)

**Required IAM Policies:**

```
Allow group <group-name> to manage instances in compartment <compartment-name>
Allow group <group-name> to manage instance-images in compartment <compartment-name>
Allow group <group-name> to use virtual-network-family in compartment <compartment-name>
Allow group <group-name> to manage volume-family in compartment <compartment-name>
```

**Authentication Methods:**

1. `oci setup config` - Interactive configuration
2. Config file: `~/.oci/config` with API signing key
3. Environment variables: `OCI_USER_OCID`, `OCI_TENANCY_OCID`, `OCI_FINGERPRINT`, `OCI_REGION`
4. Instance Principal (when running on OCI)

**Required Config Values:**

- User OCID
- Tenancy OCID
- Compartment OCID
- API signing key fingerprint
- Private key file path

**Credential Check:** CLI runs `oci iam region list` to verify authentication.

#### Alibaba Cloud

**Required RAM Permissions:**

```json
{
  "Version": "1",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "ecs:CreateImage",
        "ecs:DeleteImage",
        "ecs:DescribeImages",
        "ecs:ModifyImageAttribute",
        "ecs:CopyImage",
        "ecs:CreateInstance",
        "ecs:DeleteInstance",
        "ecs:StartInstance",
        "ecs:StopInstance",
        "ecs:DescribeInstances",
        "ecs:DescribeInstanceStatus",
        "ecs:AllocatePublicIpAddress",
        "ecs:CreateKeyPair",
        "ecs:DeleteKeyPairs",
        "ecs:ImportKeyPair",
        "ecs:DescribeKeyPairs",
        "ecs:CreateSecurityGroup",
        "ecs:AuthorizeSecurityGroup",
        "ecs:DescribeSecurityGroups"
      ],
      "Resource": "*"
    },
    {
      "Effect": "Allow",
      "Action": [
        "vpc:DescribeVpcs",
        "vpc:DescribeVSwitches",
        "vpc:AllocateEipAddress",
        "vpc:AssociateEipAddress",
        "vpc:UnassociateEipAddress",
        "vpc:ReleaseEipAddress",
        "vpc:DescribeEipAddresses"
      ],
      "Resource": "*"
    }
  ]
}
```

**Authentication Methods:**

1. `aliyun configure` - Interactive setup
2. Environment variables: `ALICLOUD_ACCESS_KEY`, `ALICLOUD_SECRET_KEY` (or `ALIBABA_CLOUD_ACCESS_KEY_ID`, `ALIBABA_CLOUD_ACCESS_KEY_SECRET`)
3. RAM role (when running on Alibaba Cloud ECS)

**Credential Check:** CLI checks for `ALICLOUD_ACCESS_KEY` env var or runs `aliyun configure list`.

### 4.2 GitHub Actions Workflow Requirements

For CI/CD workflows, use **OIDC (OpenID Connect)** authentication instead of long-lived secrets. This is the [recommended best practice for 2025-2026](https://docs.github.com/en/actions/concepts/security/openid-connect).

#### OIDC vs Static Credentials

| Aspect   | OIDC (Recommended)              | Static Secrets                 |
| -------- | ------------------------------- | ------------------------------ |
| Security | Short-lived tokens, auto-expire | Long-lived, risk of exposure   |
| Rotation | Automatic                       | Manual rotation required       |
| Scope    | Per-job, fine-grained           | Broad, often over-permissioned |
| Audit    | Full traceability               | Limited                        |

#### AWS OIDC Setup

**1. Create Identity Provider in IAM:**

```bash
aws iam create-open-id-connect-provider \
  --url https://token.actions.githubusercontent.com \
  --client-id-list sts.amazonaws.com \
  --thumbprint-list 6938fd4d98bab03faadb97b34396831e3780aea1
```

**2. Create IAM Role with Trust Policy:**

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Principal": {
        "Federated": "arn:aws:iam::<ACCOUNT_ID>:oidc-provider/token.actions.githubusercontent.com"
      },
      "Action": "sts:AssumeRoleWithWebIdentity",
      "Condition": {
        "StringEquals": {
          "token.actions.githubusercontent.com:aud": "sts.amazonaws.com"
        },
        "StringLike": {
          "token.actions.githubusercontent.com:sub": "repo:<ORG>/<REPO>:*"
        }
      }
    }
  ]
}
```

**3. GitHub Secrets Required:**

- `AWS_ROLE_ARN` - ARN of the IAM role to assume

**4. Workflow Usage:**

```yaml
permissions:
  id-token: write
  contents: read

- uses: aws-actions/configure-aws-credentials@v4
  with:
    role-to-assume: ${{ secrets.AWS_ROLE_ARN }}
    aws-region: us-west-2
```

#### Azure OIDC Setup

**1. Create App Registration:**

```bash
az ad app create --display-name "github-actions-sindri"
```

**2. Configure Federated Credentials:**

```bash
az ad app federated-credential create \
  --id <APPLICATION_ID> \
  --parameters '{
    "name": "github-main",
    "issuer": "https://token.actions.githubusercontent.com",
    "subject": "repo:<ORG>/<REPO>:ref:refs/heads/main",
    "audiences": ["api://AzureADTokenExchange"]
  }'
```

**3. Create Service Principal and Assign Roles:**

```bash
az ad sp create --id <APPLICATION_ID>
az role assignment create \
  --assignee <APPLICATION_ID> \
  --role Contributor \
  --scope /subscriptions/<SUBSCRIPTION_ID>/resourceGroups/<RESOURCE_GROUP>
```

**4. GitHub Secrets Required:**

- `AZURE_CLIENT_ID` - Application (client) ID
- `AZURE_TENANT_ID` - Directory (tenant) ID
- `AZURE_SUBSCRIPTION_ID` - Azure subscription ID

**5. Workflow Usage:**

```yaml
permissions:
  id-token: write
  contents: read

- uses: azure/login@v2
  with:
    client-id: ${{ secrets.AZURE_CLIENT_ID }}
    tenant-id: ${{ secrets.AZURE_TENANT_ID }}
    subscription-id: ${{ secrets.AZURE_SUBSCRIPTION_ID }}
```

#### GCP OIDC Setup (Workload Identity Federation)

**1. Create Workload Identity Pool:**

```bash
gcloud iam workload-identity-pools create github-pool \
  --location=global \
  --display-name="GitHub Actions Pool"
```

**2. Create Provider:**

```bash
gcloud iam workload-identity-pools providers create-oidc github-provider \
  --location=global \
  --workload-identity-pool=github-pool \
  --issuer-uri=https://token.actions.githubusercontent.com \
  --attribute-mapping="google.subject=assertion.sub,attribute.repository=assertion.repository" \
  --attribute-condition="assertion.repository=='<ORG>/<REPO>'"
```

**3. Grant Service Account Access:**

```bash
gcloud iam service-accounts add-iam-policy-binding <SA_EMAIL> \
  --role=roles/iam.workloadIdentityUser \
  --member="principalSet://iam.googleapis.com/projects/<PROJECT_NUMBER>/locations/global/workloadIdentityPools/github-pool/attribute.repository/<ORG>/<REPO>"
```

**4. GitHub Secrets Required:**

- `GCP_WORKLOAD_IDENTITY_PROVIDER` - Full provider resource name
- `GCP_SERVICE_ACCOUNT` - Service account email
- `GCP_PROJECT_ID` - GCP project ID

**5. Workflow Usage:**

```yaml
permissions:
  id-token: write
  contents: read

- uses: google-github-actions/auth@v2
  with:
    workload_identity_provider: ${{ secrets.GCP_WORKLOAD_IDENTITY_PROVIDER }}
    service_account: ${{ secrets.GCP_SERVICE_ACCOUNT }}
```

#### OCI GitHub Actions Setup

OCI does not yet support OIDC federation with GitHub Actions. Use API key authentication:

**GitHub Secrets Required:**

- `OCI_USER_OCID` - User OCID
- `OCI_TENANCY_OCID` - Tenancy OCID
- `OCI_FINGERPRINT` - API key fingerprint
- `OCI_PRIVATE_KEY` - Private key contents (base64 encoded)
- `OCI_REGION` - Default region

**Workflow Setup:**

```yaml
- name: Configure OCI CLI
  run: |
    mkdir -p ~/.oci
    echo '${{ secrets.OCI_PRIVATE_KEY }}' | base64 -d > ~/.oci/oci_api_key.pem
    chmod 600 ~/.oci/oci_api_key.pem
    cat > ~/.oci/config << EOF
    [DEFAULT]
    user=${{ secrets.OCI_USER_OCID }}
    fingerprint=${{ secrets.OCI_FINGERPRINT }}
    tenancy=${{ secrets.OCI_TENANCY_OCID }}
    region=${{ secrets.OCI_REGION }}
    key_file=~/.oci/oci_api_key.pem
    EOF
```

#### Alibaba Cloud GitHub Actions Setup

Alibaba Cloud supports OIDC via RAM roles for OIDC:

**1. Create OIDC Provider:**

```bash
aliyun ram CreateOIDCProvider \
  --OIDCProviderName github-actions \
  --Fingerprints 6938fd4d98bab03faadb97b34396831e3780aea1 \
  --IssuerUrl https://token.actions.githubusercontent.com \
  --ClientIds sts.aliyuncs.com
```

**2. Create RAM Role for OIDC:**

```json
{
  "Statement": [
    {
      "Action": "sts:AssumeRoleWithOIDC",
      "Effect": "Allow",
      "Principal": {
        "OIDC": ["acs:ram::<ACCOUNT_ID>:oidc-provider/github-actions"]
      },
      "Condition": {
        "StringEquals": {
          "oidc:sub": "repo:<ORG>/<REPO>:ref:refs/heads/main"
        }
      }
    }
  ],
  "Version": "1"
}
```

**GitHub Secrets Required (fallback to static):**

- `ALIBABA_ACCESS_KEY` - AccessKey ID
- `ALIBABA_SECRET_KEY` - AccessKey Secret

### 4.3 Secrets Required by Current Workflows

Based on the current workflow implementations:

| Workflow           | Secret                           | Purpose                   | Required    |
| ------------------ | -------------------------------- | ------------------------- | ----------- |
| v3-packer-build    | `AWS_ROLE_ARN`                   | AWS OIDC role             | Yes (AWS)   |
| v3-packer-build    | `AZURE_CLIENT_ID`                | Azure app ID              | Yes (Azure) |
| v3-packer-build    | `AZURE_TENANT_ID`                | Azure tenant              | Yes (Azure) |
| v3-packer-build    | `AZURE_SUBSCRIPTION_ID`          | Azure subscription        | Yes (Azure) |
| v3-packer-build    | `GCP_WORKLOAD_IDENTITY_PROVIDER` | GCP WIF provider          | Yes (GCP)   |
| v3-packer-build    | `GCP_SERVICE_ACCOUNT`            | GCP service account       | Yes (GCP)   |
| v3-packer-build    | `GCP_PROJECT_ID`                 | GCP project               | Yes (GCP)   |
| v3-packer-test     | `AWS_SSH_PRIVATE_KEY`            | SSH key for testing       | Yes (AWS)   |
| v3-packer-test     | `AWS_SECURITY_GROUP_ID`          | SG for test instances     | Yes (AWS)   |
| v3-packer-test     | `AWS_SUBNET_ID`                  | Subnet for test instances | Yes (AWS)   |
| v3-provider-packer | All cloud secrets                | Per-cloud auth            | Per cloud   |

### 4.4 Missing Secrets for OCI/Alibaba Workflows

The following secrets need to be configured to enable OCI and Alibaba builds:

**OCI:**

- `OCI_USER_OCID`
- `OCI_TENANCY_OCID`
- `OCI_FINGERPRINT`
- `OCI_PRIVATE_KEY`
- `OCI_COMPARTMENT_OCID`
- `OCI_SUBNET_OCID`

**Alibaba:**

- `ALIBABA_ACCESS_KEY`
- `ALIBABA_SECRET_KEY`
- `ALIBABA_VSWITCH_ID`
- `ALIBABA_SECURITY_GROUP_ID`

---

## Part 5: Image Distribution & Public Sharing

This section covers how to distribute Sindri VM images publicly according to 2025-2026 cloud best practices.

### 5.1 AWS AMI Distribution

**Options:**

| Method          | Audience                | Best For               |
| --------------- | ----------------------- | ---------------------- |
| Private AMI     | Owner account only      | Development            |
| Shared AMI      | Specific AWS accounts   | Partners, customers    |
| Public AMI      | All AWS users           | Open source, community |
| AWS Marketplace | Commercial distribution | Monetization           |

**Public AMI Best Practices ([AWS Documentation](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/building-shared-amis.html)):**

1. **Security Requirements:**
   - Disable root password login
   - Remove all SSH authorized_keys
   - Remove AWS credentials from image
   - Remove bash history and sensitive logs
   - Use cloud-init for first-boot configuration

2. **Deprecation Policy:**
   - Public AMIs auto-deprecate after 2 years
   - If unused for 6+ months after deprecation, AWS removes public sharing
   - Set explicit deprecation dates for version control

3. **Making AMI Public:**

   ```bash
   aws ec2 modify-image-attribute \
     --image-id ami-xxxx \
     --launch-permission "Add=[{Group=all}]"
   ```

4. **Block Public Access (Security):**
   - Since October 2023, new AWS accounts have public AMI sharing blocked by default
   - Must explicitly enable public sharing at account level

**AWS Marketplace Publishing:**

- Requires [AWS Partner registration](https://aws.amazon.com/partners/)
- Security review required
- Supports paid, BYOL, and free models
- [Best practices guide](https://docs.aws.amazon.com/marketplace/latest/userguide/best-practices-for-building-your-amis.html)

### 5.2 Azure Image Distribution

**Options:**

| Method                            | Audience                          | Best For     |
| --------------------------------- | --------------------------------- | ------------ |
| Managed Image                     | Single subscription               | Development  |
| Azure Compute Gallery (Direct)    | Up to 30 subscriptions, 5 tenants | Enterprise   |
| Azure Compute Gallery (Community) | All Azure users                   | Open source  |
| Azure Marketplace                 | Commercial distribution           | Monetization |

**Community Gallery ([Microsoft Documentation](https://learn.microsoft.com/en-us/azure/virtual-machines/share-gallery-community)):**

Community galleries are the Azure equivalent of AWS public AMIs:

1. **Creating a Community Gallery:**

   ```bash
   az sig create \
     --resource-group myResourceGroup \
     --gallery-name myGallery \
     --sharing-profile "Community" \
     --publisher-uri "https://sindri.dev" \
     --publisher-email "support@sindri.dev" \
     --eula "https://sindri.dev/eula"
   ```

2. **Requirements:**
   - Valid publisher URL
   - Valid contact email
   - Legal agreement/EULA URL
   - Gallery prefix (appended with GUID for uniqueness)

3. **2025 API Changes:**
   - Starting with API 2025-03-03, image definitions default to TrustedLaunch validation
   - Automatic security validation for Trusted Launch compatibility

**Azure Marketplace Publishing:**

- Requires [Partner Center registration](https://partner.microsoft.com/)
- Security review and certification required
- Supports transactable and BYOL models

### 5.3 GCP Image Distribution

**Options:**

| Method                | Audience                | Best For     |
| --------------------- | ----------------------- | ------------ |
| Project Image         | Single project          | Development  |
| Cross-Project Sharing | Specific projects       | Organization |
| Public Image          | All GCP users           | Open source  |
| Cloud Marketplace     | Commercial distribution | Monetization |

**Cross-Project Sharing ([GCP Documentation](https://cloud.google.com/compute/docs/images/managing-access-custom-images)):**

1. **Grant Access to Specific Users/Projects:**

   ```bash
   gcloud compute images add-iam-policy-binding my-image \
     --member='user:user@example.com' \
     --role='roles/compute.imageUser'
   ```

2. **Make Image Public:**

   ```bash
   gcloud compute images add-iam-policy-binding my-image \
     --member='allAuthenticatedUsers' \
     --role='roles/compute.imageUser'
   ```

3. **Security Warning:**
   - Granting `allAuthenticatedUsers` allows ANY Google account to use the image
   - Not recommended for sensitive images
   - Consider using organization-level sharing instead

**Image Families ([Best Practices](https://cloud.google.com/compute/docs/images/image-families-best-practices)):**

- Use image families for semantic versioning
- Latest image in family is auto-selected
- Enables rolling updates without changing references

**Cloud Marketplace (formerly Cloud Launcher):**

- Contact Google to onboard images
- Requires partner agreement
- Supports paid and free models

### 5.4 OCI Image Distribution

**Options:**

| Method                      | Audience           | Best For             |
| --------------------------- | ------------------ | -------------------- |
| Custom Image                | Single tenancy     | Development          |
| Cross-Tenancy Export/Import | Specific tenancies | Partners             |
| OCI Marketplace             | All OCI users      | Community/Commercial |

**OCI Marketplace Publishing ([Oracle Documentation](https://docs.oracle.com/en-us/iaas/Content/Marketplace/Concepts/marketoverview.htm)):**

1. **Image Requirements:**
   - Use paravirtualized mode for best compatibility
   - Build on official OCI base images when possible
   - Remove sensitive data using `oci-image-cleanup` utility
   - Test with flexible shapes

2. **Publishing Process:**
   - Register at [Oracle Cloud Marketplace Partner Portal](https://cloudmarketplace.oracle.com/)
   - Create listing with image, description, pricing
   - Submit for Oracle review
   - Supports free, BYOL, and paid models

3. **Community Applications:**
   - New feature for sharing non-commercial images
   - Similar to AWS public AMIs

**Cross-Tenancy Sharing:**

```bash
# Export image
oci compute image export to-object \
  --image-id <image_ocid> \
  --bucket-name my-bucket \
  --name sindri-image.oci

# Import in target tenancy
oci compute image import from-object \
  --compartment-id <target_compartment> \
  --bucket-name shared-bucket \
  --name sindri-image.oci
```

### 5.5 Alibaba Cloud Image Distribution

**Options:**

| Method                    | Audience                        | Best For     |
| ------------------------- | ------------------------------- | ------------ |
| Custom Image              | Single account                  | Development  |
| Shared Image              | Specific accounts (same region) | Partners     |
| Copy to Marketplace       | All Alibaba Cloud users         | Community    |
| Alibaba Cloud Marketplace | Commercial distribution         | Monetization |

**Image Sharing ([Alibaba Documentation](https://www.alibabacloud.com/help/en/ecs/user-guide/share-a-custom-image)):**

1. **Limitations:**
   - Cannot share across regions (must copy first)
   - Cannot share Marketplace-based images
   - Sharees assume security responsibility

2. **Share with Specific Accounts:**
   ```bash
   aliyun ecs ModifyImageSharePermission \
     --RegionId us-west-1 \
     --ImageId m-xxxx \
     --AddAccount.1 <target_account_id>
   ```

**Alibaba Cloud Marketplace ([Publishing Guide](https://www.alibabacloud.com/help/en/marketplace/publish-image-products)):**

- Security review required before publishing
- Cannot republish purchased Marketplace images
- Supports paid and free models

### 5.6 Recommended Distribution Strategy for Sindri

**Tier 1: Community/Open Source (Free)**

| Cloud   | Method                                | Status      |
| ------- | ------------------------------------- | ----------- |
| AWS     | Public AMI with deprecation schedule  | Recommended |
| Azure   | Community Gallery                     | Recommended |
| GCP     | Public Image or allAuthenticatedUsers | Recommended |
| OCI     | Marketplace (Community Application)   | Recommended |
| Alibaba | Marketplace (Free listing)            | Recommended |

**Tier 2: Enterprise/Partner**

| Cloud   | Method                                    | Use Case             |
| ------- | ----------------------------------------- | -------------------- |
| AWS     | Shared AMI to specific accounts           | Enterprise customers |
| Azure   | Direct Share Gallery (30 subs, 5 tenants) | Enterprise customers |
| GCP     | Cross-project IAM binding                 | Organization sharing |
| OCI     | Cross-tenancy export/import               | Partner distribution |
| Alibaba | Account-level sharing                     | Partner distribution |

**Tier 3: Commercial (If Monetizing)**

All clouds support marketplace publishing with paid models. Requires:

- Partner registration with each cloud
- Security review and certification
- Legal agreements (EULA, support terms)
- Pricing model definition

### 5.7 Image Security Checklist for Distribution

Before making any image public, verify:

- [ ] Root password disabled or randomized
- [ ] SSH authorized_keys cleared
- [ ] Cloud credentials removed (`~/.aws`, `~/.azure`, `~/.config/gcloud`, `~/.oci`)
- [ ] Bash history cleared
- [ ] Log files cleared or rotated
- [ ] Git credentials removed
- [ ] Docker credentials removed
- [ ] Private keys removed
- [ ] Environment variables with secrets cleared
- [ ] Cloud-init configured for first-boot setup
- [ ] Security hardening applied (CIS benchmarks)
- [ ] Vulnerability scan completed

The Sindri CLI's `cleanup.sh.tera` template handles most of these automatically.

---

## Part 6: Documentation Gap Analysis

### 6.1 Existing Documentation Structure

```
v3/docs/
├── providers/
│   ├── README.md      # Provider overview
│   ├── DOCKER.md      # Docker provider (675 lines)
│   ├── FLY.md         # Fly.io provider (585 lines)
│   ├── DEVPOD.md      # DevPod provider (537 lines)
│   ├── E2B.md         # E2B provider (534 lines)
│   ├── KUBERNETES.md  # K8s provider (712 lines)
│   └── PACKER.md      # **MISSING**
```

### 6.2 Critical Missing Documentation

| Document                      | Priority | Description                            |
| ----------------------------- | -------- | -------------------------------------- |
| `providers/VM.md`             | **P0**   | Main Packer provider guide             |
| `providers/AWS-PACKER.md`     | **P1**   | AWS-specific setup and usage           |
| `providers/AZURE-PACKER.md`   | **P1**   | Azure-specific setup and usage         |
| `providers/GCP-PACKER.md`     | **P1**   | GCP-specific setup and usage           |
| `providers/OCI-PACKER.md`     | **P1**   | OCI-specific setup and usage           |
| `providers/ALIBABA-PACKER.md` | **P1**   | Alibaba-specific setup and usage       |
| CLI.md updates                | **P1**   | Complete `packer` subcommand reference |
| TROUBLESHOOTING.md updates    | **P2**   | Packer troubleshooting section         |

### 6.3 Documentation Completeness Assessment

| Category               | Score | Notes                                     |
| ---------------------- | ----- | ----------------------------------------- |
| Packer Provider Docs   | 0%    | No documentation exists                   |
| Cloud Getting Started  | 0%    | No cloud-specific guides                  |
| CLI Reference          | 10%   | Brief mentions only, no command reference |
| Configuration Examples | 5%    | Only one example in SECRETS_MANAGEMENT.md |
| Prerequisites          | 0%    | CLI tools not documented                  |
| Troubleshooting        | 0%    | No Packer section                         |

### 6.4 Existing Resources

The following planning documents exist but are not user-facing:

- **ADR-031:** `/v3/docs/architecture/adr/031-packer-vm-provisioning-architecture.md` (Accepted)
- **Planning Doc:** `/v3/docs/planning/complete/packer-vm-provisioning-architecture.md` (2,832 lines)

These provide excellent technical detail but need to be translated into user documentation.

---

## Part 7: Recommendations

### 7.1 Critical (P0) - Immediate Action Required

1. **Fix v3-provider-packer.yml**
   - Replace raw Packer invocation with CLI commands
   - Remove assumption of `v3/packer/` template directory
   - Implement caching for all clouds, not just AWS

2. **Complete v3-packer-build.yml - Add Missing Clouds**
   - Add `build-oci` job following existing pattern
   - Add `build-alibaba` job following existing pattern
   - Configure OIDC/credentials for OCI and Alibaba
   - Update the `summary` job to include all 5 clouds
   - The CLI supports all 5 clouds but workflow only builds 3

3. **Refactor v3-packer-test.yml to use custom actions**
   - Replace inline instance management with `.github/actions/packer/launch-instance`
   - Add `test-oci` and `test-alibaba` jobs for full coverage
   - Fix workflow_run trigger name: `"Build Sindri VM Images"` → `"v3: Build Sindri VM Images"`
   - Verify InSpec profile path exists: `v3/test/integration/sindri/`

4. **Create providers/VM.md**
   - Follow pattern of DOCKER.md, FLY.md
   - Include quick start, prerequisites, configuration, examples
   - Estimated: 400-500 lines

### 7.2 High Priority (P1) - Near-term

3. **Create Cloud-Specific Guides**
   - AWS-PACKER.md: IAM setup, region selection, AMI management
   - AZURE-PACKER.md: Service principal, resource groups, Shared Image Gallery
   - GCP-PACKER.md: Service account, project setup, image families
   - OCI-PACKER.md: Compartment setup, API keys, availability domains
   - ALIBABA-PACKER.md: RAM user, region selection, VPC setup

4. **Update CLI.md**
   - Add complete `sindri vm` command reference
   - Document all subcommands with examples
   - Include cloud-specific options

5. **Fix Custom Actions Inconsistencies**
   - AWS: Add SSH key generation (like Azure/OCI/Alibaba) or document key pair requirement
   - GCP: Resolve OS Login vs direct SSH key path mismatch
   - Standardize SSH authentication pattern across all providers

6. **Configure Cloud Credentials in GitHub**
   - Set up OIDC/secrets for OCI (OCI_USER_OCID, OCI_TENANCY_OCID, etc.)
   - Set up OIDC/secrets for Alibaba (ALIBABA_ACCESS_KEY, etc.)
   - Verify existing AWS/Azure/GCP credentials are current

### 7.3 Medium Priority (P2) - Backlog

7. **Add TROUBLESHOOTING.md section**
   - Common Packer errors
   - Credential configuration issues
   - Cloud-specific debugging

8. **Create VM_SECURITY.md**
   - CIS hardening process
   - OpenSCAP scanning workflow
   - Image security best practices

9. **Update CONFIGURATION.md**
   - Add Packer configuration section
   - Include cloud-specific examples

10. **Create IMAGE_DISTRIBUTION.md**
    - Document public sharing strategy for each cloud
    - AWS public AMI setup and deprecation policies
    - Azure Community Gallery configuration
    - GCP public image IAM setup
    - OCI Marketplace publishing process
    - Alibaba Cloud Marketplace listing

11. **Implement Image Distribution Automation**
    - Add workflow job for making images public after validation
    - Configure cross-region replication for AWS/Azure
    - Set up image deprecation schedules

### 7.4 Workflow Architecture Recommendations

The current workflow split creates complexity:

| Current                                    | Recommended                         |
| ------------------------------------------ | ----------------------------------- |
| `v3-packer-build.yml` - 3/5 clouds via CLI | Add OCI and Alibaba build jobs      |
| `v3-provider-packer.yml` - broken hybrid   | Fix to use CLI for builds           |
| Custom actions for instance mgmt           | Keep, but document CLI alternatives |

Long-term consideration: Evaluate whether instance launch/terminate should also use CLI via `sindri vm deploy` to maintain consistency.

---

## Part 8: Summary

### What Works Well

- CLI architecture is excellent - clean provider abstraction, embedded templates, unified interface
- All 5 cloud providers are fully implemented in the CLI with feature parity
- Custom GitHub actions exist for all 5 clouds (OCI/Alibaba most complete)
- Planning and ADR documentation is comprehensive
- OIDC authentication is correctly configured for AWS, Azure, GCP in build workflow
- Credential checking is implemented in CLI for all providers

### What Needs Attention

1. **Critical:** `v3-packer-build.yml` only supports 3/5 clouds (AWS, Azure, GCP) - missing OCI and Alibaba jobs
2. **Critical:** `v3-provider-packer.yml` is broken - bypasses CLI, references non-existent templates
3. **Critical:** `v3-packer-test.yml` duplicates custom actions inline instead of using them, missing OCI/Alibaba
4. **Critical:** No user-facing Packer documentation exists
5. **High:** GitHub secrets not configured for OCI and Alibaba (blocking those workflows)
6. **High:** Custom actions have inconsistent SSH key handling (AWS requires pre-existing, GCP has OS Login mismatch)
7. **High:** No cloud-specific getting started guides
8. **High:** CLI reference incomplete for Packer commands
9. **High:** No image distribution/public sharing strategy documented or implemented
10. **Medium:** CLI `sindri vm deploy` command not used anywhere in workflows
11. **Medium:** IAM/RAM permission requirements not documented for users

### Risk Assessment

| Risk                                                      | Likelihood | Impact | Mitigation                                            |
| --------------------------------------------------------- | ---------- | ------ | ----------------------------------------------------- |
| Cannot build OCI/Alibaba images via CI                    | Certain    | High   | Add missing build jobs to v3-packer-build.yml         |
| Cannot test OCI/Alibaba images via CI                     | Certain    | High   | Add test-oci, test-alibaba jobs to v3-packer-test.yml |
| OCI/Alibaba workflows blocked by missing secrets          | Certain    | High   | Configure GitHub secrets for OCI and Alibaba          |
| Users cannot build images via v3-provider-packer workflow | High       | High   | Fix workflow to use CLI                               |
| Test workflow may not auto-trigger                        | High       | Medium | Fix workflow_run trigger name                         |
| GCP tests may fail due to SSH mismatch                    | Medium     | Medium | Fix OS Login vs SSH key inconsistency                 |
| AWS tests require manual key setup                        | Certain    | Low    | Document or add key generation                        |
| Duplicate code maintenance burden                         | Certain    | Medium | Refactor test workflow to use custom actions          |
| Users cannot learn to use Packer                          | High       | Medium | Create documentation                                  |
| Users cannot configure cloud credentials                  | High       | Medium | Document IAM/RAM requirements per cloud               |
| Inconsistent behavior between workflows                   | High       | Medium | Standardize on CLI usage                              |
| Cloud setup confusion                                     | High       | Medium | Create cloud-specific guides                          |
| No public image distribution strategy                     | High       | Medium | Define and document image sharing approach            |
| Images cannot be consumed by public                       | High       | Medium | Implement public sharing workflows per cloud          |

---

## Appendix A: File Reference

### CLI Implementation

- `/v3/crates/sindri/src/commands/packer.rs` - Command handlers
- `/v3/crates/sindri-packer/src/lib.rs` - Provider factory
- `/v3/crates/sindri-packer/src/traits.rs` - PackerProvider trait
- `/v3/crates/sindri-packer/src/aws.rs` - AWS implementation
- `/v3/crates/sindri-packer/src/azure.rs` - Azure implementation
- `/v3/crates/sindri-packer/src/gcp.rs` - GCP implementation
- `/v3/crates/sindri-packer/src/oci.rs` - OCI implementation
- `/v3/crates/sindri-packer/src/alibaba.rs` - Alibaba implementation

### Templates

- `/v3/crates/sindri-packer/src/templates/hcl/*.tera` - Embedded HCL2 templates

### Workflows

- `/.github/workflows/v3-packer-build.yml` - Build workflow (uses CLI, 3/5 clouds)
- `/.github/workflows/v3-packer-test.yml` - Test workflow (duplicates actions, 3/5 clouds)
- `/.github/workflows/v3-provider-packer.yml` - Extension test workflow (broken build step)

### Custom Actions

- `/.github/actions/packer/launch-instance/action.yml` - Multi-cloud dispatcher
- `/.github/actions/packer/terminate-instance/action.yml` - Multi-cloud cleanup
- `/.github/actions/packer/providers/aws/action.yml` - AWS EC2 (basic, needs key pair)
- `/.github/actions/packer/providers/azure/action.yml` - Azure VM (complete)
- `/.github/actions/packer/providers/gcp/action.yml` - GCP Compute (OS Login issue)
- `/.github/actions/packer/providers/oci/action.yml` - OCI Compute (complete)
- `/.github/actions/packer/providers/alibaba/action.yml` - Alibaba ECS (complete)

### Documentation

- `/v3/docs/architecture/adr/031-packer-vm-provisioning-architecture.md` - ADR
- `/v3/docs/planning/complete/packer-vm-provisioning-architecture.md` - Planning doc

---

## Appendix B: Research Sources

### Image Sharing & Distribution (2025-2026)

**AWS:**

- [Understand shared AMI usage](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/sharing-amis.html)
- [Recommendations for creating shared Linux AMIs](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/building-shared-amis.html)
- [Make your AMI publicly available](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/sharingamis-intro.html)
- [Best practices for building AMIs for AWS Marketplace](https://docs.aws.amazon.com/marketplace/latest/userguide/best-practices-for-building-your-amis.html)

**Azure:**

- [Overview of Azure Compute Gallery](https://learn.microsoft.com/en-us/azure/virtual-machines/azure-compute-gallery)
- [Share Azure Compute Gallery Resources with a Community Gallery](https://learn.microsoft.com/en-us/azure/virtual-machines/share-gallery-community)
- [Share Resources in Azure Compute Gallery](https://learn.microsoft.com/en-us/azure/virtual-machines/share-gallery)

**GCP:**

- [Manage access to custom images](https://cloud.google.com/compute/docs/images/managing-access-custom-images)
- [Image management best practices](https://cloud.google.com/compute/docs/images/image-management-best-practices)
- [Image families best practices](https://docs.cloud.google.com/compute/docs/images/image-families-best-practices)

**OCI:**

- [Managing Custom Images](https://docs.oracle.com/en-us/iaas/Content/Compute/Tasks/managingcustomimages.htm)
- [Overview of Marketplace](https://docs.oracle.com/en-us/iaas/Content/Marketplace/Concepts/marketoverview.htm)
- [Guidelines for Images](https://docs.oracle.com/en-us/iaas/Content/Marketplace/app-publisher-guidelines-images.htm)

**Alibaba Cloud:**

- [Image Overview](https://www.alibabacloud.com/help/en/ecs/user-guide/image-overview)
- [Share a custom image](https://www.alibabacloud.com/help/en/ecs/user-guide/share-a-custom-image)
- [Publish Image Products](https://www.alibabacloud.com/help/en/marketplace/publish-image-products)

### GitHub Actions OIDC Authentication

- [GitHub OIDC Documentation](https://docs.github.com/en/actions/concepts/security/openid-connect)
- [Configuring OIDC in AWS](https://docs.github.com/actions/deployment/security-hardening-your-deployments/configuring-openid-connect-in-amazon-web-services)
- [Configuring OIDC in Azure](https://docs.github.com/actions/deployment/security-hardening-your-deployments/configuring-openid-connect-in-azure)
- [GCP Workload Identity Federation](https://cloud.google.com/iam/docs/workload-identity-federation)
- [Best practices for Workload Identity Federation](https://cloud.google.com/iam/docs/best-practices-for-using-workload-identity-federation)
