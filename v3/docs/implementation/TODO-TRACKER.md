# v3 TODO Tracker

**Last Updated:** 2026-01-22 (Medium Priority Implementation Complete)
**Status:** Active Development - Phase 2/5 Complete, All High + Medium Priority TODOs ✅
**Document Location:** `/alt/home/developer/workspace/projects/sindri/v3/docs/implementation/TODO-TRACKER.md`

This document tracks all TODO comments in the v3 codebase, categorized by priority and status.

For detailed implementation notes, see:

- [v3 Enhancements Implementation Summary](./v3-enhancements-implementation-summary.md)
- [v3 Dockerfile Validation Checklist](./v3-dockerfile-validation-checklist.md)

---

## 🚀 ACTIVE: Configuration Refactoring Initiative (2026-01-22)

**Objective:** Eliminate hard-coded values and logic, improve maintainability and flexibility

### Progress: 40% Complete (2/5 Phases)

```
✅ Phase 1: Configuration Foundation     [████████████████████] 100%
✅ Phase 2: Platform Configuration       [████████████████████] 100%
⬜ Phase 3: Retry & Network Policies     [░░░░░░░░░░░░░░░░░░░░]   0%
⬜ Phase 4: Git Workflow Templates       [░░░░░░░░░░░░░░░░░░░░]   0%
⬜ Phase 5: Test Refactoring             [░░░░░░░░░░░░░░░░░░░░]   0%
```

### Impact

- **50+ hard-coded values** externalized to configuration
- **8 major areas** converted to declarative rules
- **40-50% reduction** potential in test code duplication
- **Zero breaking changes** - fully backward compatible

---

## ✅ Completed TODOs (2026-01-22 Implementation Session)

### Extension Commands

- ✅ `sindri extension install --from-config sindri.yaml` - **IMPLEMENTED** (2026-01-22)
  - **PR/Commit**: v3-enhancements-2026-01-22
  - **Files Modified**: `cli.rs`, `extension.rs`, `entrypoint.sh`
  - **Implementation**: Added `--from-config` flag that reads `sindri.yaml` and installs extensions from `extensions.active`, `extensions.additional`, or `extensions.profile`

- ✅ `sindri extension install --profile <name>` - **IMPLEMENTED** (2026-01-22)
  - **PR/Commit**: v3-enhancements-2026-01-22
  - **Files Modified**: `cli.rs`, `extension.rs`, `profile.rs`, `entrypoint.sh`
  - **Implementation**: Added `--profile` flag that delegates to `profile::install()` with unified UX

- ✅ `sindri extension status` - **IMPLEMENTED** to read from actual manifest (2026-01-22)
  - **PR/Commit**: v3-enhancements-2026-01-22
  - **Files Modified**: `extension.rs`
  - **Code Location**: `extension.rs:455-520`
  - **Implementation**: Replaced mock data with `ManifestManager::load_default()` to read from `~/.sindri/state/manifest.yaml`

- ✅ JSON serialization for status command - **IMPLEMENTED** (2026-01-22)
  - **PR/Commit**: v3-enhancements-2026-01-22
  - **Files Modified**: `extension.rs`
  - **Implementation**: Added `serde::Serialize` derive to `StatusRow` struct, proper JSON output with `serde_json::to_string_pretty()`

### CLI Commands

- ✅ `sindri upgrade` self-update command - **IMPLEMENTED** (2026-01-22)
  - **Original TODO**: upgrade.rs:85 - Implement self-update using self_update crate
  - **Files Modified**: `upgrade.rs`
  - **Implementation**: Full self-update functionality with:
    - `list_versions()` - Lists available versions from GitHub releases
    - `check_for_updates()` - Checks if newer version is available
    - `show_compatibility()` - Checks extension compatibility before upgrade
    - `do_upgrade()` - Downloads and installs new CLI version
  - **Features**: Pre-release support, compatibility matrix checking, rollback safety

### Infrastructure

- ✅ ARM64 architecture support - **IMPLEMENTED** (2026-01-22)
  - **PR/Commit**: v3-enhancements-2026-01-22
  - **Files Modified**: `v3/Dockerfile`, `.github/workflows/release-v3.yml`
  - **Implementation**: Multi-arch Docker builds for `linux/amd64` and `linux/arm64` using Docker Buildx and QEMU

- ✅ Comprehensive health check system - **IMPLEMENTED** (2026-01-22)
  - **PR/Commit**: v3-enhancements-2026-01-22
  - **Files Created**: `v3/docker/scripts/healthcheck.sh`
  - **Files Modified**: `v3/Dockerfile`
  - **Implementation**: 7-point health check covering SSH, CLI, directories, extensions, writability, and user

---

## 🎉 Recent Major Enhancements (2026-01-22)

Beyond the TODO items tracked in code comments, we completed several significant enhancements:

### 1. Multi-Architecture Docker Support

