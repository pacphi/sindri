# V3 Extension & Packer Image Testing Framework

## Executive Summary

This document defines the comprehensive testing strategy for Sindri V3, covering:

1. **Extension Testing**: Full lifecycle testing of **44 extensions** across **5 runtime providers** (Docker, Fly.io, DevPod, E2B, Kubernetes) with **serial** and **parallel** execution schemes
2. **Packer Image Testing**: VM image building and validation across **5 cloud providers** (AWS, Azure, GCP, OCI, Alibaba) with InSpec compliance testing

**Status**: Active Implementation (87.5% Complete)
**Last Updated**: 2026-01-26
**Version**: 1.1.0

### Implementation Progress Summary

| Phase | Description                    | Status                                             |
| ----- | ------------------------------ | -------------------------------------------------- |
| 1     | Extension Test Infrastructure  | ✅ COMPLETE                                        |
| 2     | Extension Lifecycle Tests      | 🟡 PARTIAL (60%) - missing removal & upgrade tests |
| 3     | Extension CI Workflow          | ✅ COMPLETE                                        |
| 4     | Local Extension Testing        | ✅ COMPLETE                                        |
| 5     | Packer Test Infrastructure     | ✅ COMPLETE                                        |
| 6     | InSpec Controls Expansion      | ✅ COMPLETE (missing performance.rb)               |
| 7     | Packer CI Workflow Enhancement | ✅ COMPLETE                                        |
| 8     | Local Packer Testing           | ✅ COMPLETE                                        |

**Remaining Work:**

- `removal_lifecycle_tests.rs` - Extension removal tests
- `upgrade_lifecycle_tests.rs` - Extension upgrade tests
- `performance.rb` - InSpec performance controls

---

## Table of Contents

