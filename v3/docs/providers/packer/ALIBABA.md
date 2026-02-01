# Alibaba Cloud Packer Provider

> **Version:** 3.x
> **Last Updated:** 2026-02

Building Sindri VM images for Alibaba Cloud ECS using HashiCorp Packer.

## Overview

This guide covers Alibaba Cloud-specific configuration for building Sindri VM images. For general Packer usage, see [PACKER.md](PACKER.md).

**Best for:** Enterprise deployments in mainland China and Asia-Pacific regions, multi-cloud infrastructure extending to Alibaba Cloud, pre-baked development environments for Chinese markets.

## Prerequisites

### 1. Aliyun CLI Installation

**macOS:**

```bash
brew install aliyun-cli
```

**Linux:**

```bash
# Download latest release
curl -L https://aliyuncli.alicdn.com/aliyun-cli-linux-latest-amd64.tgz -o aliyun-cli.tgz
tar -xzf aliyun-cli.tgz
sudo mv aliyun /usr/local/bin/
rm aliyun-cli.tgz

# Verify installation
aliyun --version
```

**Windows:**

```powershell
# Download from https://github.com/aliyun/aliyun-cli/releases
# Or use scoop
scoop install aliyun
```

### 2. HashiCorp Packer

```bash
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

### 3. Account Setup

1. Create an Alibaba Cloud account at https://www.alibabacloud.com/
2. Complete real-name verification (required for mainland China regions)
3. Create a RAM user with programmatic access
4. Generate AccessKey ID and AccessKey Secret

## RAM Permissions

### Required RAM Policy

Create a custom policy with the following permissions for Packer image building:

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
        "ecs:DescribeSecurityGroups",
        "ecs:DescribeRegions",
        "ecs:DescribeAvailableResource",
        "ecs:DescribeInstanceTypes",
        "ecs:CreateSnapshot",
        "ecs:DeleteSnapshot",
        "ecs:DescribeSnapshots",
        "ecs:DescribeDisks",
        "ecs:CreateDisk",
        "ecs:DeleteDisk",
        "ecs:AttachDisk",
        "ecs:DetachDisk",
        "ecs:ModifyDiskAttribute",
        "ecs:ReInitDisk",
        "ecs:ResizeDisk"
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
    },
    {
      "Effect": "Allow",
      "Action": ["ram:PassRole"],
      "Resource": "*",
      "Condition": {
        "StringEquals": {
          "acs:Service": "ecs.aliyuncs.com"
        }
      }
    }
  ]
}
```

### Attaching the Policy

**Via RAM Console:**

