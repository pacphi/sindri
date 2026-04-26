# Sindri Documentation

Welcome. **This `main` branch is the umbrella** — it carries no product source. Every version of Sindri lives on its own branch with version-specific docs alongside the code.

Pick the version you're using, or read the [Migration Hub](migration/) if you're choosing between them.

## Versions at a glance

| Branch | Status | Stack | Start here |
| --- | --- | --- | --- |
| [`v1`](https://github.com/pacphi/sindri/tree/v1) | End-of-life — security backports only | Legacy bash | [v1 README](https://github.com/pacphi/sindri/blob/v1/README.md) |
| [`v2`](https://github.com/pacphi/sindri/tree/v2) | Maintenance | Bash + Docker | [v2 Quickstart](https://github.com/pacphi/sindri/blob/v2/v2/docs/QUICKSTART.md) |
| [`v3`](https://github.com/pacphi/sindri/tree/v3) | **Active** — recommended for new projects | Rust workspace + npm wrapper | [v3 Getting Started](https://github.com/pacphi/sindri/blob/v3/v3/docs/GETTING_STARTED.md) |
| [`v4`](https://github.com/pacphi/sindri/tree/v4) | Pre-release | Rust, redesigned | [v4 ADRs](https://github.com/pacphi/sindri/tree/v4/v4/docs/ADRs) |

## Per-version documentation

### Sindri v2 (Bash + Docker, maintenance)

The original implementation. Stable, battle-tested, Docker-centric.

- [Quickstart](https://github.com/pacphi/sindri/blob/v2/v2/docs/QUICKSTART.md)
- [Architecture](https://github.com/pacphi/sindri/blob/v2/v2/docs/ARCHITECTURE.md)
- [Configuration](https://github.com/pacphi/sindri/blob/v2/v2/docs/CONFIGURATION.md)
- [CLI Reference](https://github.com/pacphi/sindri/blob/v2/v2/docs/CLI.md)
- [Extensions Catalog](https://github.com/pacphi/sindri/blob/v2/v2/docs/EXTENSIONS.md) · [Extension Authoring](https://github.com/pacphi/sindri/blob/v2/v2/docs/EXTENSION_AUTHORING.md)
- [Schema Reference](https://github.com/pacphi/sindri/blob/v2/v2/docs/SCHEMA.md)
- [Secrets Management](https://github.com/pacphi/sindri/blob/v2/v2/docs/SECRETS_MANAGEMENT.md)
- [Troubleshooting](https://github.com/pacphi/sindri/blob/v2/v2/docs/TROUBLESHOOTING.md)

### Sindri v3 (Rust CLI, active development)

Rust workspace shipped as a native binary plus an npm wrapper. Recommended for new projects.

- [Getting Started](https://github.com/pacphi/sindri/blob/v3/v3/docs/GETTING_STARTED.md) · [Quickstart](https://github.com/pacphi/sindri/blob/v3/v3/docs/QUICKSTART.md)
- [Architecture](https://github.com/pacphi/sindri/blob/v3/v3/docs/ARCHITECTURE.md) · [Architecture ADRs](https://github.com/pacphi/sindri/tree/v3/v3/docs/architecture/adr)
- [Configuration](https://github.com/pacphi/sindri/blob/v3/v3/docs/CONFIGURATION.md) · [Runtime Configuration](https://github.com/pacphi/sindri/blob/v3/v3/docs/RUNTIME_CONFIGURATION.md)
- [CLI Reference](https://github.com/pacphi/sindri/blob/v3/v3/docs/CLI.md) · [CLI ↔ Extension Compatibility](https://github.com/pacphi/sindri/blob/v3/v3/docs/CLI_EXTENSION_COMPATIBILITY_GUIDE.md)
- [Extensions](https://github.com/pacphi/sindri/blob/v3/v3/docs/EXTENSIONS.md) · [Maintainer Guide](https://github.com/pacphi/sindri/blob/v3/v3/docs/MAINTAINER_GUIDE.md)
- [Schema Reference](https://github.com/pacphi/sindri/blob/v3/v3/docs/SCHEMA.md)
- [Secrets Management](https://github.com/pacphi/sindri/blob/v3/v3/docs/SECRETS_MANAGEMENT.md)
- [Doctor](https://github.com/pacphi/sindri/blob/v3/v3/docs/DOCTOR.md) · [Troubleshooting](https://github.com/pacphi/sindri/blob/v3/v3/docs/TROUBLESHOOTING.md) · [v3 FAQ](https://github.com/pacphi/sindri/blob/v3/v3/docs/FAQ.md)

### Sindri v4 (Rust, redesigned, pre-release)

A from-scratch redesign around BOM/manifest as source of truth, OCI-only registries, and a target plugin model. Read the design first.

- [Architecture Decision Records](https://github.com/pacphi/sindri/tree/v4/v4/docs/ADRs)
- [Domain Designs (DDDs)](https://github.com/pacphi/sindri/tree/v4/v4/docs/DDDs)
- [Plan & research notes](https://github.com/pacphi/sindri/tree/v4/v4/docs)

## Cross-cutting guides (this branch)

- [Migration Hub](migration/) — comparison + step-by-step migration between versions
- [IDE Integration](ides/) — VS Code, IntelliJ, Zed, Eclipse, Warp setup
- [FAQ](https://sindri-faq.fly.dev) — interactive, searchable, hosted

## Quick decision

- **New project, want the modern CLI?** → v3.
- **Existing v2 deployment, no time to migrate?** → stay on v2 (maintenance), read the [Migration Guide](migration/MIGRATION_GUIDE.md) when you're ready.
- **Want to follow / contribute to the next architecture?** → v4 (read the ADRs first).
- **Stuck on v1?** → upgrade. v1 is end-of-life; see [v1 README](https://github.com/pacphi/sindri/blob/v1/README.md) for context.

## Contributing

- Open PRs against the relevant `v*` branch — never `main`.
- See [CONTRIBUTING.md](../CONTRIBUTING.md) and [SECURITY.md](../SECURITY.md).
