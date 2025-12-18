# Security Audit Remediation Addendum

**Document Date:** December 17, 2025
**Related Report:** [SECURITY_AUDIT_REPORT.md](./SECURITY_AUDIT_REPORT.md)
**Status:** Implementation Planning Phase
**Remaining Findings:** 3 Critical, 2 High Severity

---

## Executive Summary

This addendum provides detailed implementation guidance for the remaining critical and high-severity security findings from the Sindri Security Audit Report. Through comprehensive research of industry best practices, OWASP guidelines, and analysis of 73 extensions, we've developed a phased remediation approach that balances security improvements with functional requirements.

**Key Findings:**

- 68 of 73 extensions (93%) require external network access
- 9 extensions/scripts use curl|shell patterns
- 0 extensions currently use binary download method (gap exists in executor.sh)
- SSH workflow considerations require careful handling of GitHub token changes
- DNS validation must balance security with development environment usability

**Estimated Effort:** 10-18 hours (1.5-2 days) after decision points resolved

---

## Research Methodology

Six parallel research agents conducted comprehensive analysis:

1. **Curl-to-Shell Security** - Analyzed SLSA Framework, Rustup, Homebrew patterns
2. **GitHub Token Management** - Studied GitHub CLI security, PAT scoping, CWE-522/214
3. **Binary Integrity Verification** - Reviewed Sigstore, SLSA provenance, package managers
4. **DNS Validation Best Practices** - Examined NIST SP 800-81-2, retry strategies
5. **Docker Network Security** - Analyzed CIS Docker Benchmark, DevContainer patterns
6. **Extension Impact Analysis** - Evaluated all 73 extensions for breaking changes

---

## Critical Findings - Detailed Implementation Plans

### C-3: Unvalidated Curl Piped to Shell

**Priority:** URGENT
**Complexity:** Medium
**Impact:** 9 affected (7 extensions + 2 core installers)

#### Current Vulnerability

Scripts download and execute code without integrity verification:

```bash
# docker/scripts/install-mise.sh
curl -fsSL https://mise.run | MISE_INSTALL_PATH="$MISE_INSTALL_PATH" sh

# docker/scripts/install-claude.sh
curl -fsSL https://claude.ai/install.sh | bash
```

**Affected Extensions:**

- ai-toolkit
- cloud-tools (Azure CLI)
- goose (Block installer)
- jvm (SDKMAN)
- monitoring (UV package manager)
- ollama (official installer)
- php (add-apt-repository)
- infra-tools (Pulumi, Crossplane)

#### Industry Best Practice (2025)

**Download → Verify → Execute Pattern** (Rustup standard):

1. Download installer to temporary file
2. Verify cryptographic checksum from separate URL
3. Execute from file (not piped)
4. Secure cleanup with trap

#### Recommended Implementation

```bash
#!/bin/bash
# docker/scripts/install-mise.sh - Secure version

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

# Configuration - UPDATE WHEN MISE RELEASES NEW VERSIONS
MISE_VERSION="2025.1.0"
MISE_INSTALL_PATH="${MISE_INSTALL_PATH:-/usr/local/bin/mise}"
MISE_INSTALLER_URL="https://mise.run/install.sh"
MISE_CHECKSUM_URL="https://mise.run/install.sh.sha256"

# Create secure temporary directory
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

print_status "Downloading mise installer v${MISE_VERSION}..."
if ! curl -fsSL "$MISE_INSTALLER_URL" -o "$TEMP_DIR/install.sh"; then
    print_error "Failed to download mise installer"
    exit 1
fi

print_status "Downloading checksum..."
if ! curl -fsSL "$MISE_CHECKSUM_URL" -o "$TEMP_DIR/install.sh.sha256"; then
    print_error "Failed to download checksum"
    exit 1
fi

print_status "Verifying installer integrity..."
if ! (cd "$TEMP_DIR" && sha256sum -c install.sh.sha256 --quiet); then
    print_error "SHA256 verification failed!"
    print_error "Expected: $(cat "$TEMP_DIR/install.sh.sha256")"
    print_error "Actual: $(sha256sum "$TEMP_DIR/install.sh")"
    exit 1
fi

print_success "Checksum verified"

print_status "Installing mise to $MISE_INSTALL_PATH..."
MISE_INSTALL_PATH="$MISE_INSTALL_PATH" bash "$TEMP_DIR/install.sh"

# Verify installation
if [[ ! -x "$MISE_INSTALL_PATH" ]]; then
    print_error "mise installation failed - binary not found"
    exit 1
fi

print_success "mise installed: $($MISE_INSTALL_PATH --version)"
```

#### Automated Hash Update Workflow

