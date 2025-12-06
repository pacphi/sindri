# CI Testing Deep Dive: Extension and Integration Tests

This document provides an in-depth analysis of how extension tests and integration
tests function within the Sindri CI workflow, including step-by-step flows,
inputs/outputs, and detailed explanations of what is tested and how.

## Table of Contents

- [CI Testing Deep Dive: Extension and Integration Tests](#ci-testing-deep-dive-extension-and-integration-tests)
  - [Table of Contents](#table-of-contents)
  - [Overview](#overview)
  - [CI Workflow Architecture](#ci-workflow-architecture)
    - [Entry Point: `.github/workflows/ci.yml`](#entry-point-githubworkflowsciyml)
    - [Provider Matrix Generation](#provider-matrix-generation)
  - [Extension Tests](#extension-tests)
    - [Phase 1: Schema Validation (Static)](#phase-1-schema-validation-static)
    - [Phase 2: Cross-Reference Validation](#phase-2-cross-reference-validation)
    - [Phase 3: Runtime Extension Tests](#phase-3-runtime-extension-tests)
      - [Test Suite Mapping](#test-suite-mapping)
      - [All Test Phases](#all-test-phases)
      - [Detailed Phase Descriptions](#detailed-phase-descriptions)
        - [Phase 4: Functionality Tests](#phase-4-functionality-tests)
        - [Phase 5: Idempotency Tests](#phase-5-idempotency-tests)
        - [Phase 6: File System Checks](#phase-6-file-system-checks)
        - [Phase 7: Environment Checks](#phase-7-environment-checks)
        - [Phase 8: Uninstall \& Cleanup](#phase-8-uninstall--cleanup)
  - [Integration Tests](#integration-tests)
    - [Smoke Tests](#smoke-tests)
    - [Integration Test Suite](#integration-test-suite)
    - [Full Test Suite](#full-test-suite)
  - [Test Execution by Provider](#test-execution-by-provider)
    - [How `run_on_provider` Works](#how-run_on_provider-works)
    - [CLI Tests Flow](#cli-tests-flow)
  - [Resource Calculation](#resource-calculation)
  - [Helper Libraries](#helper-libraries)
    - [Test Helpers (`lib/test-helpers.sh`)](#test-helpers-libtest-helperssh)
    - [Assertions (`lib/assertions.sh`)](#assertions-libassertionssh)
  - [Debugging Failed Tests](#debugging-failed-tests)
    - [Viewing Test Results](#viewing-test-results)
    - [Log Artifacts](#log-artifacts)
    - [Manual Debugging](#manual-debugging)
    - [Common Failure Scenarios](#common-failure-scenarios)
  - [Related Documentation](#related-documentation)

---

## Overview

The Sindri CI system implements a **unified provider testing** model where each
selected provider (Docker, Fly.io, DevPod variants) receives complete test
coverage including:

1. **CLI Tests** - Validate sindri and extension-manager commands work
2. **Extension Tests** - Validate extension installation, configuration, and functionality
3. **Integration Tests** - End-to-end workflow validation

```text
┌─────────────────────────────────────────────────────────────────┐
│                      CI TESTING PIPELINE                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐          │
│  │  VALIDATION │───>│    BUILD    │───>│    TEST     │          │
│  │   (Static)  │    │   (Image)   │    │  (Runtime)  │          │
│  └─────────────┘    └─────────────┘    └─────────────┘          │
│        │                  │                  │                  │
│        v                  v                  v                  │
│  ┌───────────┐     ┌───────────┐     ┌─────────────────┐        │
│  │ shellcheck│     │ Dockerfile│     │ test-provider   │        │
│  │ yamllint  │     │ Multi-stg │     │ (per provider)  │        │
│  │ markdownln│     │   build   │     │                 │        │
│  │ schemas   │     └───────────┘     │ - CLI tests     │        │
│  └───────────┘                       │ - Ext tests     │        │
│                                      │ - Integration   │        │
│                                      └─────────────────┘        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## CI Workflow Architecture

### Entry Point: `.github/workflows/ci.yml`

The main CI orchestrator triggers on:

- **Push** to `main` or `develop` branches
- **Pull requests** to `main` or `develop`
- **Manual dispatch** with custom parameters

**Workflow Inputs (manual dispatch):**

| Input               | Type    | Default                 | Description                         |
| ------------------- | ------- | ----------------------- | ----------------------------------- |
| `providers`         | string  | `docker,fly,devpod-k8s` | Comma-separated list or `all`       |
| `test-suite`        | choice  | `integration`           | `smoke`, `integration`, or `full`   |
| `extension-profile` | choice  | `minimal`               | Profile to test                     |
| `skip-cleanup`      | boolean | `false`                 | Skip resource cleanup for debugging |

**Job Dependency Graph:**

```text
shellcheck ─────┐
                │
markdownlint ───┼──> build ──> test-providers (matrix)
                │        │
validate-yaml ──┘        └──> ci-required ──> ci-status
                                   │
generate-matrix ───────────────────┘
```

### Provider Matrix Generation

The `generate-matrix` job determines which providers to test:

**Input:** Event type and manual provider selection

**Output:** JSON array of providers, e.g., `["docker", "fly", "devpod-k8s"]`

**Logic:**

```bash
if [[ "${{ github.event_name }}" == "workflow_dispatch" ]]; then
  # Use user-specified providers
  INPUT_PROVIDERS="${{ github.event.inputs.providers }}"
  if [[ "$INPUT_PROVIDERS" == "all" ]]; then
    PROVIDERS='["docker", "fly", "devpod-aws", "devpod-gcp", ...]'
  else
    PROVIDERS=$(echo "$INPUT_PROVIDERS" | jq -Rc 'split(",")')
  fi
else
  # Default for push/PR events
  PROVIDERS='["docker", "fly", "devpod-k8s"]'
fi
```

---

## Extension Tests

Extension testing occurs in multiple phases, some static (pre-deployment) and
some runtime (post-deployment).

### Phase 1: Schema Validation (Static)

**Location:** `.github/workflows/validate-yaml.yml`

**When:** Before any deployment, as part of validation jobs

**What is Tested:**

Each `extension.yaml` file is validated against `docker/lib/schemas/extension.schema.json`

**Flow:**

```text
┌───────────────────────────────────────────────────────────────────┐
│                    SCHEMA VALIDATION FLOW                         │
├───────────────────────────────────────────────────────────────────┤
│                                                                   │
│  For each extension in docker/lib/extensions/*/extension.yaml:    │
│                                                                   │
│  1. Convert YAML to JSON                                          │
│     ┌─────────────────────────────────────────────────────────┐   │
│     │  yq -o=json "$ext" > "$tmpfile"                         │   │
│     └─────────────────────────────────────────────────────────┘   │
│                          │                                        │
│                          v                                        │
│  2. Validate against schema using ajv-cli                         │
│     ┌─────────────────────────────────────────────────────────┐   │
│     │  ajv validate \                                         │   │
│     │    -c ajv-formats \                                     │   │
│     │    -s docker/lib/schemas/extension.schema.json \        │   │
│     │    -d "$tmpfile"                                        │   │
│     └─────────────────────────────────────────────────────────┘   │
│                          │                                        │
│                          v                                        │
│  3. Report pass/fail                                              │
│     ┌─────────────────────────────────────────────────────────┐   │
│     │  SUCCESS: "Validating: docker/lib/extensions/nodejs/..."│   │
│     │  FAILURE: Detailed error with line numbers              │   │
│     └─────────────────────────────────────────────────────────┘   │
│                                                                   │
└───────────────────────────────────────────────────────────────────┘
```

**Schema Validation Checks:**

| Field                    | Validation                                          |
| ------------------------ | --------------------------------------------------- |
| `metadata.name`          | Required, must match directory name                 |
| `metadata.version`       | Semver format                                       |
| `metadata.category`      | Must be valid category                              |
| `install.method`         | One of: `mise`, `script`, `apt`, `binary`, `hybrid` |
| `validate.commands[]`    | Must have `name`, optional `expectedPattern`        |
| `requirements.diskSpace` | Integer (MB)                                        |
| `requirements.domains`   | Array of domain strings                             |

**Inputs:**

- `docker/lib/extensions/*/extension.yaml` files
- `docker/lib/schemas/extension.schema.json`

**Outputs:**

- Pass/fail status for each extension
- Aggregated failure count
- Exit code 0 (success) or 1 (failures)

### Phase 2: Cross-Reference Validation

**Location:** `.github/workflows/validate-yaml.yml` → `cross-references` job

**Script:** `test/unit/yaml/test-cross-references.sh`

**What is Tested:**

1. **Registry Consistency:** Each extension in `registry.yaml` exists in
   `docker/lib/extensions/`
2. **Profile Validity:** Each extension referenced in `profiles.yaml` exists
3. **Category Consistency:** Extension categories match between `extension.yaml`
   and `registry.yaml`
4. **Naming Consistency:** Directory names match `metadata.name` values

**Flow:**

```text
┌─────────────────────────────────────────────────────────────────────┐
│                  CROSS-REFERENCE VALIDATION                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────────────────────────────────────────────┐        │
│  │  Check: registry.yaml extensions exist                  │        │
│  │                                                         │        │
│  │  for ext in $(yq '.extensions | keys | .[]' registry);  │        │
│  │    if [[ ! -d "docker/lib/extensions/$ext" ]]; then     │        │
│  │      FAIL "$ext in registry but missing directory"      │        │
│  │    fi                                                   │        │
│  └─────────────────────────────────────────────────────────┘        │
│                          │                                          │
│                          v                                          │
│  ┌─────────────────────────────────────────────────────────┐        │
│  │  Check: profiles.yaml extensions exist                  │        │
│  │                                                         │        │
│  │  for profile in $(yq '.profiles | keys | .[]' ...);     │        │
│  │    for ext in $(yq ".profiles.$profile.extensions[]");  │        │
│  │      if [[ ! -d "docker/lib/extensions/$ext" ]]; then   │        │
│  │        FAIL "$ext in profile $profile but missing"      │        │
│  │      fi                                                 │        │
│  └─────────────────────────────────────────────────────────┘        │
│                          │                                          │
│                          v                                          │
│  ┌─────────────────────────────────────────────────────────┐        │
│  │  Check: directory name matches metadata.name            │        │
│  │                                                         │        │
│  │  for ext_dir in docker/lib/extensions/*/; do            │        │
│  │    dir_name=$(basename "$ext_dir")                      │        │
│  │    yaml_name=$(yq '.metadata.name' extension.yaml)      │        │
│  │    if [[ "$dir_name" != "$yaml_name" ]]; then           │        │
│  │      FAIL "Mismatch: $dir_name vs $yaml_name"           │        │
│  │    fi                                                   │        │
│  └─────────────────────────────────────────────────────────┘        │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Phase 3: Runtime Extension Tests

**Location:** `.github/workflows/test-provider.yml` → `extension-tests` step

**When:** After infrastructure deployment, on the running container/VM

**What is Tested:**

Runtime extension tests provide comprehensive validation of extensions with up to 9 phases,
depending on the test suite. Visual separators in the log output make each phase easy to identify.

#### Test Suite Mapping

The number of phases executed depends on the test suite:

| Test Suite    | Phases Run | Description                                          |
| ------------- | ---------- | ---------------------------------------------------- |
| `smoke`       | 1, 2, 3, 9 | Quick validation (install, validate, summary)        |
| `integration` | 1-9        | Full lifecycle including functionality and uninstall |
| `full`        | 1-9        | Comprehensive testing with all phases                |

#### All Test Phases

| Phase | Name                 | Test Suite        | Purpose                                          |
| ----- | -------------------- | ----------------- | ------------------------------------------------ |
| 1     | Profile Installation | All               | Install the extension profile                    |
| 2     | Extension Discovery  | All               | List extensions in the profile                   |
| 3     | Extension Validation | All               | Validate each extension                          |
| 4     | Functionality Tests  | integration, full | Extension-specific command and tool verification |
| 5     | Idempotency Tests    | integration, full | Verify reinstallation works correctly            |
| 6     | File System Checks   | integration, full | Verify expected files exist                      |
| 7     | Environment Checks   | integration, full | Verify environment variables are set             |
| 8     | Uninstall & Cleanup  | integration, full | Remove extensions and verify cleanup             |
| 9     | Results Summary      | All               | Aggregate and report results                     |

**Example Log Output (Integration/Full Test Suites):**

```text
========================================
  PHASE 1: PROFILE INSTALLATION
========================================

Profile: minimal
Provider: fly

✅ Profile 'minimal' installed successfully

========================================
  PHASE 2: EXTENSION DISCOVERY
========================================

Extensions in profile 'minimal': 2
  - nodejs
  - python

========================================
  PHASE 3: EXTENSION VALIDATION
========================================

[1/2] Validating: nodejs
        ✅ nodejs - PASSED
[2/2] Validating: python
        ✅ python - PASSED

----------------------------------------
Validation Summary: 2 passed, 0 failed
----------------------------------------

========================================
  PHASE 4: FUNCTIONALITY TESTS
========================================

[nodejs] Running functionality tests...
  ✅ node command
  ✅ npm command
[python] Running functionality tests...
  ✅ python command
  ✅ pip command

----------------------------------------
Functionality Tests: 4 passed, 0 failed
----------------------------------------

========================================
  PHASE 5: IDEMPOTENCY TESTS
========================================

Reinstalling profile 'minimal'...
        ✅ Profile reinstalled successfully
Validating extensions after reinstall...
        ✅ All extensions still valid (2/2)

========================================
  PHASE 6: FILE SYSTEM CHECKS
========================================

[nodejs] ✅ .mise.toml exists
[python] ✅ .mise.toml exists

========================================
  PHASE 7: ENVIRONMENT CHECKS
========================================

[nodejs] ✅ NODE_ENV is set (development)
[python] ✅ PYTHONPATH is set

========================================
  PHASE 8: UNINSTALL & CLEANUP
========================================

Removing extensions in reverse order...
[1/2] Removing: python
        ✅ python removed
[2/2] Removing: nodejs
        ✅ nodejs removed

Verifying uninstall...
        ✅ All extensions removed (2/2)

----------------------------------------
Uninstall Summary: 2 removed, 0 failed
----------------------------------------

========================================
  PHASE 9: RESULTS SUMMARY
========================================

Profile Installation: passed
Extensions Tested: 2
Test Suite: integration
Overall Status: ✅ PASSED
```

**Execution Flow:**

```text
┌─────────────────────────────────────────────────────────────────────┐
│                   RUNTIME EXTENSION TESTS                           │
│              (runs inside deployed environment)                     │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Phases 1-3, 9: Run for ALL test suites                             │
│  Phases 4-8: Run for integration/full test suites only              │
│                                                                     │
│  ┌───────────────────────────────────────────────────────┐          │
│  │  PHASE 1: Profile Installation                        │          │
│  │  - Install profile via extension-manager              │          │
│  │  - Resolve dependencies in topological order          │          │
│  └───────────────────────────────────────────────────────┘          │
│                          │                                          │
│                          v                                          │
│  ┌───────────────────────────────────────────────────────┐          │
│  │  PHASE 2: Extension Discovery                         │          │
│  │  - Query profile for extension list                   │          │
│  │  - Count extensions for reporting                     │          │
│  └───────────────────────────────────────────────────────┘          │
│                          │                                          │
│                          v                                          │
│  ┌───────────────────────────────────────────────────────┐          │
│  │  PHASE 3: Extension Validation                        │          │
│  │  - Run extension-manager validate for each            │          │
│  │  - Check commands exist and patterns match            │          │
│  └───────────────────────────────────────────────────────┘          │
│                          │                                          │
│                          v                                          │
│             [integration/full only from here]                       │
│                          │                                          │
│                          v                                          │
│  ┌───────────────────────────────────────────────────────┐          │
│  │  PHASE 4: Functionality Tests                         │          │
│  │  - Extension-specific command tests                   │          │
│  │  - Verify tools work correctly                        │          │
│  └───────────────────────────────────────────────────────┘          │
│                          │                                          │
│                          v                                          │
│  ┌───────────────────────────────────────────────────────┐          │
│  │  PHASE 5: Idempotency Tests                           │          │
│  │  - Reinstall profile                                  │          │
│  │  - Revalidate all extensions                          │          │
│  └───────────────────────────────────────────────────────┘          │
│                          │                                          │
│                          v                                          │
│  ┌───────────────────────────────────────────────────────┐          │
│  │  PHASE 6: File System Checks                          │          │
│  │  - Verify mise configs, sockets, etc.                 │          │
│  └───────────────────────────────────────────────────────┘          │
│                          │                                          │
│                          v                                          │
│  ┌───────────────────────────────────────────────────────┐          │
│  │  PHASE 7: Environment Checks                          │          │
│  │  - Verify environment variables are set               │          │
│  └───────────────────────────────────────────────────────┘          │
│                          │                                          │
│                          v                                          │
│  ┌───────────────────────────────────────────────────────┐          │
│  │  PHASE 8: Uninstall & Cleanup                         │          │
│  │  - Remove extensions in reverse dependency order      │          │
│  │  - Verify complete cleanup                            │          │
│  └───────────────────────────────────────────────────────┘          │
│                          │                                          │
│                          v                                          │
│  ┌───────────────────────────────────────────────────────┐          │
│  │  PHASE 9: Results Summary                             │          │
│  │  - Aggregate all test results                         │          │
│  │  - Report overall status                              │          │
│  └───────────────────────────────────────────────────────┘          │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**Provider-Specific Command Execution:**

The `run_on_provider` function abstracts command execution across providers:

| Provider   | Execution Method                                |
| ---------- | ----------------------------------------------- |
| `docker`   | `docker exec "$CONTAINER" bash -c "$cmd"`       |
| `fly`      | `flyctl ssh console -a "$APP" --command "$cmd"` |
| `devpod-*` | `devpod ssh "$WORKSPACE" --command "$cmd"`      |

#### Detailed Phase Descriptions

##### Phase 4: Functionality Tests

**When:** integration and full test suites only

**Purpose:** Verify extension-specific commands and tools work correctly

**Extension-specific tests:**

| Extension    | Functionality Tests                                   |
| ------------ | ----------------------------------------------------- |
| `nodejs`     | `node --version`, `npm --version`                     |
| `python`     | `python --version`, `pip --version`                   |
| `golang`     | `go version`                                          |
| `rust`       | `rustc --version`, `cargo --version`                  |
| `ruby`       | `ruby --version`, `gem --version`, `bundle --version` |
| `docker`     | `docker --version`, `docker compose version`          |
| `github-cli` | `gh --version`                                        |

##### Phase 5: Idempotency Tests

**When:** integration and full test suites only

**Purpose:** Verify extensions can be reinstalled without errors

**What is tested:**

- Reinstall the entire profile
- Verify all extensions still validate after reinstall
- Confirm idempotent behavior (no state corruption)

##### Phase 6: File System Checks

**When:** integration and full test suites only

**Purpose:** Verify expected files exist after installation

**Checks by extension type:**

| Extension Type                    | File Check              |
| --------------------------------- | ----------------------- |
| mise-based (nodejs, python, etc.) | `$WORKSPACE/.mise.toml` |
| docker                            | `/var/run/docker.sock`  |

##### Phase 7: Environment Checks

**When:** integration and full test suites only

**Purpose:** Verify environment variables are configured

**Variables by extension:**

| Extension | Variable     |
| --------- | ------------ |
| nodejs    | `NODE_ENV`   |
| python    | `PYTHONPATH` |
| golang    | `GOPATH`     |

##### Phase 8: Uninstall & Cleanup

**When:** integration and full test suites only

**Purpose:** Verify extensions can be cleanly removed

**What is tested:**

- Remove all extensions in reverse dependency order using `extension-manager remove <ext> --force`
- Verify extensions are no longer installed
- Verify commands are no longer available

---

## Integration Tests

Integration tests verify end-to-end functionality of the deployed environment.

### Smoke Tests

**Location:** `.github/workflows/test-provider.yml` → `smoke-tests` step

**When:** `test-suite == 'smoke'` or `test-suite == 'full'`

**Purpose:** Quick health check to verify basic connectivity and functionality

**What is Tested:**

```text
┌─────────────────────────────────────────────────────────────────────┐
│                        SMOKE TESTS                                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Per-Provider Smoke Checks:                                         │
│                                                                     │
│  Docker:                                                            │
│  ┌─────────────────────────────────────────────────────────┐        │
│  │  docker ps                      # Container running?    │        │
│  │  docker exec $CONTAINER \                               │        │
│  │    sindri --version             # CLI available?        │        │
│  └─────────────────────────────────────────────────────────┘        │
│                                                                     │
│  Fly.io:                                                            │
│  ┌─────────────────────────────────────────────────────────┐        │
│  │  flyctl ssh console -a $APP \                           │        │
│  │    --command "sindri --version" # SSH + CLI works?      │        │
│  └─────────────────────────────────────────────────────────┘        │
│                                                                     │
│  DevPod (all variants):                                             │
│  ┌─────────────────────────────────────────────────────────┐        │
│  │  devpod ssh $WORKSPACE \                                │        │
│  │    --command "sindri --version" # Workspace + CLI?      │        │
│  └─────────────────────────────────────────────────────────┘        │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**Inputs:**

- Running infrastructure (container, VM, or pod)
- Provider-specific credentials

**Outputs:**

- Pass/fail status
- Step outcome for summary

### Integration Test Suite

**Location:** `.github/workflows/test-provider.yml` → `integration-tests` step

**When:** `test-suite == 'integration'` or `test-suite == 'full'`

**Purpose:** Comprehensive validation of the deployed environment

**What is Tested:**

```text
┌─────────────────────────────────────────────────────────────────────┐
│                    INTEGRATION TEST SUITE                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Step 1: Check for provider-specific test script                    │
│  ┌───────────────────────────────────────────────────────────┐      │
│  │  TEST_SCRIPT=".github/scripts/test-provider-$PROVIDER.sh" │      │
│  │  if [[ -f "$TEST_SCRIPT" ]]; then                         │      │
│  │    bash "$TEST_SCRIPT" "$DEPLOYMENT_ID"                   │      │
│  │  else                                                     │      │
│  │    # Run generic tests...                                 │      │
│  │  fi                                                       │      │
│  └───────────────────────────────────────────────────────────┘      │
│                                                                     │
│  Step 2: Generic integration tests (if no custom script)            │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────┐        │
│  │  For Docker:                                            │        │
│  │    docker exec $CONTAINER extension-manager bom         │        │
│  │    docker exec $CONTAINER sindri secrets list           │        │
│  │                                                         │        │
│  │  For Fly.io:                                            │        │
│  │    flyctl ssh console -a $APP \                         │        │
│  │      --command "extension-manager bom"                  │        │
│  │                                                         │        │
│  │  For DevPod:                                            │        │
│  │    devpod ssh $WORKSPACE \                              │        │
│  │      --command "extension-manager bom"                  │        │
│  └─────────────────────────────────────────────────────────┘        │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**Provider-Specific Integration Tests:**

Additional tests from `.github/actions/providers/*/test/action.yml`:

```text
┌─────────────────────────────────────────────────────────────────────┐
│              PROVIDER-SPECIFIC INTEGRATION TESTS                    │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Fly.io (fly/test/action.yml):                                      │
│  ┌─────────────────────────────────────────────────────────┐        │
│  │  1. Environment Validation:                             │        │
│  │     - Check machine state == "started"                  │        │
│  │     - pwd, whoami, ls -la $WORKSPACE                    │        │
│  │                                                         │        │
│  │  2. Test Commands:                                      │        │
│  │     - sindri --version                                  │        │
│  │     - extension-manager list                            │        │
│  │                                                         │        │
│  │  3. Persistence Test:                                   │        │
│  │     - Create file, restart machine, verify file exists  │        │
│  │                                                         │        │
│  │  4. Integration Tests:                                  │        │
│  │     - sindri config validate                            │        │
│  │     - sindri --help                                     │        │
│  │     - extension-manager list-profiles                   │        │
│  │     - extension-manager validate-all                    │        │
│  │     - Check age key exists (~/.secrets/age.key)         │        │
│  └─────────────────────────────────────────────────────────┘        │
│                                                                     │
│  DevPod (devpod/test/action.yml):                                   │
│  ┌─────────────────────────────────────────────────────────┐        │
│  │  1. Environment Validation:                             │        │
│  │     - Workspace state == "running" (case insensitive)   │        │
│  │     - pwd, whoami, ls -la $WORKSPACE                    │        │
│  │                                                         │        │
│  │  2. Test Commands:                                      │        │
│  │     - sindri --version                                  │        │
│  │     - extension-manager list                            │        │
│  │                                                         │        │
│  │  3. Volume Persistence:                                 │        │
│  │     - Write file to $WORKSPACE/test-persistence.txt     │        │
│  │     - Read it back                                      │        │
│  │                                                         │        │
│  │  4. Integration Tests:                                  │        │
│  │     - sindri config validate                            │        │
│  │     - sindri --help                                     │        │
│  │     - extension-manager list-profiles                   │        │
│  │     - extension-manager validate-all                    │        │
│  └─────────────────────────────────────────────────────────┘        │
│                                                                     │
└────────────────────────────────────────────────────────────────────┘
```

### Full Test Suite

**When:** `test-suite == 'full'`

The full test suite runs both smoke tests AND integration tests sequentially.

---

## Test Execution by Provider

### How `run_on_provider` Works

The test-provider workflow abstracts command execution with provider-specific methods:

```text
┌─────────────────────────────────────────────────────────────────────┐
│                  RUN_ON_PROVIDER ABSTRACTION                        │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Input: command to execute (e.g., "extension-manager validate X")   │
│                                                                     │
│  Switch on $PROVIDER:                                               │
│                                                                     │
│  ┌──────────────┬─────────────────────────────────────────────┐     │
│  │   Provider   │   Execution Method                          │     │
│  ├──────────────┼─────────────────────────────────────────────┤     │
│  │   docker     │   docker exec "$CONTAINER" bash -c "$cmd"   │     │
│  │              │                                             │     │
│  │   fly        │   flyctl ssh console -a "$APP" \            │     │
│  │              │     --command "$cmd"                        │     │
│  │              │   (requires FLY_API_TOKEN)                  │     │
│  │              │                                             │     │
│  │   devpod-*   │   devpod ssh "$WORKSPACE" --command "$cmd"  │     │
│  │   kubernetes │   (uses kubectl exec under the hood)        │     │
│  │   ssh        │                                             │     │
│  └──────────────┴─────────────────────────────────────────────┘     │
│                                                                     │
│  Output: stdout + stderr from command                               │
│  Exit code: Command's exit code                                     │
│                                                                     │
└────────────────────────────────────────────────────────────────────┘
```

### CLI Tests Flow

**Location:** `.github/actions/core/test-cli/action.yml`

**Input Parameters:**

| Parameter             | Description                       | Example                                |
| --------------------- | --------------------------------- | -------------------------------------- |
| `test-commands`       | JSON array of CLI commands        | `["sindri --version", "ext-mgr list"]` |
| `provider`            | Provider type                     | `docker`, `fly`, `devpod-k8s`          |
| `container-name`      | Docker container name             | `sindri-minimal`                       |
| `fly-app-name`        | Fly.io app name                   | `sindri-test-fly-12345`                |
| `devpod-workspace`    | DevPod workspace ID               | `sindri-12345`                         |
| `expected-exit-codes` | Map of commands to expected codes | `{"invalid-cmd": 1}`                   |

**Execution Flow:**

```text
┌─────────────────────────────────────────────────────────────────────┐
│                      CLI TEST EXECUTION                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Input: JSON array of commands                                      │
│  ["sindri --version", "extension-manager list"]                     │
│                                                                     │
│  For each command:                                                  │
│  ┌─────────────────────────────────────────────────────────┐        │
│  │  1. Get expected exit code (default: 0)                 │        │
│  │     EXPECTED=$(echo "$EXPECTED_CODES" | \               │        │
│  │                jq -r --arg cmd "$cmd" '.[$cmd] // 0')   │        │
│  │                                                         │        │
│  │  2. Execute command on provider                         │        │
│  │     OUTPUT=$(docker exec $CONTAINER bash -c "$cmd")     │        │
│  │     EXIT_CODE=$?                                        │        │
│  │                                                         │        │
│  │  3. Compare result                                      │        │
│  │     if [[ "$EXIT_CODE" -eq "$EXPECTED" ]]; then         │        │
│  │       STATUS="passed"                                   │        │
│  │       echo "  ✅ Passed (exit code: $EXIT_CODE)"        │        │
│  │     else                                                │        │
│  │       STATUS="failed"                                   │        │
│  │       ALL_PASSED="false"                                │        │
│  │       echo "  ❌ Failed (expected: $EXPECTED)"          │        │
│  │     fi                                                  │        │
│  │                                                         │        │
│  │  4. Store result in JSON                                │        │
│  │     RESULTS=$(echo "$RESULTS" | jq \                    │        │
│  │       '.[$cmd] = {status, exit_code, output}')          │        │
│  └─────────────────────────────────────────────────────────┘        │
│                                                                     │
│  Outputs:                                                           │
│  - results: '{"sindri --version": {"status": "passed", ...}}'       │
│  - all-passed: "true" or "false"                                    │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Resource Calculation

**Location:** `.github/scripts/calculate-profile-resources.sh`

**Purpose:** Determine VM size, disk, memory, and timeout based on extension profile

**Flow:**

```text
┌─────────────────────────────────────────────────────────────────────┐
│                  RESOURCE CALCULATION FLOW                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Input: profile name (e.g., "minimal", "fullstack")                 │
│         provider name (e.g., "fly", "aws")                          │
│                                                                     │
│  Step 1: Get extensions in profile                                  │
│  ┌─────────────────────────────────────────────────────────┐        │
│  │  EXTENSIONS=$(yq ".profiles.${PROFILE}.extensions[]"    │        │
│  │               docker/lib/profiles.yaml)                 │        │
│  └─────────────────────────────────────────────────────────┘        │
│                                                                     │
│  Step 2: Sum resource requirements                                  │
│  ┌─────────────────────────────────────────────────────────┐        │
│  │  for ext in $EXTENSIONS; do                             │        │
│  │    DISK=$(yq '.requirements.diskSpace' ext.yaml)        │        │
│  │    MEM=$(yq '.requirements.memory' ext.yaml)            │        │
│  │    TIME=$(yq '.requirements.installTime' ext.yaml)      │        │
│  │    TOTAL_DISK += DISK                                   │        │
│  │    TOTAL_MEMORY += MEM                                  │        │
│  │    TOTAL_TIME += TIME                                   │        │
│  │  done                                                   │        │
│  └─────────────────────────────────────────────────────────┘        │
│                                                                     │
│  Step 3: Calculate timeout with buffer                              │
│  ┌─────────────────────────────────────────────────────────┐        │
│  │  BASE_TIMEOUT=300  # 5 minutes                          │        │
│  │  OVERHEAD=20%                                           │        │
│  │  TIMEOUT = BASE + INSTALL_TIME + (INSTALL_TIME * 0.2)   │        │
│  └─────────────────────────────────────────────────────────┘        │
│                                                                     │
│  Step 4: Determine VM size tier                                     │
│  ┌─────────────────────────────────────────────────────────┐        │
│  │  if MEMORY < 2048MB:  tier = "small"                    │        │
│  │  elif MEMORY < 4096MB: tier = "medium"                  │        │
│  │  elif MEMORY < 8192MB: tier = "large"                   │        │
│  │  else: tier = "xlarge"                                  │        │
│  └─────────────────────────────────────────────────────────┘        │
│                                                                     │
│  Step 5: Get provider-specific sizes from vm-sizes.yaml             │
│  ┌─────────────────────────────────────────────────────────┐        │
│  │  VM_SIZE=$(yq ".providers.${PROVIDER}.sizes.${TIER}"    │        │
│  │            docker/lib/vm-sizes.yaml)                    │        │
│  │                                                         │        │
│  │  Examples:                                              │        │
│  │    fly + small = "shared-cpu-1x"                        │        │
│  │    aws + medium = "t3.medium"                           │        │
│  │    gcp + large = "e2-standard-4"                        │        │
│  └─────────────────────────────────────────────────────────┘        │
│                                                                     │
│  Outputs (GitHub Actions format):                                   │
│  - profile=minimal                                                  │
│  - extension_count=3                                                │
│  - disk_mb=500                                                      │
│  - memory_mb=1024                                                   │
│  - vm_size_tier=small                                               │
│  - provider_vm_size=shared-cpu-1x                                   │
│  - recommended_timeout=15                                           │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Helper Libraries

### Test Helpers (`lib/test-helpers.sh`)

| Function                 | Purpose                                |
| ------------------------ | -------------------------------------- |
| `log_info`               | Print blue `[INFO]` message            |
| `log_success`            | Print green `[SUCCESS]` message        |
| `log_warning`            | Print yellow `[WARNING]` message       |
| `log_error`              | Print red `[ERROR]` message            |
| `retry_with_backoff`     | Retry command with exponential backoff |
| `check_command_exists`   | Check if command exists on remote VM   |
| `run_on_vm`              | Execute command on Fly.io VM via SSH   |
| `is_extension_installed` | Check extension installation status    |
| `wait_for_vm`            | Wait for VM to become ready            |
| `test_persistence`       | Test volume persistence across restart |
| `test_idempotency`       | Test extension can be installed twice  |
| `version_gt`             | Compare version strings                |

### Assertions (`lib/assertions.sh`)

| Function                  | Purpose                                    |
| ------------------------- | ------------------------------------------ |
| `assert_command`          | Assert command exists (local or remote)    |
| `assert_file_exists`      | Assert file exists (local or remote)       |
| `assert_directory_exists` | Assert directory exists                    |
| `assert_equals`           | Assert two strings are equal               |
| `assert_contains`         | Assert string contains substring           |
| `assert_success`          | Assert command exits with 0                |
| `assert_failure`          | Assert command exits non-zero              |
| `assert_exit_code`        | Assert specific exit code                  |
| `assert_matches`          | Assert output matches regex pattern        |
| `assert_numeric`          | Assert numeric comparison (-lt, -gt, etc.) |

---

## Debugging Failed Tests

### Viewing Test Results

GitHub Actions Step Summary shows:

```markdown
## Provider Test Summary: fly

| Test Phase        | Status |
| ----------------- | ------ |
| CLI Tests         | passed |
| Extension Tests   | passed |
| Smoke Tests       | passed |
| Integration Tests | passed |

**Overall Status**: success
**Test Suite**: integration
**Extension Profile**: minimal
```

### Log Artifacts

On failure, logs are automatically collected:

**Fly.io:**

- `fly-logs.txt` - Recent container logs
- `fly-status.json` - Machine status

**DevPod:**

- `devpod.log` - DevPod logs
- `status.json` - Workspace status
- `provider.log` - Provider logs
- `extension-manager.log` - Extension manager logs from workspace

### Manual Debugging

```bash
# Skip cleanup to inspect failed deployment
./cli/sindri deploy --provider fly --skip-cleanup

# Connect to investigate
flyctl ssh console -a sindri-test-fly-12345

# Check extension manager logs
cat $WORKSPACE/.system/logs/extension-manager.log

# Re-run validation manually
extension-manager validate nodejs
```

### Common Failure Scenarios

| Symptom                      | Cause                              | Solution                           |
| ---------------------------- | ---------------------------------- | ---------------------------------- |
| Schema validation fails      | Invalid extension.yaml             | Check against schema, fix YAML     |
| Extension install timeout    | Network issues or slow download    | Increase timeout, check domains    |
| Validation command not found | Extension not fully installed      | Check install script, dependencies |
| Persistence test fails       | Volume not mounted properly        | Check volume configuration         |
| SSH connection fails         | Machine not ready or network issue | Increase wait time, check firewall |

---

## Related Documentation

- [Testing Guide](TESTING.md) - Overview of testing philosophy
- [Extension Authoring](EXTENSION_AUTHORING.md) - Creating new extensions
- [Architecture](ARCHITECTURE.md) - System architecture overview
- [Contributing Guide](CONTRIBUTING.md) - Development workflow
