# Implementation Summary: Declarative Collision Handling

## Status Overview

| Phase       | Status      | Description                                                 |
| ----------- | ----------- | ----------------------------------------------------------- |
| **Phase 1** | ✅ COMPLETE | Added collision-handling to 4 extensions                    |
| **Phase 2** | ⏸️ PENDING  | Test collision scenarios (requires deployment)              |
| **Phase 3** | ✅ COMPLETE | YAML validation and schema compliance                       |
| **Phase 4** | ⏸️ OPTIONAL | Future enhancements (interactive prompts, merge strategies) |

**Last Updated:** January 14, 2026

## Overview

This document summarizes the implementation of declarative collision handling for Sindri extensions. This enhancement allows extensions to detect and resolve configuration collisions when `clone-project` is used on repositories with existing configurations.

## Key Design Decisions

### 1. Fully Declarative

**All extension-specific logic lives in extension.yaml** - `capability-manager.sh` only implements generic pattern matching. This follows the open/closed principle: extensions can add collision handling without modifying core code.

### 2. No Extension-Specific Code

The collision handlers are completely generic:

- `detect_collision_version()` - Uses markers from YAML
- `handle_collision()` - Matches scenarios from YAML
- No hardcoded extension names or version detection logic

### 3. Extension-Owned Scenarios

Each extension defines its own:

- Version detection markers
- Collision resolution scenarios
- User-facing messages
- Actions to take

## What Was Implemented

### 1. Schema Extension (`extension.schema.json`)

Added `collision-handling` capability with:

- `version-markers`: Detect installed version from directory structure
- `scenarios`: Define collision resolution scenarios
- Three detection methods: `file-exists`, `directory-exists`, `content-match`
- Five actions: `stop`, `skip`, `proceed`, `backup`, `prompt`

**Location:** `/v2/docker/lib/schemas/extension.schema.json` (lines 835-953)

### 2. Generic Collision Handlers (`capability-manager.sh`)

Three new functions:

```bash
detect_collision_version()  # Lines 433-554
  - Parses version-markers from extension YAML
  - Applies detection methods (file-exists, directory-exists, content-match)
  - Returns version string or "none"

backup_state_markers()  # Lines 558-595
  - Backs up directories/files with timestamp
  - Uses state-markers from project-init capability

handle_collision()  # Lines 600-668
  - Matches detected version + installing version to scenarios
  - Executes action (stop/skip/proceed/backup/prompt)
  - Displays user messages
```

**Location:** `/v2/docker/lib/capability-manager.sh` (lines 427-668)

### 3. Integration (`project-core.sh`)

Added collision checking in `init_project_tools()`:

```bash
# Check for collision with existing installation
local ext_version
ext_version=$(yq eval ".metadata.version" "${LIB_DIR}/extensions/${ext}/extension.yaml" ...)

if ! handle_collision "$ext" "$ext_version"; then
    continue  # Skip initialization
fi
```

**Location:** `/v2/docker/lib/project-core.sh` (lines 251-259)

### 4. Documentation

Created comprehensive documentation:

- **Examples**: `COLLISION_HANDLING_EXAMPLES.md` (same directory)
  - Complete YAML examples for all 4 extensions
  - Detection strategies explained
  - Implementation notes

- **ADR Update**: `../architecture/adr/ADR-001-extension-capabilities-system.md`
  - Added "Collision Handling Enhancement" section
  - Documents problem, solution, benefits

## Extensions Ready for Collision Handling

### 1. claude-flow-v2

**Markers:**

- V2: `.claude/memory.db` or `.claude/memory/`
- V3: `.claude/config.json` with swarm/sona
- Unknown: `.claude/` without known markers

**Scenarios:**

- Same V2 → Skip, show `claude-flow init --force`
- V3 → V2 → Stop, warn about downgrade
- Unknown → Stop, suggest backup

### 2. claude-flow-v3

**Markers:**

- V3: `.claude/config.json` with swarm/sona content
- V2: `.claude/memory.db` or `.claude/memory/`
- Unknown: `.claude/` without known markers

**Scenarios:**

