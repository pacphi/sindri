# Security Policy

## Supported Versions

We actively maintain and provide security updates for the following versions:

| Version | Status                | Support Level                |
| ------- | --------------------- | ---------------------------- |
| v3.x    | ✅ Active Development | Full security support        |
| v2.x    | 🔧 Maintenance Mode   | Critical security fixes only |
| v1.x    | ❌ End of Life        | No security updates          |

## Reporting a Vulnerability

We take security vulnerabilities seriously and appreciate your efforts to responsibly disclose your findings.

### How to Report

**DO NOT** create a public GitHub issue for security vulnerabilities.

Instead, please report security issues privately using one of these methods:

1. **GitHub Security Advisories** (Recommended)
   - Navigate to the [Security tab](https://github.com/pacphi/sindri/security/advisories)
   - Click "Report a vulnerability"
   - Fill out the advisory form with details

2. **Email**
   - Send details to the project maintainers via GitHub discussions
   - Mark the discussion as private/security-related

### What to Include

When reporting a vulnerability, please provide:

- **Description** - Clear explanation of the vulnerability
- **Impact** - What an attacker could achieve
- **Affected Versions** - Which versions are impacted
- **Steps to Reproduce** - Detailed reproduction steps
- **Proof of Concept** - Code or commands demonstrating the issue (if applicable)
- **Suggested Fix** - If you have ideas for mitigation

### Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Fix Timeline**: Varies by severity
  - Critical: 7-14 days
  - High: 14-30 days
  - Medium: 30-60 days
  - Low: Next scheduled release

### Security Update Process

1. **Acknowledgment** - We confirm receipt of your report
2. **Investigation** - We verify and assess the vulnerability
3. **Fix Development** - We develop and test a fix
4. **Coordinated Disclosure** - We coordinate release timing with you
5. **Release** - We publish the fix and security advisory
6. **Credit** - We acknowledge your contribution (unless you prefer to remain anonymous)

## Security Best Practices

For deployment and usage security guidelines, see:

- **[v2 Security Best Practices](../v2/docs/SECURITY.md)** - Docker/Bash platform security
- **[v2 Security Audit Report](../v2/docs/security/SECURITY_AUDIT_REPORT.md)** - Comprehensive security audit findings
- **[v2 Security Audit Addendum](../v2/docs/security/SECURITY_AUDIT_ADDENDUM.md)** - Additional security recommendations

## Security Features

### v3 (Rust Platform)

- Memory-safe Rust implementation
- Dependency vulnerability scanning via `cargo audit`
- Secure extension installation with signature verification
- Sandboxed extension execution
- Encrypted secrets management

### v2 (Bash/Docker Platform)

- Container-based isolation
- SSH key authentication (no passwords)
- Provider-specific secrets management
- SBOM (Software Bill of Materials) tracking
- Network isolation via provider policies

## Security Advisories

Published security advisories can be found at:
https://github.com/pacphi/sindri/security/advisories

## Bug Bounty

We currently do not offer a bug bounty program, but we deeply appreciate security research contributions and will acknowledge researchers in our security advisories and release notes.

## Questions?

For general security questions (not vulnerability reports), please:

- Open a [GitHub Discussion](https://github.com/pacphi/sindri/discussions)
- Tag it with the `security` label
