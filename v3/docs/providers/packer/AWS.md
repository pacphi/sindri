# AWS Packer Provider

> **Version:** 3.x
> **Last Updated:** 2026-02

Build Amazon Machine Images (AMIs) using HashiCorp Packer with the `amazon-ebs` builder.

## Overview

The AWS Packer provider builds EC2 AMIs with Sindri pre-installed, enabling fast instance launches without runtime provisioning. Built images can be shared across accounts, regions, and optionally made public.

**Best for:** Enterprise deployments, golden image pipelines, multi-region infrastructure, pre-baked development environments

## Prerequisites

### Required Tools

| Requirement      | Version | Check Command      | Install                                                                       |
| ---------------- | ------- | ------------------ | ----------------------------------------------------------------------------- |
| HashiCorp Packer | 1.9+    | `packer --version` | https://developer.hashicorp.com/packer/install                                |
| AWS CLI          | 2.x     | `aws --version`    | https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html |

### Verify Installation

```bash
# Check all prerequisites
sindri vm doctor --cloud aws
```

**Expected output:**

```
Packer Prerequisites Check

Packer installed: 1.10.0

AWS Prerequisites
  CLI installed: 2.15.0
  Credentials configured
```

## IAM Permissions

### Minimal IAM Policy for Packer Operations

Create an IAM user or role with the following policy to enable all Packer operations:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "PackerEC2Permissions",
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
      "Sid": "PackerSTSPermissions",
      "Effect": "Allow",
      "Action": ["sts:GetCallerIdentity"],
      "Resource": "*"
    }
  ]
}
```

### Additional Permissions for Advanced Features

**Cross-Region AMI Copy:**

```json
{
  "Sid": "CrossRegionCopy",
  "Effect": "Allow",
  "Action": ["ec2:CopyImage", "ec2:ModifyImageAttribute"],
  "Resource": "*"
}
```

**VPC/Subnet Selection:**

```json
{
  "Sid": "VPCAccess",
  "Effect": "Allow",
  "Action": [
    "ec2:CreateSecurityGroup",
    "ec2:DeleteSecurityGroup",
    "ec2:AuthorizeSecurityGroupIngress",
    "ec2:RevokeSecurityGroupIngress"
  ],
  "Resource": "*"
}
```

**KMS Encryption:**

```json
{
  "Sid": "KMSEncryption",
  "Effect": "Allow",
  "Action": [
    "kms:CreateGrant",
    "kms:Decrypt",
    "kms:DescribeKey",
    "kms:GenerateDataKeyWithoutPlaintext",
    "kms:ReEncrypt*"
  ],
  "Resource": "arn:aws:kms:*:*:key/*"
}
```

## Authentication Methods

### Method 1: AWS Configure (Recommended for Local Development)

```bash
aws configure
```

You will be prompted for:

- AWS Access Key ID
- AWS Secret Access Key
- Default region name (e.g., `us-west-2`)
- Default output format (e.g., `json`)

**Verify configuration:**

```bash
aws sts get-caller-identity
```

### Method 2: AWS SSO Login

For organizations using AWS IAM Identity Center:

```bash
# Configure SSO (one-time)
aws configure sso

# Login (session-based)
aws sso login --profile <profile-name>

# Use profile
export AWS_PROFILE=<profile-name>
```

### Method 3: Environment Variables

```bash
export AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE
export AWS_SECRET_ACCESS_KEY=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
export AWS_DEFAULT_REGION=us-west-2

