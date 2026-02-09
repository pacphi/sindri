# Extension System Guides

This directory contains comprehensive guides for working with the Sindri V3 extension system.

## Guides

### Extension Development

- **[AUTHORING.md](AUTHORING.md)** - Complete guide to creating V3 extensions
  - Extension structure and schema
  - Installation methods (mise, apt, binary, npm, script, hybrid)
  - Configuration templates and lifecycle hooks
  - Testing and publishing extensions

### Advanced Topics

- **[CONDITIONAL_TEMPLATES_MIGRATION.md](CONDITIONAL_TEMPLATES_MIGRATION.md)** - Environment-based template selection
  - Using conditions in template configuration
  - Migration patterns from static templates
  - Examples and best practices

- **[SOURCING_MODES.md](SOURCING_MODES.md)** - Extension loading mechanisms
  - Build-from-source vs release-based deployment
  - Extension discovery and versioning
  - Volume mount considerations

### Support Files

- **[SUPPORT_FILE_INTEGRATION.md](SUPPORT_FILE_INTEGRATION.md)** - Complete implementation overview
  - How support files are distributed and loaded
  - Docker build and runtime integration
  - Fallback mechanisms

- **[SUPPORT_FILE_VERSION_HANDLING.md](SUPPORT_FILE_VERSION_HANDLING.md)** - Version matching system
  - Semantic versioning for support files
  - Version compatibility checks
  - GitHub release integration

- **[SUPPORT_FILES_CLI_COMMAND.md](SUPPORT_FILES_CLI_COMMAND.md)** - CLI command reference
  - `sindri support-files` command usage
  - Downloading and updating support files
  - Troubleshooting and diagnostics

## Quick Links

- [Extension Registry](../../EXTENSIONS.md) - List of all available extensions
- [Extension Schema](../../SCHEMA.md) - YAML schema reference
- [Individual Extension Docs](../../EXTENSIONS.md) - Use `sindri extension docs <name>` to generate docs on-demand

## Related Documentation

- [Architecture Decision Records](../../architecture/adr/) - Design rationale
- [Configuration Guide](../../CONFIGURATION.md) - Sindri configuration
- [Troubleshooting](../../TROUBLESHOOTING.md) - Common issues and solutions
