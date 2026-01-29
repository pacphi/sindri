# Collision Handling Examples

This document provides complete `collision-handling` capability examples for extensions that initialize project-specific configurations.

## Design Principles

1. **No extension-specific logic in capability-manager.sh** - All logic is declarative in extension.yaml
2. **Generic pattern matching** - capability-manager.sh implements generic detection/resolution
3. **Extension-owned scenarios** - Each extension defines its own collision scenarios
4. **Fully declarative** - No bash scripts needed for collision handling

## Extension Examples

### 1. claude-flow-v2

**Detection Strategy:**

- V2 marker: `.claude/memory.db` file OR `.claude/memory/` directory
- Same version: Has V2 markers
- Unknown: Has `.claude/` but no V2 markers

```yaml
capabilities:
  collision-handling:
    enabled: true

    version-markers:
      # V2 marker: memory.db file
      - path: ".claude/memory.db"
        type: file
        version: "v2"
        detection:
          method: file-exists

      # V2 marker: memory subdirectory
      - path: ".claude/memory"
        type: directory
        version: "v2"
        detection:
          method: directory-exists

      # V3 marker (for cross-version detection)
      - path: ".claude/config.json"
        type: file
        version: "v3"
        detection:
          method: content-match
          patterns:
            - '"swarm"'
            - '"sona"'
          match-any: true

      # Unknown marker
      - path: ".claude"
        type: directory
        version: "unknown"
        detection:
          method: directory-exists
          exclude-if:
            - ".claude/memory.db"
            - ".claude/memory"
            - ".claude/config.json"

    scenarios:
      # Scenario 1: Same version already installed
      - name: "same-version-v2"
        detected-version: "v2"
        installing-version: "2.7.47"
        action: skip
        message: |
          ✓ Claude Flow V2 is already initialized

          To force a fresh V2 installation, run:
            claude-flow init --force

      # Scenario 2: V3 detected when installing V2 (downgrade)
      - name: "v3-to-v2-downgrade"
        detected-version: "v3"
        installing-version: "2.7.47"
        action: stop
        message: |
          ⚠️  Claude Flow V3 installation detected

          Your project has Claude Flow V3 installed.
          Downgrading to V2 is not recommended and may cause data loss.

          If you really want V2:
            1. Backup .claude directory
            2. Remove claude-flow-v3: extension-manager remove claude-flow-v3
            3. Install claude-flow-v2: extension-manager install claude-flow-v2
            4. Run: claude-flow init --force

          Skipping automatic initialization to prevent conflicts.

      # Scenario 3: Unknown .claude directory
      - name: "unknown-origin"
        detected-version: "unknown"
        installing-version: "2.7.47"
        action: stop
        message: |
          ⚠️  Existing .claude directory detected (unknown origin)

          Your project has a .claude directory that doesn't match
          known Claude Flow V2 or V3 structure.

          Options:
            1. Backup and reinitialize: mv .claude .claude.backup && claude-flow init --force
            2. Skip initialization: (do nothing)

          Skipping automatic initialization for safety.
```

### 2. claude-flow-v3

**Detection Strategy:**

- V3 marker: `.claude/config.json` with `"swarm"` or `"sona"` content
- V2 marker: `.claude/memory.db` or `.claude/memory/`
- Same version: Has V3 markers
- Unknown: Has `.claude/` but no V2/V3 markers

```yaml
capabilities:
  collision-handling:
    enabled: true

    version-markers:
      # V3 marker: unified config with swarm/sona
      - path: ".claude/config.json"
        type: file
        version: "v3"
        detection:
          method: content-match
          patterns:
            - '"swarm"'
            - '"sona"'
          match-any: true

      # V2 marker: memory.db
      - path: ".claude/memory.db"
        type: file
        version: "v2"
        detection:
          method: file-exists

      # V2 marker: memory directory
      - path: ".claude/memory"
        type: directory
        version: "v2"
        detection:
          method: directory-exists

      # Unknown marker
      - path: ".claude"
        type: directory
        version: "unknown"
        detection:
          method: directory-exists
          exclude-if:
            - ".claude/config.json"
            - ".claude/memory.db"
            - ".claude/memory"

    scenarios:
      # Scenario 1: V2 detected when installing V3 (upgrade)
      - name: "v2-to-v3-upgrade"
        detected-version: "v2"
        installing-version: "3.0.0"
        action: stop
        message: |
          ⚠️  Claude Flow V2 installation detected

          Your project has an existing Claude Flow V2 setup.
          To migrate to V3, please run these commands manually:

            1. Install claude-flow-v3:
               extension-manager install claude-flow-v3

            2. Migrate memory (AgentDB → HNSW):
               cf-memory-migrate --from v2 --to v3

            3. Migrate configuration:
               cf-doctor-fix

          📖 Full migration guide:
             https://github.com/ruvnet/claude-flow/wiki/Migrating-to-V3

          Skipping automatic initialization to prevent data loss.

      # Scenario 2: Same version already installed
      - name: "same-version-v3"
        detected-version: "v3"
        installing-version: "3.0.0"
        action: skip
        message: |
          ✓ Claude Flow V3 is already initialized

          To force a fresh V3 installation, run:
            claude-flow init --full --force

      # Scenario 3: Unknown .claude directory
      - name: "unknown-origin"
        detected-version: "unknown"
        installing-version: "3.0.0"
        action: stop
        message: |
          ⚠️  Existing .claude directory detected (unknown origin)

          Your project has a .claude directory that doesn't match
          known Claude Flow V2 or V3 structure.

          Options:
            1. Backup and reinitialize: mv .claude .claude.backup && claude-flow init --full --force
            2. Skip initialization: (do nothing)

          Skipping automatic initialization for safety.
```

