# VM Image Distribution Guide

> **Version:** 3.x
> **Last Updated:** 2026-02

This guide covers strategies for distributing Sindri VM images to users across all supported cloud platforms.

## Overview

After building VM images with `sindri packer build`, you may want to share them with:

- **Internal teams** - Development, QA, and operations teams within your organization
- **Partners** - External organizations that need access to your images
- **Public users** - Open source community or commercial customers

Each cloud provider offers different mechanisms for image sharing, from private images to public marketplaces.

### Use Cases

| Use Case                | Distribution Method | Clouds Supported |
| ----------------------- | ------------------- | ---------------- |
| Development/Testing     | Private (default)   | All              |
| Enterprise Customers    | Shared/Direct       | All              |
| Open Source Community   | Public/Community    | All              |
| Commercial Distribution | Marketplace         | All              |

---

## AWS AMI Distribution

AWS AMIs (Amazon Machine Images) can be distributed through several mechanisms.

### Distribution Options

| Method          | Audience                | Best For               |
| --------------- | ----------------------- | ---------------------- |
| Private AMI     | Owner account only      | Development            |
| Shared AMI      | Specific AWS accounts   | Partners, customers    |
| Public AMI      | All AWS users           | Open source, community |
| AWS Marketplace | Commercial distribution | Monetization           |

### Making an AMI Public

```bash
# Make AMI public
aws ec2 modify-image-attribute \
  --image-id ami-xxxx \
  --launch-permission "Add=[{Group=all}]"

# Verify public status
aws ec2 describe-image-attribute \
  --image-id ami-xxxx \
  --attribute launchPermission
```

### Sharing with Specific Accounts

```bash
# Share with specific AWS accounts
aws ec2 modify-image-attribute \
  --image-id ami-xxxx \
  --launch-permission "Add=[{UserId=123456789012},{UserId=098765432109}]"

# Remove sharing
aws ec2 modify-image-attribute \
  --image-id ami-xxxx \
  --launch-permission "Remove=[{UserId=123456789012}]"
```

### Deprecation Policy

AWS enforces deprecation policies for public AMIs:

- **Auto-deprecation:** Public AMIs auto-deprecate after 2 years
- **Removal:** If unused for 6+ months after deprecation, AWS removes public sharing
- **Best Practice:** Set explicit deprecation dates for version control

```bash
# Set deprecation date (2 years from now)
aws ec2 enable-image-deprecation \
  --image-id ami-xxxx \
  --deprecate-at "2028-02-01T00:00:00Z"

# Check deprecation status
aws ec2 describe-images \
  --image-ids ami-xxxx \
  --query 'Images[].DeprecationTime'
```

### Public AMI Access Controls

Since October 2023, new AWS accounts have public AMI sharing blocked by default:

```bash
# Check if public sharing is enabled
aws ec2 get-image-block-public-access-state

# Enable public sharing (requires account-level permissions)
aws ec2 enable-image-block-public-access --state unblocked
```

### Cross-Region Copying

To distribute AMIs across multiple regions:

```bash
# Copy AMI to another region
aws ec2 copy-image \
  --source-image-id ami-xxxx \
  --source-region us-west-2 \
  --region eu-west-1 \
  --name "sindri-v3-eu"
```

### AWS Marketplace Publishing

For commercial distribution:

1. Register at [AWS Partner Network](https://aws.amazon.com/partners/)
2. Complete security review
3. Define pricing model (paid, BYOL, or free)
4. Submit AMI for review

**Requirements:**

- Clean, well-documented AMI
- Security hardening verification
- Support and maintenance commitment
- Legal agreements (EULA, support terms)

**Resources:**

- [Building AMIs for AWS Marketplace](https://docs.aws.amazon.com/marketplace/latest/userguide/best-practices-for-building-your-amis.html)

---

## Azure Image Distribution

Azure provides Azure Compute Gallery for image distribution.

### Distribution Options

| Method                            | Audience                          | Best For     |
| --------------------------------- | --------------------------------- | ------------ |
| Managed Image                     | Single subscription               | Development  |
| Azure Compute Gallery (Direct)    | Up to 30 subscriptions, 5 tenants | Enterprise   |
| Azure Compute Gallery (Community) | All Azure users                   | Open source  |
| Azure Marketplace                 | Commercial distribution           | Monetization |

### Azure Compute Gallery Types

#### Direct Shared Gallery

Share with specific subscriptions or tenants:

```bash
# Create a gallery
az sig create \
  --resource-group myResourceGroup \
  --gallery-name myGallery

# Add image definition
az sig image-definition create \
  --resource-group myResourceGroup \
  --gallery-name myGallery \
  --gallery-image-definition sindri-v3 \
  --publisher Sindri \
  --offer SindriVM \
  --sku v3 \
  --os-type Linux \
  --os-state Generalized

# Share with specific subscriptions
az sig share add \
  --resource-group myResourceGroup \
  --gallery-name myGallery \
  --subscription-ids <subscription-id-1> <subscription-id-2>

# Share with specific tenants
az sig share add \
  --resource-group myResourceGroup \
  --gallery-name myGallery \
  --tenant-ids <tenant-id>
```

#### Community Gallery Setup

Community galleries are Azure's equivalent of AWS public AMIs:

```bash
# Create a community gallery
az sig create \
  --resource-group myResourceGroup \
  --gallery-name myGallery \
  --sharing-profile "Community" \
  --publisher-uri "https://sindri.dev" \
  --publisher-email "support@sindri.dev" \
  --eula "https://sindri.dev/eula"

# The gallery will receive a public name with format:
# <prefix>-<random-guid>
```

**Community Gallery Requirements:**

- Valid publisher URL
- Valid contact email
- Legal agreement/EULA URL
- Gallery prefix (appended with GUID for uniqueness)

### Cross-Subscription Sharing

```bash
# List shared galleries available to you
az sig list-shared --location westus2

# Use a shared image
az vm create \
  --resource-group myResourceGroup \
  --name myVM \
  --image "/SharedGalleries/<gallery-unique-name>/Images/sindri-v3/Versions/latest"
```

### Azure Marketplace Publishing

For commercial distribution:

1. Register at [Partner Center](https://partner.microsoft.com/)
2. Create a Marketplace offer
3. Complete security certification
4. Define pricing (transactable or BYOL)

---

## GCP Image Distribution

GCP uses IAM policies for image sharing.

### Distribution Options

| Method                | Audience                | Best For     |
| --------------------- | ----------------------- | ------------ |
| Project Image         | Single project          | Development  |
| Cross-Project Sharing | Specific projects       | Organization |
| Public Image          | All GCP users           | Open source  |
| Cloud Marketplace     | Commercial distribution | Monetization |

### Cross-Project Sharing with IAM

```bash
# Grant access to specific users
gcloud compute images add-iam-policy-binding sindri-v3 \
  --member='user:developer@example.com' \
  --role='roles/compute.imageUser'

# Grant access to a service account
gcloud compute images add-iam-policy-binding sindri-v3 \
  --member='serviceAccount:my-sa@other-project.iam.gserviceaccount.com' \
  --role='roles/compute.imageUser'

# Grant access to entire project
gcloud compute images add-iam-policy-binding sindri-v3 \
  --member='projectViewer:other-project-id' \
  --role='roles/compute.imageUser'
```

### Making an Image Public

```bash
# Make image public (all authenticated GCP users)
gcloud compute images add-iam-policy-binding sindri-v3 \
  --member='allAuthenticatedUsers' \
  --role='roles/compute.imageUser'

# Verify permissions
gcloud compute images get-iam-policy sindri-v3
```

**Security Warning:** Granting `allAuthenticatedUsers` allows ANY Google account to use the image. Consider using organization-level sharing instead for sensitive images.

### Image Families for Versioning

Use image families for semantic versioning:

```bash
# Create image in a family
gcloud compute images create sindri-v3-20260201 \
  --source-disk=sindri-disk \
  --family=sindri-v3

# Users can reference the family (gets latest)
gcloud compute instances create my-vm \
  --image-family=sindri-v3 \
  --image-project=my-project
```

**Benefits:**

- Latest image in family is auto-selected
- Enables rolling updates without changing references
- Supports deprecation of old versions

```bash
# Deprecate an old image
gcloud compute images deprecate sindri-v3-20250101 \
  --state=DEPRECATED \
  --replacement=sindri-v3-20260201
```

### GCP Cloud Marketplace

For commercial distribution:

1. Contact Google Cloud partner team
2. Sign partner agreement
3. Submit images for review
4. Define pricing (paid or free)

---

## OCI Image Distribution

Oracle Cloud Infrastructure uses compartments and the OCI Marketplace.

### Distribution Options

| Method                      | Audience           | Best For             |
| --------------------------- | ------------------ | -------------------- |
| Custom Image                | Single tenancy     | Development          |
| Cross-Tenancy Export/Import | Specific tenancies | Partners             |
| OCI Marketplace             | All OCI users      | Community/Commercial |

### Cross-Tenancy Export/Import

OCI does not support direct image sharing between tenancies. Use Object Storage for transfer:

```bash
# Step 1: Export image to Object Storage
oci compute image export to-object \
  --image-id <source-image-ocid> \
  --bucket-name my-bucket \
  --name sindri-v3.oci

# Step 2: Share the Object Storage bucket or object
# (Configure bucket policies or pre-authenticated requests)

# Step 3: Import in target tenancy
oci compute image import from-object \
  --compartment-id <target-compartment-ocid> \
  --bucket-name shared-bucket \
  --name sindri-v3.oci \
  --display-name "Sindri V3"
```

### Pre-Authenticated Requests for Sharing

```bash
# Create a pre-authenticated request for the image
oci os preauth-request create \
  --bucket-name my-bucket \
  --object-name sindri-v3.oci \
  --access-type ObjectRead \
  --time-expires "2026-12-31T23:59:59Z"
```

### OCI Marketplace Publishing

For community or commercial distribution:

1. Register at [Oracle Cloud Marketplace Partner Portal](https://cloudmarketplace.oracle.com/)
2. Create listing with image, description, pricing
3. Submit for Oracle review
4. Supports free, BYOL, and paid models

**Image Requirements:**

- Use paravirtualized mode for best compatibility
- Build on official OCI base images when possible
- Remove sensitive data using `oci-image-cleanup` utility
- Test with flexible shapes

### Community Applications

OCI now supports Community Applications for sharing non-commercial images, similar to AWS public AMIs.

---

## Alibaba Cloud Distribution

Alibaba Cloud supports regional image sharing and Marketplace publishing.

### Distribution Options

| Method                    | Audience                        | Best For     |
| ------------------------- | ------------------------------- | ------------ |
| Custom Image              | Single account                  | Development  |
| Shared Image              | Specific accounts (same region) | Partners     |
| Copy to Marketplace       | All Alibaba Cloud users         | Community    |
| Alibaba Cloud Marketplace | Commercial distribution         | Monetization |

### Account-Level Sharing

```bash
# Share with specific accounts (same region only)
aliyun ecs ModifyImageSharePermission \
  --RegionId us-west-1 \
  --ImageId m-xxxx \
  --AddAccount.1 <target-account-id-1> \
  --AddAccount.2 <target-account-id-2>

# Check sharing status
aliyun ecs DescribeImageSharePermission \
  --RegionId us-west-1 \
  --ImageId m-xxxx

# Remove sharing
aliyun ecs ModifyImageSharePermission \
  --RegionId us-west-1 \
  --ImageId m-xxxx \
  --RemoveAccount.1 <target-account-id>
```

**Limitations:**

- Cannot share across regions (must copy first)
- Cannot share Marketplace-based images
- Sharees assume security responsibility

### Cross-Region Copying

```bash
# Copy image to another region
aliyun ecs CopyImage \
  --RegionId us-west-1 \
  --ImageId m-xxxx \
  --DestinationRegionId eu-central-1 \
  --DestinationImageName "sindri-v3-eu"

# Then share in the new region
aliyun ecs ModifyImageSharePermission \
  --RegionId eu-central-1 \
  --ImageId <new-image-id> \
  --AddAccount.1 <target-account-id>
```

### Alibaba Cloud Marketplace Publishing

For community or commercial distribution:

1. Register at Alibaba Cloud Marketplace
2. Complete security review
3. Create listing with image and documentation
4. Submit for review
5. Supports paid and free models

**Resources:**

- [Publish Image Products Guide](https://www.alibabacloud.com/help/en/marketplace/publish-image-products)

---

## Security Checklist

Before making any image public, verify the following security requirements are met.

### Pre-Distribution Verification

**Authentication & Access:**

- [ ] Root password disabled or randomized
- [ ] SSH authorized_keys cleared (cloud-init will add keys at boot)
- [ ] No default passwords for any accounts

**Credentials & Secrets:**

- [ ] Cloud credentials removed (`~/.aws`, `~/.azure`, `~/.config/gcloud`, `~/.oci`)
- [ ] Git credentials removed (`~/.gitconfig`, `~/.git-credentials`)
- [ ] Docker credentials removed (`~/.docker/config.json`)
- [ ] Private keys removed (`~/.ssh/id_*`)
- [ ] Environment variables with secrets cleared

**History & Logs:**

- [ ] Bash history cleared (`~/.bash_history`, `/root/.bash_history`)
- [ ] Log files cleared or rotated (`/var/log/*`)
- [ ] Temporary files removed (`/tmp/*`, `/var/tmp/*`)

**Security Hardening:**

- [ ] Cloud-init configured for first-boot setup
- [ ] Security hardening applied (CIS benchmarks recommended)
- [ ] Vulnerability scan completed (OpenSCAP or similar)
- [ ] No development tools or debug packages in production images

### Automated Cleanup

The Sindri CLI's `cleanup.sh.tera` template handles most of these automatically during image build:

```bash
# Verify cleanup was performed
sindri packer build --cloud aws --dry-run
```

### Vulnerability Scanning

Run security scans before distribution:

```bash
# Using OpenSCAP (if installed in image)
sudo oscap xccdf eval \
  --profile xccdf_org.ssgproject.content_profile_cis \
  --results scan-results.xml \
  /usr/share/xml/scap/ssg/content/ssg-ubuntu2204-ds.xml

# Using Trivy
trivy image ami-xxxx
```

---

## Recommended Distribution Strategy

### Tier 1: Community/Open Source (Free)

For open source projects or community editions:

| Cloud   | Recommended Method                   | Notes                   |
| ------- | ------------------------------------ | ----------------------- |
| AWS     | Public AMI with deprecation schedule | Set 2-year deprecation  |
| Azure   | Community Gallery                    | Requires publisher info |
| GCP     | Public Image (allAuthenticatedUsers) | Or use image families   |
| OCI     | Marketplace (Community Application)  | Free listing            |
| Alibaba | Marketplace (Free listing)           | Requires review         |

**Example Workflow:**

```bash
# Build image
sindri packer build --cloud aws --region us-west-2

# Make public
aws ec2 modify-image-attribute \
  --image-id ami-xxxx \
  --launch-permission "Add=[{Group=all}]"

# Set deprecation
aws ec2 enable-image-deprecation \
  --image-id ami-xxxx \
  --deprecate-at "2028-02-01T00:00:00Z"
```

### Tier 2: Enterprise/Partner Distribution

For controlled distribution to specific organizations:

| Cloud   | Recommended Method              | Limits                      |
| ------- | ------------------------------- | --------------------------- |
| AWS     | Shared AMI to specific accounts | No limit                    |
| Azure   | Direct Share Gallery            | 30 subscriptions, 5 tenants |
| GCP     | Cross-project IAM binding       | No limit                    |
| OCI     | Cross-tenancy export/import     | Manual process              |
| Alibaba | Account-level sharing           | Same region only            |

**Example Workflow:**

```bash
# Build image
sindri packer build --cloud aws --region us-west-2

# Share with enterprise customers
aws ec2 modify-image-attribute \
  --image-id ami-xxxx \
  --launch-permission "Add=[{UserId=111111111111},{UserId=222222222222}]"
```

### Tier 3: Commercial (Monetization)

For paid products or BYOL models:

| Cloud   | Marketplace               | Requirements                          |
| ------- | ------------------------- | ------------------------------------- |
| AWS     | AWS Marketplace           | Partner registration, security review |
| Azure   | Azure Marketplace         | Partner Center account, certification |
| GCP     | Google Cloud Marketplace  | Partner agreement                     |
| OCI     | Oracle Cloud Marketplace  | Partner portal registration           |
| Alibaba | Alibaba Cloud Marketplace | Security review                       |

**Common Requirements:**

- Partner/vendor registration with each cloud
- Security review and certification
- Legal agreements (EULA, support terms)
- Pricing model definition
- Ongoing maintenance and support commitment

---

## Multi-Cloud Distribution Automation

For distributing across all clouds simultaneously:

```bash
#!/bin/bash
# distribute-images.sh

VERSION="v3.1.0"
REGIONS_AWS="us-west-2 us-east-1 eu-west-1"
REGIONS_AZURE="westus2 eastus westeurope"
REGIONS_GCP="us-west1 us-east1 europe-west1"

# Build for all clouds
for cloud in aws azure gcp oci alibaba; do
  sindri packer build --cloud $cloud --version $VERSION
done

# AWS: Copy to regions and make public
for region in $REGIONS_AWS; do
  if [ "$region" != "us-west-2" ]; then
    aws ec2 copy-image \
      --source-image-id ami-xxxx \
      --source-region us-west-2 \
      --region $region \
      --name "sindri-$VERSION"
  fi
done

# Make all copies public
for region in $REGIONS_AWS; do
  aws ec2 modify-image-attribute \
    --region $region \
    --image-id ami-xxxx \
    --launch-permission "Add=[{Group=all}]"
done
```

---

## Best Practices

### Version Naming

Use consistent naming across clouds:

```
sindri-v3.1.0-20260201-ubuntu2204
```

Format: `<project>-<version>-<date>-<os>`

### Image Lifecycle

1. **Development:** Build and test in private
2. **Staging:** Share with internal QA teams
3. **Release:** Publish to community/marketplace
4. **Deprecation:** Mark old versions as deprecated
5. **Removal:** Remove unsupported versions

### Documentation

Include with distributed images:

- Release notes
- Known issues
- Upgrade instructions
- Security advisories

### Monitoring

Track image usage:

- AWS: CloudWatch, AWS Config
- Azure: Azure Monitor, Activity Log
- GCP: Cloud Monitoring, Audit Logs
- OCI: Audit service
- Alibaba: ActionTrail

---

## Related Documentation

- [Packer Provider Guide](providers/PACKER.md) - Building VM images
- [AWS Packer Guide](providers/AWS-PACKER.md) - AWS-specific configuration
- [Azure Packer Guide](providers/AZURE-PACKER.md) - Azure-specific configuration
- [GCP Packer Guide](providers/GCP-PACKER.md) - GCP-specific configuration
- [OCI Packer Guide](providers/OCI-PACKER.md) - OCI-specific configuration
- [Alibaba Packer Guide](providers/ALIBABA-PACKER.md) - Alibaba-specific configuration
- [Secrets Management](SECRETS_MANAGEMENT.md) - Managing credentials securely

## External Resources

### AWS

- [Sharing AMIs](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/sharing-amis.html)
- [Building Shared AMIs](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/building-shared-amis.html)
- [AWS Marketplace Publishing](https://docs.aws.amazon.com/marketplace/latest/userguide/best-practices-for-building-your-amis.html)

### Azure

- [Azure Compute Gallery](https://learn.microsoft.com/en-us/azure/virtual-machines/azure-compute-gallery)
- [Community Gallery](https://learn.microsoft.com/en-us/azure/virtual-machines/share-gallery-community)

### GCP

- [Managing Custom Images](https://cloud.google.com/compute/docs/images/managing-access-custom-images)
- [Image Families](https://cloud.google.com/compute/docs/images/image-families-best-practices)

### OCI

- [Managing Custom Images](https://docs.oracle.com/en-us/iaas/Content/Compute/Tasks/managingcustomimages.htm)
- [OCI Marketplace](https://docs.oracle.com/en-us/iaas/Content/Marketplace/Concepts/marketoverview.htm)

### Alibaba Cloud

- [Share Custom Images](https://www.alibabacloud.com/help/en/ecs/user-guide/share-a-custom-image)
- [Marketplace Publishing](https://www.alibabacloud.com/help/en/marketplace/publish-image-products)