# Optional: Session token (for temporary credentials)
export AWS_SESSION_TOKEN=...
```

### Method 4: IAM Instance Profile (For EC2)

When running on EC2, the instance automatically receives credentials via the instance metadata service. No configuration required if the instance has an appropriate IAM role attached.

**Verify instance profile:**

```bash
curl -s http://169.254.169.254/latest/meta-data/iam/security-credentials/
```

### Credential Precedence

AWS credentials are resolved in this order:

1. Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
2. Shared credentials file (`~/.aws/credentials`)
3. AWS SSO credentials
4. IAM Instance Profile (EC2)
5. ECS Task Role (ECS containers)

## Region Selection

### Available Regions

| Region Code    | Region Name   | Notes                 |
| -------------- | ------------- | --------------------- |
| us-east-1      | N. Virginia   | Oldest, most services |
| us-east-2      | Ohio          |                       |
| us-west-1      | N. California |                       |
| us-west-2      | Oregon        | **Default**, popular  |
| eu-west-1      | Ireland       | EU primary            |
| eu-west-2      | London        |                       |
| eu-central-1   | Frankfurt     |                       |
| ap-southeast-1 | Singapore     | APAC primary          |
| ap-southeast-2 | Sydney        |                       |
| ap-northeast-1 | Tokyo         |                       |
| ap-south-1     | Mumbai        |                       |
| sa-east-1      | Sao Paulo     |                       |
| ca-central-1   | Canada        |                       |

### Region-Specific Base AMIs

Sindri automatically selects the latest Ubuntu 24.04 LTS AMI for your target region. The source AMI filter:

```hcl
source_ami_filter {
  filters = {
    name                = "ubuntu/images/hvm-ssd-gp3/ubuntu-noble-24.04-amd64-server-*"
    root-device-type    = "ebs"
    virtualization-type = "hvm"
  }
  most_recent = true
  owners      = ["099720109477"]  # Canonical
}
```

**Find current base AMI manually:**

```bash
aws ec2 describe-images \
  --region us-west-2 \
  --owners 099720109477 \
  --filters "Name=name,Values=ubuntu/images/hvm-ssd-gp3/ubuntu-noble-24.04-amd64-server-*" \
  --query 'sort_by(Images, &CreationDate)[-1].{Id:ImageId,Name:Name}'
```

## Configuration Examples

### Basic Configuration

```yaml
version: "1.0"
name: sindri-dev

deployment:
  provider: packer

providers:
  packer:
    cloud: aws
    image_name: sindri-dev

    aws:
      region: us-west-2
      instance_type: t3.large
      volume_size: 60
```

### Production Configuration

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
    cloud: aws
    image_name: sindri-production
    description: "Production Sindri environment with full toolchain"

    build:
      sindri_version: "3.0.0"
      cache: true
      ssh_timeout: "20m"
      security:
        cis_hardening: true
        clean_sensitive_data: true
        remove_ssh_keys: true

    aws:
      region: us-west-2
      instance_type: t3.xlarge
      volume_size: 100
      volume_type: gp3
      encrypt_boot: true
      ami_regions:
        - us-east-1
        - eu-west-1
      ami_users:
        - "123456789012"
        - "987654321098"

    tags:
      Environment: production
      Team: platform
      CostCenter: engineering
```

### VPC-Specific Build

```yaml
providers:
  packer:
    cloud: aws
    aws:
      region: us-west-2
      instance_type: t3.large
      vpc_id: vpc-0123456789abcdef0
      subnet_id: subnet-0123456789abcdef0
      volume_size: 80
```

### Full AWS Configuration Reference

```yaml
providers:
  packer:
    cloud: aws
    image_name: sindri-custom
    description: "Custom Sindri image"

    aws:
      # Region (required for operations)
      region: us-west-2

      # Instance Configuration
      instance_type: t3.large # Build instance type
      volume_size: 80 # Root volume size (GB)
      volume_type: gp3 # gp2, gp3, io1, io2

      # Security
      encrypt_boot: true # Encrypt root volume

      # Network (optional - uses default VPC if not specified)
      vpc_id: vpc-0123456789 # Specific VPC
      subnet_id: subnet-0123456 # Specific subnet (must have internet)

      # AMI Distribution
      ami_regions: # Copy to additional regions
        - us-east-1
        - eu-west-1
      ami_users: # Share with AWS account IDs
        - "123456789012"
      ami_groups: # Share with groups
        - "all" # Makes AMI public (use with caution!)

    tags:
      Environment: production
      Team: platform
```

## Quick Start

```bash
# 1. Verify prerequisites
sindri vm doctor --cloud aws

# 2. Build an AWS AMI
sindri vm build --cloud aws --name my-sindri-image --profile fullstack

# 3. List your images
sindri vm list --cloud aws

# 4. Deploy an instance from the image
sindri vm deploy --cloud aws ami-0123456789abcdef0
```

