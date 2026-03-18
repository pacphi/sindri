# Sindri Changelog

This project maintains separate changelogs for each major version to improve clarity and reduce merge conflicts during concurrent development.

---

## 📖 Version-Specific Changelogs

Choose the changelog for your version:

### Current & Supported Versions

- **[v3 Changelog](v3/CHANGELOG.md)** - Current stable version (Rust-based CLI)
  - **Status**: Active development
  - **Architecture**: Rust binary with extension system
  - **Latest**: See [releases](https://github.com/pacphi/sindri/releases?q=v3)

- **[v2 Changelog](v2/CHANGELOG.md)** - Previous stable version (Bash/Docker)
  - **Status**: Maintenance mode
  - **Architecture**: Bash scripts with Docker containers
  - **Latest**: See [releases](https://github.com/pacphi/sindri/releases?q=v2)

### Archived Versions

- **[v1 Changelog](v1/CHANGELOG.md)** - Legacy version (archived)
  - **Status**: No longer supported
  - **End of Life**: January 2026
  - **Migration Guide**: [v1 → v2 Migration](RELEASE_NOTES.v2.md)

---

## 🔄 Migration & Release Guides

Upgrading between major versions? These guides will help:

- **[v3.1.0 Release Notes](RELEASE_NOTES.v3.1.0.md)** - Sindri 3.1.0 (March 2026)
  - Multi-distro support, collision handling, service framework, 8 new extensions, 24 extension upgrades
- **[v2 → v3 Migration Guide](RELEASE_NOTES.v3.md)** - Upgrading from v2 (Bash/Docker) to v3 (Rust)
  - Breaking changes, code examples, step-by-step migration
- **[v1 → v2 Migration Guide](RELEASE_NOTES.v2.md)** - Upgrading from v1 to v2
  - Extension system changes, capability-based architecture, removed extensions

---

## 🚀 Quick Links

- **[Latest Release](https://github.com/pacphi/sindri/releases/latest)** - Download the newest version
- **[All Releases](https://github.com/pacphi/sindri/releases)** - Browse release history
- **[Contributing Guide](docs/CONTRIBUTING.md)** - How to contribute to Sindri
- **[Security Policy](docs/SECURITY.md)** - Report security vulnerabilities

---

## 📋 Changelog Format

All changelogs follow the [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format and adhere to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

### Changelog Sections

- **Added** - New functionality and capabilities
- **Fixed** - Resolved issues and error corrections
- **Documentation** - Documentation improvements
- **Dependencies** - Dependency updates and changes
- **Performance** - Performance optimizations
- **Changed** - Code restructuring and behavior changes
- **Tests** - Test additions and improvements
- **Maintenance** - Chores, CI/CD, and infrastructure updates

---

## 🤔 Need Help?

- **Questions**: [GitHub Discussions](https://github.com/pacphi/sindri/discussions)
- **Bug Reports**: [GitHub Issues](https://github.com/pacphi/sindri/issues)
- **Feature Requests**: [GitHub Discussions](https://github.com/pacphi/sindri/discussions/categories/ideas)