### 3. agentic-qe

**Detection Strategy:**

- Installed marker: `.agentic-qe/` directory exists
- Version detection not needed (single version)

```yaml
capabilities:
  collision-handling:
    enabled: true

    version-markers:
      # Agentic QE marker
      - path: ".agentic-qe"
        type: directory
        version: "installed"
        detection:
          method: directory-exists

    scenarios:
      # Scenario 1: Already installed
      - name: "already-initialized"
        detected-version: "installed"
        installing-version: "1.0.0"
        action: skip
        message: |
          ✓ Agentic QE is already initialized

          To force a fresh installation, run:
            aqe init --yes --force

      # Note: No version migration scenarios needed for single-version extension
```

### 4. agentic-flow

**Detection Strategy:**

- Installed marker: `.agentic-flow/` directory exists (if created by init)
- Note: Based on research, agentic-flow may not create a persistent directory marker
- Alternative: Check for hooks pretrained state

```yaml
capabilities:
  collision-handling:
    enabled: true

    version-markers:
      # Agentic Flow marker (if init creates directory)
      - path: ".agentic-flow"
        type: directory
        version: "installed"
        detection:
          method: directory-exists

      # Alternative marker: hooks directory
      - path: ".agentic-flow/hooks"
        type: directory
        version: "installed"
        detection:
          method: directory-exists

    scenarios:
      # Scenario 1: Already initialized
      - name: "already-initialized"
        detected-version: "installed"
        installing-version: "1.0.0"
        action: skip
        message: |
          ✓ Agentic Flow is already initialized

          To force a fresh installation, run:
            npx agentic-flow init --force

      # Note: No version migration scenarios needed
```

## Implementation Notes

### Generic Detection Algorithm

The `detect_collision_version()` function in `capability-manager.sh` implements this logic:

1. **Load version-markers** from extension YAML
2. **Iterate through markers** in order
3. **Apply detection method**:
   - `file-exists`: Check if file exists
   - `directory-exists`: Check if directory exists (with exclude-if logic)
   - `content-match`: Check file content matches patterns (with match-any logic)
4. **Return version** of first matching marker

### Generic Resolution Algorithm

The `handle_collision()` function implements:

1. **Check if collision-handling enabled** for extension
2. **Detect installed version** using markers
3. **Find matching scenario** (detected-version + installing-version)
4. **Execute action**:
   - `stop`: Print message, return 1 (stop init)
   - `skip`: Print message, return 1 (skip init)
   - `proceed`: Return 0 (continue init)
   - `backup`: Print message, backup directory, return 0
   - `prompt`: Print message with options (future: interactive)

### Adding New Extensions

To add collision handling to a new extension:

1. **Define version-markers** that identify installed versions
2. **Define scenarios** for each collision case
3. **Write clear messages** with actionable commands
4. **No code changes needed** in capability-manager.sh

## Testing Collision Scenarios

### Test Setup

```bash
# Create test project
mkdir -p /tmp/test-project && cd /tmp/test-project

# Simulate V2 installation
mkdir -p .claude/memory
touch .claude/memory.db

# Run clone-project (will detect V2)
clone-project https://github.com/example/repo
```

### Expected Behavior

When collision is detected:

1. User sees clear message explaining what was found
2. User sees specific commands to resolve the collision
3. Init is skipped (no data loss)
4. User retains full control

## Future Enhancements

1. **Interactive prompts** - Ask user to choose resolution
2. **Merge strategies** - Intelligently merge configurations
3. **Environment variable control** - `COLLISION_STRATEGY=backup|skip|prompt`
4. **Detailed version detection** - Parse version numbers from config files