- [Part 1: Extension Testing](#part-1-extension-testing)
  - [Current State Assessment](#current-state-assessment-extensions)
  - [Extension Testing Schemes](#extension-testing-schemes)
  - [Runtime Provider Constraints](#runtime-provider-constraints)
  - [Test Infrastructure](#extension-test-infrastructure)
- [Part 2: Packer Image Testing](#part-2-packer-image-testing)
  - [Current State Assessment](#current-state-assessment-packer)
  - [Packer Testing Architecture](#packer-testing-architecture)
  - [Cloud Provider Matrix](#cloud-provider-matrix)
  - [InSpec Controls](#inspec-controls)
- [Implementation Plan](#implementation-plan)
- [File Changes Summary](#file-changes-summary)
- [Verification Plan](#verification-plan)
- [Success Criteria](#success-criteria)
- [Risk Mitigation](#risk-mitigation)
- [Research Sources](#research-sources)

---

## Part 1: Extension Testing

### Current State Assessment (Extensions)

**Effectiveness Rating: 8/10** _(Updated 2026-01-26)_

| Component                   | Status     | Effectiveness                                      |
| --------------------------- | ---------- | -------------------------------------------------- |
| Unit tests (37 total)       | ✅ Exists  | Medium - covers validation, manifest, dependencies |
| Lifecycle integration tests | ✅ Exists  | Good - install, validate, hooks, configure covered |
| Hook execution tests        | ✅ Exists  | Good - hooks_lifecycle_tests.rs implemented        |
| Provider-based tests        | 🟡 Partial | Docker tested, others TBD                          |
| V2 extension workflow       | ✅ Exists  | Good pattern to follow                             |
| sindri-test.sh              | ✅ Exists  | Good - serial/parallel/profile/quick levels        |
| Test infrastructure         | ✅ Exists  | Excellent - builders, mocks, assertions, fixtures  |
| CI workflow                 | ✅ Exists  | Good - v3-extension-test.yml with matrix support   |

### Remaining Gaps _(Updated 2026-01-26)_

1. ~~**NO lifecycle integration tests**~~ ✅ RESOLVED - install, validate, hooks, configure tests exist
2. ~~**NO hook testing**~~ ✅ RESOLVED - hooks_lifecycle_tests.rs implemented
3. **Removal lifecycle tests** - `removal_lifecycle_tests.rs` not yet created
4. **Upgrade lifecycle tests** - `upgrade_lifecycle_tests.rs` not yet created
5. ~~**NO serial vs parallel execution modes**~~ ✅ RESOLVED - v3-extension-test.sh supports both
6. ~~**Test helpers scattered**~~ ✅ RESOLVED - Consolidated in sindri-extensions/tests/common/

### Extension Testing Schemes

#### Scheme A: Serial Execution

```
┌─────────────────┐    ┌─────────────────────────────────────────────┐
│ Single Instance │───▶│ Ext1→Ext2→Ext3→...→ExtN (sequential)        │
└─────────────────┘    └─────────────────────────────────────────────┘
```

- **Use Case**: Resource-constrained environments, dependency validation
- **Pros**: Lower resource usage, easier debugging, deterministic order
- **Cons**: Slower execution, no isolation between tests

#### Scheme B: Parallel Execution

```
┌──────────────┐    ┌────────┐    ┌────────┐    ┌────────┐
│ Instance 1   │───▶│ Ext1   │    │ Ext3   │    │ Ext5   │
├──────────────┤    ├────────┤    ├────────┤    ├────────┤
│ Instance 2   │───▶│ Ext2   │    │ Ext4   │    │ Ext6   │
└──────────────┘    └────────┘    └────────┘    └────────┘
```

- **Use Case**: CI/CD pipelines, full regression testing
- **Pros**: Faster execution, isolated failures
- **Cons**: Higher resource usage, more complex orchestration

### Runtime Provider Constraints

| Provider               | max-parallel | Memory/Instance | Cost | Startup Time |
| ---------------------- | ------------ | --------------- | ---- | ------------ |
| Docker (local)         | 2            | ~7GB shared     | Free | ~5s          |
| Fly.io                 | 8            | Configurable    | $$$  | ~30s         |
| DevPod (AWS/GCP/Azure) | 6            | Configurable    | $$$$ | ~60s         |
| E2B                    | 10           | Sandboxed       | $$   | ~10s         |
| Kind/K3d               | 1            | Single cluster  | Free | ~45s         |

### Extension Test Infrastructure

#### Test Crate Structure

```
v3/crates/sindri-extensions/tests/
├── common/
│   ├── mod.rs              # Module exports
│   ├── constants.rs        # Test constants
│   ├── builders.rs         # ExtensionBuilder
│   ├── fixtures.rs         # Fixture loading
│   ├── mocks.rs            # MockExecutor, MockProvider, MockFilesystem
│   ├── assertions.rs       # Lifecycle assertions
│   └── test_extensions.rs  # Pre-defined test extensions
├── fixtures/
│   ├── extensions/         # Test extension YAMLs
│   └── manifests/          # Test manifest YAMLs
├── unit/                   # Unit tests per module
├── integration/            # Lifecycle integration tests
│   ├── install_lifecycle_tests.rs
│   ├── validate_lifecycle_tests.rs
│   ├── removal_lifecycle_tests.rs
│   ├── upgrade_lifecycle_tests.rs
│   └── hooks_lifecycle_tests.rs
└── e2e/                    # Feature-gated real extension tests
```

#### Test Dependencies

```toml
[dev-dependencies]
tokio-test = "0.4"
wiremock = "0.6"
mockall = "0.13"
proptest = "1.5"
test-case = "3.3"
assert_fs = "1.1"
predicates = "3.1"
```

### Lifecycle Coverage Matrix

| Lifecycle Phase  | Test Cases                                   | Priority |
| ---------------- | -------------------------------------------- | -------- |
| Install (script) | Success, timeout, failure, hooks             | P0       |
| Install (mise)   | Config loading, reshim, tool install         | P0       |
| Install (binary) | Download, extract, permissions               | P1       |
| Install (hybrid) | Multi-method sequencing                      | P1       |
| Validation       | Command checks, pattern matching, mise tools | P0       |
| Configuration    | Templates, environment vars                  | P1       |
| Hooks            | Pre/post install, failure handling           | P0       |
| Removal          | Script cleanup, manifest update              | P1       |
| Upgrade          | Version check, in-place, rollback            | P2       |
| Dependencies     | Resolution order, circular detection         | P0       |

---

## Part 2: Packer Image Testing

### Current State Assessment (Packer)

**Effectiveness Rating: 6/10**

| Component                    | Status    | Effectiveness                              |
| ---------------------------- | --------- | ------------------------------------------ |
| sindri-packer crate          | ✅ Exists | Good - 5 cloud providers                   |
| HCL2 templates (Tera)        | ✅ Exists | Good - embedded templates                  |
| v3-packer-build.yml workflow | ✅ Exists | Good - parallel cloud builds               |
| v3-packer-test.yml workflow  | ✅ Exists | Medium - InSpec tests exist                |
| InSpec controls              | ✅ Exists | Basic - sindri_installed, docker_installed |
| Unit tests per provider      | ✅ Exists | Medium - template generation tests         |
| Security hardening           | ✅ Exists | CIS benchmarks implemented                 |

### Critical Gaps

1. **Limited InSpec coverage** - only 2 control groups (sindri, docker)
2. **NO extension validation in images** - pre-installed extensions not tested
3. **NO cross-cloud image parity tests** - images may differ between clouds
4. **NO image boot/connectivity tests** - SSH validation is basic (60s wait)
5. **NO security scan integration** - OpenSCAP planned but not implemented
6. **NO image lifecycle tests** - build→deploy→validate→destroy not automated

### Packer Testing Architecture

#### Image Build Pipeline

```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│   Template   │───▶│    Build     │───▶│  Validate    │───▶│   Publish    │
│  Generation  │    │   (Packer)   │    │   (InSpec)   │    │   (Share)    │
└──────────────┘    └──────────────┘    └──────────────┘    └──────────────┘
        │                   │                   │                   │
        ▼                   ▼                   ▼                   ▼
   Tera render         packer build      InSpec profile      AMI/Image ID
   HCL2 output         per cloud         compliance tests    Multi-region
```

### Cloud Provider Matrix

| Provider | Builder       | Base Image   | Default Region | Test Instance       |
| -------- | ------------- | ------------ | -------------- | ------------------- |
| AWS      | amazon-ebs    | Ubuntu 24.04 | us-west-2      | t3.large            |
| Azure    | azure-arm     | Ubuntu 24.04 | westus2        | Standard_D4s_v4     |
| GCP      | googlecompute | Ubuntu 24.04 | us-west1-a     | e2-standard-4       |
| OCI      | oracle-oci    | Ubuntu 24.04 | AD-specific    | VM.Standard.E4.Flex |
| Alibaba  | alicloud-ecs  | Ubuntu 24.04 | cn-hangzhou    | ecs.g6.xlarge       |

### InSpec Controls

#### Planned Control Groups

| Control Group | Controls | Priority | Description                                     |
| ------------- | -------- | -------- | ----------------------------------------------- |
| sindri        | 4        | P0       | CLI installed, directories exist, doctor passes |
| docker        | 3        | P0       | Docker installed, running, user in group        |
| extensions    | 5+       | P0       | Extension manager, pre-installed extensions     |
| mise          | 3        | P1       | Mise installed, configured, tools available     |
| security      | 8+       | P1       | SSH hardening, firewall, updates                |
| performance   | 3        | P2       | Disk space, memory, network                     |

---

## Implementation Plan

### Phase Timeline _(Updated 2026-01-26)_

| Phase | Description                    | Duration | Status           |
| ----- | ------------------------------ | -------- | ---------------- |
| 0     | Documentation                  | Day 1    | ✅ Complete      |
| 1     | Extension Test Infrastructure  | Week 1-2 | ✅ Complete      |
| 2     | Extension Lifecycle Tests      | Week 2-3 | 🟡 Partial (60%) |
| 3     | Extension CI Workflow          | Week 3-4 | ✅ Complete      |
| 4     | Local Extension Testing        | Week 4-5 | ✅ Complete      |
| 5     | Packer Test Infrastructure     | Week 5-6 | ✅ Complete      |
| 6     | InSpec Controls Expansion      | Week 6-7 | ✅ Complete      |
| 7     | Packer CI Workflow Enhancement | Week 7-8 | ✅ Complete      |
| 8     | Local Packer Testing           | Week 8   | ✅ Complete      |

### Phase 1: Extension Test Infrastructure

**Objective**: Create the foundational test infrastructure for sindri-extensions

**Deliverables**:

- Test helper modules (common/\*)
- Test fixtures directory structure
- Mock implementations for Executor, Provider, Filesystem
- Extension builder utilities

### Phase 2: Extension Lifecycle Tests

**Objective**: Comprehensive integration tests for all lifecycle phases

**Key Test Categories**:

1. Installation tests (all 6 methods)
2. Validation tests
3. Hook execution tests
4. Dependency resolution tests
5. Error handling tests

### Phase 3: Extension CI Workflow

**Objective**: GitHub Actions workflow for automated extension testing

**Workflow File**: `.github/workflows/test-extensions-v3.yml`

```yaml
name: V3 Extension Tests

on:
  workflow_call:
    inputs:
      scheme:
        type: string
        default: "parallel"
      extensions:
        type: string
        default: "all"
      provider:
        type: string
        default: "docker"
      max-parallel:
        type: number
        default: 4

jobs:
  generate-matrix:
    # Dynamic extension discovery
    # Filter heavy extensions (>4GB)

  test-parallel:
    if: inputs.scheme == 'parallel'
    strategy:
      fail-fast: false
      max-parallel: ${{ inputs.max-parallel }}

  test-serial:
    if: inputs.scheme == 'serial'
    # Single job, sequential execution
```

### Phase 4: Local Extension Testing

**Objective**: Makefile targets for local development testing

**Targets**:

- `v3-ext-test-serial`
- `v3-ext-test-parallel`
- `v3-ext-test-profile`
- `v3-ext-test-quick`

### Phase 5: Packer Test Infrastructure

**Objective**: Expand sindri-packer test helpers

**Deliverables**:

- Mock cloud API helpers
- Template rendering utilities
- Build lifecycle test helpers

### Phase 6: InSpec Controls Expansion

**Objective**: Comprehensive InSpec compliance profile

**New Control Files**:

- `extensions.rb` - Extension validation
- `security.rb` - Security hardening
- `mise.rb` - Mise environment
- `performance.rb` - Performance baselines

### Phase 7: Packer CI Workflow Enhancement

**Objective**: Enhanced v3-packer-test.yml with extension validation

**Enhancements**:

- Extension validation step
- Security test integration
- Cross-cloud parity checks

### Phase 8: Local Packer Testing

**Objective**: Makefile targets for local Packer testing

**Targets**:

- `v3-packer-validate`
- `v3-packer-test-local`
- `v3-packer-test`
- `v3-inspec-check`

---

## File Changes Summary

### New Files

| File                                                      | Purpose                   |
| --------------------------------------------------------- | ------------------------- |
| **Strategy Documentation**                                |                           |
| `v3/docs/planning/active/overarching-testing-strategy.md` | This document             |
| **Extension Testing**                                     |                           |
| `v3/crates/sindri-extensions/tests/common/mod.rs`         | Test module exports       |
| `v3/crates/sindri-extensions/tests/common/constants.rs`   | Test constants            |
| `v3/crates/sindri-extensions/tests/common/builders.rs`    | Extension builders        |
| `v3/crates/sindri-extensions/tests/common/fixtures.rs`    | Fixture loading           |
| `v3/crates/sindri-extensions/tests/common/mocks.rs`       | Mock implementations      |
| `v3/crates/sindri-extensions/tests/common/assertions.rs`  | Lifecycle assertions      |
| `v3/crates/sindri-extensions/tests/integration/*.rs`      | Lifecycle tests           |
| `.github/workflows/test-extensions-v3.yml`                | Extension CI workflow     |
| `.github/actions/v3/test-extension/action.yml`            | Composite action          |
| `scripts/v3-extension-test.sh`                            | Local test runner         |
| **Packer Testing**                                        |                           |
| `v3/crates/sindri-packer/tests/common/mod.rs`             | Test module exports       |
| `v3/crates/sindri-packer/tests/common/mock_cloud.rs`      | Cloud API mocks           |
| `v3/crates/sindri-packer/tests/common/assertions.rs`      | Build assertions          |
| `v3/crates/sindri-packer/tests/integration/*.rs`          | Build lifecycle tests     |
| `v3/test/integration/sindri/controls/extensions.rb`       | Extension InSpec controls |
| `v3/test/integration/sindri/controls/security.rb`         | Security InSpec controls  |
| `v3/test/integration/sindri/controls/mise.rb`             | Mise InSpec controls      |
| `.github/actions/packer/launch-instance/action.yml`       | Instance launch action    |
| `.github/actions/packer/terminate-instance/action.yml`    | Instance cleanup action   |
| `scripts/v3-packer-test.sh`                               | Local Packer test script  |

### Modified Files

| File                                     | Changes                           |
| ---------------------------------------- | --------------------------------- |
| `v3/crates/sindri-extensions/Cargo.toml` | Add test dependencies             |
| `v3/crates/sindri-packer/Cargo.toml`     | Add test dependencies             |
| `Makefile`                               | Add v3-ext-_, v3-packer-_ targets |
| `.github/workflows/ci-v3.yml`            | Integrate test workflows          |
| `.github/workflows/v3-packer-test.yml`   | Enhanced InSpec testing           |
| `v3/test/integration/sindri/inspec.yml`  | Add new control dependencies      |

---

## Verification Plan

### Documentation Verification

```bash
# Verify strategy document exists and is well-formed
cat v3/docs/planning/active/overarching-testing-strategy.md | head -50

# Check document is in active planning
ls -la v3/docs/planning/active/
```

### Extension Testing Verification

```bash
# Unit tests
cd v3 && cargo test --package sindri-extensions

# Integration tests
cd v3 && cargo test --package sindri-extensions --test integration

# Local serial testing
make v3-ext-test-serial V3_EXT_LIST="nodejs,python,golang"

# Local parallel testing
make v3-ext-test-parallel V3_EXT_LIST="nodejs,python" V3_EXT_MAX_PARALLEL=2

# CI workflow
gh workflow run test-extensions-v3.yml -f scheme=parallel -f extensions=minimal
```

### Packer Testing Verification

```bash
# Unit tests
cd v3 && cargo test --package sindri-packer

# Integration tests (feature-gated)
cd v3 && cargo test --package sindri-packer --features cloud-tests

# Template validation
make v3-packer-validate

# InSpec profile check
make v3-inspec-check

# Full packer test (requires cloud credentials)
gh workflow run v3-packer-test.yml -f clouds='["aws"]' -f profile=minimal
```

### End-to-End Verification

```bash
# Build image with extensions, test it, validate extensions work
gh workflow run v3-packer-build.yml \
  -f clouds='["aws"]' \
  -f profile=ai-dev \
  -f extensions=python,nodejs,rust

# Then run comprehensive tests
gh workflow run v3-packer-test.yml \
  -f run-security-tests=true
```

---

## Success Criteria

### Extension Testing (Criteria 1-6)

| #   | Criterion          | Metric                | Target                                                     |
| --- | ------------------ | --------------------- | ---------------------------------------------------------- |
| 1   | Lifecycle coverage | Phases tested         | 6/6 (install, validate, configure, hooks, remove, upgrade) |
| 2   | Extension coverage | Extensions with tests | 44/44                                                      |
| 3   | Provider coverage  | Providers tested      | Docker (primary), Fly.io (CI), DevPod (manual)             |
| 4   | Scheme support     | Execution schemes     | Serial ✓, Parallel ✓                                       |
| 5   | Local dev          | Make targets working  | `v3-ext-test-*` commands                                   |
| 6   | CI integration     | Workflow integration  | test-extensions-v3.yml in ci-v3.yml                        |

### Packer Testing (Criteria 7-13)

| #   | Criterion            | Metric               | Target                             |
| --- | -------------------- | -------------------- | ---------------------------------- |
| 7   | InSpec coverage      | Control groups       | 15+ controls                       |
| 8   | Cloud coverage       | Clouds tested        | AWS, Azure, GCP (primary)          |
| 9   | Extension validation | Extensions verified  | Pre-installed extensions in images |
| 10  | Security validation  | CIS controls         | Hardening verified via InSpec      |
| 11  | Template testing     | Templates rendered   | 5/5 cloud templates                |
| 12  | Cache testing        | Cache logic verified | Lookup/miss scenarios              |
| 13  | Local dev            | Make targets working | `v3-packer-*` commands             |

---

## Risk Mitigation

| Risk                        | Impact   | Likelihood | Mitigation                                            |
| --------------------------- | -------- | ---------- | ----------------------------------------------------- |
| Heavy extensions OOM        | High     | Medium     | Filter with HEAVY_EXTENSIONS list, test on Fly.io     |
| Flaky network tests         | Medium   | High       | Retry logic, wiremock for deterministic mocking       |
| Cloud credential exposure   | Critical | Low        | Feature-gate cloud tests, use OIDC                    |
| InSpec SSH timeouts         | Medium   | Medium     | Increase wait time, add connectivity checks           |
| Cross-cloud parity failures | Low      | Medium     | Document expected differences, skip where appropriate |
| Packer build costs          | Medium   | Medium     | Use smallest instance types, clean up immediately     |
| K8s test instability        | Medium   | High       | Remove continue-on-error, fix or skip flaky tests     |

---

## Research Sources

### Extension Testing

- [E2E Testing Frameworks 2026](https://www.kellton.com/kellton-tech-blog/ultimate-guide-end-to-end-testing-tools-frameworks-2026)
- [Tokio Testing Best Practices](https://tokio.rs/tokio/topics/testing)
- [GitHub Actions Matrix Strategy](https://codefresh.io/learn/github-actions/github-actions-matrix/)
- [Testcontainers Infrastructure](https://testcontainers.com/)

### Packer Testing

- [Packer Image Testing with ServerSpec](https://medium.com/sumup-engineering/image-creation-and-testing-with-hashicorp-packer-and-serverspec-bb2bd065441)
- [Testing Packer Builds with ServerSpec](https://annaken.github.io/testing-packer-builds-with-serverspec/)
- [InSpec vs ServerSpec Comparison](https://medium.com/@Joachim8675309/serverspec-vs-inspec-17272df2718f)
- [Goss for Image Validation](https://image-builder.sigs.k8s.io/capi/goss/goss)
- [HCP Packer CI/CD Workflows](https://developer.hashicorp.com/validated-patterns/terraform/vulnerability-and-patch-management)
- [Packer Validate Command](https://developer.hashicorp.com/packer/docs/commands/validate)

---

## Appendix A: Extension Categories

| Category      | Count | Examples                      |
| ------------- | ----- | ----------------------------- |
| languages     | 12    | python, nodejs, golang, rust  |
| ai-dev        | 8     | claude-code, cursor, windsurf |
| devops        | 6     | docker, terraform, kubernetes |
| mcp           | 5     | mcp-core, mcp-servers         |
| cloud         | 4     | aws-cli, gcloud, azure-cli    |
| testing       | 3     | playwright, cypress           |
| productivity  | 3     | tmux, neovim                  |
| documentation | 2     | mkdocs, docusaurus            |
| research      | 1     | perplexity                    |

---

## Appendix B: Heavy Extensions (>4GB)

These extensions require special handling in CI due to memory/disk constraints:

| Extension   | Size | Reason                | CI Strategy      |
| ----------- | ---- | --------------------- | ---------------- |
| cuda        | ~8GB | GPU drivers + toolkit | Fly.io only      |
| ollama      | ~6GB | LLM runtime + models  | Skip in matrix   |
| android-sdk | ~5GB | Android build tools   | DevPod only      |
| xcode-cli   | ~4GB | macOS only            | Skip in Linux CI |

---

## Appendix C: Test Fixtures

### Minimal Test Extension

```yaml
metadata:
  name: test-minimal
  version: "1.0.0"
  description: Minimal test extension
  category: testing

install:
  method: script
  script:
    path: scripts/install.sh
    timeout: 60

validate:
  commands:
    - name: echo
      versionFlag: "test"
```

### Full-Featured Test Extension

```yaml
metadata:
  name: test-full
  version: "1.0.0"
  description: Full-featured test extension
  category: testing

requirements:
  install_timeout: 300
  memory_gb: 2
  disk_gb: 5

install:
  method: hybrid
  script:
    path: scripts/install.sh
    timeout: 120
  mise:
    configFile: mise.toml
    reshim_after_install: true

validate:
  commands:
    - name: test-cmd
      versionFlag: "--version"
      expected_pattern: "test-cmd \\d+\\.\\d+\\.\\d+"

capabilities:
  hooks:
    pre_install:
      command: "echo 'Pre-install hook'"
      description: "Runs before installation"
    post_install:
      command: "echo 'Post-install hook'"
      description: "Runs after installation"
```

---

_Document generated as part of Sindri V3 Testing Strategy Initiative_