- **What**: Docker images now build for both `linux/amd64` and `linux/arm64`
- **Why**: Support Apple Silicon Macs, ARM cloud instances, Raspberry Pi
- **Impact**: Expands deployment options, improves user experience on ARM devices
- **Build Time**: ~15 minutes for both architectures (parallel)
- **Image Size**: ~800MB per architecture

### 2. Comprehensive Health Check System

- **What**: 7-point health check replacing simple SSH check
- **Checks**: SSH daemon, Sindri CLI, directories, extensions, writability, user existence
- **Why**: Catch failures early, improve observability, better monitoring
- **Impact**: More reliable deployments, faster troubleshooting
- **Location**: `v3/docker/scripts/healthcheck.sh`

### 3. Unified Extension Install Interface

- **What**: Three installation modes via single command
  1. By name: `sindri extension install python`
  2. From config: `sindri extension install --from-config sindri.yaml`
  3. From profile: `sindri extension install --profile minimal`
- **Why**: Consistent UX, simpler entrypoint logic, more flexible
- **Impact**: Easier to use, better suited for Docker entrypoint automation

### 4. TODO Tracking System

- **What**: Comprehensive tracking of all TODOs with prioritization
- **Why**: Organize work, prevent TODO drift, communicate progress
- **Impact**: Better project management, clearer roadmap
- **Location**: This document!

### 5. Enhanced Documentation

- **Created**:
  - `v3-enhancements-implementation-summary.md` (comprehensive guide)
  - `v3-dockerfile-validation-checklist.md` (testing guide)
  - Updated TODO-TRACKER.md (this file)
- **Why**: Onboard contributors, document decisions, guide testing
- **Impact**: Better maintainability, easier contributions, clearer architecture

---

## ✅ Completed: Configuration Refactoring (Phases 1-2)

### Phase 1: Configuration Foundation ✅ COMPLETE

**Completed:** 2026-01-22
**Files Created:**

- `v3/crates/sindri-core/src/types/runtime_config.rs` (464 lines)
- `v3/crates/sindri-core/src/types/platform_matrix.rs` (335 lines)
- `v3/crates/sindri-core/src/config/hierarchical_loader.rs` (362 lines)
- `v3/embedded/config/runtime-defaults.yaml` (95 lines)
- `v3/embedded/config/platform-rules.yaml` (62 lines)
- `v3/schemas/runtime-config.schema.json` (174 lines)
- `v3/schemas/platform-rules.schema.json` (58 lines)

**Configuration Types Implemented:**

1. **RuntimeConfig** - Operational parameters
   - Network: HTTP timeouts (300s), chunk size (1MB), user agent
   - Retry Policies: Max attempts (3), exponential backoff (2x multiplier)
   - GitHub: Repo owner/name, API URLs
   - Backup: Max backups (2), timestamp format
   - Git Workflow: Default branch ("main"), commit messages
   - Display: Preview lines (10), context lines (2)

2. **PlatformMatrix** - Multi-platform support
   - 5 platforms: Linux x86_64/ARM64, macOS x86_64/ARM64, Windows x86_64
   - Target triple mappings (e.g., "x86_64-unknown-linux-musl")
   - Asset filename patterns with {version} placeholder
   - Priority-based selection, OS/arch normalization

3. **HierarchicalConfigLoader** - Multi-source configuration
   - Precedence: Embedded defaults → Global config → Env vars → CLI flags
   - Automatic config directory creation (~/.sindri)
   - Configuration merging and validation
   - Environment variable overrides (SINDRI\_\* prefix)

**Tests:** 28 tests passing in sindri-core

### Phase 2: Platform Configuration ✅ COMPLETE

**Completed:** 2026-01-22
**Files Modified:**

- `v3/crates/sindri-update/src/download.rs` (Lines 356-400 refactored)
- `v3/crates/sindri-update/src/releases.rs` (Lines 160-172 refactored)
- `v3/crates/sindri-update/src/compatibility.rs` (GitHub config integration)

**Refactoring Details:**

1. **download.rs**
   - ❌ Before: 5 hard-coded match cases for platform detection
   - ✅ After: `PlatformMatrix::find_platform()` lookup
   - ❌ Before: Duplicate platform lists in multiple functions
   - ✅ After: `enabled_platforms()` from matrix
   - ❌ Before: Hard-coded timeout (300s), chunk size (1MB), retries (3)
   - ✅ After: Configurable via RuntimeConfig

2. **releases.rs**
   - ❌ Before: Hard-coded `REPO_OWNER` and `REPO_NAME` constants
   - ✅ After: Configurable via `GitHubConfig`
   - ❌ Before: Hard-coded GitHub API URLs
   - ✅ After: Configurable API base URL
   - ❌ Before: Duplicate platform detection logic
   - ✅ After: Shared platform matrix

3. **compatibility.rs**
   - ❌ Before: Hard-coded repository information
   - ✅ After: GitHubConfig integration
   - ❌ Before: Hard-coded user agent string
   - ✅ After: Configurable user agent from RuntimeConfig

