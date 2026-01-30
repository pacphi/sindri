# v3 TODO Tracker

**Last Updated:** 2026-01-22 (Tool Dependency Management & Clippy Cleanup)
**Status:** Active Development - Phase 5/5 Complete, All High + Medium Priority TODOs ✅
**Document Location:** `/alt/home/developer/workspace/projects/sindri/v3/docs/planning/active/TODO-TRACKER.md`

This document tracks all TODO comments in the v3 codebase, categorized by priority and status.

For detailed implementation notes, see:

- [v3 Enhancements Implementation Summary](../complete/v3-enhancements-implementation-summary.md)
- [v3 Dockerfile Validation Checklist](../complete/v3-dockerfile-validation-checklist.md)

---

## 🚀 ACTIVE: Configuration Refactoring Initiative (2026-01-22)

**Objective:** Eliminate hard-coded values and logic, improve maintainability and flexibility

### Progress: 100% Complete (5/5 Phases) 🎉

```
✅ Phase 1: Configuration Foundation     [████████████████████] 100%
✅ Phase 2: Platform Configuration       [████████████████████] 100%
✅ Phase 3: Retry & Network Policies     [████████████████████] 100%
✅ Phase 4: Git Workflow Templates       [████████████████████] 100%
✅ Phase 5: Test Refactoring             [████████████████████] 100%
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

## ✅ Completed: Configuration Refactoring (Phases 3-5)

### Phase 3: Retry & Network Policies ✅ COMPLETE

**Completed:** 2026-01-22
**Files Created:**

- `v3/crates/sindri-core/src/retry/mod.rs` - Module definition and public exports
- `v3/crates/sindri-core/src/retry/executor.rs` - RetryExecutor and RetryExecutorBuilder
- `v3/crates/sindri-core/src/retry/strategies.rs` - Strategy implementations (calculate_delay, predicates)
- `v3/crates/sindri-core/src/retry/observer.rs` - RetryObserver trait, TracingObserver, StatsObserver
- `v3/crates/sindri-core/src/retry/error.rs` - RetryError enum
- `v3/crates/sindri-core/src/retry/tests.rs` - 59 comprehensive unit tests

**Files Modified:**

- `v3/crates/sindri-core/src/lib.rs` - Added `pub mod retry;`
- `v3/crates/sindri-core/Cargo.toml` - Added `rand = "0.8"` for jitter
- `v3/crates/sindri-update/src/download.rs` - Integrated retry policy from RuntimeConfig
- `v3/embedded/config/runtime-defaults.yaml` - Added operation-specific retry policies

**Key Implementations:**

1. **RetryExecutor** - Policy-based retry execution engine
   - Configurable strategies: None, Fixed, Exponential, Linear
   - Jitter support for preventing thundering herd
   - Observer pattern for logging/metrics

2. **RetryObserver Trait** - Retry event notifications
   - `TracingObserver` - Logs retry events via tracing crate
   - `StatsObserver` - Collects retry statistics
   - `NoOpObserver` - Silent observer for testing

3. **RetryPredicate Trait** - Error classification
   - `AlwaysRetry`, `NeverRetry` - Simple predicates
   - `ClosurePredicate` - Custom retry logic
   - `MessagePredicate` - Error message matching
   - Composable with AND/OR logic

**Tests:** 59 tests passing in sindri-core retry module

---

### Phase 4: Git Workflow Templates ✅ COMPLETE

**Completed:** 2026-01-22
**Files Modified:**

- `v3/crates/sindri-projects/src/git/init.rs` - Uses GitWorkflowConfig for default branch and commit message
- `v3/crates/sindri-projects/src/git/remote.rs` - Uses config for remote names (origin, upstream)
- `v3/crates/sindri-projects/src/git/config.rs` - Added `detect_main_branch()` helper, configurable aliases
- `v3/crates/sindri-projects/src/git/clone.rs` - Uses config for upstream remote name

**Key Implementations:**

1. **GitWorkflowConfig Integration**
   - `default_branch` - Configurable default branch name (default: "main")
   - `initial_commit_message` - Configurable initial commit message
   - `origin_remote` / `upstream_remote` - Configurable remote names
   - `main_branch_names` - List of recognized main branch names for detection

2. **Helper Functions**
   - `detect_main_branch()` - Auto-detects main branch from list of candidates
   - `branch_exists()` - Checks if a branch exists in repository

3. **Alias Template Updates**
   - Fork aliases now use format!() with config values
   - Supports custom upstream/origin remote names
   - Configurable main branch in all aliases

**Tests:** 20 tests passing in sindri-projects git module

---

### Phase 5: Test Refactoring ✅ COMPLETE

**Completed:** 2026-01-22
**Files Created:**

Test Infrastructure (`v3/crates/sindri-update/tests/common/`):

- `mod.rs` (39 lines) - Module re-exports
- `constants.rs` (60 lines) - VERSION*\*, PLATFORM*\*, FAKE_BINARY_CONTENT
- `builders.rs` (261 lines) - ReleaseBuilder, ReleaseAssetBuilder
- `extensions.rs` (107 lines) - standard_extensions(), extensions_from_pairs()
- `mock_server.rs` (78 lines) - mock_binary_download(), mock_flaky_download()
- `assertions.rs` (131 lines) - assert_compatible(), assert_incompatible()
- `fixtures.rs` (52 lines) - load_matrix_v1(), load_matrix_conflicts()
- `updater_helpers.rs` (77 lines) - create_fake_binary(), create_version_script()

Test Fixtures (`v3/crates/sindri-update/tests/fixtures/`):

- `compatibility_matrix_v1.yaml`
- `compatibility_matrix_conflicts.yaml`
- `compatibility_matrix_complex.yaml`
- `compatibility_matrix_multi_version.yaml`
- `compatibility_matrix_empty.yaml`
- `compatibility_matrix_schema_v2.yaml`

**Files Modified:**

- `v3/crates/sindri-update/tests/compatibility_tests.rs` (514→399 lines, 22% reduction)
- `v3/crates/sindri-update/tests/download_tests.rs` (519→372 lines, 28% reduction)
- `v3/crates/sindri-update/tests/updater_tests.rs` (519→465 lines, 10% reduction)

**Metrics:**

| Metric               | Before | After | Change     |
| -------------------- | ------ | ----- | ---------- |
| Test file lines      | 1,552  | 1,236 | -20%       |
| Infrastructure lines | 0      | 805   | (reusable) |
| Duplicated patterns  | 70+    | 0     | -100%      |

**Key Infrastructure Components:**

1. **Constants Module** - Centralized version strings, platform identifiers, test content
2. **Builder Pattern** - Fluent API for Release and ReleaseAsset construction
3. **Extension Helpers** - Factory functions for HashMap creation
4. **Mock Server Helpers** - Wiremock setup utilities
5. **Assertion Helpers** - Semantic test assertions
6. **Fixture Loaders** - YAML matrix file loading

**Tests:** 72 tests passing in sindri-update (30 compatibility + 18 download + 24 updater)

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
2. ✅ Configuration refactoring (ALL 5 PHASES COMPLETE)
3. Low Priority: Full backup/restore system (8 TODOs)

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
**Completed**: 5 (All Phases) 🎉
**In Progress**: 0
**Remaining**: 0
**Overall Progress**: 100%

### v3.1.0 Progress

**Target for v3.1.0**: 100% of medium priority TODOs
**Current Progress**: 100% ✅ (22/22 completed) - All medium priority tasks complete!

### Recent Activity

- **2026-01-22 (Configuration Refactoring Complete)**: ALL PHASES COMPLETE 🎉
  - **Phase 3: Retry & Network Policies** - Full implementation
    - Created `sindri-core/src/retry/` module (5 files, ~800 lines)
    - Implemented RetryExecutor, strategies, observers, predicates
    - 59 comprehensive unit tests
  - **Phase 4: Git Workflow Templates** - Full integration
    - Updated 4 git module files to use GitWorkflowConfig
    - Added `detect_main_branch()` helper
    - Configurable aliases and remote names
    - 20 tests passing
  - **Phase 5: Test Refactoring** - Full infrastructure
    - Created `tests/common/` module (8 files, 805 lines)
    - Created `tests/fixtures/` directory (6 YAML files)
    - Refactored 3 test files with 20% line reduction
    - 72 tests passing
  - **All clippy warnings resolved**
  - **Full workspace test suite passing**
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

- **[v3 Enhancements Implementation Summary](../complete/v3-enhancements-implementation-summary.md)**
  - Complete documentation of 2026-01-22 enhancements
  - Detailed implementation notes for CLI flags, ARM64 support, health checks
  - Testing recommendations and validation steps
  - Migration notes from v2 to v3

- **[v3 Dockerfile Validation Checklist](../complete/v3-dockerfile-validation-checklist.md)**
  - Comprehensive testing checklist for Docker implementation
  - 11 detailed test scenarios with expected results
  - Performance benchmarks and metrics
  - Rollback plan and troubleshooting guide

### Architecture Documents

- **[v3 Architecture ADRs](../../architecture/adr/README.md)**
  - Design decisions and technical rationale
  - Provider architecture (ADR-002, ADR-003, ADR-005, ADR-007)
  - Extension system architecture (ADR-008 through ADR-013)
  - Secrets and backup architecture (ADR-015 through ADR-018)

### Planning Documents

- **[Rust CLI Migration Plan](../complete/rust-cli-migration-v3.md)**
  - Original v3 migration strategy
  - Phase-by-phase implementation plan
  - Comparison with v2 architecture

---

## 🧹 Rust Code Cleanup Opportunities

The following `#[allow(dead_code)]` and `#[allow(unused_imports)]` statements were identified in the codebase. These represent cleanup opportunities once we fully understand the purpose of each item.