```yaml
# .github/workflows/update-installer-hashes.yml
name: Update Installer Hashes

on:
  schedule:
    - cron: "0 2 * * *" # Daily at 2 AM UTC
  workflow_dispatch:

jobs:
  update-hashes:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Update mise installer hash
        run: |
          INSTALLER_URL="https://mise.run/install.sh"
          NEW_HASH=$(curl -fsSL "$INSTALLER_URL" | sha256sum | cut -d' ' -f1)

          sed -i "s/MISE_SHA256=\"[^\"]*\"/MISE_SHA256=\"${NEW_HASH}\"/" docker/scripts/install-mise.sh

      - name: Create Pull Request
        uses: peter-evans/create-pull-request@v5
        with:
          commit-message: "chore: update mise installer hash"
          title: "Security: Update mise installer checksum"
          body: |
            Automated update of mise installer SHA256 checksum.

            Please verify the new hash before merging.
```

#### ⚠️ DECISION REQUIRED: C-3 Implementation Approach

**Problem:** mise.run and claude.ai don't currently publish `.sha256` checksum files.

**Option A: Embed Checksums in Scripts** (Recommended)

- **Pros:**
  - Works immediately without upstream changes
  - Full verification before execution
  - CI/CD can automate hash updates
- **Cons:**
  - Manual/automated hash updates when versions change
  - Risk of stale hashes if automation fails
- **Effort:** 4-6 hours (implement + CI/CD)

**Option B: Vendor the Installers**

- **Pros:**
  - Complete control over installer content
  - Can verify once, commit to repo
  - No runtime download risk
- **Cons:**
  - Larger repository size
  - Manual updates required
  - Divergence from upstream updates
- **Effort:** 2-3 hours (vendor + update workflow)

**Option C: Request Upstream Checksum Publishing**

- **Pros:**
  - Industry best practice if adopted
  - Aligns with Rustup, Homebrew patterns
  - No maintenance burden
- **Cons:**
  - Depends on upstream timeline
  - May not be prioritized
  - Requires interim solution
- **Effort:** 1 hour (submit requests) + wait time

**Option D: Accept Risk with Mitigation**

- **Pros:**
  - No code changes needed
  - Existing timeout protection (install-claude.sh)
  - TLS validation via curl
- **Cons:**
  - Remains in audit report as unresolved critical finding
  - Vulnerable to MITM/CDN compromise
  - Violates CWE-494 recommendations
- **Effort:** 30 minutes (document accepted risk)

**Recommendation:** **Option A** (embed checksums) with automated CI/CD updates. Provides immediate security improvement while maintaining operational simplicity.

**Decision:** [ ] A [ ] B [ ] C [ ] D

---

### C-7: Insecure GITHUB_TOKEN Propagation

**Priority:** URGENT
**Complexity:** High (SSH workflow considerations)
**Impact:** 68 extensions with github.com domain requirement

#### Current Vulnerability

Tokens stored and propagated insecurely:

```bash
# docker/scripts/entrypoint.sh:262-277
cat > "${ALT_HOME}/.git-credential-helper.sh" << 'GITCRED'
#!/bin/bash
if [ "$1" = "get" ]; then
    echo "password=$GITHUB_TOKEN"  # ⚠️ PLAINTEXT TOKEN
fi
GITCRED

# Token propagated via /etc/profile.d/
export GITHUB_TOKEN="${GITHUB_TOKEN}"  # ⚠️ VISIBLE IN PROCESS LIST
```

**Security Issues:**

- Token visible in `/proc/<pid>/cmdline` and `/proc/<pid>/environ`
- Plaintext credential helper script readable by all processes
- No token encryption or rotation
- Token persists indefinitely

#### Industry Best Practice (GitHub 2025 Recommendations)

1. **Use GitHub CLI credential helper** (encrypted keyring when available)
2. **Fine-grained Personal Access Tokens** with minimal scope
3. **Token via stdin** (not command arguments or environment variables)
4. **7-day expiration** for development, 1-hour for CI/CD
5. **Automatic token rotation**

#### SSH Workflow Consideration

**User Concern:** Changes to GITHUB_TOKEN handling may break git operations when users SSH into container.

**Current SSH Session Flow:**

1. User SSHs into container: `ssh developer@container`
2. Git clone/pull operations use credential helper
3. Helper script returns `$GITHUB_TOKEN` from environment
4. Git operations succeed

**If we change token handling, we must ensure:**

- Git operations still work in SSH sessions
- Token remains accessible to git credential helper
- No manual re-authentication required per SSH session

#### Recommended Implementation Options

**Option A: Use GitHub CLI Credential Helper** (Recommended Security)

```bash
#!/bin/bash
# docker/scripts/entrypoint.sh - Enhanced GitHub authentication

setup_github_authentication() {
    local developer_user="${DEVELOPER_USER:-developer}"
    local alt_home="${ALT_HOME:-/alt/home/developer}"

    if [[ -z "${GITHUB_TOKEN:-}" ]]; then
        print_warning "No GITHUB_TOKEN provided - GitHub operations may be rate-limited"
        return 0
    fi

    # Pass token via stdin (not visible in process list)
    print_status "Configuring GitHub authentication..."
    if ! echo "$GITHUB_TOKEN" | su - "$developer_user" -c "gh auth login --with-token" 2>/dev/null; then
        print_error "GitHub CLI authentication failed"
        return 1
    fi

    # Use gh's built-in credential helper
    su - "$developer_user" -c "git config --global credential.helper '!gh auth git-credential'"

    # Verify authentication
    if su - "$developer_user" -c "gh auth status" &>/dev/null; then
        print_success "GitHub authentication configured"
        security_log_auth "$developer_user" "github_auth_configured" "success" "gh CLI credential helper"
    fi

    # Clear token from environment (no longer needed)
    unset GITHUB_TOKEN
}
```

