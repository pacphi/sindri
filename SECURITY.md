# Security Policy

## Supported Versions

Each major version is a line with its own long-lived branches. Lines are maintained independently:
a fix is never merged or cherry-picked from one line to another. When an advisory affects several
lines, each line gets its own fix, made against that line's own dependencies.

| Line | Development branch | Maintenance branch | Latest release | Status         | Receives                                                                   |
| ---- | ------------------ | ------------------ | -------------- | -------------- | -------------------------------------------------------------------------- |
| v4   | `v4`               | none               | none           | 🚧 Pre-release | Security and dependency fixes on the branch; no releases before 4.0.0      |
| v3   | `v3`               | `v3.1.x`           | v3.1.3         | ✅ Supported   | Security and in-major dependency fixes, toolchain patches; v3.1.x releases |
| v2   | `v2`               | `v2.5.x`           | v2.5.1         | 🔧 Maintenance | Security and in-major dependency fixes, toolchain patches; v2.5.x releases |
| v1   | `v1` (locked)      | none               | v1.13.0        | ❌ End of Life | No updates                                                                 |

### How the branches are used

- **Maintenance branches** (`v3.1.x`, `v2.5.x`) start at the latest release of their line and carry
  only fixes on top of it. Patch releases (`v3.1.4`, `v2.5.2`, ...) are tagged from them, so a
  patch release never ships unreleased features.
- **Development branches** (`v2`, `v3`, `v4`) carry each line's ongoing work, which reaches users
  in that line's next minor release (or, for v4, its first release).
- **No crossing streams.** Within a line, dependencies stay within their current major version
  (for 0.x crates and packages, within their current minor). Toolchains get patch updates only:
  Rust stays on the line's pinned minor (1.94.x for `v3.1.x`), and base images stay on their
  release. Moving a line to a new major of anything is a deliberate, separate change.
- A transitive 0.x bump that is forced by an in-major security fix is allowed, and must be listed
  in the PR.
- **Dependency updates** come from Renovate, configured once in `renovate.json` on `main`, which
  targets every supported branch with these rules. Commits are scoped to the branch they land
  on, for example `deps(v3.1.x): ...` or `deps(v4): ...`.
- **Vulnerability scanning.** Each supported branch's CI runs a dependency audit, and a
  scheduled audit on `main` checks every supported branch (see "Dependency auditing" below).
  GitHub's own Dependabot alerts only cover `main`, which holds no product code.

### Dependency auditing

Every supported branch (`v2`, `v2.5.x`, `v3`, `v3.1.x`, `v4`) runs the same audit:

- **Job name:** `Dependency Audit`, on every push and pull request for the branch (and on manual
  dispatch). It is the one required check to configure; the branch's "Required Checks" job fails
  when it fails.
- **Rust lines** (`v3`, `v3.1.x`, `v4`): `cargo audit --deny warnings` (cargo-audit 0.22.2) in
  `v3/` or `v4/`. Exceptions live only in the line's `.cargo/audit.toml`, under
  `[advisories] ignore`.
- **npm** (`v2`, `v2.5.x`): `npm audit --audit-level=high` on every committed `package-lock.json`
  under `v2/`, run by `.github/scripts/v2/npm-audit.sh`. Exceptions live in `v2/npm-audit-ignore`.
  The script fails when the registry returns no report, and warns about exceptions that no
  longer match anything.
- **Exceptions** are only for advisories with no fix inside the current major. Each names the
  advisory and its link, the dependency path, why it can't be fixed in-major, its reachability,
  and the condition for removing it.
- **Where it runs:** for `v2`, `v3` and `v4`, in `main`'s `ci-v2.yml`, `ci-v3.yml` and
  `ci-v4.yml`; for `v2.5.x` and `v3.1.x`, in the branch's own `ci-v2.yml` or `ci-v3.yml`.
- **Schedule:** `dependency-audit.yml` on `main` audits every supported branch each Monday at
  06:00 UTC. Findings open or update one issue per branch, titled `Dependency Audit: <branch>` and
  labelled `security`, which is closed when the audit is clean.

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

- **[v2 Security Best Practices](https://github.com/pacphi/sindri/blob/v2/v2/docs/SECURITY.md)** - Docker/Bash platform security
- **[v2 Security Audit Report](https://github.com/pacphi/sindri/blob/v2/v2/docs/security/SECURITY_AUDIT_REPORT.md)** - Comprehensive security audit findings
- **[v2 Security Audit Addendum](https://github.com/pacphi/sindri/blob/v2/v2/docs/security/SECURITY_AUDIT_ADDENDUM.md)** - Additional security recommendations

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
