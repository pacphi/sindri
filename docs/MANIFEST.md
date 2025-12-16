# Extension Manifest Reference

Complete reference for Sindri extension manifests - tracking installed extensions and configuration.

## Overview

Extension manifests are JSON files stored in `$WORKSPACE/.system/manifest/` that track which extensions are installed, their activation state, and execution configuration. The extension-manager uses manifests to manage extension lifecycles.

**Manifest Location:**

```text
$WORKSPACE/.system/manifest/
├── <extension-name>.json     # Per-extension manifest
├── profile-<name>.json        # Profile installation record
└── .manifest-lock             # Lock file for concurrent access
```

**Schema:** `docker/lib/schemas/manifest.schema.json`

## Manifest Structure

### Basic Manifest

```json
{
  "version": "1.0",
  "extensions": [
    {
      "name": "nodejs",
      "active": true,
      "category": "language",
      "protected": false,
      "dependencies": ["mise-config"]
    }
  ]
}
```

## Top-Level Properties

### version

**Type:** string
**Required:** Yes
**Format:** `<major>.<minor>` (e.g., `"1.0"`)

Manifest schema version for compatibility checking.

```json
{
  "version": "1.0"
}
```

### extensions

**Type:** array
**Required:** Yes

List of extension entries in the manifest.

### config

**Type:** object
**Optional:** Yes

Configuration for extension execution and validation behavior.

## Extension Entry Properties

Each extension in the `extensions` array has these properties:

### name

**Type:** string
**Required:** Yes
**Pattern:** `^[a-z][a-z0-9-]*$`

Unique extension identifier (lowercase, hyphens allowed).

### active

**Type:** boolean
**Required:** Yes

Whether the extension is currently active/installed.

- `true` - Extension is installed and available
- `false` - Extension is present but deactivated

### protected

**Type:** boolean
**Default:** `false`

If `true`, the extension cannot be deactivated or removed via extension-manager.

**Protected extensions:**

- System-critical extensions (base)
- Dependency chain roots
- Extensions marked protected in registry.yaml

```json
{
  "name": "mise-config",
  "active": true,
  "protected": true,
  "category": "base"
}
```

### category

**Type:** string (enum)
**Required:** Yes

Extension category from the official category list.

**Valid Categories:**

- `base` - Core system components
- `agile` - Agile project management
- `language` - Programming languages
- `dev-tools` - Development utilities
- `infrastructure` - Cloud, containers, orchestration
- `ai` - AI/ML frameworks
- `utilities` - General purpose tools
- `desktop` - Desktop environments
- `monitoring` - Observability tools
- `database` - Database servers and clients
- `mobile` - Mobile development SDKs

### dependencies

**Type:** array of strings
**Optional:** Yes

List of extension names this extension depends on.

```json
{
  "name": "python",
  "dependencies": ["mise-config"],
  "active": true,
  "category": "language"
}
```

The extension-manager ensures dependencies are:

1. Installed before the extension
2. Not removed while dependents are active
3. Validated in topological order

## Configuration Section

The optional `config` section controls extension execution and validation behavior.

### config.execution

Controls how extensions are installed and managed.

#### config.execution.parallel

**Type:** boolean
**Default:** `false`

Enable parallel extension installation for faster setup.

```json
{
  "config": {
    "execution": {
      "parallel": true
    }
  }
}
```

**Trade-offs:**

- **Enabled**: Faster installation (~2-3x speedup), but harder to debug
- **Disabled**: Sequential installation, easier error tracking

**Recommendation:** Enable for production deploys, disable for debugging.

#### config.execution.failFast

**Type:** boolean
**Default:** `true`

Stop execution immediately on first extension failure.

```json
{
  "config": {
    "execution": {
      "failFast": false
    }
  }
}
```

**When to disable:** Installing best-effort profiles where some extensions may fail.

#### config.execution.timeout

**Type:** integer (seconds)
**Default:** `600` (10 minutes)
**Minimum:** `0` (no timeout)

Maximum time allowed for extension installation.

```json
{
  "config": {
    "execution": {
      "timeout": 1200
    }
  }
}
```

**Use cases:**

- Increase for slow networks or large extensions
- Decrease for faster failure detection in CI

### config.validation

Controls extension validation behavior.

#### config.validation.schemaValidation

**Type:** boolean
**Default:** `true`

Validate extension.yaml against JSON schema before installation.

```json
{
  "config": {
    "validation": {
      "schemaValidation": true
    }
  }
}
```

**Always recommended:** Catches configuration errors early.

#### config.validation.dnsCheck

**Type:** boolean
**Default:** `true`

Check DNS resolution for domains listed in `requirements.domains`.

```json
{
  "config": {
    "validation": {
      "dnsCheck": false
    }
  }
}
```

**When to disable:** Offline environments, air-gapped deployments.

#### config.validation.dependencyCheck

**Type:** boolean
**Default:** `true`

Verify all extension dependencies are available before installation.

```json
{
  "config": {
    "validation": {
      "dependencyCheck": true
    }
  }
}
```

**Never disable** unless bypassing broken dependency metadata.

## Complete Example

Production manifest with all options:

