# Sindri Version Migration Resources

This directory holds umbrella resources for **comparing Sindri versions and migrating between them**. Per-version source and reference documentation lives on each version branch — links below point at the right branch.

---

## Quick decision: which version should I use?

### Use v2 if you:

- Need VisionFlow extensions (`vf-*` prefix)
- Require proven production stability
- Have existing v2 configurations and deployment workflows
- Are risk-averse and not pressed to upgrade

### Use v3 if you:

- Want a 10–50× faster CLI vs. the bash implementation
- Need native Windows support
- Want built-in CVE remediation and enhanced security posture
- Need self-learning capabilities (SONA)
- Are starting a new project
- Want multi-provider LLM load balancing

### Watch v4 if you:

- Are designing for a future BOM/manifest-driven, OCI-only registry world
- Want to influence or contribute to the next architecture before it stabilizes
- Are comfortable on a pre-release branch and reading [ADRs](https://github.com/pacphi/sindri/tree/v4/v4/docs/ADRs) before code

### Skip v1:

- v1 is **archived in a separate repository**: [pacphi/sindri-legacy](https://github.com/pacphi/sindri-legacy). Source and tagged releases (`v1.0.0-alpha.1` through `v1.0.0-rc.5`) live there. Plan an upgrade path to v2 or v3.

---

## Documentation in this hub

### 📖 [Migration Guide](MIGRATION_GUIDE.md)

**For:** teams actively migrating from v2 to v3.

Step-by-step practical instructions:

- Pre-migration checklist and preparation
- Breaking changes and compatibility issues
- Command mapping (v2 → v3)
- Phase-by-phase migration timeline (six phases)
- Rollback procedures
- Post-migration validation
- Common issues and solutions
- CI/CD pipeline considerations

**Read this when:** planning a v2 → v3 migration, executing one, troubleshooting it, or designing a rollback.

---

## Migration workflow

```text
┌─────────────────────────────────────────────────────────────┐
│  1. Evaluate                                                │
│     Read: Comparison Guide                                  │
│     Goal: Decide if migration is right for your team        │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  2. Prepare                                                 │
│     Read: Migration Guide — Pre-Migration Checklist         │
│     Goal: Inventory current state, create backups           │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  3. Test (Staging)                                          │
│     Read: Migration Guide — Step-by-Step Migration          │
│     Goal: Validate migration in non-production environment  │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  4. Execute (Production)                                    │
│     Read: Migration Guide — Production Migration            │
│     Goal: Deploy v3 to production with monitoring           │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  5. Validate                                                │
│     Read: Migration Guide — Post-Migration Validation       │
│     Goal: Verify all systems operational                    │
└─────────────────────────────────────────────────────────────┘
```

---

## Per-version reference docs

These live alongside the code on each version branch — not on `main`.

| Version | Source tree | Top-level docs |
| --- | --- | --- |
| v1 | [`pacphi/sindri-legacy`](https://github.com/pacphi/sindri-legacy) (separate repo) | [Legacy releases](https://github.com/pacphi/sindri-legacy/releases) |
| v2 | [`v2` branch](https://github.com/pacphi/sindri/tree/v2/v2) | [v2 docs](https://github.com/pacphi/sindri/tree/v2/v2/docs) |
| v3 | [`v3` branch](https://github.com/pacphi/sindri/tree/v3/v3) | [v3 docs](https://github.com/pacphi/sindri/tree/v3/v3/docs) |
| v4 | [`v4` branch](https://github.com/pacphi/sindri/tree/v4/v4) | [v4 ADRs / DDDs](https://github.com/pacphi/sindri/tree/v4/v4/docs) |

### Getting help

- **GitHub Issues**: <https://github.com/pacphi/sindri/issues>
- **GitHub Discussions**: <https://github.com/pacphi/sindri/discussions>
- **FAQ** (interactive, hosted): <https://sindri-faq.fly.dev>

---

## Version support timeline

| Version | Status | Recommendation |
| --- | --- | --- |
| **v1** | Archived in [pacphi/sindri-legacy](https://github.com/pacphi/sindri-legacy) | Plan an upgrade |
| **v2** | Maintenance | Bug/security fixes; no new features |
| **v3** | Active development | Recommended for new projects |
| **v4** | Pre-release | New architecture (ADRs/DDDs published); not yet GA |

A v3 → v4 migration guide will land here when v4 reaches a stable surface.

---

## Quick links

- [← Back to Docs Home](../README.md)
- [IDE Integration](../ides/)
- [Main Repository README](../../README.md)
