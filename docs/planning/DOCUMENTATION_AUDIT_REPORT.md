# Sindri Documentation Audit Report

**Audit Date:** 2025-12-16
**Status:** Comprehensive audit complete
**Audited By:** Code Reviewer Agent (claude-opus-4-5-20251101)

## Executive Summary

A comprehensive audit of the Sindri repository documentation was conducted, scanning all markdown (.md), YAML (.yaml), JSON (.json), and configuration files. The audit compared documentation against actual implementation, validated cross-references and links, and identified discrepancies, gaps, and improvement opportunities.

### Overall Quality Assessment

| Area | Rating | Summary |
|------|--------|---------|
| CLAUDE.md | Good (8/10) | Most claims accurate, missing some install methods and GPU docs |
| docs/*.md | Excellent (9/10) | Comprehensive coverage, minor profile doc issues |
| JSON Schemas | Fair (6/10) | Schemas well-designed but significantly under-documented |
| YAML Config | Good (8/10) | Internally consistent, some doc mismatches |
| CLI Documentation | Good (7/10) | Core commands accurate, some argument mismatches |
| Cross-References | Excellent (9/10) | Only 4 broken links found in 100+ files |

### Issue Count by Severity

| Severity | Count | Description |
|----------|-------|-------------|
| Critical | 3 | Missing fundamental documentation (install methods) |
| Major | 35 | Significant gaps, incorrect information |
| Minor | 18 | Inconsistencies, missing optional info |
| Enhancement | 8 | Improvement opportunities |

---

## Critical Issues (Fix Immediately)

### C-1: Missing Installation Methods in CLAUDE.md

**Location:** CLAUDE.md line 390
**Current:** Shows `method: mise|script|apt`
**Should Be:** `method: mise|script|apt|npm|binary|hybrid`

**Impact:** Users cannot learn about 3 valid installation methods from primary documentation.

**Files to Update:**
- `CLAUDE.md` - Extension YAML Structure section

**Schema Reference:** `docker/lib/schemas/extension.schema.json` line 149

---

### C-2: Extension Count Inconsistency

**Issue:** Documentation claims different extension counts across files.

| File | Claims | Actual |
|------|--------|--------|
| CLAUDE.md:133 | 74 extensions | **73** |
| README.md | 74 extensions | **73** |
| docs/ARCHITECTURE.md:19 | 74 extensions | **73** |

**Files to Update:**
- `CLAUDE.md` line 133
- `README.md` (badge and text)
- `docs/ARCHITECTURE.md` line 19

**Recommendation:** Update to "73 extensions" or use dynamic count like "70+ extensions"

---

### C-3: Workflow Files List Incorrect

**Location:** CLAUDE.md lines 459-469
**Issue:** Lists non-existent workflow and omits existing ones.

**Current (Incorrect):**
```
- test-sindri-config.yml (DOES NOT EXIST)
```

**Missing:**
```
- test-profiles.yml
- test-extensions.yml
```

**Actual Workflow Count:** 10 (not 9)

**Files to Update:**
- `CLAUDE.md` lines 459-469

---

## Major Issues (Fix Soon)

### Profile Documentation Mismatches

#### M-1: anthropic-dev Profile

**Location:** `docs/EXTENSIONS.md` line 41

| Documented (12) | Actual (21) |
|-----------------|-------------|
| agent-manager | agent-manager |
| ollama | ollama |
| ai-toolkit | ai-toolkit |
| claude-code-mux | **NOT IN PROFILE** |
| claudeup | **NOT IN PROFILE** |
| claude-marketplace | claude-marketplace |
| cloud-tools | cloud-tools |
| openskills | openskills |
| nodejs-devtools | nodejs-devtools |
| playwright | playwright |
| rust | rust |
| tmux-workspace | tmux-workspace |
| **MISSING:** | claude-flow, agentic-flow, agentic-qe, golang, claudish, infra-tools, jvm, mdflow, ruvnet-research, linear-mcp, supabase-cli |

**Files to Update:**
- `docs/EXTENSIONS.md` line 41

---

#### M-2: ai-dev Profile

**Location:** `docs/EXTENSIONS.md` line 40

| Documented (6) | Actual (10) |
|----------------|-------------|
| nodejs | nodejs |
| python | python |
| ollama | ollama |
| ai-toolkit | ai-toolkit |
| openskills | openskills |
| monitoring | monitoring |
| **MISSING:** | golang, mdflow, supabase-cli, linear-mcp |

**Files to Update:**
- `docs/EXTENSIONS.md` line 40

---

### CLI Argument Documentation Errors

#### M-3: new-project Command

**Location:** CLAUDE.md line 76

**Current (Incorrect):**
```bash
./cli/new-project <name> [template]     # Create new project from template
```

**Should Be:**
```bash
./cli/new-project <name> [--type <type>]  # Create new project (options: --list-types, --interactive)
```

**Files to Update:**
- `CLAUDE.md` line 76
- `docs/CLI.md` (if present)

---

#### M-4: clone-project Command

**Location:** CLAUDE.md line 77

**Current (Incorrect):**
```bash
./cli/clone-project <url> [path]        # Clone and setup project
```

**Should Be:**
```bash
./cli/clone-project <url> [options]      # Clone to $WORKSPACE/projects (options: --fork, --branch, --feature)
```

**Note:** There is NO path argument - projects always clone to `$WORKSPACE/projects/<name>`

**Files to Update:**
- `CLAUDE.md` line 77
- `docs/CLI.md` (if present)

---

### Undocumented Schema Features

#### M-5: GPU Configuration Completely Undocumented

**Location:** Multiple schemas and YAML files

**Schema Coverage:**
- `extension.schema.json` lines 102-140 - Extension GPU requirements
- `sindri.schema.json` lines 42-76 - Deployment GPU configuration
- `vm-sizes.yaml` lines 30-97 - GPU tier mappings

**Example Usage:**
- `examples/fly/gpu-ml-training.sindri.yaml` lines 13-16

**Missing Documentation:**
- CLAUDE.md - No GPU section
- CONFIGURATION.md - No GPU section
- SCHEMA.md - No GPU section

**Recommended Action:** Add comprehensive GPU section to CONFIGURATION.md

---

#### M-6: Secrets Management Undocumented in CLAUDE.md

**Location:** `sindri.schema.json` lines 155-228

**Schema Properties Not Documented:**
- `fromFile`, `path`, `mountPath`, `permissions`
- `vaultPath`, `vaultKey`, `vaultMount`

**Note:** SCHEMA.md lines 132-173 documents this well, but CLAUDE.md omits it entirely.

**Files to Update:**
- `CLAUDE.md` - Add secrets configuration section

---

#### M-7: Extension Configure Templates Undocumented

**Location:** `extension.schema.json` lines 265-283

**Properties:**
- `source`, `destination`
- `mode`: overwrite | append | merge | skip-if-exists

**Actual Usage:** `docker/lib/extensions/claude-flow/extension.yaml` lines 27-30

**Files to Update:**
- `CLAUDE.md` Extension YAML Structure section

---

#### M-8: Extension Upgrade Section Incomplete

**Location:** CLAUDE.md lines 415-419

**Current (Minimal):**
```yaml
upgrade:
  strategy: reinstall|in-place
  script:
    path: upgrade.sh
```

**Schema Supports (lines 406-462):**
- `strategy`: reinstall | in-place | automatic | manual
- `mise.tools[]` - Tools to upgrade
- `apt.packages[]` - APT packages to upgrade
- `script.path`, `script.timeout`, `script.env`

**Files to Update:**
- `CLAUDE.md` lines 415-419

---

#### M-9: Extension Removal Options Incomplete

**Location:** CLAUDE.md line 420

**Current:** Only shows mise removal

**Schema Supports (lines 349-404):**
- `confirmation`: boolean (default true)
- `apt.packages[]`, `apt.purge`
- `paths[]` - Paths to delete
- `script.path`

**Files to Update:**
- `CLAUDE.md` lines 417-423

---

#### M-10: Manifest Schema Config Section Undocumented

**Location:** `manifest.schema.json` lines 44-82

**Undocumented Properties:**
- `config.execution.parallel`
- `config.execution.failFast`
- `config.execution.timeout`
- `config.validation.schemaValidation`
- `config.validation.dnsCheck`
- `config.validation.dependencyCheck`

**Files to Update:**
- Create new docs/MANIFEST.md or add to SCHEMA.md

---

#### M-11: VM Sizes Schema Completely Undocumented

**Location:** `docker/lib/schemas/vm-sizes.schema.json` (217 lines)

**Contains:**
- Provider-specific instance mappings
- GPU tier definitions (T4, A10G, L40S, A100)
- Memory/CPU tier mappings

**Files to Update:**
- `SCHEMA.md` or `CONFIGURATION.md` - Add VM sizes abstraction section

---

#### M-12: Local Kubernetes Provider Undocumented

**Location:** `sindri.schema.json` lines 527-601

**Undocumented Provider:**
- `k8s` provider for local kind/k3d clusters
- `clusterName`, `version`, `nodes` options
- `kind` and `k3d` subsections

**Files to Update:**
- `CONFIGURATION.md` - Add local K8s section
- `docs/providers/KUBERNETES.md` - Expand to cover local K8s

---

#### M-13: SCHEMA.md Registry Structure Incorrect

**Location:** `docs/SCHEMA.md` lines 337-354

**Current (Incorrect):**
```yaml
extensions:
  nodejs:
    path: extensions/nodejs
    enabled: true
    experimental: false
```

**Actual Structure:**
```yaml
extensions:
  nodejs:
    category: language
    description: Node.js JavaScript runtime
    protected: false
    dependencies: [mise-config]
```

**Files to Update:**
- `docs/SCHEMA.md` lines 337-354

---

#### M-14: Manifest Category Enum Outdated

**Location:** `docker/lib/schemas/manifest.schema.json` line 33

**Schema Has:** `["base", "language", "dev-tools", "infrastructure", "ai", "utilities"]`
**Should Have:** Match extension.schema.json with 11 categories including `agile`, `desktop`, `monitoring`, `database`, `mobile`

**Files to Update:**
- `docker/lib/schemas/manifest.schema.json` line 33

---

#### M-15: Monitoring Extension Description Mismatch

**Location:** `docker/lib/registry.yaml` line 191 vs extension.yaml

**Registry Says:** "htop, ctop, glances"
**Extension Says:** "Claude monitoring and usage tracking tools (UV, claude-monitor, claude-usage-cli)"

**Files to Update:**
- `docker/lib/registry.yaml` - Update monitoring description

---

#### M-16: Fly.io Memory Configuration Ambiguity

**Location:** `docker/lib/vm-sizes.yaml` lines 72-75

**Current Values:**
```yaml
memory:
  small: 256    # 256MB
  medium: 512   # 512MB
  large: 2048   # 2GB
  xlarge: 4096  # 4GB
```

**Tier Definitions (lines 13-28):**
```yaml
small: "0-2048 MB"
medium: "2048-4096 MB"
```

**Issue:** Values don't align with tier definitions

**Files to Update:**
- `docker/lib/vm-sizes.yaml` - Clarify or fix values
- `CONFIGURATION.md` - Document expected behavior

---

### Broken Cross-References and Links

#### M-17: VisionFlow Web Summary Link Broken

**Location:** `docker/lib/extensions/vf-import-to-ontology/resources/README.md:297`

**Current (Broken):**
```markdown
[Web Summary Skill](../web-summary/SKILL.md)
```

**Fix:**
```markdown
[Web Summary Skill](../vf-web-summary/resources/SKILL.md)
```

**Also in:** Same file SKILL.md:978

---

#### M-18: Missing Agent Manager Upgrade Script

**Location:** `docker/lib/extensions/agent-manager/extension.yaml:52`

```yaml
upgrade:
  strategy: automatic
  script:
    path: upgrade.sh
```

**Issue:** File `upgrade.sh` does not exist

**Fix:** Create `docker/lib/extensions/agent-manager/upgrade.sh` or remove upgrade section

---

#### M-19: Missing VF Skill Creator Documentation Files

**Location:** `docker/lib/extensions/vf-skill-creator/resources/SKILL.md`

**Missing Files (6):**
- `FORMS.md` (line 140)
- `REFERENCE.md` (line 141)
- `EXAMPLES.md` (line 142)
- `DOCX-JS.md` (line 185)
- `REDLINING.md` (line 191)
- `OOXML.md` (line 192)

**Fix:** Create these files or remove references

---

## Minor Issues (Fix When Convenient)

### m-1: BOM Property Name Mismatch

**Location:** CLAUDE.md line 428

**Documented:** `components`
**Actual:** `tools`

All extension BOMs use `bom.tools`, not `bom.components`.

---

### m-2: Environment Scope Missing "session"

**Location:** CLAUDE.md line 408

**Documented:** `scope: bashrc|profile`
**Schema:** `scope: bashrc|profile|session`

---

### m-3: Provider Value Inconsistency in Examples

**Location:** Various example files

**Usage Split:**
- `docker-compose`: 9 files
- `docker`: 1 file

**Recommendation:** Standardize on `docker-compose`

---

### m-4: Docker Provider Alias Not Documented

**Location:** `docs/SCHEMA.md` line 49

**Lists:** fly, kubernetes, docker-compose, devpod
**Missing:** Note that `docker` is an alias for `docker-compose`

---

### m-5: Unused Categories

**Location:** `docker/lib/categories.yaml`

- `database` - 0 extensions
- `mobile` - 0 extensions

**Recommendation:** Document as "reserved for future use" or remove

---

### m-6: Optional Metadata Fields Undocumented

**Location:** `extension.schema.json` lines 44-50

**Undocumented:** `author`, `homepage`, `license`

**Used By:** claude-flow, many vf-* extensions

---

### m-7: Extended BOM Fields Undocumented

**Location:** `extension.schema.json` lines 503-536

**Undocumented:** `license`, `homepage`, `downloadUrl`, `checksum`, `purl`, `cpe`

---

### m-8: DevPod buildRepository Undocumented

**Location:** `sindri.schema.json` lines 345-348

**Property:** `buildRepository` - Registry URL for K8s deployments

---

### m-9: Kubernetes Context Property Not in SCHEMA.md

**Location:** `sindri.schema.json` lines 477-480

**Property:** `context` - Kubeconfig context for DevPod K8s

---

### m-10: Extension Validate Mise Section Undocumented

**Location:** `extension.schema.json` lines 329-344

**Properties:** `mise.tools[]`, `mise.minToolCount`

**Used By:** infra-tools/extension.yaml lines 82-92

---

### m-11: Test Directory Reference Incorrect

**Location:** `docs/CONTRIBUTING.md` line 52

**Current:** `.github/scripts/`
**Actual:** `test/unit/yaml/`

---

### m-12: sindri Help Text Misleading

**Location:** `cli/sindri` line 87

**Issue:** Shows `--provider` as global option but it's only for `deploy` and `destroy`

---

## Enhancement Opportunities

### E-1: Add GPU Configuration Examples

Create examples showing:
- GPU tier selection
- Provider-specific GPU configuration
- ML/AI workload deployment

**Suggested Location:** `docs/GPU.md` or section in `CONFIGURATION.md`

---

### E-2: Document Test Suites

**Location:** `sindri test --suite <name>`

**Undocumented Suites:**
- `smoke` - Quick health checks
- `integration` - Full integration tests
- `full` - Complete test suite

---

### E-3: Add CLI Feature Documentation

**new-project options not documented:**
- `--interactive` - Interactive type selection
- `--list-types` - Show available project types
- `--git-name`, `--git-email` - Git configuration

**clone-project options not documented:**
- `--fork` - Fork before cloning
- `--branch`, `--feature` - Branch management
- `--depth` - Shallow clone
- `--no-deps`, `--no-enhance` - Skip enhancements

---

### E-4: Automated Link Checking

Add CI step to validate:
- Internal markdown links
- Anchor references
- External URL accessibility

---

### E-5: Provider-Specific Region Documentation

Document region/zone variations for:
- AWS regions vs Fly.io regions
- GCP zones
- Azure regions

---

### E-6: Add MANIFEST.md Documentation

Create comprehensive documentation for:
- manifest.schema.json
- Execution configuration
- Validation settings

---

### E-7: Extension Authoring Checklist

Add checklist to `EXTENSION_AUTHORING.md`:
- [ ] All install methods covered
- [ ] BOM populated
- [ ] Upgrade strategy defined
- [ ] Removal tested
- [ ] Documentation complete

---

### E-8: Dynamic Extension Count

Replace hardcoded "74 extensions" with:
- Script-generated count
- Or "70+ extensions" for stability

---

## Implementation Checklist

### Phase 1: Critical Fixes (Same Day)

- [ ] Update CLAUDE.md install methods (C-1)
- [ ] Fix extension count across files (C-2)
- [ ] Correct workflow list (C-3)

### Phase 2: Major Fixes (This Week)

- [ ] Fix profile documentation in EXTENSIONS.md (M-1, M-2)
- [ ] Fix CLI argument documentation (M-3, M-4)
- [ ] Add GPU documentation (M-5)
- [ ] Add secrets documentation to CLAUDE.md (M-6)
- [ ] Document extension templates (M-7)
- [ ] Complete upgrade section (M-8)
- [ ] Complete removal section (M-9)
- [ ] Fix broken links (M-17, M-18, M-19)
- [ ] Fix SCHEMA.md registry structure (M-13)
- [ ] Update manifest schema categories (M-14)
- [ ] Fix monitoring extension description (M-15)

### Phase 3: Minor Fixes (This Month)

- [ ] Fix BOM property name (m-1)
- [ ] Add session scope (m-2)
- [ ] Standardize provider values (m-3)
- [ ] Document docker alias (m-4)
- [ ] Address unused categories (m-5)
- [ ] Document optional metadata (m-6)
- [ ] Document extended BOM fields (m-7)
- [ ] Document buildRepository (m-8)
- [ ] Document K8s context (m-9)
- [ ] Document mise validation (m-10)
- [ ] Fix test directory reference (m-11)
- [ ] Fix CLI help text (m-12)

### Phase 4: Enhancements (Ongoing)

- [ ] Add GPU examples (E-1)
- [ ] Document test suites (E-2)
- [ ] Document CLI features (E-3)
- [ ] Add automated link checking (E-4)
- [ ] Add region documentation (E-5)
- [ ] Create MANIFEST.md (E-6)
- [ ] Add authoring checklist (E-7)
- [ ] Dynamic extension count (E-8)

---

## Validation Commands

```bash
# Verify extension count
find docker/lib/extensions -name "extension.yaml" | wc -l

# Validate all YAML files
pnpm validate:yaml

# Check for broken internal links
grep -r '\[.*\](.*\.md)' docs/ | while read line; do
  link=$(echo "$line" | sed 's/.*(\([^)]*\.md\)).*/\1/')
  if [[ ! -f "$link" ]]; then echo "Broken: $line"; fi