**Tests:** 47 tests passing in sindri-update (30 compatibility + 17 download)

**Benefits Realized:**

- ✅ New platforms can be added via YAML configuration
- ✅ Network parameters tunable without rebuilding
- ✅ Fork-friendly (change repo owner/name via config)
- ✅ Platform detection logic centralized (no duplication)

---

## 📋 Pending: Configuration Refactoring (Phases 3-5)

### Phase 3: Retry & Network Policies ⬜ PENDING

**Target:** v3.1.0
**Estimated Effort:** 6-8 hours
**Priority:** Medium

**Objectives:**

- [ ] Implement policy-based retry execution engine
  - **Location:** `v3/crates/sindri-core/src/retry/mod.rs` (new file)
  - **Description:** Execute operations with configurable retry strategies
  - **Strategies:** None, FixedDelay, ExponentialBackoff, LinearBackoff
  - **Configuration:** Per-operation policies in `operation-policies.yaml`

- [ ] Make all network timeouts configurable
  - **Locations:**
    - `download.rs:136` - HTTP timeout
    - `cli.rs:161` - Deploy timeout
  - **Description:** Replace remaining hard-coded timeouts with config lookups
  - **Configuration:** `RuntimeConfig::network::*_timeout_secs`

- [ ] Create operation-policies.yaml
  - **Location:** `v3/embedded/config/operation-policies.yaml` (new file)
  - **Description:** Define retry strategies for different operations
  - **Example:**
    ```yaml
    policies:
      download:
        strategy: exponential-backoff
        max-attempts: 3
        backoff-multiplier: 2.0
      deploy:
        strategy: linear-backoff
        max-attempts: 5
        initial-delay-ms: 2000
    ```

**Files to Create:**

- `v3/crates/sindri-core/src/retry/mod.rs`
- `v3/crates/sindri-core/src/retry/executor.rs`
- `v3/crates/sindri-core/src/retry/strategies.rs`
- `v3/embedded/config/operation-policies.yaml`
- `v3/schemas/operation-policies.schema.json`

**Files to Modify:**

- `v3/crates/sindri-update/src/download.rs` - Use retry executor
- `v3/crates/sindri/src/cli.rs` - Use configurable deploy timeout

**Tests Required:**

- Retry strategy tests (exponential, linear, fixed)
- Policy loading and validation
- Timeout configuration integration

**Success Criteria:**

- ✅ All retry logic uses policy-based executor
- ✅ No hard-coded retry counts in business logic
- ✅ Per-operation timeout configuration works
- ✅ All existing tests pass

---

### Phase 4: Git Workflow Templates ⬜ PENDING

**Target:** v3.2.0
**Estimated Effort:** 5-6 hours
**Priority:** Low

**Objectives:**

- [ ] Create git-workflows.yaml with common patterns
  - **Location:** `v3/embedded/config/git-workflows.yaml` (new file)
  - **Description:** Define workflow templates for different git conventions
  - **Example:**
    ```yaml
    workflows:
      github-fork:
        remotes:
          origin: { type: fork }
          upstream: { type: original }
        branch-patterns:
          main-branches: [main, master]
          feature-prefix: "feature/"
      gitlab-enterprise:
        remotes:
          origin: { type: main }
        branch-patterns:
          main-branches: [develop, master]
          feature-prefix: "feat/"
    ```

- [ ] Refactor git operations to use workflow definitions
  - **Locations:**
    - `v3/crates/sindri-projects/src/git/init.rs:22-23`
    - `v3/crates/sindri-projects/src/git/config.rs:168-207`
  - **Description:** Replace hard-coded remote names and branch assumptions
  - **Current Hard-coded Values:**
    - Default branch: "main"
    - Initial commit message: "chore: initial commit"
    - Fork remote aliases (5 hard-coded with "main" branch assumptions)

- [ ] Support custom workflow configurations
  - **Description:** Allow users to define custom workflows in `~/.sindri/git-workflows.yaml`
  - **Use Cases:**
    - Enterprise git conventions
    - Multi-remote workflows
    - Custom branch naming schemes

**Files to Create:**

- `v3/crates/sindri-core/src/types/git_workflow.rs`
- `v3/embedded/config/git-workflows.yaml`
- `v3/schemas/git-workflows.schema.json`

**Files to Modify:**

- `v3/crates/sindri-projects/src/git/init.rs`
- `v3/crates/sindri-projects/src/git/config.rs`
- `v3/crates/sindri-core/src/config/hierarchical_loader.rs` (add workflow loading)

**Tests Required:**

- Workflow template loading
- Remote name resolution from workflow
- Branch pattern matching
- Custom workflow override

**Success Criteria:**

- ✅ No hard-coded remote names in git operations
- ✅ Support for multiple git workflow conventions
- ✅ User-configurable workflows
- ✅ All existing tests pass

