---
description: Create a new Sindri extension with all required documentation
allowed-tools: [Read, Write, Edit, Bash, Glob, Grep, WebFetch, TodoWrite]
argument-hint: "<extension-name> [install-source: github-url|npm-package|apt-package|binary-url]"
---

# Create New Sindri Extension

Create a complete Sindri extension including all required documentation updates.

**Arguments:** $ARGUMENTS

- First argument: extension name (required)
- Second argument: install source for research (optional)
  - GitHub URL: `https://github.com/org/repo`
  - npm package: `npm:package-name`
  - APT package: `apt:package-name`
  - Binary URL: `https://example.com/tool.tar.gz`

## MANDATORY WORKFLOW

You MUST complete ALL phases below. Use TodoWrite to track progress through each phase.

### Phase 1: Research & Planning

1. If an install source is provided, fetch/research information about the tool
2. Determine the appropriate:
   - Category (ai, dev-tools, language, infrastructure, utilities, etc.)
   - Installation method (mise, npm, apt, binary, script, hybrid)
   - Dependencies on other extensions
3. Create a todo list with all tasks from all phases

### Phase 2: Create Extension Files

1. Create directory: `docker/lib/extensions/{name}/`
2. Create `extension.yaml` with all required sections:
   - metadata (name, version, description, category, dependencies)
   - requirements (domains, diskSpace)
   - install (method and configuration)
   - validate (commands to verify installation)
   - remove (cleanup configuration)
   - upgrade (strategy)
   - bom (bill of materials)
3. Create `mise.toml` if using mise method with npm packages
4. Create install scripts if using script/hybrid method
5. Mark this phase complete in todo list

### Phase 3: Update Registry (REQUIRED)

1. Add extension to `docker/lib/registry.yaml`
   - Place in correct category section
   - Include category, description, dependencies
2. Mark this phase complete in todo list

### Phase 4: Create Extension Documentation (REQUIRED)

1. Create `docs/extensions/{NAME}.md` (uppercase filename)
2. Use the standard template with:
   - Overview table (category, version, installation, disk space, dependencies)
   - Description section
   - Installed Tools table
   - Network Requirements
   - Installation command
   - Usage examples
   - Validation command
   - Source project link
   - Related extensions
3. Mark this phase complete in todo list

### Phase 5: Update Extension Catalog (REQUIRED)

1. Edit `docs/EXTENSIONS.md`
2. Add extension to the appropriate category table
3. Include link to the extension doc
4. Mark this phase complete in todo list

### Phase 6: Update Slides (REQUIRED for AI/notable extensions)

1. Edit `docs/slides/extensions.html`
2. Add to appropriate category slide (e.g., AI & ML Extensions table)
3. Update extension count (search for "Pre-built Tools" and update number)
4. Update summary slide extension count
5. Mark this phase complete in todo list

### Phase 7: Consider Profile Inclusion

1. Review `docker/lib/profiles.yaml`
2. If extension fits existing profiles (ai-dev, anthropic-dev, etc.), add it
3. Mark this phase complete in todo list

### Phase 8: Validate Everything (REQUIRED)

Run ALL validation commands:

```bash
pnpm validate:yaml
pnpm lint:md
./cli/extension-manager info {name}
```

### Phase 9: Summary Report

Provide a final summary listing:

- All files created
- All files updated
- Validation results
- Extension ready for use

## CRITICAL RULES

1. **DO NOT** skip any required phases
2. **DO NOT** mark the task complete until Phase 8 validation passes
3. **DO NOT** forget the slides update for AI/notable extensions
4. **USE** TodoWrite to track progress through all phases
5. **FIX** any validation errors before completing

## Example Usage

```
/extension/new mdflow https://github.com/johnlindquist/mdflow
/extension/new typescript-tools npm:typescript
/extension/new docker-helper apt:docker-ce
/extension/new my-tool
```