**Build time:** 10-20 minutes depending on profile and extensions

## AMI Management

### Listing AMIs

```bash
# List all Sindri AMIs
sindri vm list --cloud aws

# Filter by name prefix
sindri vm list --cloud aws --name production-sindri

# List in specific region
sindri vm list --cloud aws --region us-east-1

# JSON output for scripting
sindri vm list --cloud aws --json
```

**Using AWS CLI directly:**

```bash
# List all owned AMIs with Sindri tag
aws ec2 describe-images \
  --owners self \
  --filters "Name=tag:ManagedBy,Values=sindri" \
  --query 'Images[*].{ID:ImageId,Name:Name,Created:CreationDate,State:State}'
```

### Copying AMIs Cross-Region

**Via Configuration (Recommended):**

```yaml
providers:
  packer:
    aws:
      region: us-west-2
      ami_regions:
        - us-east-1
        - eu-west-1
        - ap-southeast-1
```

**Manual Copy:**

```bash
aws ec2 copy-image \
  --source-region us-west-2 \
  --source-image-id ami-0123456789abcdef0 \
  --region us-east-1 \
  --name "sindri-production-copy" \
  --description "Copied from us-west-2"
```

### Making AMIs Public

> **Warning:** Public AMIs are visible to all AWS users. Ensure no sensitive data is included.

**Via Configuration:**

```yaml
providers:
  packer:
    aws:
      ami_groups:
        - "all" # Makes AMI public
```

**Manual:**

```bash
aws ec2 modify-image-attribute \
  --image-id ami-0123456789abcdef0 \
  --launch-permission "Add=[{Group=all}]"
```

**Verify public status:**

```bash
aws ec2 describe-images \
  --image-ids ami-0123456789abcdef0 \
  --query 'Images[0].Public'
```

### Sharing with Specific Accounts

```yaml
providers:
  packer:
    aws:
      ami_users:
        - "123456789012"
        - "987654321098"
```

**Manual:**

```bash
aws ec2 modify-image-attribute \
  --image-id ami-0123456789abcdef0 \
  --launch-permission "Add=[{UserId=123456789012}]"
```

### AMI Deprecation Policies

AWS supports setting deprecation dates on AMIs:

```bash
# Deprecate AMI in 90 days
aws ec2 enable-image-deprecation \
  --image-id ami-0123456789abcdef0 \
  --deprecate-at "$(date -d '+90 days' --iso-8601=seconds)"

# Check deprecation status
aws ec2 describe-images \
  --image-ids ami-0123456789abcdef0 \
  --query 'Images[0].DeprecationTime'

# Cancel deprecation
aws ec2 disable-image-deprecation \
  --image-id ami-0123456789abcdef0
```

**Best Practices:**

- Set deprecation dates for all production AMIs
- Public AMIs auto-deprecate after 2 years
- AWS removes public sharing for AMIs unused 6+ months after deprecation

### Deleting AMIs

```bash
# Delete with confirmation
sindri vm delete --cloud aws ami-0123456789abcdef0

# Force delete (no confirmation)
sindri vm delete --cloud aws ami-0123456789abcdef0 --force
```

**Manual deletion (includes snapshots):**

```bash
# Get associated snapshots
SNAPSHOTS=$(aws ec2 describe-images \
  --image-ids ami-0123456789abcdef0 \
  --query 'Images[0].BlockDeviceMappings[*].Ebs.SnapshotId' \
  --output text)

# Deregister AMI
aws ec2 deregister-image --image-id ami-0123456789abcdef0

# Delete snapshots
for snap in $SNAPSHOTS; do
  aws ec2 delete-snapshot --snapshot-id $snap
done
```

## VPC/Subnet Configuration

### Default VPC (Simplest)

If no VPC is specified, Packer uses the default VPC in the target region. This requires:

- Default VPC exists
- Default subnet has auto-assign public IP enabled
- Internet gateway attached

### Custom VPC Requirements

When specifying a VPC/subnet:

| Requirement      | Description                                   |
| ---------------- | --------------------------------------------- |
| Internet Gateway | Required for package downloads during build   |
| Public IP        | Subnet must auto-assign public IPs OR use NAT |
| Security Group   | Allow SSH (port 22) inbound from Packer       |
| Route Table      | 0.0.0.0/0 routed to Internet Gateway or NAT   |

### Public Subnet Configuration

```yaml
providers:
  packer:
    aws:
      vpc_id: vpc-0123456789abcdef0
      subnet_id: subnet-public-0123456789 # Must have IGW route
```

**Packer creates a temporary security group allowing SSH from anywhere during build.**

### Private Subnet with NAT Gateway

For builds in private subnets:

```yaml
providers:
  packer:
    aws:
      vpc_id: vpc-0123456789abcdef0
      subnet_id: subnet-private-0123456789 # Routed through NAT
```

**Additional requirements:**

- NAT Gateway in public subnet
- Route table: 0.0.0.0/0 -> NAT Gateway
- Consider using Session Manager instead of SSH for connectivity

### Security Group Considerations

Packer automatically creates a temporary security group with:

- Inbound: SSH (port 22) from 0.0.0.0/0
- Outbound: All traffic

For stricter security, pre-create a security group:

```bash
aws ec2 create-security-group \
  --group-name packer-build-sg \
  --description "Security group for Packer builds" \
  --vpc-id vpc-0123456789abcdef0

aws ec2 authorize-security-group-ingress \
  --group-id sg-0123456789abcdef0 \
  --protocol tcp \
  --port 22 \
  --cidr <your-ip>/32
```

## Cost Optimization

### Instance Type Selection

| Instance Type | vCPUs | Memory | Hourly Cost | Best For            |
| ------------- | ----- | ------ | ----------- | ------------------- |
| t3.medium     | 2     | 4 GB   | $0.0416     | Minimal profiles    |
| t3.large      | 2     | 8 GB   | $0.0832     | Standard builds     |
| t3.xlarge     | 4     | 16 GB  | $0.1664     | Full-stack profiles |
| t3.2xlarge    | 8     | 32 GB  | $0.3328     | Large extensions    |

**Recommendation:** Use `t3.large` for most builds (~$0.02 per 15-minute build)

### Spot Instances for Builds

> **Note:** Spot instances can be interrupted. Use for non-critical builds only.

Add to Packer template (manual configuration):

```hcl
source "amazon-ebs" "sindri" {
  spot_price                          = "auto"
  spot_instance_types                 = ["t3.large", "t3a.large"]
  spot_tags                           = { Name = "Packer Spot Build" }
  fleet_tags                          = { Name = "Packer Fleet" }
}
```

### Storage Costs

| Storage Type | Cost/GB/Month | Notes                          |
| ------------ | ------------- | ------------------------------ |
| AMI (EBS)    | $0.05         | Per region                     |
| Snapshot     | $0.05         | Underlying AMI storage         |
| gp3          | $0.08         | Standard SSD                   |
| gp2          | $0.10         | Legacy SSD                     |
| io1/io2      | $0.125+       | Provisioned IOPS (unnecessary) |

**Cost Optimization Tips:**

1. **Use gp3 volumes** - Better performance, lower cost than gp2
2. **Clean up old AMIs** - Delete unused images and snapshots
3. **Build once, copy selectively** - Only copy to needed regions
4. **Enable caching** - Avoid rebuilding unchanged configurations
5. **Right-size volumes** - Start with 60GB, increase if needed

### Estimated Build Costs

| Scenario                   | Instance  | Time   | Cost      |
| -------------------------- | --------- | ------ | --------- |
| Minimal profile            | t3.large  | 10 min | ~$0.014   |
| Full-stack profile         | t3.large  | 15 min | ~$0.021   |
| Full-stack + CIS hardening | t3.xlarge | 20 min | ~$0.055   |
| Multi-region (3 regions)   | t3.large  | 15 min | ~$0.021\* |

\*Copy operations are billed separately for cross-region data transfer

## Troubleshooting

### Credentials Not Configured

**Symptom:** `Credentials not configured` in doctor output

**Solutions:**

```bash
# Option 1: Configure credentials
aws configure

# Option 2: Set environment variables
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...

# Option 3: Login via SSO
aws sso login --profile your-profile
export AWS_PROFILE=your-profile

# Verify
aws sts get-caller-identity
```