---

### Phase 5: Test Refactoring ⬜ PENDING

**Target:** v3.1.0
**Estimated Effort:** 10-12 hours
**Priority:** Medium

**Objectives:**

- [ ] Create test constants module
  - **Location:** `v3/crates/sindri-update/tests/constants/mod.rs` (new file)
  - **Description:** Centralize all hard-coded test values
  - **Current Duplication:**
    - Version "3.0.0": 33 occurrences across 3 test files
    - Platform "x86_64-unknown-linux-musl": 15+ occurrences
    - TEST_MATRIX_YAML: 54 lines of embedded YAML duplicated

- [ ] Implement test builder patterns
  - **Location:** `v3/crates/sindri-update/tests/helpers/builders.rs` (new file)
  - **Description:** Builder pattern for test data structures
  - **Example:**
    ```rust
    let release = ReleaseBuilder::new()
        .version("3.0.0")
        .with_all_platforms(TEST_URL_BASE)
        .build();
    ```
  - **Current Issue:** Release{} struct creation repeated 15+ times with 20 lines each

- [ ] Extract test fixtures to external files
  - **Location:** `v3/crates/sindri-update/tests/fixtures/` (new directory)
  - **Description:** Move embedded test data to external files
  - **Files to Create:**
    - `test_matrix_basic.yaml`
    - `test_matrix_conflicts.yaml`
    - `checksums/known_checksums.txt`
    - `releases/v3.0.0.json`

- [ ] Create test helper utilities
  - **Locations:**
    - `v3/crates/sindri-update/tests/helpers/mock_server.rs`
    - `v3/crates/sindri-update/tests/helpers/assertions.rs`
    - `v3/crates/sindri-update/tests/helpers/filesystem.rs`
    - `v3/crates/sindri-update/tests/helpers/binaries.rs`
  - **Description:** Reusable test utilities
  - **Current Duplication:**
    - Mock::given() patterns: 10+ repetitions
    - Backup file filtering: 4 identical fs::read_dir blocks
    - Binary creation: Scattered throughout tests

- [ ] Refactor existing tests to use new infrastructure
  - **Files to Modify:**
    - `v3/crates/sindri-update/tests/compatibility_tests.rs` (287 lines)
    - `v3/crates/sindri-update/tests/download_tests.rs` (849 lines)
    - `v3/crates/sindri-update/tests/updater_tests.rs` (408 lines)
  - **Total Test Code:** 1,544 lines
  - **Expected Reduction:** ~500 lines (40-50% reduction → ~1,000 lines)

**Test Infrastructure Structure:**

```
v3/crates/sindri-update/tests/
├── constants/
│   └── mod.rs              # All hard-coded test values
├── fixtures/               # External test data
│   ├── test_matrix_basic.yaml
│   ├── test_matrix_conflicts.yaml
│   ├── checksums/
│   │   └── known_checksums.txt
│   └── releases/
│       └── v3.0.0.json
├── helpers/                # Reusable utilities
│   ├── mod.rs
│   ├── builders.rs         # ReleaseBuilder, etc.
│   ├── assertions.rs       # Custom assertions
│   ├── mock_server.rs      # Server helpers
│   ├── filesystem.rs       # File operations
│   └── binaries.rs         # Binary creation
├── compatibility_tests.rs
├── download_tests.rs
└── updater_tests.rs
```

**Success Criteria:**

- ✅ 40-50% reduction in test code duplication
- ✅ No hard-coded version strings in test bodies
- ✅ Consistent test data creation via builders
- ✅ All tests pass with new infrastructure
- ✅ Faster test authoring for new scenarios

---

## 🔥 High Priority TODOs (Target: v3.0.0)

### Extension System

- [x] **extension.rs:379** - Implement full validation - **COMPLETED** (2026-01-22)
  - **Files Modified**: `v3/crates/sindri/src/commands/extension.rs`
  - **Implementation**: Full validation pipeline with registry, dependency, and conflict checking
    - Loads extension from file OR registry (with installed fallback)
    - Schema validation using `ExtensionValidator`
    - Dependency existence verification via registry
    - Circular dependency detection using `DependencyResolver`
    - Conflict detection with installed extensions via manifest
  - **Lines Changed**: 367-673 (300+ lines of implementation)

### Distribution & Registry

- [x] **distribution.rs:442** - Add comprehensive validation - **COMPLETED** (2026-01-22)
  - **Files Modified**: `v3/crates/sindri-extensions/src/distribution.rs`
  - **Implementation**: Three-tiered validation approach
    - `validate_extension()`: Structural/semantic validation via `ExtensionValidator`
    - `validate_extension_with_registry()`: Adds dependency graph validation
    - `validate_extension_with_checksum()`: Adds SHA256 checksum verification
  - **Dependencies Added**: `sha2 = "0.10"` for checksum computation
  - **Lines Changed**: 433-571 (140+ lines of implementation)

