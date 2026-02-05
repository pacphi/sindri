# Sindri Version Migration Resources

This directory contains comprehensive resources for comparing Sindri versions and migrating between them. Whether you're evaluating which version to use or planning an active migration, you'll find the guidance you need here.

---

## Quick Decision: Which version should I use?

### Use V2 if you:

- ✅ Need VisionFlow extensions (vf-\* prefixed extensions)
- ✅ Require proven production stability
- ✅ Are risk-averse with deployments
- ✅ Have existing V2 configurations and workflows

### Use V3 if you:

- ✅ Want 10-50x faster CLI performance
- ✅ Need native Windows support
- ✅ Want built-in CVE remediation and enhanced security
- ✅ Need self-learning capabilities (SONA)
- ✅ Are starting a new project
- ✅ Want multi-provider LLM load balancing

---

## Documentation

### 📊 [Comparison Guide](COMPARISON_GUIDE.md)

**For: Decision Makers, Architects, and Evaluators**

Comprehensive feature and architectural comparison between Sindri versions. Use this to make informed decisions about which version best fits your needs.

**Contents:**

- Executive summary and at-a-glance metrics
- Feature matrices across 6 categories
- Detailed extension comparison (all 77 V2 vs 44 V3 extensions)
- Persona-based analysis (Developers, DevOps, QA, Security)
- Performance benchmarks
- Architecture and technology stack comparison
- User stories by role

**When to read:**

- Evaluating Sindri for the first time
- Deciding between V2 and V3
- Understanding architectural differences
- Assessing extension availability for your use case

---

### 📖 [Migration Guide](MIGRATION_GUIDE.md)

**For: Teams Actively Migrating from V2 to V3**

Step-by-step practical instructions for migrating from Sindri V2 to V3. Use this when you're ready to execute a version transition.

**Contents:**

- Pre-migration checklist and preparation steps
- Breaking changes and compatibility issues
- Command mapping (V2 → V3)
- Phase-by-phase migration timeline (6 phases)
- Rollback procedures
- Post-migration validation
- Common issues and solutions
- CI/CD pipeline considerations

**When to read:**

- Planning a V2 → V3 migration
- Actively executing a migration
- Troubleshooting migration issues
- Need rollback procedures

---

## Migration Workflow

```
┌─────────────────────────────────────────────────────────────┐
│  1. Evaluate                                                │
│     Read: Comparison Guide                                  │
│     Goal: Decide if migration is right for your team        │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  2. Prepare                                                 │
│     Read: Migration Guide - Pre-Migration Checklist         │
│     Goal: Inventory current state, create backups           │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  3. Test (Staging)                                          │
│     Read: Migration Guide - Step-by-Step Migration          │
│     Goal: Validate migration in non-production environment  │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  4. Execute (Production)                                    │
│     Read: Migration Guide - Production Migration            │
│     Goal: Deploy V3 to production with monitoring           │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│  5. Validate                                                │
│     Read: Migration Guide - Post-Migration Validation       │
│     Goal: Verify all systems operational                    │
└─────────────────────────────────────────────────────────────┘
```

---

## Key Resources

### Support Documentation

- **FAQ**: [https://sindri-faq.fly.dev](https://sindri-faq.fly.dev) (V2-focused)
- **V2 Docs**: `v2/docs/`
- **V3 Docs**: `v3/docs/`
- **IDE Integration**: `docs/ides/`

### Getting Help

- **GitHub Issues**: [https://github.com/pacphi/sindri/issues](https://github.com/pacphi/sindri/issues)
- **GitHub Discussions**: [https://github.com/pacphi/sindri/discussions](https://github.com/pacphi/sindri/discussions)

---

## Version Support Timeline

| Version | Status             | Recommendation                        |
| ------- | ------------------ | ------------------------------------- |
| **V2**  | Maintenance mode   | Security fixes only                   |
| **V3**  | Active development | Recommended for new projects          |
| **V4**  | Future (TBD)       | When V4 is released, new guides added |

---

## Quick Links

- [← Back to Docs Home](../README.md)
- [V2 Documentation](../../v2/docs/)
- [V3 Documentation](../../v3/docs/)
- [Main Repository README](../../README.md)

---

_Last updated: 2026-02-05_