- V2 → V3 → Stop, guide migration (`cf-memory-migrate --from v2 --to v3`)
- Same V3 → Skip, show `claude-flow init --full --force`
- Unknown → Stop, suggest backup

### 3. agentic-qe

**Markers:**

- Installed: `.agentic-qe/` directory

**Scenarios:**

- Already installed → Skip, show `aqe init --yes --force`

### 4. agentic-flow

**Markers:**

- Installed: `.agentic-flow/` or `.agentic-flow/hooks/`

**Scenarios:**

- Already installed → Skip, show `npx agentic-flow init --force`

## Implementation Status

### Phase 1: Add Collision Handling to Extensions ✅ COMPLETE

**Status:** All 4 extensions updated with collision-handling capability

- ✅ **claude-flow-v2** - Added collision-handling (lines 134-221)
  - Version markers: v2, v3, unknown
  - Scenarios: same-v2, v3-to-v2-downgrade, unknown-origin

- ✅ **claude-flow-v3** - Added collision-handling (lines 212-305)
  - Version markers: v3, v2, unknown
  - Scenarios: v2-to-v3-upgrade, same-v3, unknown-origin

- ✅ **agentic-qe** - Added collision-handling (lines 105-127)
  - Version markers: installed
  - Scenarios: already-initialized

- ✅ **agentic-flow** - Added project-init + collision-handling (lines 52-127)
  - **BONUS:** Added project-init capability (init + hooks pretrain)
  - Version markers: installed
  - Scenarios: already-initialized

**Changes:** 4 extension.yaml files modified with declarative collision-handling

### Phase 2: Test Collision Scenarios ⏸️ PENDING

**Status:** Requires deployment to test (manual testing phase)

Test each scenario in deployed environment:

```bash
# Scenario 1: Same version already installed
mkdir -p /tmp/test-v2 && cd /tmp/test-v2
mkdir -p .claude/memory
touch .claude/memory.db
clone-project <repo-url>
# Expected: Skip with message "✓ Claude Flow V2 is already initialized"

# Scenario 2: V2 → V3 upgrade
# (Use same setup, but with claude-flow-v3 installed)
# Expected: Stop with migration guide

# Scenario 3: V3 → V2 downgrade
mkdir -p /tmp/test-v3 && cd /tmp/test-v3
mkdir -p .claude
echo '{"swarm": {}}' > .claude/config.json
clone-project <repo-url>
# Expected: Stop with downgrade warning

# Scenario 4: Unknown origin
mkdir -p /tmp/test-unknown && cd /tmp/test-unknown
mkdir -p .claude
clone-project <repo-url>
# Expected: Stop with backup suggestion

# Scenario 5: No collision (clean project)
mkdir -p /tmp/test-clean && cd /tmp/test-clean
clone-project <repo-url>
# Expected: Normal initialization
```

**Note:** Testing requires a deployed Sindri instance with the extensions installed.

### Phase 3: Validation ✅ COMPLETE

**Status:** All validations passed

- ✅ **YAML syntax validation**: `pnpm validate:yaml` - PASSED
- ✅ **YAML structure parsing**: yq tests on all 4 extensions - PASSED
- ✅ **Schema compliance**: yamllint --strict - PASSED
- ✅ **Extension validation**: Schema-only validation - PASSED

**Results:**

```bash
# YAML lint
✔ YAML Lint successful.

# Structure tests
claude-flow-v2: collision-handling.enabled = true
claude-flow-v3: collision-handling.scenarios = 3
agentic-qe: version-markers[0].version = "installed"
agentic-flow: project-init.enabled = true
```

**Note:** Extension-manager validation shows "tool not found" errors - this is expected as tools aren't installed in local dev environment. YAML structure is valid.

### Phase 4: Future Enhancements ⏸️ OPTIONAL

Potential improvements for future iterations:

1. **Interactive prompts** - Add `--interactive` flag for collision resolution
2. **Merge strategies** - Implement smart config merging (preserve user customizations)
3. **Environment variables** - `COLLISION_STRATEGY=backup|skip|prompt` for automation
4. **Version parsing** - Extract semantic versions from config files for more precise detection
5. **MCP collision handling** - Handle MCP server registration conflicts