---

## ✅ Medium Priority TODOs - COMPLETED (2026-01-22)

### Extension Commands ✅ COMPLETE

- [x] **extension.rs:930** - Implement versions command - **COMPLETED** (2026-01-22)
  - **Files Modified**: `v3/crates/sindri/src/commands/extension.rs`, `v3/crates/sindri-extensions/src/distribution.rs`
  - **Implementation**: Lists all available versions from GitHub releases with compatibility info
    - Shows version, release date, compatibility status, and installation state
    - Supports JSON output format (`--json` flag)
    - Sorted newest first with upgrade hints

- [x] **extension.rs:1164** - Implement rollback functionality - **COMPLETED** (2026-01-22)
  - **Files Modified**: `v3/crates/sindri/src/commands/extension.rs`, `v3/crates/sindri-extensions/src/distribution.rs`
  - **Implementation**: Restores extension to previous version from manifest history
    - Gets current/previous versions from manifest
    - User confirmation prompt (skippable with `--yes`)
    - Uses `ExtensionDistributor.rollback()` for the operation

### Enhancement Module ✅ COMPLETE

The enhancement module (`sindri-projects/src/enhancement/mod.rs`) has been fully implemented:

- [x] **enhancement/mod.rs:40-45** - Enhancement manager initialization - **COMPLETED** (2026-01-22)
- [x] **enhancement/mod.rs:65-128** - Extension activation - **COMPLETED** (2026-01-22)
- [x] **enhancement/mod.rs:164-231** - Dependency installation - **COMPLETED** (2026-01-22)
- [x] **enhancement/mod.rs:281-329** - CLAUDE.md creation - **COMPLETED** (2026-01-22)
- [x] **enhancement/mod.rs:379-411** - Enhancement setup orchestration - **COMPLETED** (2026-01-22)
- [x] **enhancement/mod.rs:442-480** - Claude auth check - **COMPLETED** (2026-01-22)
- [x] **enhancement/mod.rs:491-511** - Command existence verification - **COMPLETED** (2026-01-22)
  - **Files Modified**: `v3/crates/sindri-projects/src/enhancement/mod.rs`
  - **Implementation**: Full enhancement system with:
    - `EnhancementManager::new()` - Template manager initialization
    - `activate_extensions()` - Extension validation and activation tracking
    - `install_dependencies()` - Dependency detection with glob patterns
    - `create_claude_md()` - Template-based CLAUDE.md generation
    - `setup_enhancements()` - Full orchestration flow
    - `check_claude_auth()` - Claude CLI authentication verification
    - `command_exists()` - Cross-platform command availability check
  - **Tests Added**: 10 unit tests covering all functionality

### Project Templates System ✅ COMPLETE

Project command template system with YAML-driven configuration:

- [x] **project.rs** - Extension manager integration - **COMPLETED** (2026-01-22)
- [x] **project.rs** - Type detection using project-templates.yaml - **COMPLETED** (2026-01-22)
- [x] **project.rs** - Template alias resolution - **COMPLETED** (2026-01-22)
- [x] **project.rs** - Template loading from embedded YAML - **COMPLETED** (2026-01-22)
- [x] **project.rs** - Dependency detection and installation - **COMPLETED** (2026-01-22)
- [x] **project.rs** - Capability manager integration - **COMPLETED** (2026-01-22)
- [x] **project.rs** - Extension manager query for initialized extensions - **COMPLETED** (2026-01-22)
  - **Files Modified**: `v3/crates/sindri/src/commands/project.rs`, `v3/crates/sindri/Cargo.toml`
  - **Dependencies Added**: `sindri-projects`, `glob`, `regex`
  - **Implementation**: YAML-driven template system with:
    - `activate_extension()` - Extension installation via distributor
    - `detect_type_from_name()` - Fuzzy type detection from project names
    - `resolve_template_alias()` - Alias resolution (nodejs→node, py→python, etc.)
    - `load_template()` - Template loading from embedded YAML
    - `detect_and_install_dependencies()` - Auto dependency installation
    - `initialize_project_tools()` - Capability-based tool initialization
    - `get_initialized_extensions_for_project()` - Manifest query for active extensions

### Secrets Management (S3) ✅ COMPLETE

