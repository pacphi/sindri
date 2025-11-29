# Workspace Structure

Workspace directory structure initialization.

## Overview

| Property         | Value  |
| ---------------- | ------ |
| **Category**     | base   |
| **Version**      | 1.0.0  |
| **Installation** | script |
| **Disk Space**   | 10 MB  |
| **Dependencies** | None   |

## Description

Workspace directory structure initialization - creates the base directory structure for user workspace. This is a foundational extension that sets up the filesystem layout.

## Created Directories

```text
/workspace/
├── projects/         # User projects
├── config/          # User configuration files
├── bin/             # User binaries (added to PATH)
├── scripts/         # User scripts
├── templates/       # Project templates
├── .local/          # Local installations (mise, etc.)
├── .config/         # Tool configurations
└── .system/         # Extension state
    ├── manifest/    # Active extensions manifest
    ├── logs/        # Extension logs
    └── bom/         # Bill of materials tracking
```

## Installation

**Automatically installed** at first boot - no manual installation required.

```bash
# If needed manually:
extension-manager install workspace-structure
```

## Validation

No commands to validate - validates directory existence.

## Upgrade

**Strategy:** none

Directory structure is static.

## Removal

Not recommended - this is a base extension.

```bash
extension-manager remove workspace-structure
```

Note: Removal does not delete existing directories.

## Related Extensions

- [mise-config](MISE-CONFIG.md) - Tool version manager