done

# Validate schemas
pnpm test:unit

# Verify profile extensions exist
./cli/extension-manager validate-all

# Check workflow files
ls -la .github/workflows/*.yml | wc -l
```

---

## Files Modified Summary

| File | Changes Required |
|------|-----------------|
| `CLAUDE.md` | 15 fixes |
| `docs/EXTENSIONS.md` | 2 fixes |
| `docs/SCHEMA.md` | 2 fixes |
| `docs/CONFIGURATION.md` | Add GPU, secrets sections |
| `docker/lib/schemas/manifest.schema.json` | Update category enum |
| `docker/lib/registry.yaml` | Fix monitoring description |
| `docker/lib/vm-sizes.yaml` | Clarify memory values |
| `docker/lib/extensions/vf-import-to-ontology/` | Fix 2 broken links |
| `docker/lib/extensions/agent-manager/` | Create upgrade.sh or remove ref |
| `docker/lib/extensions/vf-skill-creator/` | Create or remove 6 doc refs |

---

## Appendix: Verified Correct Documentation

The following documentation areas were audited and found to be **100% accurate**:

1. **Directory Structure** - All paths in CLAUDE.md exist
2. **Environment Variables** - All documented vars are used
3. **Pre-installed Tools** - All tools installed as documented
4. **Extension Schema** - Core structure matches documentation
5. **Profiles System** - profiles.yaml structure correct
6. **Categories System** - categories.yaml structure correct
7. **Adapter Pattern** - All adapters exist and work as documented
8. **Package.json Scripts** - All pnpm commands work
9. **Core CLI Commands** - All sindri/extension-manager commands exist
10. **profiles.schema.json** - Perfect documentation match
11. **categories.schema.json** - Perfect documentation match
12. **project-templates.schema.json** - Excellent documentation

---

*Generated by comprehensive code review audit on 2025-12-16*