- [x] **S3 client initialization** - **COMPLETED** (2026-01-22)
- [x] **Secret exists check** - **COMPLETED** (2026-01-22)
- [x] **S3 upload with encryption** - **COMPLETED** (2026-01-22)
- [x] **S3 download with decryption** - **COMPLETED** (2026-01-22)
- [x] **Sync logic** - **COMPLETED** (2026-01-22)
  - **Files Created**:
    - `v3/crates/sindri-secrets/src/s3/types.rs` - S3 configuration and metadata types
    - `v3/crates/sindri-secrets/src/s3/encryption.rs` - ChaCha20-Poly1305 + age envelope encryption
    - `v3/crates/sindri-secrets/src/s3/backend.rs` - S3 client wrapper with CRUD operations
    - `v3/crates/sindri-secrets/src/s3/cache.rs` - TTL-based local secret cache
    - `v3/crates/sindri-secrets/src/s3/resolver.rs` - Orchestrator combining backend + encryption + cache
  - **Files Modified**: `v3/crates/sindri-secrets/src/s3/mod.rs`, `v3/crates/sindri/src/commands/secrets_s3.rs`
  - **Architecture** (per ADR-020):
    - Envelope encryption: Random DEK per secret → ChaCha20-Poly1305 → age X25519
    - 5 security layers: S3 SSE, client-side encryption, master key encryption, IAM, TLS
    - Local cache with TTL, sync with conflict detection, key rotation support

---

## 🔮 Low Priority TODOs (Target: v3.2.0+)

### Backup System

- [ ] **backup.rs:253** - Implement actual backup creation
  - **Description**: Create tar.gz backup of workspace/extensions
  - **Current**: Placeholder implementation
  - **Estimated Effort**: 5-6 hours

### Restore System

- [ ] **restore.rs:204** - Show file list from backup
- [ ] **restore.rs:248** - Implement actual restore using tar extraction
- [ ] **restore.rs:316** - Implement S3 and HTTPS download
- [ ] **restore.rs:333** - Implement age decryption
- [ ] **restore.rs:340** - Implement checksum verification
- [ ] **restore.rs:354** - Implement manifest reading from tar
- [ ] **restore.rs:376** - Implement actual analysis
  - **Description**: Full backup/restore system implementation
  - **Current**: Basic structure in place
  - **Estimated Effort**: 10-12 hours
  - **Dependencies**: tar, age encryption, checksum validation

---

## 📝 Documentation TODOs

These are not code TODOs but documentation placeholders:

- Extension examples (haskell, ai-toolkit, claude-flow-v3) - **Not code, documentation only**
- ADR examples with placeholder tokens (VAULT_TOKEN=xxx) - **Examples, not code**
- Migration planning documents - **Planning, not code**

---

## 🚫 False Positives / Out of Scope