**SSH Session Behavior:**

- ✅ Git operations work (gh CLI provides credentials on-demand)
- ✅ Token stored in gh CLI's secure storage (encrypted if keyring available, fallback to `~/.config/gh/hosts.yaml`)
- ⚠️ Token expires after 7 days (fine-grained PAT) - user must re-authenticate
- ⚠️ If gh CLI unavailable, git operations fail

**Pros:**

- Industry standard approach
- Token not visible in process listings
- Leverages GitHub's official credential management
- Supports token refresh via `gh auth refresh`

**Cons:**

- Requires gh CLI available in container (already installed)
- Token expiration requires re-authentication
- Fallback storage still plaintext in headless containers

---

**Option B: Secure tmpfs Storage** (Minimal Change)

```bash
#!/bin/bash
# docker/scripts/entrypoint.sh - Minimal security improvement

setup_github_authentication() {
    local developer_user="${DEVELOPER_USER:-developer}"
    local alt_home="${ALT_HOME:-/alt/home/developer}"

    if [[ -z "${GITHUB_TOKEN:-}" ]]; then
        return 0
    fi

    # Store token in tmpfs (memory-only, not disk)
    local secure_token_file="/dev/shm/.git-token-$$"
    echo "$GITHUB_TOKEN" > "$secure_token_file"
    chmod 600 "$secure_token_file"

    # Credential helper reads from secure file
    cat > "${alt_home}/.git-credential-helper.sh" << EOF
#!/bin/bash
if [ "\$1" = "get" ]; then
    if [[ -f "$secure_token_file" ]]; then
        echo "protocol=https"
        echo "host=github.com"
        echo "username=git"
        echo "password=\$(cat '$secure_token_file')"
    fi
fi
EOF
    chmod 700 "${alt_home}/.git-credential-helper.sh"
    chown "$developer_user:$developer_user" "${alt_home}/.git-credential-helper.sh"

    # Configure git to use helper
    su - "$developer_user" -c "git config --global credential.helper '${alt_home}/.git-credential-helper.sh'"

    # Cleanup on container stop
    trap "shred -vfz '$secure_token_file' 2>/dev/null || rm -f '$secure_token_file'" EXIT TERM

    # Clear from environment
    unset GITHUB_TOKEN

    security_log_auth "$developer_user" "github_auth_configured" "success" "tmpfs credential helper"
}
```

**SSH Session Behavior:**

- ✅ Git operations work (credential helper remains functional)
- ✅ Token stored in memory (/dev/shm), not disk
- ✅ No expiration issues
- ⚠️ Token still readable by any process as developer user
- ⚠️ Token persists until container restart

**Pros:**

- Minimal changes to existing workflow
- SSH sessions work identically
- No token expiration concerns
- Memory-only storage (not persisted to disk)

**Cons:**

- Token still accessible to compromised processes
- Doesn't address CWE-522 (insufficiently protected credentials)
- Remains partially vulnerable

---

**Option C: Document as Accepted Risk**

```markdown
# Accepted Risk: C-7 GITHUB_TOKEN Storage

**Rationale:** Development environment requires persistent GitHub authentication
for SSH sessions. Current approach prioritizes usability over defense-in-depth.

**Mitigations in place:**

- Use fine-grained PATs with read-only scope (contents:read)
- Recommend 7-day token expiration
- SSH hardening (H-1) limits unauthorized container access
- Rate limiting (H-11) prevents token abuse
- Security logging (H-12) tracks authentication events

**Residual risk:** MEDIUM - Token exposure requires compromised container access
```

**Pros:**

- No code changes
- No workflow disruption
- Acknowledge trade-off explicitly

**Cons:**

- Critical finding remains unresolved
- Violates CWE-522 recommendations
- May fail compliance audits (SOC 2, ISO 27001)

---

#### ⚠️ DECISION REQUIRED: C-7 Implementation Approach

**Recommendation:** **Option A** (gh CLI credential helper) for production-ready security, with documentation that token expires after 7 days and requires `gh auth login` to refresh.

**Alternative:** **Option B** (tmpfs storage) if SSH session persistence without re-authentication is critical requirement.

**Decision:** [ ] A [ ] B [ ] C

**Additional Context Needed:**

- How often do users SSH into containers?
- Is 7-day token expiration acceptable?
- Are automated refreshes possible (e.g., via cron)?

---

### C-8: Unvalidated Binary Downloads

**Priority:** URGENT
**Complexity:** Low
**Impact:** 0 extensions currently (preventive measure)

#### Current Vulnerability

`executor.sh` lines 364-424 implement binary download method without checksum verification:

```bash
install_via_binary() {
    # ... download code ...
    curl -fsSL -o "$temp_file" "$url" || return 1

    # ⚠️ NO CHECKSUM VERIFICATION

    if [[ "$extract" == "true" ]]; then
        tar -xzf "$temp_file" -C "$destination"
    else
        mv "$temp_file" "$destination/$name"
        chmod +x "$destination/$name"
    fi
}
```

#### Industry Best Practice

**SLSA Framework + Package Manager Pattern:**

- SHA256/SHA512 checksums **optional by default** (matches Homebrew, GoReleaser)
- Auto-detect checksum files (`.sha256`, `SHA256SUMS`, `CHECKSUMS.txt`)
- Verify when available, warn if missing
- Don't block installation for missing checksums (usability)

#### Recommended Implementation

```bash
#!/bin/bash
# cli/extension-manager-modules/executor.sh - Enhanced binary method

install_via_binary() {
    local ext_yaml="$1"
    local ext_name="$2"

    # Load binary download configuration
    local download_count
    download_count=$(load_yaml "$ext_yaml" '.install.binary.downloads | length' 2>/dev/null || echo 0)

    if [[ $download_count -eq 0 ]]; then
        print_error "No binary downloads specified in extension"
        return 1
    fi

    local i=0
    while [[ $i -lt $download_count ]]; do
        local name url destination extract
        name=$(load_yaml "$ext_yaml" ".install.binary.downloads[$i].name")
        url=$(load_yaml "$ext_yaml" ".install.binary.downloads[$i].url")
        destination=$(load_yaml "$ext_yaml" ".install.binary.downloads[$i].destination")
        extract=$(load_yaml "$ext_yaml" ".install.binary.downloads[$i].extract" 2>/dev/null || echo "false")

        print_status "Downloading $name from $url..."

        # Create secure temporary file
        local temp_file
        temp_file=$(mktemp)
        trap "rm -f '$temp_file'" EXIT ERR

        # Download with timeout
        if ! timeout 300 curl -fsSL -o "$temp_file" "$url"; then
            print_error "Failed to download $name"
            rm -f "$temp_file"
            return 1
        fi

        # NEW: Optional checksum verification
        local checksum_algo checksum_value checksum_required
        checksum_algo=$(load_yaml "$ext_yaml" ".install.binary.downloads[$i].integrity.algorithm" 2>/dev/null || echo "")
        checksum_value=$(load_yaml "$ext_yaml" ".install.binary.downloads[$i].integrity.value" 2>/dev/null || echo "")
        checksum_required=$(load_yaml "$ext_yaml" ".install.binary.downloads[$i].integrity.required" 2>/dev/null || echo "false")

        if [[ -n "$checksum_value" && -n "$checksum_algo" ]]; then
            print_status "Verifying $checksum_algo checksum..."

            # Compute actual checksum
            local actual_checksum
            case "$checksum_algo" in
                sha256)
                    actual_checksum=$(sha256sum "$temp_file" | cut -d' ' -f1)
                    ;;
                sha512)
                    actual_checksum=$(sha512sum "$temp_file" | cut -d' ' -f1)
                    ;;
                *)
                    print_error "Unsupported checksum algorithm: $checksum_algo"
                    return 1
                    ;;
            esac

            # Compare checksums
            if [[ "$actual_checksum" != "$checksum_value" ]]; then
                print_error "Checksum verification failed for $name"
                print_error "  Algorithm: $checksum_algo"
                print_error "  Expected:  $checksum_value"
                print_error "  Actual:    $actual_checksum"
                security_log_install "$ext_name" "checksum_failed" "$name" "$url"
                rm -f "$temp_file"
                return 1
            fi

            print_success "Checksum verified ($checksum_algo)"
            security_log_install "$ext_name" "checksum_verified" "$name" "$url"

        elif [[ "$checksum_required" == "true" ]]; then
            print_error "Checksum required but not provided for $name"
            security_log_install "$ext_name" "checksum_missing_required" "$name" "$url"
            rm -f "$temp_file"
            return 1

        else
            print_warning "No checksum provided for $name - integrity not verified"
            security_log_install "$ext_name" "checksum_missing_optional" "$name" "$url"
        fi

        # Install binary
        mkdir -p "$destination"

        if [[ "$extract" == "true" ]]; then
            print_status "Extracting $name to $destination..."
            tar -xzf "$temp_file" -C "$destination"
        else
            print_status "Installing $name to $destination..."
            mv "$temp_file" "$destination/$name"
            chmod +x "$destination/$name"
        fi

        print_success "Installed $name"

        ((i++))
    done

    return 0
}
```