**Scan Date:** 2026-01-22

### Dead Code Allow Statements (`#[allow(dead_code)]`)

| File                                  | Line | Comment/Context                                      |
| ------------------------------------- | ---- | ---------------------------------------------------- |
| `sindri/src/commands/project.rs`      | 1166 | Used for future dependency installation enhancements |
| `sindri/src/output.rs`                | 55   | Reserved for future use                              |
| `sindri-providers/src/kubernetes.rs`  | 905  | No comment                                           |
| `sindri-providers/src/kubernetes.rs`  | 913  | No comment                                           |
| `sindri-providers/src/fly.rs`         | 721  | Used in plan() for resource details                  |
| `sindri-providers/src/docker.rs`      | 291  | No comment                                           |
| `sindri-providers/src/utils.rs`       | 32   | Reserved for future use                              |
| `sindri-providers/src/utils.rs`       | 52   | Reserved for future use                              |
| `sindri-providers/src/utils.rs`       | 75   | Reserved for future use                              |
| `sindri-providers/src/utils.rs`       | 94   | Reserved for future use                              |
| `sindri-update/src/download.rs`       | 132  | No comment                                           |
| `sindri-core/src/retry/strategies.rs` | 229  | No comment                                           |
| `sindri-core/src/retry/strategies.rs` | 235  | No comment                                           |
| `sindri-core/src/retry/strategies.rs` | 255  | No comment                                           |
| `sindri-core/src/retry/strategies.rs` | 261  | No comment                                           |