### Insufficient Permissions

**Symptom:** `Access Denied` or `UnauthorizedOperation` errors

**Solution:** Verify IAM policy includes all required permissions (see IAM Permissions section). Common missing permissions:

- `ec2:CreateImage` - Required for AMI creation
- `ec2:RunInstances` - Required to launch build instance
- `ec2:CreateKeyPair` - Required for SSH access during build
- `sts:GetCallerIdentity` - Required for credential verification

### Build Timeout

**Symptom:** `SSH timeout` or build fails after extended wait

**Solutions:**

1. **Increase timeout:**

   ```yaml
   providers:
     packer:
       build:
         ssh_timeout: "30m"
   ```

2. **Use larger instance:**

   ```bash
   sindri vm build --cloud aws --instance-type t3.xlarge
   ```

3. **Check network connectivity:**
   - Verify subnet has internet access
   - Verify security group allows SSH inbound

### VPC/Subnet Issues

**Symptom:** `Unable to find a subnet` or instance never becomes reachable

**Diagnosis:**

```bash
# Check default VPC exists
aws ec2 describe-vpcs --filters "Name=isDefault,Values=true"

# Check subnet has auto-assign public IP
aws ec2 describe-subnets \
  --subnet-ids subnet-0123456789 \
  --query 'Subnets[0].MapPublicIpOnLaunch'

# Check internet gateway
aws ec2 describe-internet-gateways \
  --filters "Name=attachment.vpc-id,Values=vpc-0123456789"
```

**Solutions:**

- Specify working VPC/subnet explicitly
- Create default VPC: `aws ec2 create-default-vpc`
- Enable auto-assign public IP on subnet

### No Base AMI Found

**Symptom:** `No Ubuntu 24.04 AMI found in region`

**Solution:** Verify Ubuntu AMIs are available in your region:

```bash
aws ec2 describe-images \
  --region us-west-2 \
  --owners 099720109477 \
  --filters "Name=name,Values=ubuntu/images/hvm-ssd-gp3/ubuntu-noble-24.04-amd64-server-*" \
  --query 'Images[0].ImageId'
```

Some newer regions may have delayed AMI availability.

### AMI Copy Failures

**Symptom:** Cross-region copy fails or times out

**Solutions:**

- Verify IAM permissions include `ec2:CopyImage` in destination regions
- Check for encryption key access in destination region
- Ensure destination region is not opted-out

### Debug Mode

Enable debug output for troubleshooting:

```bash
# Via CLI
sindri vm build --cloud aws --debug

# Or set environment variable
PACKER_LOG=1 sindri vm build --cloud aws
```

## CI/CD Integration

### GitHub Actions with OIDC

```yaml
name: Build AWS AMI

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
      - uses: actions/checkout@v4

      - name: Configure AWS credentials
        uses: aws-actions/configure-aws-credentials@v4
        with:
          role-to-assume: ${{ secrets.AWS_ROLE_ARN }}
          aws-region: us-west-2

      - name: Setup Packer
        uses: hashicorp/setup-packer@main
        with:
          version: "1.10.0"

      - name: Build AMI
        run: |
          sindri vm build --cloud aws \
            --name "sindri-${{ github.sha }}" \
            --json > build-result.json

      - name: Upload result
        uses: actions/upload-artifact@v4
        with:
          name: ami-build
          path: build-result.json
```

### OIDC IAM Trust Policy

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

## Related Documentation

- [Packer Provider Overview](../PACKER.md)
- [Azure Packer Guide](AZURE.md)
- [GCP Packer Guide](GCP.md)
- [OCI Packer Guide](OCI.md)
- [Alibaba Packer Guide](ALIBABA.md)
- [Provider Overview](../README.md)
- [Configuration Reference](../../CONFIGURATION.md)
- [Secrets Management](../../SECRETS_MANAGEMENT.md)
- [CLI Reference](../../CLI.md)
- [AWS EC2 User Guide](https://docs.aws.amazon.com/ec2/)
- [HashiCorp Packer AWS Builder](https://developer.hashicorp.com/packer/plugins/builders/amazon)