- **extensions/haskell/extension.yaml:13** - `hackage.haskell.org` is a URL, not a TODO
- **extensions/ai-toolkit/docs/** - Documentation examples showing TODO usage
- **extensions/claude-flow-v3/commands/** - Template examples with placeholder IDs (REQ-XXX, TASK-XXX)
- **docs/architecture/adr/016** - Example configuration with placeholder secrets
- **docs/architecture/adr/023** - Example code showing `todo!()` as architectural placeholder
- **docs/planning/rust-cli-migration-v3.md:722** - Comment in planning document, not actionable code
- **sindri-projects/src/templates/detector.rs:163** - Test data "todo-rail-service" (not a TODO marker)
- **sindri-projects/src/templates/parser.rs:224** - Test assertion for "todo-rail-service" pattern

---

## Implementation Strategy

### For v3.0.0 Release ✅ COMPLETE

Focus on:

1. ✅ Core extension installation (DONE)
2. ✅ Profile-based installation (DONE)
3. ✅ Config file installation (DONE)
4. ✅ Full validation for extension install (DONE)
5. ✅ Comprehensive distribution validation (DONE)

### For v3.1.0 Release ✅ COMPLETE

All medium priority items implemented:

1. ✅ Rollback functionality (DONE)
2. ✅ Version listing and management (DONE)
3. ✅ S3 secrets backend (DONE)
4. ✅ Enhancement module (DONE)
5. ✅ Project templates system (DONE)

### For v3.2.0+ Releases

Remaining items:

1. ✅ Self-update capability (ALREADY DONE - `sindri upgrade`)
2. Low Priority: Full backup/restore system (8 TODOs)
3. Phase 3-5: Configuration refactoring (Retry policies, Git workflows, Test refactoring)

---

## How to Use This Document

### Adding a New TODO

1. Add code comment: `// TODO: Brief description`
2. Add entry to this document with:
   - File location and line number
   - Full description
   - Current workaround (if any)
   - Estimated effort
   - Dependencies
   - Target version

### Closing a TODO

1. Implement the feature
2. Remove the TODO comment from code
3. Move entry to "Completed TODOs" section
4. Add completion date

### Reviewing TODOs

- **Weekly**: Review high priority TODOs
- **Monthly**: Review medium priority TODOs
- **Quarterly**: Review low priority TODOs and re-prioritize

---

## Metrics

### Current Status (2026-01-22)

**Total TODOs**: 45 (30 existing + 15 newly discovered from codebase audit)
**Completed**: 37 (15 previous + 22 medium priority) 🎉
**High Priority**: 0 ✅ (all complete)
**Medium Priority**: 0 ✅ (all complete - 2 extension + 7 enhancement + 8 project + 5 secrets)
**Low Priority**: 8 (1 backup + 7 restore)
**Completion Rate**: 82.2% ⬆️⬆️ (massive increase from medium priority completions)

### Medium Priority Completion (2026-01-22)

| Category           | Count  | Status      | Location                                 |
| ------------------ | ------ | ----------- | ---------------------------------------- |
| Extension Commands | 2      | ✅ Complete | `sindri/src/commands/extension.rs`       |
| Enhancement Module | 7      | ✅ Complete | `sindri-projects/src/enhancement/mod.rs` |
| Project Templates  | 8      | ✅ Complete | `sindri/src/commands/project.rs`         |
| S3 Secrets         | 5      | ✅ Complete | `sindri-secrets/src/s3/*.rs`             |
| **Total**          | **22** | ✅ Complete |                                          |

### Configuration Refactoring Progress

**Total Phases**: 5
**Completed**: 2 (Phase 1: Foundation, Phase 2: Platform Config)
**In Progress**: 0
**Remaining**: 3 (Phase 3-5)
**Overall Progress**: 40%

### v3.1.0 Progress

**Target for v3.1.0**: 100% of medium priority TODOs
**Current Progress**: 100% ✅ (22/22 completed) - All medium priority tasks complete!

### Recent Activity

- **2026-01-22 (Comprehensive Medium Priority Implementation)**: ALL MEDIUM PRIORITY COMPLETE 🎉
  - Implemented **Extension Commands** (2 TODOs):
    - `versions` command - Lists available versions from GitHub with compatibility info
    - `rollback` command - Restores previous version from manifest history
  - Implemented **Enhancement Module** (7 TODOs):
    - Full `EnhancementManager` with template integration
    - Extension activation, dependency installation, CLAUDE.md generation
    - Claude auth checking, command verification
    - 10 unit tests added
  - Implemented **Project Templates System** (8 TODOs):
    - YAML-driven template loading and type detection
    - Fuzzy pattern matching for project name → type inference
    - Alias resolution, dependency auto-detection
    - Capability manager integration
  - Implemented **S3 Secrets Management** (5 TODOs):
    - Full S3 backend with envelope encryption (ChaCha20-Poly1305 + age)
    - Local TTL-based cache, sync with conflict detection
    - Key rotation support, 5 security layers per ADR-020
  - **Build Status**: ✅ All lint checks pass, release build successful
  - **Lines Added**: ~2,500+ lines of production code
- **2026-01-22 (Late Night)**: High Priority Validation Implementation
  - Completed **extension.rs:379** - Full validation in CLI validate command
  - Completed **distribution.rs:442** - Comprehensive validation in distribution
- **2026-01-22 (Night)**: Codebase TODO audit and tracker sync
- **2026-01-22 (Evening)**: Completed Phases 1-2 of configuration refactoring
- **2026-01-22 (Afternoon)**: Completed 6 major enhancements
- **Completion Velocity**: 35 TODOs completed in single day 🚀
- **Next Milestone**: Phase 3 (Retry & Network Policies) OR Low Priority items

---

## Related Documents

### Implementation Guides

- **[v3 Enhancements Implementation Summary](./v3-enhancements-implementation-summary.md)**
  - Complete documentation of 2026-01-22 enhancements
  - Detailed implementation notes for CLI flags, ARM64 support, health checks
  - Testing recommendations and validation steps
  - Migration notes from v2 to v3

- **[v3 Dockerfile Validation Checklist](./v3-dockerfile-validation-checklist.md)**
  - Comprehensive testing checklist for Docker implementation
  - 11 detailed test scenarios with expected results
  - Performance benchmarks and metrics
  - Rollback plan and troubleshooting guide

### Architecture Documents

- **[v3 Architecture ADRs](../architecture/adr/README.md)**
  - Design decisions and technical rationale
  - Provider architecture (ADR-002, ADR-003, ADR-005, ADR-007)
  - Extension system architecture (ADR-008 through ADR-013)
  - Secrets and backup architecture (ADR-015 through ADR-018)

### Planning Documents

- **[Rust CLI Migration Plan](../planning/rust-cli-migration-v3.md)**
  - Original v3 migration strategy
  - Phase-by-phase implementation plan
  - Comparison with v2 architecture

---

## Contributing

When adding a TODO:

1. Keep it specific and actionable
2. Estimate effort (hours)
3. Note any dependencies
4. Add to this tracker
5. Assign a priority
6. Set target version
7. Reference related ADRs or design docs

When closing a TODO:

1. Implement the feature
2. Remove the TODO comment from code
3. Update this document
4. Update metrics
5. Link to PR/commit that resolved it
6. Document any breaking changes
7. Update related documentation

When reviewing TODOs:

1. **Weekly**: Review high priority TODOs, update estimates
2. **Monthly**: Review medium priority TODOs, re-prioritize if needed
3. **Quarterly**: Review low priority TODOs, close stale items, add new ones
4. **Before Release**: Ensure all target TODOs are complete or moved to next version

---

## Document Changelog

### 2026-01-22 (v1.5.0) - Medium Priority Implementation Complete 🎉

- ✅ **COMPLETED**: All medium priority TODOs (22/22)
- ✅ Implemented Extension Commands:
  - `versions` command with GitHub release integration
  - `rollback` command with manifest version history
- ✅ Implemented Enhancement Module (7 TODOs):
  - Full `EnhancementManager` class with template integration
  - Extension activation, dependency installation, CLAUDE.md creation
  - Claude auth checking, command verification
  - 10 unit tests for comprehensive coverage
- ✅ Implemented Project Templates System (8 TODOs):
  - YAML-driven template loading from embedded config
  - Fuzzy type detection with pattern matching
  - Alias resolution and template inheritance
  - Dependency detection and auto-installation
  - Capability manager and extension manager integration
- ✅ Implemented S3 Secrets Management (5 TODOs):
  - Full S3 backend with envelope encryption
  - ChaCha20-Poly1305 + age X25519 encryption
  - Local TTL-based cache with sync support
  - Key rotation and conflict detection
- ✅ All clippy lint checks pass
- ✅ Release build successful
- ✅ Updated metrics: 82.2% completion rate (up from 33.3%)
- ✅ v3.1.0 medium priority progress: 100% complete

### 2026-01-22 (v1.4.0) - High Priority Validation Implementation

- ✅ **COMPLETED**: All high priority TODOs (0 remaining)
- ✅ Implemented full validation in `extension.rs` CLI validate command
  - Registry validation with fallback to installed extensions
  - Dependency checking via `DependencyResolver`
  - Conflict detection with installed extensions
  - 300+ lines of implementation
- ✅ Implemented comprehensive validation in `distribution.rs`
  - Three-tiered validation approach (basic, with-registry, with-checksum)
  - SHA256 checksum verification
  - Dependency graph validation
  - 140+ lines of implementation
- ✅ Updated metrics: 33.3% completion rate (up from 28.9%)
- ✅ v3.0.0 high priority progress: 100% complete
- ✅ All 36 sindri-extensions tests passing

### 2026-01-22 (v1.3.0) - Codebase TODO Audit

- ✅ Performed comprehensive codebase scan for TODO/FIXME comments
- ✅ Discovered 15 previously untracked TODOs:
  - 7 in Enhancement Module (`sindri-projects/src/enhancement/mod.rs`)
  - 8 in Project Templates (`sindri/src/commands/project.rs`)
- ✅ Marked `sindri upgrade` command as COMPLETED (full implementation exists)
- ✅ Updated line numbers for 10 tracked items (code shifts from refactoring):
  - extension.rs: 411→297, 1014→930, 1245→1164
  - restore.rs: 205→204, 245→248, 313→316, 331→333, 339→340, 353→354, 375→376
- ✅ Added new "Enhancement Module" section in Medium Priority
- ✅ Added new "Project Templates System" section in Medium Priority
- ✅ Updated metrics: 28.9% completion rate (adjusted for newly discovered TODOs)
- ✅ Total TODOs now tracked: 45 (was 26)

### 2026-01-22 (v1.2.0) - Configuration Refactoring Update

- ✅ Added major "Configuration Refactoring Initiative" section
- ✅ Documented Phases 1-2 completion (40% of refactoring complete)
- ✅ Added detailed specifications for Phases 3-5 (Retry Policies, Git Workflows, Test Refactoring)
- ✅ Created comprehensive file lists for each phase
- ✅ Updated metrics: 46.2% completion rate (up from 43.5%)
- ✅ Added configuration refactoring progress tracking
- ✅ Documented 1,800+ lines of new code across 7 files
- ✅ Listed all refactored files and line ranges
- ✅ Added success criteria for each pending phase

### 2026-01-22 (v1.1.0) - Major Enhancement Update

- ✅ Moved document to `v3/docs/implementation/` directory
- ✅ Completed 6 major enhancements (CLI flags, status command, ARM64, health checks)
- ✅ Updated metrics: 43.5% completion rate (up from 17.4%)
- ✅ Added "Recent Major Enhancements" section documenting infrastructure improvements
- ✅ Added "Related Documents" section with links to implementation guides
- ✅ Expanded "Contributing" section with review schedule
- ✅ Added this changelog section

### 2026-01-22 (v1.0.0) - Initial Version

- ✅ Created comprehensive TODO tracking system
- ✅ Categorized 23 TODOs by priority (High/Medium/Low)
- ✅ Added effort estimates and target versions
- ✅ Documented 4 completed TODOs (initial CLI work)
- ✅ Established metrics tracking system
- ✅ Created contribution guidelines

---

**Document Version:** 1.5.0
**Document Owner:** Sindri Core Team
**Review Frequency:** Weekly (High Priority), Monthly (Medium/Low Priority)
**Last Review:** 2026-01-22