```json
{
  "version": "1.0",
  "extensions": [
    {
      "name": "mise-config",
      "active": true,
      "protected": true,
      "category": "base",
      "dependencies": []
    },
    {
      "name": "nodejs",
      "active": true,
      "protected": false,
      "category": "language",
      "dependencies": ["mise-config"]
    },
    {
      "name": "python",
      "active": true,
      "protected": false,
      "category": "language",
      "dependencies": ["mise-config"]
    },
    {
      "name": "docker",
      "active": false,
      "protected": false,
      "category": "infrastructure",
      "dependencies": []
    }
  ],
  "config": {
    "execution": {
      "parallel": true,
      "failFast": true,
      "timeout": 900
    },
    "validation": {
      "schemaValidation": true,
      "dnsCheck": true,
      "dependencyCheck": true
    }
  }
}
```

## Manifest Management

### CLI Commands

```bash
# View current manifest
cat $WORKSPACE/.system/manifest/sindri.json

# Regenerate manifests from installed extensions
./cli/extension-manager validate-all

# Export manifest
./cli/extension-manager bom --format json > manifest.json
```

### Manual Editing

**Not recommended.** Use extension-manager commands instead:

```bash
# Install extension (updates manifest)
./cli/extension-manager install nodejs

# Remove extension (updates manifest)
./cli/extension-manager remove nodejs

# Install profile (creates profile manifest)
./cli/extension-manager install-profile fullstack
```

Manual edits may cause:

- State inconsistency
- Broken dependency chains
- Validation failures

### Lock Files

The extension-manager uses `.manifest-lock` to prevent concurrent manifest modifications:

```bash
# Check for lock
if [[ -f "$WORKSPACE/.system/manifest/.manifest-lock" ]]; then
    echo "Extension manager is running"
fi
```

**Lock behavior:**

- Created: Before manifest modification
- Removed: After successful operation
- Stale lock detection: Timeout after 5 minutes

## Validation

### Schema Validation

```bash
# Validate manifest against schema
yq -oj eval manifest.json | \
  ajv validate \
    -s docker/lib/schemas/manifest.schema.json \
    -d -
```

### Dependency Validation

```bash
# Check for circular dependencies
./cli/extension-manager resolve nodejs

# Validate all dependencies
./cli/extension-manager validate-domains
```

## Manifest vs. Registry vs. Extension YAML

**Three levels of configuration:**

| File               | Purpose              | Location                        | Format |
| ------------------ | -------------------- | ------------------------------- | ------ |
| **extension.yaml** | Extension definition | `docker/lib/extensions/<name>/` | YAML   |
| **registry.yaml**  | Extension catalog    | `docker/lib/registry.yaml`      | YAML   |
| **manifest.json**  | Installed extensions | `$WORKSPACE/.system/manifest/`  | JSON   |

**Relationship:**

1. **extension.yaml** - The source of truth for extension behavior
2. **registry.yaml** - Index of available extensions with metadata
3. **manifest.json** - Runtime state of installed extensions

## Common Use Cases

### Install Profile from Manifest

```bash
# Create manifest with desired extensions
cat > manifest.json <<'EOF'
{
  "version": "1.0",
  "extensions": [
    {"name": "nodejs", "active": true, "category": "language"},
    {"name": "python", "active": true, "category": "language"},
    {"name": "docker", "active": true, "category": "infrastructure"}
  ]
}
EOF

# Install extensions from manifest
for ext in $(jq -r '.extensions[] | select(.active==true) | .name' manifest.json); do
    ./cli/extension-manager install "$ext"
done
```

### Deactivate Extension Without Removal

```bash
# Mark extension as inactive (keeps files)
jq '.extensions[] |= if .name == "nodejs" then .active = false else . end' \
  $WORKSPACE/.system/manifest/sindri.json > tmp.json
mv tmp.json $WORKSPACE/.system/manifest/sindri.json
```

**Warning:** This bypasses extension-manager safety checks. Use `extension-manager remove` instead.

### Export Installed Extensions

```bash
# List all active extensions
jq -r '.extensions[] | select(.active==true) | .name' \
  $WORKSPACE/.system/manifest/sindri.json

# Export to profile format
jq '{
  profile: {
    name: "custom",
    extensions: [.extensions[] | select(.active==true) | .name]
  }
}' $WORKSPACE/.system/manifest/sindri.json > custom-profile.yaml
```

## Troubleshooting

### Manifest Corruption

**Symptom:** Invalid JSON or schema validation fails

**Fix:**

```bash
# Backup corrupted manifest
mv $WORKSPACE/.system/manifest/sindri.json \
   $WORKSPACE/.system/manifest/sindri.json.bak

# Regenerate from installed extensions
./cli/extension-manager validate-all
```

### Dependency Conflicts

**Symptom:** Extension fails to install due to missing dependencies

**Diagnosis:**

```bash
# Show dependency chain
./cli/extension-manager resolve <extension>

# Validate all dependencies
./cli/extension-manager validate-domains
```

**Fix:**

```bash
# Install missing dependencies first
./cli/extension-manager install <dependency>

# Then install extension
./cli/extension-manager install <extension>
```

### Stale Lock File

**Symptom:** Extension-manager hangs or reports lock file exists

**Fix:**

```bash
# Check lock age
stat -c %Y $WORKSPACE/.system/manifest/.manifest-lock

# Remove stale lock (if >5 minutes old)
rm -f $WORKSPACE/.system/manifest/.manifest-lock
```

## See Also

- [Extension Schema Reference](SCHEMA.md#extensionschemajson) - Extension YAML structure
- [Extension Authoring Guide](EXTENSION_AUTHORING.md) - Creating extensions
- [Extension Registry](../docker/lib/registry.yaml) - Available extensions
- [Profiles Reference](SCHEMA.md#profilesschemajson) - Profile definitions
