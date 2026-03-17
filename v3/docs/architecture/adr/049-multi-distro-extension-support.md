# ADR-048: Multi-Distro Extension Support

## Status

Accepted

## Context

Sindri v3 now supports Ubuntu 24.04, Fedora 41, and openSUSE Leap 15.6 via the `DISTRO` build arg in Dockerfiles. However, the extension system was Ubuntu-only — install methods assumed `apt`, scripts used `apt-get` directly, and the registry had no concept of distro compatibility. Roughly 65% of extensions would fail on non-Ubuntu distros due to missing package managers or distro-specific package names.

Specific gaps:

- Extension install configs only supported `AptInstallConfig` — no `dnf` or `zypper` equivalents
- `InstallMethod` enum had no Dnf or Zypper variants
- The executor had no runtime distro detection — scripts could not branch on distro
- The registry and CLI had no distro filtering — users on Fedora saw extensions that would fail to install
- Extension metadata had no way to declare which distros an extension supports
- No shared shell library for distro-agnostic package operations

## Decision

### Schema 1.2: `distros` Field

Added a required `distros` field to extension metadata. For backward compatibility with schema 1.1 extensions, `distros` defaults to `[ubuntu]` when omitted. Extensions must explicitly declare every distro they support — there is no implicit "works everywhere" assumption.

```yaml
distros:
  - ubuntu
  - fedora
  - opensuse
```

### Parallel Install Configs

Added `DnfInstallConfig` and `ZypperInstallConfig` types parallel to the existing `AptInstallConfig`. Each config captures the distro-specific package names, repositories, and options:

```yaml
install:
  apt:
    packages: [build-essential]
  dnf:
    packages: [gcc, gcc-c++, make]
  zypper:
    packages: [gcc, gcc-c++, make]
```

### `InstallMethod` Enum Extension

Added `Dnf` and `Zypper` variants to the `InstallMethod` enum alongside the existing `Apt`, `Mise`, `Script`, and `Manual` variants.

### Auto-Dispatch

When the declared `method: apt` runs on Fedora, the executor checks for a `dnf:` config block and dispatches to it automatically. This means extensions with `method: apt` that also declare `dnf:` and `zypper:` configs work across distros without changing the method field. If the current distro's config is missing, installation fails with a clear error.

### Per-Distro Script Overrides

Extensions can provide distro-specific scripts via an `install.scripts` map:

```yaml
install:
  scripts:
    ubuntu: install-ubuntu.sh
    fedora: install-fedora.sh
    opensuse: install-opensuse.sh
    default: install.sh
```

The executor selects the most specific script, falling back to `default` if present.

### Runtime Distro Detection

The executor detects the current distro via two mechanisms (in order):

1. `SINDRI_DISTRO` environment variable (set at container build time)
2. `/etc/os-release` parsing (fallback for non-container environments)

Detection produces a `Distro` enum value (`Ubuntu`, `Fedora`, `OpenSuse`) used throughout the install pipeline.

### Environment Variable Injection

Two environment variables are injected into all script and hook executions:

- `SINDRI_DISTRO` — the detected distro identifier (`ubuntu`, `fedora`, `opensuse`)
- `SINDRI_PKG_MANAGER_LIB` — path to the `pkg-manager.sh` shell library

Scripts can source `$SINDRI_PKG_MANAGER_LIB` for distro-agnostic package operations or branch on `$SINDRI_DISTRO` directly.

### `pkg-manager.sh` Shell Library

A shared shell library at `docker/lib/pkg-manager.sh` provides distro-agnostic functions:

- `pkg_install <packages...>` — installs packages using the detected package manager
- `pkg_update` — updates the package index
- `pkg_is_installed <package>` — checks if a package is installed

Scripts that source this library work across all supported distros without explicit branching.

### Distro-Aware Registry Filtering

The registry and CLI filter extensions by the current distro. Users on Fedora only see extensions that declare `fedora` in their `distros` array. The `sindri extension list` command accepts `--distro` to override detection.

## Key Design Decisions

| Decision                | Choice                                          | Rationale                                                               |
| ----------------------- | ----------------------------------------------- | ----------------------------------------------------------------------- |
| Distro declaration      | Explicit `distros` array per extension          | No false positives; authors must verify each distro                     |
| Backward compat default | `[ubuntu]` when `distros` omitted               | Existing extensions continue working on Ubuntu without changes          |
| Auto-dispatch mechanism | Check for distro-specific config block          | Extensions keep `method: apt` but gain multi-distro via additional keys |
| Detection order         | `SINDRI_DISTRO` env var, then `/etc/os-release` | Fast path for containers; works outside containers too                  |
| Shell library approach  | Sourceable `pkg-manager.sh`                     | Scripts stay simple; no need for per-distro copies                      |
| Schema version          | 1.2                                             | Additive change; 1.1 extensions parse cleanly with defaults             |
| Registry filtering      | Server-side by default, `--distro` override     | Users see only installable extensions; power users can browse all       |

## Consequences

### Positive

- Extensions declare distro support explicitly — no silent failures on unsupported distros
- 47 extensions (Tier 1 and Tier 2) work on all three distros out of the box
- Extension authors can choose between per-distro configs (precise) or universal scripts with `$SINDRI_DISTRO` branching (flexible)
- `pkg-manager.sh` eliminates boilerplate distro detection in individual scripts
- Registry filtering prevents users from installing incompatible extensions
- Schema 1.1 extensions continue working on Ubuntu without modification

### Negative

- ~16 extensions require per-distro install configs where package names differ across distros
- Schema version bump to 1.2 means older CLI versions cannot parse the new `distros` field (they ignore it via forward compat)
- Extension authors must test on all declared distros — the `distros` field is a support commitment

## References

- ADR-008: Extension type system
- ADR-032: Configure processing
- ADR-042: BOM capability architecture
- ADR-047: Project-init collision handling