#### Schema Extension

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Extension Binary Install Method",
  "type": "object",
  "properties": {
    "install": {
      "properties": {
        "binary": {
          "properties": {
            "downloads": {
              "type": "array",
              "items": {
                "type": "object",
                "properties": {
                  "name": { "type": "string" },
                  "url": { "type": "string", "format": "uri" },
                  "destination": { "type": "string" },
                  "extract": { "type": "boolean", "default": false },
                  "integrity": {
                    "type": "object",
                    "description": "Optional cryptographic integrity verification",
                    "properties": {
                      "required": {
                        "type": "boolean",
                        "default": false,
                        "description": "Fail installation if checksum missing or invalid"
                      },
                      "algorithm": {
                        "type": "string",
                        "enum": ["sha256", "sha512"],
                        "description": "Checksum algorithm"
                      },
                      "value": {
                        "type": "string",
                        "pattern": "^[a-f0-9]{64,128}$",
                        "description": "Expected checksum value (hex string)"
                      }
                    },
                    "required": ["algorithm", "value"]
                  }
                },
                "required": ["name", "url", "destination"]
              }
            }
          }
        }
      }
    }
  }
}
```

#### Example Extension Usage

```yaml
# docker/lib/extensions/example-binary/extension.yaml
metadata:
  name: example-binary
  version: "1.0.0"

install:
  method: binary
  binary:
    downloads:
      - name: mytool
        url: https://github.com/owner/repo/releases/download/v1.0.0/mytool_linux_amd64.tar.gz
        destination: /usr/local/bin
        extract: true
        integrity:
          required: false
          algorithm: sha256
          value: "a3b2c1d4e5f6789012345678901234567890123456789012345678901234567890"
```

#### Implementation Plan

**Files to Modify:**

1. `cli/extension-manager-modules/executor.sh` - Add checksum verification logic
2. `docker/lib/schemas/extension.schema.json` - Add integrity property
3. `docs/EXTENSION_AUTHORING.md` - Document integrity verification

**Testing:**

1. Unit test: Checksum verification with valid hash
2. Unit test: Checksum verification with invalid hash (should fail)
3. Unit test: Missing checksum with required=true (should fail)
4. Unit test: Missing checksum with required=false (should warn and continue)
5. Integration test: Install extension with binary method + checksum

**No decision required - implementation is straightforward.**

---

## High Severity Findings - Detailed Implementation Plans

### H-7: Missing DNS Validation for External Resources

**Priority:** High
**Complexity:** Medium
**Impact:** 68 extensions with domain requirements

#### Current Implementation

```bash
# cli/extension-manager-modules/executor.sh:152-161
for domain in $domains; do
    [[ "${VERBOSE:-false}" == "true" ]] && print_status "Checking DNS: $domain"
    if ! check_dns "$domain"; then
        print_warning "Cannot resolve domain: $domain"  # ⚠️ WARNING ONLY
    fi
done
```

**Issue:** DNS failures don't prevent installation - security recommendation is to fail on critical domain resolution failures.

#### Industry Best Practice

**npm, apt, Docker Pattern:**

1. **Retry with exponential backoff** (3 attempts)
2. **Distinguish transient vs permanent failures**
3. **Mandatory for critical domains**, optional for mirrors
4. **Total timeout: ~14 seconds** (2s + 4s + 8s)

#### Recommended Implementation

```bash
#!/bin/bash
# docker/lib/common.sh - Enhanced DNS validation

retry_dns_with_backoff() {
    local domain="$1"
    local max_attempts="${2:-3}"
    local attempt=1
    local backoff_base=2

    while [[ $attempt -le $max_attempts ]]; do
        # Try DNS resolution with timeout
        if timeout 3 getent hosts "$domain" &>/dev/null; then
            return 0
        fi

        if [[ $attempt -lt $max_attempts ]]; then
            # Exponential backoff: 2^1=2s, 2^2=4s, 2^3=8s
            local wait_time=$((backoff_base ** attempt))

            # Add jitter (±20% variation) to prevent thundering herd
            local jitter=$(( ($(od -An -N2 -i /dev/urandom 2>/dev/null || echo $RANDOM) % 40) - 20 ))
            wait_time=$(( wait_time + (jitter * wait_time / 100) ))

            print_warning "DNS resolution failed for $domain, retrying in ${wait_time}s (attempt $attempt/$max_attempts)"
            sleep "$wait_time"
        fi

        ((attempt++))
    done

    return 1
}

