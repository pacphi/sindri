---
description: Update all documentation for an existing Sindri extension
allowed-tools: [Read, Write, Edit, Bash, Glob, Grep, TodoWrite]
argument-hint: "<extension-name>"
---

# Update Extension Documentation

Update all documentation for an existing Sindri extension.

**Arguments:** $ARGUMENTS

- First argument: extension name (required)

## WORKFLOW

Use TodoWrite to track progress through each step.

### Step 1: Verify Extension Exists

1. Check `docker/lib/extensions/{name}/extension.yaml` exists
2. Read the extension.yaml to understand:
   - Category
   - Description
   - Dependencies
   - Installation method
3. Run `./cli/extension-manager info {name}` to verify registration

### Step 2: Check Current Documentation State

Identify which documentation needs to be created or updated:

```bash
# Check if docs exist
ls -la docs/extensions/{NAME}.md 2>/dev/null || echo "MISSING: Extension doc"

# Check registry entry
grep -A3 "^  {name}:" docker/lib/registry.yaml || echo "MISSING: Registry entry"

# Check EXTENSIONS.md catalog
grep "{name}" docs/EXTENSIONS.md || echo "MISSING: Catalog entry"

# Check slides
grep "{name}" docs/slides/extensions.html || echo "MISSING: Slides entry"

# Check profiles
grep "{name}" docker/lib/profiles.yaml || echo "Not in profiles (may be intentional)"
```

### Step 3: Create/Update Extension Documentation

If `docs/extensions/{NAME}.md` is missing or outdated:

1. Create/update with standard template
2. Include: overview, installation, usage, requirements

### Step 4: Verify/Update Registry

If registry entry is missing or incorrect:

1. Add/update entry in `docker/lib/registry.yaml`

### Step 5: Update Extension Catalog

If catalog entry is missing:

1. Add to appropriate table in `docs/EXTENSIONS.md`
2. Include link to extension doc

### Step 6: Update Slides (for AI/notable extensions)

If extension is in AI category or notable:

1. Add to appropriate slide in `docs/slides/extensions.html`
2. Update extension counts if needed

### Step 7: Consider Profile Inclusion

1. Check if extension should be in any profiles
2. Update `docker/lib/profiles.yaml` if appropriate

### Step 8: Validate

```bash
pnpm validate:yaml
pnpm lint:md
./cli/extension-manager info {name}
```

### Step 9: Summary

Report what was created/updated.

## Example Usage

```
/extension/update-docs mdflow
/extension/update-docs nodejs
```
