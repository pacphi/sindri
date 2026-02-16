# E2E Test Report: RunPod and Northflank Provider Integration

**Date:** 2026-02-16
**Tester:** QA Automation Agent
**Status:** Draft (pre-implementation)
**Scope:** Documentation review, UX walkthrough, error message quality, cross-reference validation, edge cases, accessibility

---

## Executive Summary

The RunPod and Northflank provider integration is **substantially complete and well-documented**. Both providers have comprehensive documentation, example configurations, Rust adapter implementations with unit tests, updated deployment and configuration reference files, CHANGELOG entries, JSON schema definitions, and a working factory pattern in `lib.rs`. Nine example YAML files are present (the task specifies 10 but only 9 exist -- see Issue #1).

However, this review identified **22 issues** ranging from minor documentation inconsistencies to meaningful schema/code divergences that should be resolved before GA. No blockers were found for a draft/pre-implementation release, but several items need attention for production readiness.

---

## 1. Documentation Quality Assessment

### 1.1 RunPod Documentation (`v3/docs/providers/RUNPOD.md`)

**Overall Quality: GOOD**

Strengths:

- Comprehensive GPU type listing with approximate pricing
- Three-tier storage model is clearly explained with ASCII art diagrams
- Lifecycle behavior matrix is excellent
- Connection methods are well-documented (5 methods)
- Troubleshooting section covers common error scenarios
- Cost optimization strategies are practical and actionable
- Example scenarios cover real-world use cases (ML dev, training, budget research, CPU-only, inference)

Issues Found:

- **ISSUE-R1 (Medium):** The documentation header says "Status: Draft (pre-implementation)" but describes the provider as if it is fully functional. The implementation in `runpod.rs` exists and implements the full `Provider` trait. The status label should be updated to reflect the actual state.
- **ISSUE-R2 (Low):** The documentation says "no CLI tool installation is required" (line 30) and describes a "direct REST API integration," but the Rust implementation in `runpod.rs` uses `runpodctl` CLI extensively (`runpodctl get pod`, `runpodctl create pods`, `runpodctl connect`, etc.). This is a significant mismatch between documentation and implementation.
- **ISSUE-R3 (Low):** The `deployment.provider` docs use `sindri deploy` and `sindri plan` commands, but the Quick Start uses `sindri deploy` followed by `sindri connect`. The deployment commands section shows `sindri plan` but the Quick Start shows no plan step. Minor UX inconsistency.
- **ISSUE-R4 (Medium):** The configuration reference table lists a `spot` field (boolean), but the spot configuration example uses `spotBid: 1.50` (a numeric bid). Meanwhile the JSON schema defines `spotBid` (number) but not `spot` (boolean). The documentation shows both `spot: true` and `spotBid: 1.50` in different sections, which could confuse users.
- **ISSUE-R5 (Low):** The "Comparison to Other Providers" table does not include Northflank, though it includes Docker, Fly.io, and E2B. Similarly, the Northflank comparison table does not include RunPod.

### 1.2 Northflank Documentation (`v3/docs/providers/NORTHFLANK.md`)

**Overall Quality: GOOD**

Strengths:

- Comprehensive compute plan table with pricing
- Auto-scaling behavior is clearly documented including metric intervals and cooldown periods
- Health check configuration covers all three types (HTTP, TCP, command)
- Region list is thorough (16 managed + BYOC)
- Cost optimization tips are practical
- Pause/resume workflow is clearly explained
- Port forwarding documentation is useful

Issues Found:

- **ISSUE-N1 (Medium):** The documentation lists a `volumeMountPath` default of `/data`, but the Rust implementation in `northflank.rs` defaults to `/workspace` (line 331). Documentation and code disagree on the default mount path.
- **ISSUE-N2 (Low):** The documentation lists GPU types `nvidia-h200`, `nvidia-b200`, and `AMD MI300X`, but the comparison table at the bottom only lists "H100, B200, A100, L4, H200, MI300X". The AMD GPU string format differs from others (not prefixed with `nvidia-`). It would be helpful to provide the exact `gpuType` string for AMD GPUs.
- **ISSUE-N3 (Low):** The documentation shows `instances: 0` as a valid configuration to "pause billing," but the JSON schema specifies `"minimum": 1` for the `instances` field. A user setting `instances: 0` would fail schema validation.
- **ISSUE-N4 (Info):** Health check `type` field is documented with three values (`http`, `tcp`, `command`), but the JSON schema only defines a simplified health check object without a `type` field. The schema uses `path`, `port`, `intervalSecs`, `timeoutSecs` instead of the documented `initialDelaySeconds`, `periodSeconds`, `failureThreshold`.

### 1.3 Deployment Documentation (`v3/docs/DEPLOYMENT.md`)

**Overall Quality: EXCELLENT**

Strengths:

- Provider comparison table is comprehensive and includes both new providers
- Quick Start workflow is clear
- Provider-specific commands section covers both RunPod and Northflank
- Hybrid deployment section correctly mentions both providers
- Secrets management table includes both new providers
- GPU support table is accurate
- Related Documentation section links to both new provider docs

Issues Found:

- **ISSUE-D1 (Low):** The "Choosing a Provider" section has RunPod "Not recommended when" listing "Need auto-suspend/resume (use stop/start instead)" -- this is accurate but could mention that Northflank does support native pause/resume for users weighing options.

### 1.4 Configuration Documentation (`v3/docs/CONFIGURATION.md`)

**Overall Quality: EXCELLENT**

Strengths:

- Both RunPod and Northflank have comprehensive provider-specific configuration blocks
- Field-by-field documentation with types, defaults, and descriptions
- Complete examples for both providers
- Provider enum correctly lists `runpod` and `northflank`
- Build support table correctly shows RunPod and Northflank as "No" for Dockerfile builds

Issues Found:

- **ISSUE-C1 (Medium):** The RunPod config block in CONFIGURATION.md uses field names `gpuType`, `gpuCount`, `cpuOnly`, `containerDiskGb`, `volumeSizeGb`, etc., but the JSON schema uses `gpuTypeId`, `containerDiskGb`, `cloudType`, `exposePorts`, `spotBid`, `startSsh`. Several field names differ between documentation and schema: `gpuType` vs `gpuTypeId`, `ports` vs `exposePorts`, `spot` vs `spotBid`.
- **ISSUE-C2 (Medium):** The Northflank config block lists `autoScaling.targetCpuUtilization` and `autoScaling.targetMemoryUtilization`, but the schema uses `autoScaling.cpuTargetPercent` and does not include a memory target. Also, the schema lists `autoScaling` without an `enabled` field that the documentation shows.
- **ISSUE-C3 (Medium):** The Northflank schema `computePlan` enum only lists 5 plans (`nf-compute-10`, `nf-compute-20`, `nf-compute-50`, `nf-compute-100`, `nf-compute-200`), but the documentation and examples reference 20+ plans including `nf-compute-200-8`, `nf-compute-400-8`, `nf-compute-400-16`, `nf-compute-800-32`, etc. The schema enum is too restrictive.
- **ISSUE-C4 (Low):** The Northflank schema has `healthCheck.intervalSecs` and `healthCheck.timeoutSecs`, but the documentation uses `healthCheck.initialDelaySeconds`, `healthCheck.periodSeconds`, and `healthCheck.failureThreshold`. These are different field names with different semantics.

---

## 2. User Experience Walkthrough

### 2.1 RunPod Walkthrough

**Step 1: Read RUNPOD.md documentation** -- PASS
The documentation is well-structured with a clear progression from overview to quick start to detailed configuration.

**Step 2: Check prerequisites** -- PARTIAL
The `sindri doctor` DOCTOR.md documentation does NOT list RunPod or Northflank in its tool categories table (lines 79-91). The `--provider` flag examples only show `docker`, `fly`, `devpod`, `e2b`, `k8s`. A new user running `sindri doctor --provider runpod` might not know this is supported.

- **ISSUE-UX1 (Medium):** DOCTOR.md needs updating to include RunPod and Northflank providers.

**Step 3: Review example config** -- PASS
`v3/examples/runpod-gpu-basic.yaml` is clear with good header comments.

**Step 4: Validate config syntax** -- PARTIAL

- **ISSUE-UX2 (Medium):** The example file uses `image: ghcr.io/sindri-labs/sindri:latest` but the documentation consistently uses `ghcr.io/pacphi/sindri:3.0.0` or `ghcr.io/pacphi/sindri:latest`. The `sindri-labs` registry reference in examples is inconsistent with the canonical `pacphi` registry used elsewhere.
- **ISSUE-UX3 (Low):** The example `runpod-gpu-basic.yaml` uses `gpu.tier: "a4000"` which is not a valid tier value. The schema defines tiers as `gpu-small`, `gpu-medium`, `gpu-large`, `gpu-xlarge`. The example uses a free-form string that would fail schema validation.

**Step 5: Dry-run** -- N/A (cannot execute, pre-implementation)

**Step 6: Overall clarity** -- GOOD
The flow from prerequisites to configuration to deployment is logical and well-documented.

### 2.2 Northflank Walkthrough

**Step 1: Read NORTHFLANK.md documentation** -- PASS
Well-organized with clear examples.

**Step 2: Check prerequisites** -- PARTIAL
Same issue as RunPod (ISSUE-UX1): DOCTOR.md does not list Northflank.

**Step 3: Review example config** -- PASS
`v3/examples/northflank-basic.yaml` is clear.

**Step 4: Validate config syntax** -- PARTIAL

- **ISSUE-UX4 (Low):** The example `northflank-full.yaml` uses `region: us-east-1` but the documentation lists the region slug as `us-east` (not `us-east-1`). This could cause a deployment failure.

**Step 5: Dry-run** -- N/A (cannot execute, pre-implementation)

---

## 3. Error Message Quality

### 3.1 RunPod Error Messages (from `runpod.rs`)

| Error Scenario        | Error Message                                                | Quality   | Notes                                                      |
| --------------------- | ------------------------------------------------------------ | --------- | ---------------------------------------------------------- |
| Missing API key       | `RunPod API key not configured`                              | GOOD      | Install hint provides two options (CLI config and env var) |
| Missing CLI           | `RunPod CLI for pod management`                              | GOOD      | Install hint links to GitHub releases                      |
| Pod already exists    | `Pod '{}' already exists (id: {}). Use --force to recreate.` | EXCELLENT | Actionable with clear remediation                          |
| No image configured   | `No image configured for RunPod deployment`                  | GOOD      | Clear but could suggest adding `deployment.image`          |
| Pod creation fails    | `Failed to create RunPod pod: {}`                            | GOOD      | Includes stderr from runpodctl                             |
| Pod not found         | `No RunPod pod found for '{}'. Deploy first.`                | EXCELLENT | Suggests next action                                       |
| SSH connection failed | `SSH connection to pod {} failed`                            | ADEQUATE  | Could include troubleshooting link                         |
| Timeout waiting       | `Pod {} did not reach RUNNING state within {} seconds`       | GOOD      | Clear timeout message                                      |

### 3.2 Northflank Error Messages (from `northflank.rs`)

| Error Scenario       | Error Message                                                           | Quality   | Notes                                           |
| -------------------- | ----------------------------------------------------------------------- | --------- | ----------------------------------------------- |
| Not authenticated    | `Northflank API authentication not configured`                          | GOOD      | Provides `northflank login` and env var options |
| Missing CLI          | `Northflank CLI for service management`                                 | GOOD      | Install hint: `npm install -g @northflank/cli`  |
| Service exists       | `Service '{}' already exists in project '{}'. Use --force to recreate.` | EXCELLENT | Clear remediation                               |
| Service not found    | `No Northflank service found for '{}'. Deploy first.`                   | EXCELLENT | Suggests next action                            |
| No image             | `No image configured for Northflank deployment`                         | GOOD      | Clear                                           |
| Service create fails | `Failed to create Northflank service: {}`                               | GOOD      | Includes stderr                                 |
| Project create fails | `Failed to create project: {}`                                          | GOOD      | Includes stderr                                 |
| Timeout              | `Service {} did not reach running state within {} seconds`              | GOOD      | Clear                                           |

**Overall error message quality: GOOD to EXCELLENT.** Messages are clear, actionable, and include remediation hints where appropriate.

---

## 4. Help Text Review

The `DOCTOR.md` documentation shows CLI help text for `sindri doctor`. The `--provider` flag currently documents only `docker, fly, devpod, e2b, k8s`.

- **ISSUE-H1 (Medium):** The `--provider` flag description and examples in DOCTOR.md need to include `runpod` and `northflank`. The current text on line 53 reads: `Check tools for a specific provider (docker, fly, devpod, e2b, k8s)`.

---

## 5. Cross-Reference Validation

### 5.1 Schema vs Documentation Consistency

| Field                                    | Documentation                | Schema                  | Match?                         |
| ---------------------------------------- | ---------------------------- | ----------------------- | ------------------------------ |
| RunPod `gpuType`                         | `gpuType`                    | `gpuTypeId`             | NO                             |
| RunPod `spot`                            | `spot: true` (boolean)       | Not in schema           | NO                             |
| RunPod `spotBid`                         | `spotBid: 1.50`              | `spotBid` (number)      | YES                            |
| RunPod `ports`                           | `ports: ["8888/http"]`       | `exposePorts: ["8080"]` | NO (different name and format) |
| RunPod `publicIp`                        | `publicIp: true`             | Not in schema           | NO                             |
| RunPod `networkVolumeId`                 | `networkVolumeId: "vol-..."` | Not in schema           | NO                             |
| RunPod `cpuOnly`                         | `cpuOnly: true`              | Not in schema           | NO                             |
| RunPod `gpuCount`                        | `gpuCount: 2`                | Not in schema           | NO                             |
| RunPod `cloudType` default               | `SECURE` (docs)              | `COMMUNITY` (schema)    | NO                             |
| NF `computePlan`                         | 20+ plans                    | 5 plans in enum         | PARTIAL                        |
| NF `instances` min                       | `0` (docs, for pause)        | `1` (schema)            | NO                             |
| NF `autoScaling.enabled`                 | yes (docs)                   | Not in schema           | NO                             |
| NF `autoScaling.targetCpuUtilization`    | yes (docs)                   | `cpuTargetPercent`      | NO (name mismatch)             |
| NF `autoScaling.targetMemoryUtilization` | yes (docs)                   | Not in schema           | NO                             |
| NF `healthCheck.type`                    | `http/tcp/command` (docs)    | Not in schema           | NO                             |
| NF `healthCheck.initialDelaySeconds`     | yes (docs)                   | `intervalSecs`          | NO (different field)           |
| NF `healthCheck.failureThreshold`        | yes (docs)                   | Not in schema           | NO                             |
| NF `volumeSizeGb`                        | yes (docs)                   | Not in schema           | NO                             |
| NF `volumeMountPath` default             | `/data` (docs)               | N/A                     | N/A (not in schema)            |
| NF `registryCredentials`                 | yes (docs)                   | Not in schema           | NO                             |

**Assessment:** The JSON schema (`v3/schemas/sindri.schema.json`) is significantly behind the documentation for both providers. Many documented fields are missing from the schema, and some field names differ. The schema was likely written at an earlier design stage and not fully updated to match the final documentation.

### 5.2 Documentation vs Code Consistency

| Aspect                 | Documentation    | Code (`runpod.rs`)           | Match?  |
| ---------------------- | ---------------- | ---------------------------- | ------- |
| API integration        | REST API, no CLI | Uses `runpodctl` CLI         | NO      |
| GPU type field         | `gpuType`        | `gpu_type_id` (struct field) | PARTIAL |
| Default cloud type     | `SECURE`         | `"COMMUNITY"` (line 215)     | NO      |
| Container disk default | `50` GB          | `20` GB (line 211)           | NO      |

| Aspect                   | Documentation      | Code (`northflank.rs`)          | Match? |
| ------------------------ | ------------------ | ------------------------------- | ------ |
| Volume mount default     | `/data`            | `/workspace` (line 331)         | NO     |
| Volume size default      | Not specified      | `10` GB (line 322)              | N/A    |
| Compute plan auto-select | Based on resources | `compute_plan_from_resources()` | YES    |

### 5.3 Example Configs vs Documentation

| Example File                | Uses                                       | Documentation Says                                              | Match?                  |
| --------------------------- | ------------------------------------------ | --------------------------------------------------------------- | ----------------------- |
| `runpod-gpu-basic.yaml`     | `image: ghcr.io/sindri-labs/sindri:latest` | `image: ghcr.io/pacphi/sindri:3.0.0`                            | NO (different registry) |
| `runpod-gpu-basic.yaml`     | `gpu.tier: "a4000"`                        | tier values: `gpu-small` through `gpu-xlarge`                   | NO                      |
| `runpod-a100-training.yaml` | `gpuType: "NVIDIA A100 80GB"`              | `gpuType: "NVIDIA A100 80GB PCIe"` or `"NVIDIA A100-SXM4-80GB"` | NO (partial match)      |
| `runpod-spot.yaml`          | `gpuType: "NVIDIA RTX 4090"`               | `gpuType: "NVIDIA GeForce RTX 4090"`                            | NO (missing "GeForce")  |
| `northflank-gpu.yaml`       | `computePlan: nf-compute-400-8`            | Not in schema enum                                              | NO (but documented)     |
| `northflank-full.yaml`      | `region: us-east-1`                        | Region slug: `us-east`                                          | NO                      |
| `provider-comparison.yaml`  | `gpuType: nvidia-a4000`                    | Not in documented Northflank GPU types                          | UNCLEAR                 |

### 5.4 CHANGELOG Accuracy

The CHANGELOG `[Unreleased]` section accurately describes:

- RunPod provider with key features (REST API integration, spot, network volumes, CPU-only, three-tier storage, SSH, GPU pools, per-second billing, 31+ DCs)
- Northflank provider with key features (CLI-based, pause/resume, auto-scaling, health checks, GPU support, 20+ compute plans, persistent volumes, port forwarding, 16 regions + BYOC)
- 10 new example configurations (though only 9 files exist -- see Issue #1)
- Schema updates for both providers
- Documentation updates

**ISSUE-CL1 (Low):** CHANGELOG says "10 new example configurations" but only 9 YAML files exist in `v3/examples/`. Either a 10th example is missing or the count is wrong.

### 5.5 Provider README (`v3/docs/providers/README.md`)

**ISSUE-PR1 (Medium):** The providers README does NOT list RunPod or Northflank in its "Available Providers" table, "Quick Comparison" tables, or "Choosing a Provider" section. The file was not updated to include the new providers. All tables, comparison matrices, and examples only cover Docker, Fly.io, DevPod, E2B, Kubernetes, and VM Images.

---

## 6. Edge Cases

### 6.1 CPU-Only RunPod Deployment

- `v3/examples/runpod-cpu-only.yaml` exists and correctly uses `cpuOnly: true`
- Documentation covers CPU instance types with pricing
- The `cpuInstanceId` field is used in the example but NOT in the schema
- **ISSUE-EC1 (Low):** Schema does not define `cpuOnly` or `cpuInstanceId` fields

### 6.2 Northflank with All Features Enabled

- `v3/examples/northflank-full.yaml` exists and demonstrates comprehensive configuration
- **ISSUE-EC2 (Medium):** The example enables both `volumeSizeGb: 100` and `autoScaling.enabled: true` with `instances: 3`. The documentation explicitly states "Volumes limit the service to 1 instance" and "Cannot be used with persistent volumes." The example header warns about this, but the YAML itself has an inherent conflict that would be rejected or silently limited by the platform. The example should use a valid combination or make the conflict more visible.

### 6.3 Spot Instance Configuration (RunPod)

- `v3/examples/runpod-spot.yaml` uses `spotBid: 0.50`
- Documentation covers spot use cases and limitations
- Schema correctly defines `spotBid` as a number field
- The documentation also shows `spot: true` (boolean), which is NOT in the schema

### 6.4 Auto-Scaling Configuration (Northflank)

- `v3/examples/northflank-autoscaling.yaml` demonstrates auto-scaling
- Documentation clearly explains scaling behavior (15s intervals, 5-min cooldown)
- Auto-scaling and volume conflict is documented

### 6.5 Custom Compute Plans

- Northflank compute plans are documented but schema enum is too restrictive (only 5 of 20+)
- Examples use plans not in the schema enum (`nf-compute-400-8`, `nf-compute-800-32`)

### 6.6 Multiple GPUs

- RunPod documentation covers multi-GPU (`gpuCount: 4`) with examples
- Schema does not include a `gpuCount` field for RunPod

---

## 7. Accessibility

### 7.1 Example Files Discovery

- All 9 example files are in `v3/examples/`
- `v3/examples/README.md` exists with:
  - Table of all RunPod and Northflank examples
  - Quick reference mapping use cases to examples
  - Usage instructions (copy, edit, deploy)
  - Prerequisites for each provider
- **Assessment: EXCELLENT** -- The README is well-structured and makes examples easy to find

### 7.2 Documentation Organization

- RunPod and Northflank have dedicated docs in `v3/docs/providers/`
- `DEPLOYMENT.md` and `CONFIGURATION.md` both reference the new providers
- Cross-links between documents work (relative paths)
- **Assessment: GOOD** -- Well-organized, but providers README needs updating (ISSUE-PR1)

### 7.3 Copy-Paste-ability

- All YAML examples in documentation include proper formatting
- Quick Start sections have complete copy-paste workflows
- `cat > sindri.yaml << 'EOF'` pattern is used for inline config creation
- **Assessment: GOOD** -- Examples are ready to use

---

## 8. Issues Summary

### Critical (0)

None found.

### High Priority (0)

None found.

### Medium Priority (9)

| ID        | Component           | Description                                                                     |
| --------- | ------------------- | ------------------------------------------------------------------------------- |
| ISSUE-R2  | RUNPOD.md           | Documentation says "no CLI required" / "REST API" but code uses `runpodctl` CLI |
| ISSUE-R4  | RUNPOD.md           | Conflicting `spot` (bool) vs `spotBid` (number) config fields                   |
| ISSUE-N1  | NORTHFLANK.md       | Default `volumeMountPath` mismatch: docs say `/data`, code says `/workspace`    |
| ISSUE-N3  | NORTHFLANK.md       | Documentation says `instances: 0` valid but schema minimum is 1                 |
| ISSUE-C1  | Schema              | RunPod field name mismatches between docs, schema, and code                     |
| ISSUE-C2  | Schema              | Northflank autoScaling/healthCheck field name mismatches                        |
| ISSUE-C3  | Schema              | Northflank computePlan enum too restrictive (5 of 20+ plans)                    |
| ISSUE-UX1 | DOCTOR.md           | Doctor docs don't mention RunPod or Northflank providers                        |
| ISSUE-PR1 | providers/README.md | Providers overview does not list RunPod or Northflank                           |

### Low Priority (12)

| ID        | Component     | Description                                                              |
| --------- | ------------- | ------------------------------------------------------------------------ |
| ISSUE-R1  | RUNPOD.md     | Status says "Draft (pre-implementation)" despite implementation existing |
| ISSUE-R3  | RUNPOD.md     | Minor UX inconsistency between Quick Start and deployment commands       |
| ISSUE-R5  | RUNPOD.md     | Comparison table missing Northflank; NF comparison missing RunPod        |
| ISSUE-N2  | NORTHFLANK.md | AMD MI300X gpuType string format unclear                                 |
| ISSUE-N4  | NORTHFLANK.md | Schema healthCheck fields differ from documented fields                  |
| ISSUE-D1  | DEPLOYMENT.md | RunPod "not recommended" section could cross-reference Northflank        |
| ISSUE-UX2 | Examples      | Image registry `sindri-labs` vs `pacphi` inconsistency                   |
| ISSUE-UX3 | Examples      | GPU tier values in examples don't match schema enum                      |
| ISSUE-UX4 | Examples      | `northflank-full.yaml` uses `us-east-1` but docs say `us-east`           |
| ISSUE-EC1 | Schema        | Missing `cpuOnly`, `cpuInstanceId` fields for RunPod                     |
| ISSUE-EC2 | Examples      | `northflank-full.yaml` has volume + auto-scaling conflict                |
| ISSUE-CL1 | CHANGELOG     | Says "10 examples" but only 9 exist                                      |

### Informational (2)

| ID       | Component | Description                                                    |
| -------- | --------- | -------------------------------------------------------------- |
| ISSUE-C4 | Schema    | Northflank healthCheck schema fields differ from documentation |
| ISSUE-H1 | DOCTOR.md | `--provider` flag docs need `runpod` and `northflank` added    |

---

## 9. Recommendations

### Before Release

1. **Align schema with documentation**: The JSON schema needs a comprehensive update to match the documented field names and add missing fields for both RunPod and Northflank providers. This is the highest-impact improvement.

2. **Resolve documentation vs code mismatch for RunPod**: Either update the docs to reflect `runpodctl` CLI usage, or update the code to use direct REST API calls as documented. Currently these are contradictory.

3. **Update providers README**: Add RunPod and Northflank to the providers overview document including all comparison tables.

4. **Update DOCTOR.md**: Add RunPod and Northflank to the provider list and tool categories.

5. **Fix example registry references**: Standardize on `ghcr.io/pacphi/sindri` across all examples, or document `sindri-labs` as an alternative.

6. **Fix GPU tier values in examples**: Use valid schema tier values (`gpu-small`, `gpu-medium`, etc.) instead of free-form strings.

### Post-Release

7. **Expand Northflank computePlan enum**: Include all 20+ documented plans in the schema.

8. **Add the 10th example or fix CHANGELOG count**: Either create a missing example file or correct the "10" in the CHANGELOG.

9. **Cross-reference RunPod and Northflank in each other's comparison tables**: Each provider's comparison table should include the other.

10. **Fix `northflank-full.yaml` conflict**: Either separate the volume and auto-scaling features into different examples or add a comment explicitly explaining the platform behavior.

---

## 10. Overall Readiness Assessment

| Dimension                   | Rating | Notes                                                                       |
| --------------------------- | ------ | --------------------------------------------------------------------------- |
| Documentation completeness  | 8/10   | Comprehensive but some schema/code mismatches                               |
| Documentation accuracy      | 6/10   | Several field name mismatches between docs, schema, and code                |
| Example quality             | 7/10   | Good coverage but registry and tier value inconsistencies                   |
| Error message quality       | 9/10   | Clear, actionable, with helpful remediation hints                           |
| Code quality                | 8/10   | Clean Rust implementation with unit tests and proper trait implementation   |
| Schema completeness         | 5/10   | Many documented fields missing; field name mismatches                       |
| Cross-reference consistency | 6/10   | Providers README and DOCTOR.md not updated                                  |
| User experience flow        | 7/10   | Good overall flow, but discoverability gaps in doctor and provider overview |

**Overall Assessment: GOOD for draft/pre-implementation milestone. Needs schema and documentation alignment work before production GA.**

The core functionality is implemented and tested. The documentation is thorough and well-written. The primary gap is consistency between the three sources of truth (documentation, JSON schema, and Rust code) which need to be synchronized.