validate_domain_requirements() {
    local ext_yaml="$1"
    local ext_name="$2"

    # Load domain requirements
    # Support both legacy flat list and new categorized format
    local legacy_domains critical_domains optional_domains

    # Try new format first (domains.critical, domains.optional)
    critical_domains=$(load_yaml "$ext_yaml" '.requirements.domains.critical[]?' 2>/dev/null || true)
    optional_domains=$(load_yaml "$ext_yaml" '.requirements.domains.optional[]?' 2>/dev/null || true)

    # Fallback to legacy format (flat domains[] list)
    if [[ -z "$critical_domains" && -z "$optional_domains" ]]; then
        legacy_domains=$(load_yaml "$ext_yaml" '.requirements.domains[]?' 2>/dev/null || true)

        # Treat legacy domains as optional by default (backward compatible)
        optional_domains="$legacy_domains"
    fi

    # Validate critical domains (mandatory)
    local critical_failures=0
    for domain in $critical_domains; do
        print_status "Validating critical domain: $domain"

        if ! retry_dns_with_backoff "$domain" 3; then
            print_error "Cannot resolve critical domain: $domain (required for $ext_name)"
            security_log_install "$ext_name" "dns_validation_failed" "critical" "$domain"
            ((critical_failures++))
        else
            security_log_install "$ext_name" "dns_validation_success" "critical" "$domain"
        fi
    done

    # Validate optional domains (warn only)
    for domain in $optional_domains; do
        [[ "${VERBOSE:-false}" == "true" ]] && print_status "Checking optional domain: $domain"

        if ! retry_dns_with_backoff "$domain" 3; then
            print_warning "Cannot resolve optional domain: $domain (may affect $ext_name performance)"
            security_log_install "$ext_name" "dns_validation_warning" "optional" "$domain"
        else
            security_log_install "$ext_name" "dns_validation_success" "optional" "$domain"
        fi
    done

    # Fail if any critical domains failed
    if [[ $critical_failures -gt 0 ]]; then
        print_error "DNS validation failed for $ext_name ($critical_failures critical domain(s) unresolved)"
        return 1
    fi

    return 0
}
```

#### Schema Extension

```json
{
  "requirements": {
    "properties": {
      "domains": {
        "oneOf": [
          {
            "type": "array",
            "items": { "type": "string", "format": "hostname" },
            "description": "Legacy format: flat list of domains (treated as optional)"
          },
          {
            "type": "object",
            "properties": {
              "critical": {
                "type": "array",
                "items": { "type": "string", "format": "hostname" },
                "description": "Domains that must resolve (installation fails if unavailable)"
              },
              "optional": {
                "type": "array",
                "items": { "type": "string", "format": "hostname" },
                "description": "Domains that improve functionality but aren't required"
              }
            }
          }
        ]
      }
    }
  }
}
```

#### ⚠️ DECISION REQUIRED: Domain Categorization Strategy

**Current State:**

- 68 extensions use flat `domains[]` list
- No distinction between critical and optional

**Option A: Categorize All Extensions Retroactively**

Update all 68 extensions to use new schema:

```yaml
requirements:
  domains:
    critical:
      - registry.npmjs.org # npm packages require this
      - github.com # GitHub releases require this
      - pypi.org # Python packages require this
    optional:
      - mirrors.aliyun.com # Chinese mirror (optional)
      - cdn.jsdelivr.net # CDN (optional)
```

- **Pros:**
  - Proper security validation
  - Explicit intent for each domain
  - Clear audit trail
- **Cons:**
  - 68 extension YAML files to update
  - Risk of miscategorization
  - Significant effort
- **Effort:** 8-12 hours (review + update + test all extensions)

---

**Option B: Treat All Existing as Optional** (Recommended)

Backward compatible approach:

1. Legacy flat `domains[]` → treated as **optional** (warn only)
2. New extensions use explicit `domains.critical[]` and `domains.optional[]`
3. Gradually migrate high-risk extensions over time

```bash
# Backward compatible logic
if extension has domains.critical or domains.optional:
    use new validation (fail on critical)
else:
    treat all as optional (warn only, current behavior)
```

- **Pros:**
  - No breaking changes
  - Backward compatible
  - Incremental migration path
- **Cons:**
  - Doesn't immediately address security issue for existing extensions
  - Audit report shows partial remediation
- **Effort:** 2-3 hours (implement logic + update 5-10 high-priority extensions)

---

**Option C: Make All Critical by Default**

Aggressive security posture:

1. All domains in flat `domains[]` → treated as **critical** (fail on error)
2. Extensions must explicitly opt-in to optional domains

- **Pros:**
  - Secure by default
  - Forces explicit intent
  - Addresses security concern immediately
- **Cons:**
  - May break installations in environments with DNS issues
  - Could cause support burden
  - Reduces usability in development environments
- **Effort:** 2 hours (implement) + unknown support overhead

---

**Recommendation:** **Option B** (backward compatible) with gradual migration. Prioritize categorizing the top 20 extensions by usage.

**Decision:** [ ] A [ ] B [ ] C

---

### H-10: Unrestricted Container Networking

**Priority:** High
**Complexity:** Low
**Impact:** Minimal (transparent change)

#### Current Vulnerability

Docker Compose uses default bridge network (docker0):

```yaml
# docker-compose.yml
services:
  sindri:
    image: sindri:latest
    # ⚠️ No explicit network - uses default docker0
    # ⚠️ Can communicate with other containers on host
```

**Issue:** Violates CIS Docker Benchmark 5.31 (Use custom networks for containers)

#### Industry Best Practice

**CIS Docker Benchmark + DevContainer Standards:**

- Use custom bridge networks (isolates from unrelated containers)
- Allow internet access (required for development)
- No egress filtering needed for dev environments
- Transparent to application (same behavior)

#### Recommended Implementation

```yaml
# docker-compose.yml
version: "3.8"

