# Sindri {{TO_VERSION}} Release Notes

**Release Date:** [TO BE DETERMINED]
**Previous Version:** {{FROM_VERSION}}
**Upgrade Path:** {{FROM_VERSION}} → {{TO_VERSION}}

---

## 🚨 Breaking Changes

{{BREAKING_CHANGES}}

---

## ✨ What's New

### New Features

[TO BE COMPLETED - Add major new features with descriptions and examples]

**Highlights:**

- [Feature 1]: Description and benefit
- [Feature 2]: Description and benefit
- [Feature 3]: Description and benefit

### Improvements

[TO BE COMPLETED - List significant improvements and enhancements]

- **Performance**: [Describe performance improvements]
- **Security**: [Describe security enhancements]
- **Usability**: [Describe UX/DX improvements]

### Bug Fixes

[TO BE COMPLETED - List major bug fixes]

See the [CHANGELOG](CHANGELOG.md) for a complete list of changes.

---

## 📋 Migration Steps

{{MIGRATION_STEPS}}

---

## 🔧 Troubleshooting

### Common Migration Issues

#### Issue 1: [Common Problem Title]

**Symptom:**

```
[Error message or behavior]
```

**Cause:**
[Explanation of what causes this issue]

**Solution:**

```bash
# Steps to resolve
[command 1]
[command 2]
```

#### Issue 2: [Common Problem Title]

**Symptom:**

```
[Error message or behavior]
```

**Cause:**
[Explanation of what causes this issue]

**Solution:**

```bash
# Steps to resolve
[command 1]
[command 2]
```

### Getting Help

If you encounter issues not covered here:

1. **Search existing issues**: https://github.com/${GITHUB_REPOSITORY}/issues
2. **Check discussions**: https://github.com/${GITHUB_REPOSITORY}/discussions
3. **Report a bug**: https://github.com/${GITHUB_REPOSITORY}/issues/new
4. **Ask the community**: https://github.com/${GITHUB_REPOSITORY}/discussions/new

---

## 📦 Installation

### Docker (Recommended)

```bash
# Pull the latest version
docker pull ghcr.io/${GITHUB_REPOSITORY}:{{TO_VERSION}}

# Run with recommended settings
docker run -d --name sindri \
  -e SINDRI_PROFILE=minimal \
  -v sindri_home:/alt/home/developer \
  ghcr.io/${GITHUB_REPOSITORY}:{{TO_VERSION}}
```

### CLI Binary

Download platform-specific binaries from the [releases page](https://github.com/${GITHUB_REPOSITORY}/releases/tag/{{TO_VERSION}}).

**Linux (x86_64):**

```bash
wget https://github.com/${GITHUB_REPOSITORY}/releases/download/{{TO_VERSION}}/sindri-{{TO_VERSION}}-linux-x86_64.tar.gz
tar -xzf sindri-{{TO_VERSION}}-linux-x86_64.tar.gz
sudo mv sindri /usr/local/bin/
```

**macOS (Apple Silicon):**

```bash
wget https://github.com/${GITHUB_REPOSITORY}/releases/download/{{TO_VERSION}}/sindri-{{TO_VERSION}}-macos-aarch64.tar.gz
tar -xzf sindri-{{TO_VERSION}}-macos-aarch64.tar.gz
sudo mv sindri /usr/local/bin/
```

**Windows:**

```powershell
# Download from releases page and extract
# Add to PATH manually
```

---

## 🔒 Security & Verification

### Image Signature Verification

This release is signed with [Sigstore Cosign](https://docs.sigstore.dev/):

```bash
# Verify Docker image signature
cosign verify ghcr.io/${GITHUB_REPOSITORY}:{{TO_VERSION}} \
  --certificate-identity-regexp='https://github.com/${GITHUB_REPOSITORY}' \
  --certificate-oidc-issuer='https://token.actions.githubusercontent.com'
```

### Software Bill of Materials (SBOM)

Download the SBOM for supply chain analysis:

```bash
# Download SBOM from image
cosign download sbom ghcr.io/${GITHUB_REPOSITORY}:{{TO_VERSION}} > sbom.spdx.json

# Or download from release assets
wget https://github.com/${GITHUB_REPOSITORY}/releases/download/{{TO_VERSION}}/sbom.spdx.json
```

### Vulnerability Scanning

Images are scanned with [Trivy](https://trivy.dev/) during CI. View scan results in the [GitHub Security tab](https://github.com/${GITHUB_REPOSITORY}/security).

---

## 📊 Deprecations & Removals

### Deprecated Features

[TO BE COMPLETED - List features deprecated in this release]

**Feature Name** (deprecated in {{TO_VERSION}}, removal planned for [VERSION])

- **Reason**: [Why it's being deprecated]
- **Alternative**: [What to use instead]
- **Migration Guide**: [Link or instructions]

### Removed Features

[TO BE COMPLETED - List features removed in this release]

**Feature Name** (removed in {{TO_VERSION}})

- **Reason**: [Why it was removed]
- **Alternative**: [What to use instead]
- **Last supported version**: {{FROM_VERSION}}

---

## 🎯 Who Should Upgrade?

### High Priority

You should upgrade **immediately** if:

- [Condition 1 - e.g., critical security fix affects you]
- [Condition 2 - e.g., major bug affecting your workflow is fixed]
- [Condition 3 - e.g., you need new feature X]

### Standard Priority

Consider upgrading **within 1-2 weeks** if:

- You want to use new features
- You want performance improvements
- Your current version is more than 2 releases old

### Low Priority (Can Wait)

You can delay upgrading if:

- Current version works well for your use case
- You're in a critical project phase
- You need to coordinate upgrades with team

**Note**: We recommend staying within 2 major versions of the latest release for security and support reasons.

---

## 🙏 Acknowledgments

[TO BE COMPLETED - Thank contributors, testers, and community members]

Special thanks to:

- [Contributor names]
- Community members who reported issues and tested pre-releases
- Organizations using Sindri in production

---

## 📝 Changelog

For a complete list of changes, see:

- [Full Changelog](https://github.com/${GITHUB_REPOSITORY}/compare/{{FROM_VERSION}}...{{TO_VERSION}})
- [Version-specific CHANGELOG](CHANGELOG.md)

---

## 🗺️ Roadmap

### Upcoming in {{NEXT_VERSION}}

[TO BE COMPLETED - Preview upcoming features]

- [Planned feature 1]
- [Planned feature 2]
- [Planned improvement 1]

See the [project roadmap](https://github.com/${GITHUB_REPOSITORY}/projects) for more details.

---

**Questions?** Open a [discussion](https://github.com/${GITHUB_REPOSITORY}/discussions) or [issue](https://github.com/${GITHUB_REPOSITORY}/issues).
