# CI/CD Configuration for RunPod and Northflank Providers

## Summary

This document describes the CI/CD pipeline configuration for the new RunPod and Northflank providers in Sindri v3.

## Workflow Files

### 1. Updated: `.github/workflows/ci-v3.yml`

The main CI workflow was updated with the following new jobs:

| Job                                           | Purpose                                                                                  | Blocking?                             |
| --------------------------------------------- | ---------------------------------------------------------------------------------------- | ------------------------------------- |
| `test-providers` (matrix: runpod, northflank) | Runs unit tests for each provider via `cargo test --package sindri-providers <provider>` | Informational (reported in ci-status) |
| `test-doctor`                                 | Runs `sindri-doctor` tests (includes provider health checks)                             | Informational (reported in ci-status) |
| `validate-provider-configs`                   | Dry-run validation of all RunPod and Northflank example YAML configs                     | Informational (reported in ci-status) |

These jobs run in parallel with existing CI jobs and are included in the `ci-status` summary.

### 2. New: `.github/workflows/v3-provider-runpod.yml`

Reusable workflow for testing extensions on RunPod GPU cloud pods. Follows the same pattern as existing provider workflows (`v3-provider-fly.yml`, `v3-provider-docker.yml`, `v3-provider-devpod.yml`).

**Features:**

- Matrix-based extension testing on RunPod pods
- Configurable GPU type and parallel job count
- Automatic pod cleanup on failure
- Test result aggregation with GitHub Step Summary
- Artifact upload for test logs

**Required Secrets:**

- `RUNPOD_API_KEY` - RunPod API key for authentication

### 3. New: `.github/workflows/v3-provider-northflank.yml`

Reusable workflow for testing extensions on Northflank Kubernetes PaaS. Follows the same reusable workflow pattern.

**Features:**

- Matrix-based extension testing on Northflank services
- Configurable compute plan and parallel job count
- Automatic project/service cleanup
- Test result aggregation with GitHub Step Summary
- Artifact upload for test logs

**Required Secrets:**

- `NORTHFLANK_API_TOKEN` - Northflank API token for authentication

### 4. New: `.github/workflows/integration-test-providers.yml`

Manually-triggered workflow for real deployment testing.

**Trigger:** `workflow_dispatch` with inputs:

- `provider`: runpod, northflank, or all
- `test-mode`: dry-run (config validation only) or live (real deployments)
- `gpu-type`: Optional GPU type for RunPod

**Jobs:**

- `build` - Build sindri binary
- `validate-runpod-configs` - Dry-run validation of RunPod example configs
- `validate-northflank-configs` - Dry-run validation of Northflank example configs
- `test-runpod-integration` - Full deploy/status/destroy lifecycle (live mode only)
- `test-northflank-integration` - Full deploy/status/destroy lifecycle (live mode only)
- `summary` - Aggregated results

**Required Secrets (for live mode):**

- `RUNPOD_API_KEY` - RunPod API key
- `NORTHFLANK_API_TOKEN` - Northflank API token

### 5. Lint & Format (Already Present)

The existing `ci-v3.yml` already includes:

- `rust-format` - `cargo fmt --all -- --check`
- `rust-clippy` - `cargo clippy --workspace --all-targets --all-features -- -D warnings`

These apply to all workspace crates including `sindri-providers`.

## Test Coverage in CI

| Test Category                   | Trigger             | Scope                                                                |
| ------------------------------- | ------------------- | -------------------------------------------------------------------- |
| Unit tests (all crates)         | Push/PR to v3 paths | `cargo test --workspace --all-features`                              |
| Provider-specific unit tests    | Push/PR to v3 paths | `cargo test --package sindri-providers runpod` and `northflank`      |
| Doctor tests                    | Push/PR to v3 paths | `cargo test --package sindri-doctor`                                 |
| Config dry-run validation       | Push/PR to v3 paths | `sindri deploy --config <example> --dry-run` for all example configs |
| Clippy lints                    | Push/PR to v3 paths | `cargo clippy --workspace --all-targets --all-features`              |
| Format check                    | Push/PR to v3 paths | `cargo fmt --all -- --check`                                         |
| Extension testing on RunPod     | Manual (reusable)   | Full install/validate/remove on RunPod pods                          |
| Extension testing on Northflank | Manual (reusable)   | Full install/validate/remove on Northflank services                  |
| Live integration test           | Manual dispatch     | Full deploy/status/destroy lifecycle                                 |

## Running Tests Locally

### Provider Unit Tests

```bash
cd v3

# All provider tests
cargo test --package sindri-providers

# RunPod-specific tests
cargo test --package sindri-providers runpod

# Northflank-specific tests
cargo test --package sindri-providers northflank

# Doctor tests (includes provider health checks)
cargo test --package sindri-doctor
```

### Dry-Run Config Validation

```bash
cd v3
cargo build --release

# Validate individual configs
./target/release/sindri deploy --config examples/runpod-gpu-basic.yaml --dry-run
./target/release/sindri deploy --config examples/northflank-basic.yaml --dry-run

# Validate all RunPod configs
for config in examples/runpod-*.yaml; do
  echo "Validating: $config"
  ./target/release/sindri deploy --config "$config" --dry-run
done

# Validate all Northflank configs
for config in examples/northflank-*.yaml; do
  echo "Validating: $config"
  ./target/release/sindri deploy --config "$config" --dry-run
done
```

### Lint and Format Checks

```bash
cd v3

# Format check
cargo fmt --all -- --check

# Clippy
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Manual Integration Test Procedures

### Triggering via GitHub UI

1. Go to **Actions** tab in the repository
2. Select **"v3: Integration Test - Providers (Manual)"** workflow
3. Click **"Run workflow"**
4. Select parameters:
   - **Provider**: `runpod`, `northflank`, or `all`
   - **Test mode**: `dry-run` (safe) or `live` (creates real resources)
   - **GPU type**: Leave empty for CPU-only testing

### Triggering via CLI

```bash
# Dry-run validation only
gh workflow run integration-test-providers.yml \
  -f provider=all \
  -f test-mode=dry-run

# Live RunPod test (requires RUNPOD_API_KEY secret)
gh workflow run integration-test-providers.yml \
  -f provider=runpod \
  -f test-mode=live

# Live Northflank test (requires NORTHFLANK_API_TOKEN secret)
gh workflow run integration-test-providers.yml \
  -f provider=northflank \
  -f test-mode=live
```

## Required Secrets

| Secret                 | Provider   | Purpose                                       | Where to configure           |
| ---------------------- | ---------- | --------------------------------------------- | ---------------------------- |
| `RUNPOD_API_KEY`       | RunPod     | API authentication for live integration tests | Settings > Secrets > Actions |
| `NORTHFLANK_API_TOKEN` | Northflank | API authentication for live integration tests | Settings > Secrets > Actions |

**Note:** These secrets are only required for live integration tests (manual trigger). The automated CI pipeline runs unit tests and dry-run validation that do not require provider credentials.

## Architecture Notes

- All workflows follow the naming convention `v3-provider-<name>.yml` for reusable provider workflows
- The main CI workflow (`ci-v3.yml`) runs unit tests and dry-run validation on every push/PR
- Live integration tests are manual-only to prevent accidental resource creation and cost
- Provider workflows are designed as `workflow_call` reusable workflows, consistent with existing Fly.io, Docker, and DevPod providers
- Test results are uploaded as artifacts with 7-day retention for debugging
- All jobs use `actions/checkout@v6` and `dtolnay/rust-toolchain@stable` per project conventions