services:
  sindri:
    image: sindri:latest
    container_name: sindri-dev

    # NEW: Custom network isolation (H-10 fix)
    networks:
      - sindri_isolated

    # Existing M-8 security options (process security)
    security_opt:
      - no-new-privileges:true
      - seccomp:default
    cap_drop:
      - ALL
    cap_add:
      - CHOWN
      - DAC_OVERRIDE
      - FOWNER
      - SETUID
      - SETGID

    # Existing tmpfs security
    tmpfs:
      - /tmp:size=2G,mode=1777,noexec,nosuid,nodev

    volumes:
      - sindri-home:/alt/home/developer
      - ./workspace:/alt/home/developer/workspace

    ports:
      - "2222:2222"

    environment:
      - DEVELOPER_USER=developer

# NEW: Custom isolated network
networks:
  sindri_isolated:
    driver: bridge
    # internal: false (default - allows internet access)
    driver_opts:
      com.docker.network.bridge.default_bridge: "false"
    ipam:
      config:
        - subnet: "172.25.0.0/16"

volumes:
  sindri-home:
```

#### DevContainer Integration

```json
{
  "name": "sindri-dev",
  "dockerFile": "../Dockerfile",

  "runArgs": [
    "--network",
    "sindri_isolated",
    "--cap-drop=ALL",
    "--cap-add=CHOWN",
    "--cap-add=DAC_OVERRIDE",
    "--cap-add=FOWNER",
    "--cap-add=SETUID",
    "--cap-add=SETGID",
    "--security-opt",
    "no-new-privileges:true",
    "--security-opt",
    "seccomp=default",
    "--tmpfs",
    "/tmp:size=2G,mode=1777,noexec,nosuid,nodev"
  ]
}
```

#### Deploy Adapter Updates

```bash
# deploy/adapters/docker-adapter.sh

generate_docker_compose() {
    # ... existing code ...

    cat >> "$docker_compose_file" << EOF

networks:
  sindri_isolated:
    driver: bridge
    driver_opts:
      com.docker.network.bridge.default_bridge: "false"
EOF
}
```

#### Verification

```bash
# Verify custom network created
docker network ls | grep sindri_isolated

# Verify container attached to custom network
docker inspect sindri-dev | jq '.[0].NetworkSettings.Networks'

# Test: Internet access still works
docker exec sindri-dev curl -fsSL https://httpbin.org/ip