### Unused Imports Allow Statements (`#[allow(unused_imports)]`)

| File                                | Line | Comment/Context                                        |
| ----------------------------------- | ---- | ------------------------------------------------------ |
| `sindri-secrets/src/s3/backend.rs`  | 411  | No comment                                             |
| `sindri-update/tests/common/mod.rs` | 25   | Module-level allow for test infrastructure scaffolding |

### Module-Level Allow Statements (`#![allow(...)]`)

| File                                | Line | Directive                   | Comment/Context                                             |
| ----------------------------------- | ---- | --------------------------- | ----------------------------------------------------------- |
| `sindri-update/tests/common/mod.rs` | 24   | `#![allow(dead_code)]`      | Test infrastructure scaffolded for future tests             |
| `sindri-update/tests/common/mod.rs` | 25   | `#![allow(unused_imports)]` | Re-exports for convenience, not all used in every test file |

### Summary

- **Total `#[allow(dead_code)]`**: 15 occurrences across 8 files
- **Total `#[allow(unused_imports)]`**: 2 occurrences across 2 files
- **Total `#![allow(...)]` (module-level)**: 2 occurrences in 1 file (test infrastructure)

### Cleanup Priority

| Priority   | Criteria                                    | Count |
| ---------- | ------------------------------------------- | ----- |
| **High**   | Items without comments (need investigation) | 10    |
| **Medium** | Items marked "Reserved for future use"      | 4     |
| **Low**    | Items with specific use-case comments       | 2     |