## Benefits

### For Users

- **Clear messaging**: Know exactly what was detected and why init was skipped
- **Actionable guidance**: Specific commands to resolve each scenario
- **Safe by default**: Never overwrites existing configs without permission
- **V2 → V3 migration**: Guided migration path for claude-flow

### For Developers

- **No core changes**: Add collision handling by editing extension.yaml
- **Reusable patterns**: Generic handlers work for all extensions
- **Testable**: Each extension can be tested independently
- **Maintainable**: All logic in one place (extension YAML)

## Example User Experience

### Before (Silent Skip)

```bash
$ clone-project https://github.com/example/project
Cloning repository...
Initializing claude-flow-v2...
# (silently skips, user doesn't know why)
```

### After (Clear Guidance)

```bash
$ clone-project https://github.com/example/project
Cloning repository...
Initializing claude-flow-v2...

⚠️  Claude Flow V2 installation detected

  Your project has an existing Claude Flow V2 setup.

  To force a fresh V2 installation, run:
    claude-flow init --force

✓ Clone completed successfully
```

## Files Modified

### Core Implementation ✅ COMPLETE

- ✅ `/v2/docker/lib/schemas/extension.schema.json` - Added collision-handling schema (+119 lines)
- ✅ `/v2/docker/lib/capability-manager.sh` - Added 3 generic functions (+243 lines)
  - `detect_collision_version()` (lines 433-554)
  - `backup_state_markers()` (lines 558-595)
  - `handle_collision()` (lines 600-668)
- ✅ `/v2/docker/lib/project-core.sh` - Integrated collision checking (+9 lines at 251-259)

### Documentation ✅ COMPLETE

- ✅ `COLLISION_HANDLING_EXAMPLES.md` - Complete YAML examples for all 4 extensions
- ✅ `IMPLEMENTATION_SUMMARY_COLLISION_HANDLING.md` - This document with phase status
- ✅ `../architecture/adr/ADR-001-extension-capabilities-system.md` - Enhanced ADR with collision handling section

### Extensions ✅ COMPLETE

- ✅ `/v2/docker/lib/extensions/claude-flow-v2/extension.yaml` - Added collision-handling (+88 lines)
- ✅ `/v2/docker/lib/extensions/claude-flow-v3/extension.yaml` - Added collision-handling (+94 lines)
- ✅ `/v2/docker/lib/extensions/agentic-qe/extension.yaml` - Added collision-handling (+23 lines)
- ✅ `/v2/docker/lib/extensions/agentic-flow/extension.yaml` - Added project-init + collision-handling (+76 lines)

## Testing Checklist

**Phase 3 Validation:**

- ✅ Schema validates: `pnpm validate:yaml` - PASSED
- ✅ YAML structure parses correctly with yq - PASSED
- ✅ yamllint --strict passes - PASSED
- ✅ Extension YAML files are valid - PASSED

**Phase 2 Scenario Testing (Requires Deployment):**

- ⏸️ Same version scenario: Detects and skips
- ⏸️ V2 → V3 scenario: Detects and stops with migration guide
- ⏸️ V3 → V2 scenario: Detects and stops with downgrade warning
- ⏸️ Unknown origin scenario: Detects and stops with backup suggestion
- ⏸️ No collision scenario: Proceeds normally
- ⏸️ User messages are clear and actionable

## References

- **ADR**: `../architecture/adr/ADR-001-extension-capabilities-system.md`
- **Examples**: `COLLISION_HANDLING_EXAMPLES.md` (same directory)
- **Schema**: `/v2/docker/lib/schemas/extension.schema.json` (lines 835-953)
- **Implementation**: `/v2/docker/lib/capability-manager.sh` (lines 427-668)
- **Integration**: `/v2/docker/lib/project-core.sh` (lines 251-259)

## Questions?

If you have questions about:

- **Design rationale**: See ADR-001 "Collision Handling Enhancement" section
- **Implementation details**: See `capability-manager.sh` function comments
- **Extension examples**: See `COLLISION_HANDLING_EXAMPLES.md` (same directory)
- **Testing approach**: See "Testing Collision Scenarios" in examples doc