1. Log into the [RAM Console](https://ram.console.aliyun.com/)
2. Navigate to **Users** > select your user
3. Click **Add Permissions**
4. Choose **Custom Policy** and select your Packer policy

**Via CLI:**

```bash
# Create the policy
aliyun ram CreatePolicy \
  --PolicyName SindriPackerPolicy \
  --PolicyDocument file://packer-policy.json

# Attach to user
aliyun ram AttachPolicyToUser \
  --PolicyType Custom \
  --PolicyName SindriPackerPolicy \
  --UserName <your-ram-user>
```

### Minimal Policy for Testing

For development/testing with reduced permissions:

```json
{
  "Version": "1",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "ecs:DescribeImages",
        "ecs:DescribeInstances",
        "ecs:DescribeRegions",
        "ecs:DescribeSecurityGroups",
        "vpc:DescribeVpcs",
        "vpc:DescribeVSwitches"
      ],
      "Resource": "*"
    }
  ]
}
```

## Authentication Methods

### Method 1: Interactive Configuration (Recommended for Development)

```bash
aliyun configure
# Enter: AccessKey ID
# Enter: AccessKey Secret
# Enter: Default Region ID (e.g., cn-hangzhou)
# Enter: Default Output Format (json)
```

This creates a configuration file at `~/.aliyun/config.json`.

### Method 2: Environment Variables

Set credentials via environment variables:

```bash
# Primary credentials
export ALICLOUD_ACCESS_KEY=<your-access-key-id>
export ALICLOUD_SECRET_KEY=<your-access-key-secret>
export ALICLOUD_REGION=cn-hangzhou

# Alternative environment variable names (also supported)
export ALIBABA_CLOUD_ACCESS_KEY_ID=<your-access-key-id>
export ALIBABA_CLOUD_ACCESS_KEY_SECRET=<your-access-key-secret>
export ALIBABA_CLOUD_REGION=cn-hangzhou
```

**For CI/CD pipelines:**

```yaml
# GitHub Actions example
env:
  ALICLOUD_ACCESS_KEY: ${{ secrets.ALIBABA_ACCESS_KEY }}
  ALICLOUD_SECRET_KEY: ${{ secrets.ALIBABA_SECRET_KEY }}
  ALICLOUD_REGION: cn-hangzhou
```

### Method 3: RAM Role (On Alibaba Cloud ECS)

When running on an Alibaba Cloud ECS instance, use instance RAM roles for automatic credential management:

**1. Create a RAM role for ECS:**

```bash
aliyun ram CreateRole \
  --RoleName PackerBuildRole \
  --AssumeRolePolicyDocument '{
    "Statement": [
      {
        "Action": "sts:AssumeRole",
        "Effect": "Allow",
        "Principal": {
          "Service": ["ecs.aliyuncs.com"]
        }
      }
    ],
    "Version": "1"
  }'
```

**2. Attach the Packer policy to the role:**

```bash
aliyun ram AttachPolicyToRole \
  --PolicyType Custom \
  --PolicyName SindriPackerPolicy \
  --RoleName PackerBuildRole
```

**3. Assign the role to your ECS instance:**

```bash
aliyun ecs AttachInstanceRamRole \
  --RegionId cn-hangzhou \
  --InstanceIds '["i-xxxx"]' \
  --RamRoleName PackerBuildRole
```

**4. Verify role attachment:**

```bash
# From the ECS instance
curl http://100.100.100.200/latest/meta-data/ram/security-credentials/PackerBuildRole
```

Packer automatically uses instance metadata credentials when no other credentials are provided.

### Credential Verification

```bash
# Verify credentials are working
aliyun sts GetCallerIdentity

# Check configured profile
aliyun configure list
```

## Region Selection

### Mainland China Regions

| Region ID      | Location    | Notes                                     |
| -------------- | ----------- | ----------------------------------------- |
| cn-hangzhou    | Hangzhou    | **Default**, stable, full feature support |
| cn-shanghai    | Shanghai    | Financial hub, excellent connectivity     |
| cn-beijing     | Beijing     | Government, enterprise workloads          |
| cn-shenzhen    | Shenzhen    | South China, tech hub                     |
| cn-zhangjiakou | Zhangjiakou | Cost-effective, cold climate              |
| cn-huhehaote   | Hohhot      | Cost-effective, data sovereignty          |
| cn-wulanchabu  | Ulanqab     | Latest region, competitive pricing        |
| cn-chengdu     | Chengdu     | Southwest China coverage                  |
| cn-hongkong    | Hong Kong   | Cross-border connectivity                 |

### International Regions

| Region ID      | Location       | Notes                            |
| -------------- | -------------- | -------------------------------- |
| ap-southeast-1 | Singapore      | **Recommended for Asia-Pacific** |
| ap-southeast-2 | Sydney         | Australia/Oceania                |
| ap-southeast-3 | Kuala Lumpur   | Southeast Asia                   |
| ap-southeast-5 | Jakarta        | Indonesia                        |
| ap-southeast-6 | Manila         | Philippines                      |
| ap-southeast-7 | Bangkok        | Thailand                         |
| ap-south-1     | Mumbai         | South Asia                       |
| ap-northeast-1 | Tokyo          | Japan                            |
| ap-northeast-2 | Seoul          | South Korea                      |
| eu-west-1      | London         | Europe (UK)                      |
| eu-central-1   | Frankfurt      | Europe (Germany)                 |
| us-west-1      | Silicon Valley | US West Coast                    |
| us-east-1      | Virginia       | US East Coast                    |
| me-east-1      | Dubai          | Middle East                      |

### Region Selection Guidelines

```bash
# List all available regions
aliyun ecs DescribeRegions

# Check resource availability in a region
aliyun ecs DescribeAvailableResource \
  --RegionId cn-hangzhou \
  --DestinationResource InstanceType \
  --InstanceChargeType PostPaid
```

**Considerations:**

- **Latency:** Choose regions closest to your users
- **Compliance:** Mainland China regions require ICP filing for public services
- **Pricing:** Zhangjiakou, Hohhot, Ulanqab offer lower costs
- **Features:** cn-hangzhou and cn-shanghai have the most complete feature sets
- **Cross-border:** Hong Kong provides bridge between mainland and international

## Configuration Examples

### Basic sindri.yaml Configuration

```yaml
version: "1.0"
name: my-sindri-image

deployment:
  provider: packer

extensions:
  profile: fullstack
  additional:
    - docker
    - node

providers:
  packer:
    cloud: alibaba
    image_name: sindri-dev
    description: "Sindri development environment"

    alibaba:
      region: cn-hangzhou
      instance_type: ecs.g6.xlarge
      system_disk_size_gb: 80
      system_disk_category: cloud_essd
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
    - monitoring

providers:
  packer:
    cloud: alibaba
    image_name: sindri-production
    description: "Production Sindri environment with security hardening"

    build:
      sindri_version: "3.0.0"
      cache: true
      ssh_timeout: "25m"
      security:
        cis_hardening: true
        clean_sensitive_data: true
        remove_ssh_keys: true

    alibaba:
      region: cn-hangzhou
      instance_type: ecs.g7.xlarge
      system_disk_size_gb: 100
      system_disk_category: cloud_essd
      system_disk_performance_level: PL1
      vswitch_id: vsw-bp1xxxx
      security_group_id: sg-bp1xxxx

      # Image encryption
      image_encrypted: true

      # Cross-region copy
      image_copy_regions:
        - cn-shanghai
        - cn-beijing

      # Image sharing
      image_share_accounts:
        - "1234567890123456"

    tags:
      Environment: production
      Team: platform
      ManagedBy: sindri
```

### Multi-Region Configuration

```yaml
providers:
  packer:
    cloud: alibaba
    image_name: sindri-global

    alibaba:
      region: ap-southeast-1 # Build in Singapore
      instance_type: ecs.g6.xlarge
      system_disk_size_gb: 80
      system_disk_category: cloud_essd

      # Copy to multiple regions after build
      image_copy_regions:
        - cn-hangzhou # China
        - ap-northeast-1 # Tokyo
        - eu-central-1 # Frankfurt
        - us-west-1 # Silicon Valley
```

### Cost-Optimized Configuration

```yaml
providers:
  packer:
    cloud: alibaba

    alibaba:
      region: cn-zhangjiakou # Lower-cost region
      instance_type: ecs.t6-c1m1.large # Burstable instance
      system_disk_size_gb: 60
      system_disk_category: cloud_efficiency # Standard SSD

      # Use spot instances for builds (significant savings)
      spot_strategy: SpotAsPriceGo
      spot_price_limit: 0.5 # Maximum price per hour
```

### Instance Type Reference

| Instance Type     | vCPUs | Memory | Use Case                     |
| ----------------- | ----- | ------ | ---------------------------- |
| ecs.t6-c1m1.large | 2     | 2GB    | Minimal builds               |
| ecs.g6.large      | 2     | 8GB    | Development                  |
| ecs.g6.xlarge     | 4     | 16GB   | **Default**, general purpose |
| ecs.g7.xlarge     | 4     | 16GB   | Latest generation            |
| ecs.c6.xlarge     | 4     | 8GB    | Compute-optimized            |
| ecs.r6.xlarge     | 4     | 32GB   | Memory-optimized             |

## VPC/VSwitch Setup

### Understanding Alibaba Cloud Networking

Alibaba Cloud uses a VPC (Virtual Private Cloud) model similar to AWS:

- **VPC:** Virtual network isolated from other tenants
- **VSwitch:** Subnet within a VPC (specific to an availability zone)
- **Security Group:** Firewall rules for instances

### Create VPC and VSwitch

**Via Console:**

1. Go to [VPC Console](https://vpc.console.aliyun.com/)
2. Click **Create VPC**
3. Configure VPC CIDR (e.g., 192.168.0.0/16)
4. Add a VSwitch in your desired availability zone

**Via CLI:**

```bash
# Create VPC
aliyun vpc CreateVpc \
  --RegionId cn-hangzhou \
  --CidrBlock 192.168.0.0/16 \
  --VpcName sindri-packer-vpc \
  --Description "VPC for Sindri Packer builds"

# Note the VpcId from output, then create VSwitch
aliyun vpc CreateVSwitch \
  --RegionId cn-hangzhou \
  --ZoneId cn-hangzhou-h \
  --VpcId vpc-bp1xxxx \
  --CidrBlock 192.168.1.0/24 \
  --VSwitchName sindri-packer-vswitch
```

### VSwitch Requirements for Packer

1. **Internet Access:** The VSwitch must have internet access for:
   - Downloading packages during provisioning
   - SSH access from Packer (or use VPN/bastion)

2. **Options for Internet Access:**

   **Option A: NAT Gateway (Recommended for Security)**

   ```bash
   # Create NAT Gateway
   aliyun vpc CreateNatGateway \
     --RegionId cn-hangzhou \
     --VpcId vpc-bp1xxxx \
     --Name sindri-nat

   # Allocate EIP and bind to NAT
   aliyun vpc AllocateEipAddress --RegionId cn-hangzhou
   aliyun vpc AssociateEipAddress \
     --AllocationId eip-bp1xxxx \
     --InstanceId ngw-bp1xxxx

   # Create SNAT entry
   aliyun vpc CreateSnatEntry \
     --RegionId cn-hangzhou \
     --SnatTableId stb-bp1xxxx \
     --SourceVSwitchId vsw-bp1xxxx \
     --SnatIp 47.xx.xx.xx
   ```

   **Option B: Public IP Assignment (Simpler)**

   ```bash
   # Configure VSwitch to auto-assign public IPs
   # Or allocate EIP per instance (Packer does this automatically)
   ```

### Using Existing VPC/VSwitch

```yaml
providers:
  packer:
    alibaba:
      region: cn-hangzhou
      vswitch_id: vsw-bp1xxxx # Your existing VSwitch
      security_group_id: sg-bp1xxxx # Your existing Security Group
```

If you don't specify `vswitch_id`, Packer creates temporary networking resources.

## Security Groups

### Create Security Group for Packer

```bash
# Create security group
aliyun ecs CreateSecurityGroup \
  --RegionId cn-hangzhou \
  --VpcId vpc-bp1xxxx \
  --SecurityGroupName sindri-packer-sg \
  --Description "Security group for Sindri Packer builds"

# Note the SecurityGroupId, then add SSH rule
aliyun ecs AuthorizeSecurityGroup \
  --RegionId cn-hangzhou \
  --SecurityGroupId sg-bp1xxxx \
  --IpProtocol tcp \
  --PortRange 22/22 \
  --SourceCidrIp 0.0.0.0/0 \
  --Description "SSH for Packer"
```

### Recommended Security Group Rules

| Direction | Protocol | Port | Source       | Purpose               |
| --------- | -------- | ---- | ------------ | --------------------- |
| Inbound   | TCP      | 22   | Your IP/CIDR | SSH access for Packer |
| Outbound  | All      | All  | 0.0.0.0/0    | Package downloads     |

### Restrict SSH to Specific IPs

For better security, restrict SSH to your build environment's IP:

```bash
# Get your current public IP
curl ifconfig.me

# Authorize only your IP
aliyun ecs AuthorizeSecurityGroup \
  --RegionId cn-hangzhou \
  --SecurityGroupId sg-bp1xxxx \
  --IpProtocol tcp \
  --PortRange 22/22 \
  --SourceCidrIp "203.0.113.45/32" \
  --Description "SSH from build server"
```

### Using Security Group in Configuration

```yaml
providers:
  packer:
    alibaba:
      region: cn-hangzhou
      vswitch_id: vsw-bp1xxxx
      security_group_id: sg-bp1xxxx # Your security group
```

## EIP Allocation

### Why EIPs are Needed

Packer requires SSH access to the build instance. Options:

1. **Automatic EIP** (Default): Packer allocates a temporary EIP
2. **Pre-allocated EIP**: Use an existing EIP
3. **VPN/Bastion**: Access via private network (advanced)

### Automatic EIP Allocation

By default, Sindri Packer allocates a temporary EIP during builds:

```yaml
providers:
  packer:
    alibaba:
      region: cn-hangzhou
      # EIP is automatically allocated and released
```

### Using Pre-allocated EIP

If you have a pre-allocated EIP:

```yaml
providers:
  packer:
    alibaba:
      region: cn-hangzhou
      associate_public_ip_address: true
      internet_charge_type: PayByTraffic
      internet_max_bandwidth_out: 100 # Mbps
```

### EIP Management Commands

```bash
# Allocate EIP
aliyun vpc AllocateEipAddress \
  --RegionId cn-hangzhou \
  --Bandwidth 100 \
  --InternetChargeType PayByTraffic

# List EIPs
aliyun vpc DescribeEipAddresses --RegionId cn-hangzhou

# Release EIP (after build)
aliyun vpc ReleaseEipAddress \
  --RegionId cn-hangzhou \
  --AllocationId eip-bp1xxxx
```

### EIP Costs

| Charge Type    | Cost               |
| -------------- | ------------------ |
| PayByTraffic   | ~$0.12/GB outbound |
| PayByBandwidth | ~$0.03/Mbps/hour   |

For Packer builds, PayByTraffic is typically more cost-effective.

## Image Distribution

### Sharing with Specific Accounts

Share your image with specific Alibaba Cloud accounts:

```yaml
providers:
  packer:
    alibaba:
      image_share_accounts:
        - "1234567890123456" # Account ID
        - "9876543210987654"
```

**Via CLI:**

```bash
aliyun ecs ModifyImageSharePermission \
  --RegionId cn-hangzhou \
  --ImageId m-bp1xxxx \
  --AddAccount.1 1234567890123456 \
  --AddAccount.2 9876543210987654
```

**Check shared accounts:**

```bash
aliyun ecs DescribeImageSharePermission \
  --RegionId cn-hangzhou \
  --ImageId m-bp1xxxx
```

### Cross-Region Image Copy

Copy images to other regions for multi-region deployments:

```yaml
providers:
  packer:
    alibaba:
      region: cn-hangzhou # Build region
      image_copy_regions:
        - cn-shanghai
        - cn-beijing
        - ap-southeast-1
```

**Via CLI:**

```bash
aliyun ecs CopyImage \
  --RegionId cn-hangzhou \
  --ImageId m-bp1xxxx \
  --DestinationRegionId cn-shanghai \
  --DestinationImageName sindri-dev-shanghai
```

**Note:** Cross-region copy is asynchronous. Check status:

```bash
aliyun ecs DescribeImages \
  --RegionId cn-shanghai \
  --ImageName sindri-dev-shanghai
```

### Alibaba Cloud Marketplace Publishing

For public distribution, publish to Alibaba Cloud Marketplace:

**1. Prerequisites:**

- Complete enterprise verification
- Agree to Marketplace terms
- Prepare image documentation

**2. Publishing Process:**

1. Go to [Marketplace Partner Portal](https://partner.console.aliyun.com/)
2. Create a new product listing
3. Upload your image and configure pricing
4. Submit for security review
5. After approval, your image is publicly available

**3. Image Requirements for Marketplace:**

- No pre-configured credentials
- SSH key authentication only (no passwords)
- Cloud-init configured for first-boot setup
- Clear documentation and support contact
- Security scan passed

**4. Pricing Models:**

| Model | Description                  |
| ----- | ---------------------------- |
| Free  | No charge for image          |
| Paid  | Per-hour or subscription fee |
| BYOL  | Bring Your Own License       |

### Image Lifecycle Management

```bash
# Deprecate an old image
aliyun ecs ModifyImageAttribute \
  --RegionId cn-hangzhou \
  --ImageId m-bp1xxxx \
  --Status Deprecated

# Delete an image
aliyun ecs DeleteImage \
  --RegionId cn-hangzhou \
  --ImageId m-bp1xxxx \
  --Force true
```

## CLI Commands

### Build Image

```bash
# Basic build
sindri packer build --cloud alibaba

# Full options
sindri packer build --cloud alibaba \
  --name my-sindri-image \
  --region cn-hangzhou \
  --instance-type ecs.g6.xlarge \
  --disk-size 100 \
  --profile fullstack \
  --extensions "docker,kubernetes" \
  --cis-hardening \
  --json > build-result.json
```

### List Images

```bash
# List all Sindri images
sindri packer list --cloud alibaba

# Filter by name and region
sindri packer list --cloud alibaba \
  --name sindri-production \
  --region cn-hangzhou \
  --json
```

### Delete Image

```bash
# Delete with confirmation
sindri packer delete --cloud alibaba m-bp1xxxx

# Force delete
sindri packer delete --cloud alibaba m-bp1xxxx --force
```

### Validate Template

```bash
# Validate configuration
sindri packer validate --cloud alibaba

# Syntax check only
sindri packer validate --cloud alibaba --syntax-only
```

### Check Prerequisites

```bash
# Check Alibaba Cloud setup
sindri packer doctor --cloud alibaba
```

### Deploy from Image

```bash
# Deploy instance from built image
sindri packer deploy --cloud alibaba m-bp1xxxx

# With custom instance type
sindri packer deploy --cloud alibaba m-bp1xxxx \
  --instance-type ecs.g7.xlarge \
  --region cn-shanghai
```

## Troubleshooting

### Authentication Errors

**Symptom:** `InvalidAccessKeyId.NotFound` or `SignatureDoesNotMatch`

**Solutions:**

```bash
# Verify credentials
aliyun sts GetCallerIdentity

# Check environment variables
echo $ALICLOUD_ACCESS_KEY
echo $ALICLOUD_SECRET_KEY

# Reconfigure
aliyun configure
```

### Region/Zone Errors

**Symptom:** `InvalidZoneId` or `InvalidRegionId`

**Solutions:**

```bash
# List valid regions
aliyun ecs DescribeRegions

# List zones in a region
aliyun ecs DescribeZones --RegionId cn-hangzhou

# Check resource availability
aliyun ecs DescribeAvailableResource \
  --RegionId cn-hangzhou \
  --DestinationResource InstanceType
```

### Instance Type Unavailable

**Symptom:** `InvalidInstanceType.NotAvailable`

**Solutions:**

```bash
# Check available instance types in zone
aliyun ecs DescribeAvailableResource \
  --RegionId cn-hangzhou \
  --ZoneId cn-hangzhou-h \
  --DestinationResource InstanceType

# Try a different zone or instance type
```

### VSwitch/Network Errors

**Symptom:** `InvalidVSwitchId.NotFound` or network connectivity issues

**Solutions:**

```bash
# Verify VSwitch exists
aliyun vpc DescribeVSwitches \
  --RegionId cn-hangzhou \
  --VSwitchId vsw-bp1xxxx

# Check VSwitch is in correct zone
aliyun vpc DescribeVSwitches --RegionId cn-hangzhou --VSwitchId vsw-bp1xxxx

# Verify internet access (NAT Gateway or EIP)
aliyun vpc DescribeNatGateways --RegionId cn-hangzhou --VpcId vpc-bp1xxxx
```

### Security Group Issues

**Symptom:** SSH connection timeout

**Solutions:**

```bash
# Verify security group rules
aliyun ecs DescribeSecurityGroupAttribute \
  --RegionId cn-hangzhou \
  --SecurityGroupId sg-bp1xxxx

# Add SSH rule if missing
aliyun ecs AuthorizeSecurityGroup \
  --RegionId cn-hangzhou \
  --SecurityGroupId sg-bp1xxxx \
  --IpProtocol tcp \
  --PortRange 22/22 \
  --SourceCidrIp 0.0.0.0/0
```

### EIP Allocation Failures

**Symptom:** `QuotaExceeded.Eip` or EIP allocation errors

**Solutions:**

```bash
# Check EIP quota
aliyun vpc DescribeEipAddresses --RegionId cn-hangzhou

# Release unused EIPs
aliyun vpc ReleaseEipAddress \
  --RegionId cn-hangzhou \
  --AllocationId eip-bp1xxxx

# Request quota increase via console
```

### Image Creation Failures

**Symptom:** `IncorrectInstanceStatus` or image creation stuck

**Solutions:**

```bash
# Check instance status
aliyun ecs DescribeInstances \
  --RegionId cn-hangzhou \
  --InstanceIds '["i-bp1xxxx"]'

# Instance must be stopped for image creation
aliyun ecs StopInstance --InstanceId i-bp1xxxx

# Check image creation progress
aliyun ecs DescribeImages \
  --RegionId cn-hangzhou \
  --ImageOwnerAlias self
```

### Build Timeout

**Symptom:** Build fails with SSH timeout

**Solutions:**

```yaml
providers:
  packer:
    build:
      ssh_timeout: "30m" # Increase timeout
    alibaba:
      instance_type: ecs.g6.xlarge # Use faster instance
```

### Quota Exceeded

**Symptom:** Various `QuotaExceeded` errors

**Solutions:**

```bash
# Check current quotas
aliyun ecs DescribeAccountAttributes --RegionId cn-hangzhou

# Common quotas to check:
# - max-security-groups
# - max-elastic-ip-addresses
# - max-custom-images

# Request quota increase via ticket system
```

### Debug Mode

For detailed troubleshooting:

```bash
# Enable Packer debug output
sindri packer build --cloud alibaba --debug

# Or set environment variable
export PACKER_LOG=1
sindri packer build --cloud alibaba
```

### Common Error Codes

| Error Code                         | Meaning                   | Solution                |
| ---------------------------------- | ------------------------- | ----------------------- |
| `InvalidAccessKeyId.NotFound`      | AccessKey not valid       | Check credentials       |
| `SignatureDoesNotMatch`            | Secret key incorrect      | Reconfigure credentials |
| `InvalidRegionId`                  | Region doesn't exist      | Use valid region ID     |
| `InvalidZoneId`                    | Zone doesn't exist        | Check available zones   |
| `InvalidInstanceType.NotAvailable` | Instance type not in zone | Try different zone/type |
| `InvalidVSwitchId.NotFound`        | VSwitch doesn't exist     | Verify VSwitch ID       |
| `QuotaExceeded.Eip`                | EIP quota reached         | Release unused EIPs     |
| `InvalidSecurityGroupId.NotFound`  | Security group invalid    | Check security group    |
| `IncorrectInstanceStatus`          | Instance state wrong      | Wait or stop instance   |

## Related Documentation

- [Packer Provider Overview](../PACKER.md)
- [AWS Packer Guide](AWS.md)
- [Azure Packer Guide](AZURE.md)
- [GCP Packer Guide](GCP.md)
- [OCI Packer Guide](OCI.md)
- [Provider Overview](../README.md)
- [Configuration Reference](../../CONFIGURATION.md)
- [Secrets Management](../../SECRETS_MANAGEMENT.md)
- [CLI Reference](../../CLI.md)
- [Alibaba Cloud ECS Documentation](https://www.alibabacloud.com/help/en/ecs/)
- [Alibaba Cloud RAM Documentation](https://www.alibabacloud.com/help/en/ram/)
- [Alibaba Cloud VPC Documentation](https://www.alibabacloud.com/help/en/vpc/)