### Action Items

1. **Investigate uncommented items** - Determine if the code is truly dead or if comments are missing
2. **Review "future use" items** - Evaluate if still needed or if feature was abandoned
3. **Verify commented items** - Confirm the stated purpose is still valid
4. **Remove dead code** - Once understood, remove genuinely unused code

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

### 2026-01-22 (v1.7.0) - Tool Dependency Management & Clippy Cleanup

- ✅ **COMPLETED**: sindri-doctor crate implementation (all 4 phases)
  - Phase 1: Core doctor command with platform detection, tool registry, parallel checking
  - Phase 2: Authentication checking, version comparison, CI mode, verbose output
  - Phase 3: Auto-installation with `--fix`, `--yes`, `--dry-run` flags
  - Phase 4: Extension-specific tool checking with `--check-extensions` and `--extension` flags
  - 47 tests passing in sindri-doctor
- ✅ **COMPLETED**: Clippy warning cleanup across workspace
  - Fixed collapsible if in `sindri-doctor/src/reporter.rs`
  - Fixed 18 `io::Error::new(io::ErrorKind::Other, ...)` → `io::Error::other(...)` in sindri-core
  - Fixed 3 `field_reassign_with_default` warnings in sindri-projects git module
  - Added module-level `#![allow(dead_code)]` and `#![allow(unused_imports)]` to sindri-update test infrastructure
- ✅ Updated allow annotations tracking table with new entries
- ✅ All clippy warnings resolved - clean compile
- ✅ Full test suite passing

### 2026-01-22 (v1.6.0) - Configuration Refactoring Complete 🎉

- ✅ **COMPLETED**: All configuration refactoring phases (5/5)
- ✅ Implemented Phase 3: Retry & Network Policies
  - Created `sindri-core/src/retry/` module with 5 files
  - `RetryExecutor` with configurable strategies (None, Fixed, Exponential, Linear)
  - `RetryObserver` trait with TracingObserver, StatsObserver, NoOpObserver
  - `RetryPredicate` trait with composable predicates (AND/OR logic)
  - Jitter support for thundering herd prevention
  - 59 comprehensive unit tests
- ✅ Implemented Phase 4: Git Workflow Templates
  - Updated `init.rs`, `remote.rs`, `config.rs`, `clone.rs`
  - Added `detect_main_branch()` helper function
  - Configurable remote names (origin, upstream)
  - Configurable default branch and commit messages
  - 20 tests passing
- ✅ Implemented Phase 5: Test Refactoring
  - Created `tests/common/` module (8 files, 805 lines of reusable infrastructure)
  - Created `tests/fixtures/` directory (6 YAML fixture files)
  - Implemented builder patterns (ReleaseBuilder, ReleaseAssetBuilder)
  - Created assertion helpers, mock server helpers, extension factories
  - Refactored 3 test files with 20% line reduction (1,552→1,236 lines)
  - 72 tests passing
- ✅ All clippy warnings resolved
- ✅ Full workspace test suite passing (all crates)
- ✅ Configuration Refactoring Initiative: 100% complete

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

**Document Version:** 1.7.0
**Document Owner:** Sindri Core Team
**Review Frequency:** Weekly (High Priority), Monthly (Medium/Low Priority)
**Last Review:** 2026-01-22
