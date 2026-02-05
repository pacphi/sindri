# VM Image Security

> **Version:** 3.x
> **Last Updated:** 2026-02
> **Status:** Production

Comprehensive security guide for building, hardening, and distributing Sindri VM images.

## Table of Contents

- [Overview](#overview)
- [Why Image Security Matters](#why-image-security-matters)
- [CIS Hardening](#cis-hardening)
- [Pre-Capture Cleanup](#pre-capture-cleanup)
- [OpenSCAP Scanning](#openscap-scanning)
- [Access Control](#access-control)
- [Encryption](#encryption)
- [Vulnerability Management](#vulnerability-management)
- [Security Checklist](#security-checklist)

---

## Overview

Sindri VM images are built using HashiCorp Packer and distributed across multiple cloud providers. Security is applied at multiple layers:

| Layer                        | Implementation                   | Purpose                 |
| ---------------------------- | -------------------------------- | ----------------------- |
| **Build-time Hardening**     | CIS Benchmarks, security scripts | Reduce attack surface   |
| **Pre-capture Cleanup**      | `cleanup.sh.tera` template       | Remove sensitive data   |
| **Compliance Scanning**      | OpenSCAP, SCAP profiles          | Verify security posture |
| **Encryption at Rest**       | Cloud-native disk encryption     | Protect stored data     |
| **Access Control**           | IAM policies, image sharing      | Limit distribution      |
| **Vulnerability Management** | Regular updates, CVE scanning    | Address known issues    |

### Security Architecture

```
                                 Build Phase
                    +-----------------------------------+
                    |  Base Image (Ubuntu/Debian/RHEL)  |
                    +-----------------------------------+
                                    |
                                    v
                    +-----------------------------------+
                    |   CIS Hardening (Level 1/2)       |
                    |   - Filesystem security           |
                    |   - Network hardening             |
                    |   - Audit configuration           |
                    +-----------------------------------+
                                    |
                                    v
                    +-----------------------------------+
                    |   Sindri Provisioning             |
                    |   - CLI installation              |
                    |   - Extensions                    |
                    +-----------------------------------+
                                    |
                                    v
                    +-----------------------------------+
                    |   OpenSCAP Compliance Scan        |
                    |   - Generate reports              |
                    |   - Remediation if needed         |
                    +-----------------------------------+
                                    |
                                    v
                    +-----------------------------------+
                    |   Pre-Capture Cleanup             |
                    |   - Remove credentials            |
                    |   - Clear history                 |
                    |   - Reset SSH keys                |
                    +-----------------------------------+
                                    |
                                    v
                    +-----------------------------------+
                    |   Encrypted Image Snapshot        |
                    |   (AWS EBS / Azure / GCP / OCI)   |
                    +-----------------------------------+
```

---

## Why Image Security Matters

### Risks of Insecure VM Images

| Risk                      | Impact                           | Mitigation                                  |
| ------------------------- | -------------------------------- | ------------------------------------------- |
| **Credential Exposure**   | API keys, tokens leaked in image | Pre-capture cleanup removes all credentials |
| **SSH Key Compromise**    | Unauthorized access to instances | SSH keys regenerated on first boot          |
| **Privilege Escalation**  | Attackers gain root access       | CIS hardening, minimal sudo access          |
| **Data Breach**           | Sensitive data in logs/history   | Log rotation, history clearing              |
| **Compliance Violations** | Failed audits (SOC2, HIPAA, PCI) | CIS benchmarks, OpenSCAP scanning           |
| **Supply Chain Attack**   | Malicious code in base image     | Verified base images, integrity checks      |
| **Network Exploitation**  | Unnecessary services exposed     | Network hardening, firewall rules           |

### Shared Responsibility Model

When distributing VM images:

- **You are responsible for:** Image security, credential removal, hardening, compliance
- **Cloud provider is responsible for:** Infrastructure security, encryption keys, hypervisor isolation

### Regulatory Compliance

Proper image security helps achieve compliance with:

- **SOC 2 Type II** - Access controls, audit logging
- **HIPAA** - PHI protection, encryption
- **PCI-DSS** - Cardholder data security
- **CIS Controls** - Configuration management
- **FedRAMP** - Federal security requirements

---

## CIS Hardening

### What is CIS?

The [Center for Internet Security (CIS)](https://www.cisecurity.org/) publishes industry-standard security benchmarks. Sindri implements CIS Ubuntu/Debian Linux benchmarks.

### Benchmark Levels

| Level       | Focus                                      | Use Case                             |
| ----------- | ------------------------------------------ | ------------------------------------ |
| **Level 1** | Essential security, minimal impact         | Most deployments, development        |
| **Level 2** | Defense-in-depth, may affect functionality | High-security, production, regulated |

### Level 1 Controls (Default)

The `security-hardening.sh.tera` script applies Level 1 controls:

#### 1. Filesystem Security

```bash
# Disable uncommon/legacy filesystems
install cramfs /bin/true
install freevxfs /bin/true
install jffs2 /bin/true
install hfs /bin/true
install hfsplus /bin/true
install squashfs /bin/true
install udf /bin/true
```

**Purpose:** Prevents mounting of rarely-used filesystem types that could be exploited.

#### 2. Sudo Configuration

```bash
# Enable sudo logging
Defaults logfile="/var/log/sudo.log"

# Require re-authentication
Defaults timestamp_timeout=15
```

**Purpose:** Audit trail for privileged commands, limits session hijacking.

#### 3. Password Policies

```bash
# /etc/security/pwquality.conf
minlen = 14          # Minimum length
dcredit = -1         # Require digit
ucredit = -1         # Require uppercase
ocredit = -1         # Require special char
lcredit = -1         # Require lowercase
minclass = 4         # Require all 4 classes
maxrepeat = 3        # Max consecutive chars
```

**Purpose:** Enforces strong passwords resistant to brute-force attacks.

#### 4. SSH Hardening

```bash
# /etc/ssh/sshd_config.d/sindri-hardening.conf
Protocol 2
LogLevel VERBOSE
PermitRootLogin no
PermitEmptyPasswords no
MaxAuthTries 4
ClientAliveInterval 300
ClientAliveCountMax 3
AllowTcpForwarding no
AllowAgentForwarding no

# Strong cryptography only
Ciphers chacha20-poly1305@openssh.com,aes256-gcm@openssh.com,aes128-gcm@openssh.com
MACs hmac-sha2-512-etm@openssh.com,hmac-sha2-256-etm@openssh.com
KexAlgorithms curve25519-sha256,diffie-hellman-group-exchange-sha256
```

**Purpose:** Prevents weak ciphers, root login, and connection abuse.

#### 5. Network Security

```bash
# /etc/sysctl.d/99-sindri-security.conf
# Disable IP forwarding
net.ipv4.ip_forward = 0
net.ipv6.conf.all.forwarding = 0

# Ignore ICMP redirects
net.ipv4.conf.all.accept_redirects = 0

# Enable TCP SYN cookies
net.ipv4.tcp_syncookies = 1

# Enable ASLR
kernel.randomize_va_space = 2

# Restrict kernel pointers
kernel.kptr_restrict = 2
```

**Purpose:** Hardens network stack against common attacks (SYN flood, MITM).

#### 6. Audit System

```bash
# /etc/audit/rules.d/sindri-audit.rules
-w /var/run/utmp -p wa -k session
-w /var/log/wtmp -p wa -k session
-w /etc/sudoers -p wa -k sudoers
-w /etc/ssh/sshd_config -p wa -k sshd
```

**Purpose:** Records security-relevant events for forensics and compliance.

#### 7. AppArmor

```bash
systemctl enable apparmor
aa-enforce /etc/apparmor.d/*
```

**Purpose:** Mandatory access control limiting application capabilities.

#### 8. Fail2ban

```bash
# /etc/fail2ban/jail.local
[sshd]
enabled = true
maxretry = 3
bantime = 1h
```

**Purpose:** Automatically bans IPs after failed login attempts.

### Enabling CIS Hardening

```bash
# CLI
sindri vm build --cloud aws --cis-hardening

# Configuration
providers:
  packer:
    build:
      security:
        cis_hardening: true
```

### Level 2 Controls (Advanced)

Level 2 adds more restrictive controls that may affect functionality:

| Control               | Impact                   | When to Use               |
| --------------------- | ------------------------ | ------------------------- |
| Disable USB storage   | Blocks USB devices       | Air-gapped environments   |
| AIDE file integrity   | CPU overhead for checks  | High-security, compliance |
| Kernel module signing | Blocks unsigned modules  | Regulated environments    |
| SELinux enforcing     | Application restrictions | Federal, financial        |

Level 2 is configured via:

```yaml
providers:
  packer:
    build:
      security:
        cis_hardening: true
        cis_level: 2 # Default: 1
```

### Verifying Hardening

After building, verify controls:

```bash
# Check SSH config
sshd -T | grep -E "permitrootlogin|permitemptypasswords|protocol"

# Check sysctl settings
sysctl net.ipv4.ip_forward
sysctl kernel.randomize_va_space

# Check auditd status
systemctl status auditd
auditctl -l
```

---

## Pre-Capture Cleanup

### Why Cleanup Matters

Build-time secrets and user data **must not** be included in distributed images:

| Risk              | Example                          | Consequence          |
| ----------------- | -------------------------------- | -------------------- |
| Credential leak   | AWS keys in `~/.aws/credentials` | Account compromise   |
| SSH compromise    | Private keys in `~/.ssh/`        | Unauthorized access  |
| Identity leakage  | Git credentials, npm tokens      | Supply chain attacks |
| Forensic exposure | Bash history with passwords      | Data breach          |

### The cleanup.sh.tera Template

Sindri uses a Tera template at `v3/crates/sindri-packer/src/templates/scripts/cleanup.sh.tera` to generate cleanup scripts for each cloud provider.

**Template Variables:**

| Variable               | Default | Purpose                        |
| ---------------------- | ------- | ------------------------------ |
| `CLEAN_SENSITIVE_DATA` | `true`  | Remove credentials and secrets |
| `REMOVE_SSH_KEYS`      | `true`  | Clear SSH host and user keys   |

### What Gets Cleaned

#### SSH Keys and authorized_keys

```bash
# Host keys (regenerate on first boot)
rm -f /etc/ssh/ssh_host_*

# User keys
rm -rf /home/*/.ssh/authorized_keys
rm -rf /home/*/.ssh/id_*
rm -rf /home/*/.ssh/known_hosts
rm -rf /root/.ssh/authorized_keys
```

**Why:** Prevents SSH key reuse across instances. Keys regenerate on first boot via `dpkg-reconfigure openssh-server`.

#### Bash History

```bash
rm -f /home/*/.bash_history
rm -f /root/.bash_history
unset HISTFILE
```

**Why:** Commands often contain passwords, API keys, and sensitive paths.

#### Cloud Credentials

```bash
# AWS
rm -rf /home/*/.aws/credentials
rm -rf /root/.aws/credentials

# Azure
rm -rf /home/*/.azure/accessTokens.json

# GCP
rm -rf /home/*/.config/gcloud/credentials.db
rm -rf /home/*/.config/gcloud/access_tokens.db

# OCI
rm -rf /home/*/.oci/sessions
```

**Why:** Build-time credentials must not persist in distributed images.

#### Git Credentials

```bash
rm -rf /home/*/.git-credentials
rm -rf /home/*/.gitconfig
rm -rf /root/.git-credentials
```

**Why:** Git credentials can grant repository access or expose personal information.

#### Docker Credentials

```bash
rm -rf /home/*/.docker/config.json
rm -rf /root/.docker/config.json
```

**Why:** Docker config may contain registry authentication tokens.

#### Environment Files

```bash
rm -rf /home/*/.env
rm -rf /home/*/.env.local
```

**Why:** `.env` files commonly contain API keys, database passwords, and secrets.

#### Package Manager Tokens

```bash
rm -rf /home/*/.npmrc
rm -rf /home/*/.yarnrc
```

**Why:** npm/yarn configs may contain private registry tokens.

#### Logs

```bash
find /var/log -type f -name "*.log" -exec truncate -s 0 {} \;
find /var/log -type f -name "*.gz" -delete
journalctl --vacuum-time=1s
```

**Why:** Logs may contain IP addresses, usernames, and error details.

#### Machine ID

```bash
truncate -s 0 /etc/machine-id
rm -f /var/lib/dbus/machine-id
```

**Why:** Machine ID should be unique per instance, not shared across images.

#### Cloud-init State

```bash
cloud-init clean --logs
rm -rf /var/lib/cloud/instance
rm -rf /var/lib/cloud/instances/*
```

**Why:** Allows cloud-init to run fresh on new instances.

### Configuration

```yaml
providers:
  packer:
    build:
      security:
        clean_sensitive_data: true # Default: true
        remove_ssh_keys: true # Default: true
```

### Verifying Cleanup

After building, verify cleanup succeeded:

```bash
# Check no credentials remain
find /home -name "credentials" -o -name "*.json" -path "*/.aws/*" 2>/dev/null
find /home -name "id_rsa*" -o -name "id_ed25519*" 2>/dev/null

# Check SSH keys removed
ls -la /etc/ssh/ssh_host_*
ls -la /home/*/.ssh/

# Check history cleared
cat /home/*/.bash_history 2>/dev/null | wc -l
```

---

## OpenSCAP Scanning

### Overview

[OpenSCAP](https://www.open-scap.org/) is an open-source security compliance framework. Sindri includes an OpenSCAP scanning script at `v3/scripts/vm/openscap-scan.sh`.

### SCAP Profiles

Security Content Automation Protocol (SCAP) profiles define compliance standards:

| Profile             | Description           | Use Case         |
| ------------------- | --------------------- | ---------------- |
| `cis_level1_server` | CIS Benchmark Level 1 | Most deployments |
| `cis_level2_server` | CIS Benchmark Level 2 | High-security    |
| `stig`              | DISA STIG compliance  | DoD/Federal      |
| `standard`          | Basic hardening       | Development      |

### Using the OpenSCAP Scanner

#### Basic Scan

```bash
# Run scan with default CIS Level 1 profile
./v3/scripts/vm/openscap-scan.sh

# Output directory
ls /tmp/openscap-results/
```

#### Custom Profile

```bash
# CIS Level 2
SCAN_TYPE=cis-level2 ./openscap-scan.sh

# STIG
SCAN_TYPE=stig ./openscap-scan.sh

# Custom profile
SCAN_PROFILE=xccdf_org.ssgproject.content_profile_standard ./openscap-scan.sh
```

#### Environment Variables

| Variable               | Default                                                  | Description                               |
| ---------------------- | -------------------------------------------------------- | ----------------------------------------- |
| `SCAN_PROFILE`         | `xccdf_org.ssgproject.content_profile_cis_level1_server` | XCCDF profile ID                          |
| `SCAN_TYPE`            | `cis`                                                    | Shortcut: cis, cis-level2, stig, standard |
| `OUTPUT_DIR`           | `/tmp/openscap-results`                                  | Report output directory                   |
| `COMPLIANCE_THRESHOLD` | `70`                                                     | Minimum passing score (%)                 |

### Interpreting Scan Results

The script generates multiple output files:

| File                     | Format    | Purpose                   |
| ------------------------ | --------- | ------------------------- |
| `report_TIMESTAMP.html`  | HTML      | Human-readable report     |
| `results_TIMESTAMP.xml`  | XCCDF XML | Machine-parseable results |
| `arf_TIMESTAMP.xml`      | ARF       | Asset Reporting Format    |
| `summary_TIMESTAMP.json` | JSON      | CI/CD integration         |

#### Sample Summary Output

```json
{
  "scan_date": "2026-02-01T12:00:00Z",
  "profile": "xccdf_org.ssgproject.content_profile_cis_level1_server",
  "results": {
    "total": 250,
    "passed": 235,
    "failed": 10,
    "not_applicable": 5,
    "error": 0,
    "compliance_percentage": "95.92"
  }
}
```

#### Result Categories

| Result            | Meaning                       | Action              |
| ----------------- | ----------------------------- | ------------------- |
| **pass**          | Control implemented correctly | None                |
| **fail**          | Control not implemented       | Remediate           |
| **notapplicable** | Control doesn't apply         | Review for accuracy |
| **error**         | Scanner couldn't evaluate     | Investigate         |

### Remediation Workflow

1. **Run initial scan:**

   ```bash
   ./openscap-scan.sh
   ```

2. **Review failures:**

   ```bash
   grep 'result="fail"' /tmp/openscap-results/results_*.xml | head -20
   ```

3. **Generate remediation script:**

   ```bash
   oscap xccdf generate fix \
     --profile xccdf_org.ssgproject.content_profile_cis_level1_server \
     --output remediate.sh \
     /usr/share/xml/scap/ssg/content/ssg-ubuntu2204-ds.xml
   ```

4. **Review and apply fixes:**

   ```bash
   less remediate.sh
   sudo bash remediate.sh
   ```

5. **Re-scan to verify:**
   ```bash
   ./openscap-scan.sh
   ```

### Enabling in Packer Builds

```yaml
providers:
  packer:
    build:
      security:
        openscap_scan: true # Generate compliance report
```

**Note:** OpenSCAP scanning adds 2-5 minutes to build time but provides compliance evidence.

---

## Access Control

### Principle of Least Privilege

Apply minimal permissions required for each role:

| Role                    | Permissions          | Scope                    |
| ----------------------- | -------------------- | ------------------------ |
| **Image Builder**       | Create/delete images | Build account only       |
| **Image Consumer**      | Launch instances     | Read-only image access   |
| **Image Administrator** | Modify sharing       | Cross-account management |

### Per-Cloud Access Control

#### AWS AMI Access Control

**Private (Default):**

```bash
# AMI is only visible to owner account
```

**Shared with Specific Accounts:**

```yaml
providers:
  packer:
    aws:
      ami_users:
        - "123456789012" # Account ID
        - "987654321098"
```

**Public (Use with Caution):**

```yaml
providers:
  packer:
    aws:
      ami_groups:
        - "all" # Makes AMI public
```

**AWS CLI:**

```bash
# Share with specific account
aws ec2 modify-image-attribute \
  --image-id ami-xxxx \
  --launch-permission "Add=[{UserId=123456789012}]"

# Make public
aws ec2 modify-image-attribute \
  --image-id ami-xxxx \
  --launch-permission "Add=[{Group=all}]"

# View current permissions
aws ec2 describe-image-attribute \
  --image-id ami-xxxx \
  --attribute launchPermission
```

#### Azure Image Access Control

**Managed Image (Single Subscription):**

- Controlled via Azure RBAC on the image resource

**Shared Image Gallery:**

```yaml
providers:
  packer:
    azure:
      gallery:
        gallery_name: sindri_gallery
        # Access controlled via gallery RBAC
```

**Community Gallery (Public):**

```bash
az sig create \
  --resource-group myRG \
  --gallery-name myGallery \
  --sharing-profile "Community"
```

#### GCP Image Access Control

**Project-Level (Default):**

```bash
# Image only accessible within project
```

**Cross-Project Sharing:**

```bash
gcloud compute images add-iam-policy-binding my-image \
  --member='user:user@example.com' \
  --role='roles/compute.imageUser'

gcloud compute images add-iam-policy-binding my-image \
  --member='serviceAccount:sa@project.iam.gserviceaccount.com' \
  --role='roles/compute.imageUser'
```

**Public (All Google Users):**

```bash
gcloud compute images add-iam-policy-binding my-image \
  --member='allAuthenticatedUsers' \
  --role='roles/compute.imageUser'
```

#### OCI Image Access Control

**Compartment-Based:**

```bash
# Images inherit compartment IAM policies
```

**Cross-Tenancy Export:**

```bash
oci compute image export to-object \
  --image-id <image_ocid> \
  --bucket-name shared-bucket \
  --name sindri-image.oci
```

#### Alibaba Cloud Image Access Control

**Shared with Accounts:**

```bash
aliyun ecs ModifyImageSharePermission \
  --RegionId us-west-1 \
  --ImageId m-xxxx \
  --AddAccount.1 <target_account_id>
```

### Audit Logging

Enable image access logging for each cloud:

| Cloud   | Service          | Configuration                            |
| ------- | ---------------- | ---------------------------------------- |
| AWS     | CloudTrail       | `DescribeImages`, `ModifyImageAttribute` |
| Azure   | Activity Log     | Image operations auto-logged             |
| GCP     | Cloud Audit Logs | Compute Engine Admin Activity            |
| OCI     | Audit Service    | Image lifecycle events                   |
| Alibaba | ActionTrail      | ECS image operations                     |

---

## Encryption

### Encryption at Rest

All cloud providers support encrypting VM images:

#### AWS EBS Encryption

```yaml
providers:
  packer:
    aws:
      encrypt_boot: true # Default: true
      # Uses AWS-managed key by default
      # For custom KMS key:
      kms_key_id: "arn:aws:kms:us-west-2:111122223333:key/1234abcd-12ab-34cd-56ef-1234567890ab"
```

**Features:**

- AES-256 encryption
- Automatic key management with AWS KMS
- Encrypted snapshots and AMI copies
- Cross-account sharing with encrypted AMIs requires key sharing

**Verify Encryption:**

```bash
aws ec2 describe-images --image-ids ami-xxxx --query 'Images[0].BlockDeviceMappings[*].Ebs.Encrypted'
```

#### Azure Disk Encryption

```yaml
providers:
  packer:
    azure:
      os_disk_encryption_set_id: "/subscriptions/xxx/resourceGroups/xxx/providers/Microsoft.Compute/diskEncryptionSets/myDES"
```

**Options:**

- Platform-managed keys (default)
- Customer-managed keys (CMK) via Azure Key Vault
- Double encryption (platform + customer key)

**Verify:**

```bash
az disk show --name myDisk --resource-group myRG --query encryptionSettings
```

#### GCP CMEK (Customer-Managed Encryption Keys)

```yaml
providers:
  packer:
    gcp:
      disk_encryption_key:
        kms_key_name: "projects/my-project/locations/global/keyRings/my-ring/cryptoKeys/my-key"
```

**Features:**

- Cloud KMS integration
- Automatic key rotation support
- Cross-project key sharing

**Verify:**

```bash
gcloud compute images describe my-image --format="value(sourceDiskEncryptionKey)"
```

#### OCI Boot Volume Encryption

OCI encrypts all boot volumes by default:

```yaml
providers:
  packer:
    oci:
      # Uses Oracle-managed key by default
      # For Vault-managed key:
      kms_key_id: "ocid1.key.oc1.iad.xxxxx"
```

**Features:**

- AES-256 encryption
- Oracle-managed or Vault-managed keys
- Encrypted backup and restore

#### Alibaba Cloud Disk Encryption

```yaml
providers:
  packer:
    alibaba:
      system_disk_encrypted: true
      # For custom KMS key:
      system_disk_kms_key_id: "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
```

**Features:**

- AES-256 encryption
- KMS integration
- Encrypted snapshot copying

### Encryption in Transit

All cloud APIs use TLS 1.2+ by default. Verify:

```bash
# Check TLS version
openssl s_client -connect ec2.us-west-2.amazonaws.com:443 -tls1_2
```

### Key Management Best Practices

| Practice                         | Description                                              |
| -------------------------------- | -------------------------------------------------------- |
| **Use CMK for production**       | Customer-managed keys enable key rotation and revocation |
| **Enable automatic rotation**    | Most clouds support automatic 365-day rotation           |
| **Separate keys by environment** | Different keys for dev, staging, production              |
| **Audit key access**             | Monitor key usage via cloud audit logs                   |
| **Backup keys**                  | Export key material where supported                      |

---

## Vulnerability Management

### Regular Base Image Updates

Base images should be updated regularly:

| Frequency       | Scope                     | Trigger          |
| --------------- | ------------------------- | ---------------- |
| **Weekly**      | Security patches          | Scheduled builds |
| **Immediately** | Critical CVEs (CVSS 9.0+) | Alert-triggered  |
| **Monthly**     | Minor version updates     | Release cycle    |
| **Quarterly**   | Major version updates     | Planning cycle   |

### CVE Scanning

#### Pre-Build Scanning

Scan base images before building:

```bash
# Using Trivy
trivy image ubuntu:22.04

# Using Grype
grype ubuntu:22.04
```

#### Post-Build Scanning

Scan built images for vulnerabilities:

```bash
# Export OVA and scan
trivy vm ./sindri-image.ova

# Scan running instance
trivy rootfs --remote ssh://ubuntu@instance-ip /
```

#### CI/CD Integration

```yaml
# .github/workflows/scan.yml
- name: Scan image for vulnerabilities
  uses: aquasecurity/trivy-action@master
  with:
    image-ref: "ami-xxxx"
    format: "sarif"
    output: "trivy-results.sarif"
    severity: "CRITICAL,HIGH"
```

### Patch Management Strategy

#### Automated Updates

Include automatic security updates in images:

```bash
# Ubuntu/Debian
apt-get install unattended-upgrades
dpkg-reconfigure -plow unattended-upgrades
```

Configuration in `/etc/apt/apt.conf.d/50unattended-upgrades`:

```
Unattended-Upgrade::Allowed-Origins {
    "${distro_id}:${distro_codename}-security";
};
Unattended-Upgrade::AutoFixInterruptedDpkg "true";
Unattended-Upgrade::MinimalSteps "true";
Unattended-Upgrade::Remove-Unused-Dependencies "true";
```

#### Version Pinning

For stability, pin critical packages:

```bash
# Pin kernel version
apt-mark hold linux-image-generic

# Pin specific version
echo "package-name hold" | dpkg --set-selections
```

### Image Deprecation

Implement deprecation policies:

| Cloud   | Deprecation Feature                              |
| ------- | ------------------------------------------------ |
| AWS     | AMI deprecation date, auto-removal after 2 years |
| Azure   | Image version end-of-life dates                  |
| GCP     | Image family versioning                          |
| OCI     | Custom image lifecycle policies                  |
| Alibaba | Image quota management                           |

```bash
# AWS: Set deprecation date
aws ec2 enable-image-deprecation \
  --image-id ami-xxxx \
  --deprecate-at "2027-02-01T00:00:00Z"
```

---

## Security Checklist

Before distributing any Sindri VM image, verify all items:

### Credential Removal

- [ ] **SSH authorized_keys cleared** - No user SSH keys remain
- [ ] **SSH host keys removed** - Will regenerate on first boot
- [ ] **AWS credentials removed** - `~/.aws/credentials` deleted
- [ ] **Azure credentials removed** - `~/.azure/` cleaned
- [ ] **GCP credentials removed** - `~/.config/gcloud/` cleaned
- [ ] **OCI credentials removed** - `~/.oci/sessions` cleaned
- [ ] **Git credentials removed** - `~/.git-credentials`, `~/.gitconfig` deleted
- [ ] **Docker credentials removed** - `~/.docker/config.json` deleted
- [ ] **NPM/Yarn tokens removed** - `~/.npmrc`, `~/.yarnrc` deleted

### History and Logs

- [ ] **Bash history cleared** - No command history remains
- [ ] **System logs cleared** - `/var/log/*.log` truncated or rotated
- [ ] **Journal logs cleared** - `journalctl --vacuum-time=1s`
- [ ] **Cloud-init logs cleared** - `cloud-init clean --logs`

### System State

- [ ] **Machine ID cleared** - `/etc/machine-id` truncated
- [ ] **Cloud-init state reset** - `/var/lib/cloud/` cleaned
- [ ] **Temporary files removed** - `/tmp/*`, `/var/tmp/*` cleaned
- [ ] **Package cache cleaned** - `apt-get clean`

### Security Hardening

- [ ] **Root password disabled** - Or set to random value
- [ ] **SSH root login disabled** - `PermitRootLogin no`
- [ ] **Strong SSH ciphers** - Weak algorithms disabled
- [ ] **CIS hardening applied** - Level 1 minimum for production
- [ ] **Firewall configured** - Unnecessary ports blocked
- [ ] **Audit logging enabled** - auditd running

### Compliance

- [ ] **OpenSCAP scan completed** - Compliance report generated
- [ ] **Compliance threshold met** - Minimum 70% (or per requirements)
- [ ] **Failed controls documented** - Exceptions approved if needed
- [ ] **Vulnerability scan completed** - No critical CVEs

### Encryption

- [ ] **Disk encryption enabled** - Cloud-native or CMK
- [ ] **Encryption key documented** - Key ID recorded for sharing

### Access Control

- [ ] **Image permissions set** - Private, shared, or public as intended
- [ ] **Sharing documented** - Account IDs or users listed
- [ ] **Audit logging enabled** - Image access tracked

### Verification Commands

```bash
# Check credentials
find /home /root -name "credentials" -o -name "accessTokens*" -o -name "*.db" 2>/dev/null

# Check SSH
ls -la /etc/ssh/ssh_host_* 2>/dev/null
find /home /root -name "authorized_keys" -o -name "id_*" 2>/dev/null

# Check history
cat /home/*/.bash_history /root/.bash_history 2>/dev/null | wc -l

# Check machine-id
cat /etc/machine-id

# Verify hardening
sshd -T | grep -E "permitrootlogin|ciphers"
sysctl kernel.randomize_va_space
systemctl is-active auditd
```

---

## Related Documentation

- [VM Provider Guide](../VM.md) - Full VM image management documentation
- [Secrets Management](../../SECRETS_MANAGEMENT.md) - Build-time secrets handling
- [ADR-031: Packer Architecture](../../architecture/adr/031-packer-vm-provisioning-architecture.md)
- [CIS Benchmarks](https://www.cisecurity.org/cis-benchmarks) - Official CIS documentation
- [OpenSCAP](https://www.open-scap.org/) - SCAP framework documentation
