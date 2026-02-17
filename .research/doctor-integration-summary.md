# Doctor Integration Summary: RunPod & Northflank

## Changes Made

### 1. `v3/crates/sindri-doctor/src/platform.rs`

- Added `PackageManager::Npm` variant to support npm-based installations (used by Northflank CLI).
- Added corresponding `Display` implementation (`"npm"`).

### 2. `v3/crates/sindri-doctor/src/tool.rs`

- Added `ToolCategory::ProviderRunpod` variant with display name "RunPod Provider".
- Added `ToolCategory::ProviderNorthflank` variant with display name "Northflank Provider".

### 3. `v3/crates/sindri-doctor/src/registry.rs`

#### New Tool Definitions

- **runpodctl**: RunPod CLI tool
  - Command: `runpodctl`
  - Version flag: `version`
  - Min version: `1.14.0`
  - Auth check: `runpodctl get pod` (exit code 0)
  - Install: curl binary download for macOS/Linux, manual download for Windows
  - Docs: https://github.com/runpod/runpodctl

- **northflank**: Northflank CLI tool
  - Command: `northflank`
  - Version flag: `--version`
  - Min version: `0.10.0`
  - Auth check: `northflank list projects` (exit code 0)
  - Install: `npm install -g @northflank/cli` for all platforms
  - Docs: https://northflank.com/docs/v1/api/cli

#### Updated Methods

- `by_provider()`: Added mappings for `"runpod"` -> `ProviderRunpod` and `"northflank"` -> `ProviderNorthflank`.
- `by_command("deploy")`: Extended to include `ProviderRunpod` and `ProviderNorthflank` categories.

### 4. New Tests (7 added)

- `test_get_runpodctl` - Verifies runpodctl tool definition properties
- `test_get_northflank` - Verifies northflank tool definition properties
- `test_by_provider_runpod` - Verifies `by_provider("runpod")` returns runpodctl
- `test_by_provider_northflank` - Verifies `by_provider("northflank")` returns northflank
- `test_by_category_runpod` - Verifies category filtering for ProviderRunpod
- `test_by_category_northflank` - Verifies category filtering for ProviderNorthflank
- `test_deploy_command_includes_runpod_and_northflank` - Verifies deploy command includes both providers

## Test Results

All 69 tests pass (62 existing + 7 new).

## Usage

```bash
# Check RunPod prerequisites
sindri doctor --provider runpod

# Check Northflank prerequisites
sindri doctor --provider northflank

# Check all tools
sindri doctor --all

# Auto-install missing tools
sindri doctor --provider runpod --fix
sindri doctor --provider northflank --fix
```
