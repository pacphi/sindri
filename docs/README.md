# Sindri Documentation

Welcome to the Sindri documentation. Sindri is a development environment management platform with two major versions: V2 (Bash/Docker) and V3 (Rust CLI).

## Choose Your Version

### Sindri V2 (Bash/Docker)

The original Sindri implementation using Bash scripts and Docker. This version is mature and stable, ideal for production environments.

**Use V2 if you:**

- Need a stable, battle-tested solution
- Are using Docker-based workflows
- Have existing V2 extensions or configurations

**V2 Documentation:**

- [Extensions Catalog](../v2/docs/EXTENSIONS.md) - Available extensions and profiles
- [Extension Authoring](../v2/docs/EXTENSION_AUTHORING.md) - Create custom V2 extensions
- [V2 Architecture](../v2/docs/ARCHITECTURE.md) - System design and components
- [V2 Configuration](../v2/docs/CONFIGURATION.md) - sindri.yaml configuration reference

### Sindri V3 (Rust CLI)

The next-generation Sindri implementation built in Rust. This version offers improved performance, better error handling, and native binary distribution.

**Use V3 if you:**

- Want faster execution and lower overhead
- Need native binary installation (no Docker required for the CLI)
- Are starting a new project
- Want to use the latest features

**V3 Documentation:**

- [Getting Started](../v3/docs/GETTING_STARTED.md) - Quick start guide
- [CLI Reference](../v3/docs/CLI.md) - Command-line interface documentation
- [Configuration](../v3/docs/CONFIGURATION.md) - V3 configuration reference
- [Schema Reference](../v3/docs/SCHEMA.md) - Extension schema documentation

## Migration Resources

If you are transitioning between versions or evaluating which version to use, these guides will help:

### → [Migration Hub](migration/)

Comprehensive resources for comparing versions and migrating between them:

- **[Comparison Guide](migration/COMPARISON_GUIDE.md)** - Feature and architectural comparison between versions
- **[Migration Guide](migration/MIGRATION_GUIDE.md)** - Step-by-step migration instructions

The migration hub includes detailed extension comparison (77 V2 vs 44 V3 extensions), persona-based analysis, performance benchmarks, and practical migration workflows.

## Additional Resources

### FAQ and Troubleshooting

- [FAQ](https://sindri-faq.fly.dev) - Frequently asked questions (V2-focused)
- [V2 Troubleshooting](../v2/docs/TROUBLESHOOTING.md) - V2 common issues and solutions
- [V3 Troubleshooting](../v3/docs/TROUBLESHOOTING.md) - V3 common issues and solutions

### IDE Integration

- [IDE Setup](ides/) - IDE-specific setup guides

## Version Summary

| Feature           | V2 (Bash/Docker) | V3 (Rust CLI)          |
| ----------------- | ---------------- | ---------------------- |
| Implementation    | Bash scripts     | Rust binary            |
| Distribution      | Docker image     | Native binary + Docker |
| Extension Format  | YAML             | YAML (compatible)      |
| Performance       | Good             | Excellent              |
| Maturity          | Stable           | Active development     |
| Docker Dependency | Required         | Optional               |