# Test: Isolated from other containers
docker run -d --name other-container nginx
docker exec sindri-dev ping other-container  # Should fail (isolated)
```

#### Implementation Plan

**Files to Modify:**

1. `docker-compose.yml` - Add networks section
2. `deploy/adapters/docker-adapter.sh` - Generate network in Compose file
3. `.devcontainer/devcontainer.json` - Add --network run arg
4. `docs/DEPLOYMENT.md` - Document network isolation

**Testing:**

1. Build and start container with new network
2. Verify internet access (curl external URLs)
3. Verify isolation (cannot reach containers on docker0)
4. Run smoke test suite (should pass)

**No decision required - implementation is straightforward and low-risk.**

---

## Implementation Phases

### Phase 1: Quick Wins (No Decisions Required)

**Estimated Time:** 2-4 hours
**Risk:** Low
**Impact:** High security improvement

1. **H-10: Container Networking Isolation**
   - Add custom bridge network to docker-compose.yml
   - Update devcontainer.json
   - Update deploy adapters
   - **Files:** 3-4 files
   - **Testing:** Smoke tests should pass

2. **C-8: Binary Checksum Validation**
   - Add integrity verification to executor.sh
   - Update extension.schema.json
   - Add security logging
   - **Files:** 2-3 files
   - **Testing:** Unit tests for checksum validation

**Deliverables:**

- ✅ CIS Docker Benchmark 5.31 compliance
- ✅ Binary download integrity framework
- ✅ Security logging for both remediations

---

### Phase 2: Awaiting Decisions

**Estimated Time:** 4-8 hours (after decisions)
**Risk:** Medium
**Impact:** Critical findings addressed

3. **C-3: Curl Piped to Shell**
   - **Waiting on:** Option A/B/C/D decision
   - Update install-mise.sh and install-claude.sh
   - Optionally create CI/CD hash update workflow
   - Update 7 affected extension install scripts
   - **Files:** 2-9 files depending on approach

4. **C-7: GitHub Token Security**
   - **Waiting on:** Option A/B/C decision + SSH impact assessment
   - Update entrypoint.sh setup_git_config()
   - Add security logging
   - Test SSH git operations
   - **Files:** 1-2 files

5. **H-7: DNS Validation**
   - **Waiting on:** Option A/B/C decision
   - Implement retry_dns_with_backoff() in common.sh
   - Update validate_domain_requirements() in executor.sh
   - Update extension.schema.json
   - Optionally update 68 extensions (Option A) or 5-10 high-priority (Option B)
   - **Files:** 3 files + 5-68 extensions

---

### Phase 3: Testing & Documentation

**Estimated Time:** 4-6 hours
**Risk:** Low
**Impact:** Production readiness

6. **Comprehensive Testing**
   - Unit tests for all new functions
   - Integration tests for affected extensions
   - Smoke test suite validation
   - SSH session testing (C-7)
   - DNS failure simulation (H-7)

7. **Documentation Updates**
   - Update SECURITY_AUDIT_REPORT.md status
   - Document accepted risks (if any)
   - Update docs/SECURITY.md
   - Update docs/EXTENSION_AUTHORING.md
   - Add troubleshooting guides

8. **Audit Report Completion**
   - Mark remediated findings as ✅ FIXED
   - Document accepted risks as ⚠️ ACCEPTED RISK
   - Update risk level assessment
   - Calculate new compliance score

---

## Summary of Required Decisions

### Decision 1: C-3 Installer Checksum Strategy

**Question:** How should we handle mise and Claude installers that don't publish checksums?

- [ ] **Option A:** Embed checksums in scripts with CI/CD auto-updates (Recommended)
- [ ] **Option B:** Vendor installers in repository
- [ ] **Option C:** Request upstream checksum publishing + interim solution
- [ ] **Option D:** Accept risk with documentation

**Impact:** 2 core installers + 7 extensions

---

### Decision 2: C-7 GitHub Token Handling

**Question:** How should we secure GitHub tokens while maintaining SSH session git functionality?

- [ ] **Option A:** Use gh CLI credential helper (Recommended security, 7-day token expiry)
- [ ] **Option B:** Minimal change with tmpfs storage (Maintains current workflow)
- [ ] **Option C:** Document as accepted risk

**Additional Context Needed:**

- Do users frequently SSH into containers for git operations?
- Is 7-day token expiration acceptable?
- Can users run `gh auth login` when token expires?

**Impact:** 68 extensions with GitHub requirements

---

### Decision 3: H-7 DNS Domain Categorization

**Question:** Should we categorize all 68 extensions' domains as critical vs optional?

- [ ] **Option A:** Update all 68 extensions retroactively (Most secure, high effort)
- [ ] **Option B:** Backward compatible + gradual migration (Recommended, low risk)
- [ ] **Option C:** Make all critical by default (Aggressive, may break installs)

**Impact:** 68 extensions with domain requirements

---

## Effort Estimates

### By Phase

| Phase                       | Time Estimate   | Depends On           |
| --------------------------- | --------------- | -------------------- |
| **Phase 1** (H-10, C-8)     | 2-4 hours       | No decisions needed  |
| **Phase 2** (C-3, C-7, H-7) | 4-8 hours       | 3 decisions required |
| **Phase 3** (Testing, Docs) | 4-6 hours       | Phase 1 & 2 complete |
| **Total**                   | **10-18 hours** | **(1.5-2 days)**     |

### By Decision Impact

| Decision | Option A   | Option B  | Option C      | Option D |
| -------- | ---------- | --------- | ------------- | -------- |
| **C-3**  | 4-6 hours  | 2-3 hours | 1 hour + wait | 30 min   |
| **C-7**  | 2-3 hours  | 2 hours   | 30 min        | N/A      |
| **H-7**  | 8-12 hours | 2-3 hours | 2 hours       | N/A      |

---

## Risk Assessment After Remediation

### Current Risk Level

**MEDIUM-LOW** - 19 of 29 findings remediated (66% complete)

### After Phase 1 (H-10, C-8)

**MEDIUM-LOW** - 21 of 29 findings remediated (72% complete)

- 3 Critical remaining (C-3, C-7, C-8 framework complete)
- 1 High remaining (H-7)

### After Phase 2 (All decisions implemented)

**LOW** - 24 of 29 findings remediated (83% complete)

- 0 Critical findings remaining
- 0 High findings remaining
- 3 Medium findings remaining (M-6, M-7, M-9)
- 2 Accepted risks (M-1, M-2)

### Production Readiness

- **Current:** 🟡 CAUTION (3 Critical, 2 High)
- **After Phase 1:** 🟡 CAUTION (3 Critical, 1 High)
- **After Phase 2:** 🟢 ACCEPTABLE (0 Critical, 0 High)

---

## Next Steps

1. **Review this addendum** and provide decisions for the 3 ambiguous areas
2. **Approve Phase 1 implementation** (H-10, C-8 - no decisions needed)
3. **Schedule implementation time** (10-18 hours across 1.5-2 days)
4. **Plan testing cycles** for each phase
5. **Prepare for final security audit** after all phases complete

---

## References

### Research Sources

- OWASP Top 10 2025
- CIS Docker Benchmark 5.x
- NIST SP 800-81-2 (Secure DNS)
- SLSA Framework v1.0
- GitHub Security Best Practices (2025)
- CWE-494, CWE-522, CWE-214
- Rustup installation patterns
- Homebrew security model
- npm/apt/Docker retry strategies

### Related Documents

- [SECURITY_AUDIT_REPORT.md](./SECURITY_AUDIT_REPORT.md) - Original audit findings
- [docs/SECURITY.md](../SECURITY.md) - Security guidelines
- [docs/EXTENSION_AUTHORING.md](../EXTENSION_AUTHORING.md) - Extension development guide
- [docs/DEPLOYMENT.md](../DEPLOYMENT.md) - Deployment procedures

---

**Document Status:** Awaiting decisions on C-3, C-7, and H-7 implementation approaches
**Last Updated:** December 17, 2025
**Next Review:** After decision approval and Phase 1 completion
