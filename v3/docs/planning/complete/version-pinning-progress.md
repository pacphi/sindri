# Version Pinning Progress Report - FINAL UPDATE

**Last Updated**: 2026-02-09 17:00 UTC
**Status**: Phase 2B - 54% Complete (27 of 50 extensions)
**Achievement**: BOM-to-Install-Script synchronization working

---

## Current State

### Extensions by Status

**FULLY PINNED** (16 extensions): All tools have explicit versions, install scripts updated
**EXCEPTIONAL CASES** (11 extensions): Documented with rationale for dynamic/semantic versions
**REMAINING** (23 extensions): Need research or documentation

### Progress Breakdown

```
Total Extensions: 50
├── FULLY PINNED: 16 (32%)
│   ├── infra-tools, cloud-tools, jvm
│   ├── nodejs-devtools, playwright, supabase-cli
│   ├── php (partial), pal-mcp-server
│   ├── agent-browser, claude-flow-v3
│   ├── agentic-flow, agentic-qe, claude-flow-v2
│   ├── claudeup, claudish, compahook
│   └── openskills, ruvnet-research
│
├── EXCEPTIONAL: 11 (22%)
│   ├── Semantic: rust, nodejs, goose (3)
│   ├── Remote: context7/jira/linear/excalidraw-mcp (4)
│   └── apt-managed: docker, github-cli, dotnet, ollama (4)
│
└── REMAINING: 23 (46%)
    ├── Bundled tools: 5 extensions (ruby, python, php, nodejs secondary)
    ├── Need research: 8 extensions (haskell, ai-toolkit, etc.)
    └── apt-managed: 2 extensions (tmux-workspace, xfce-ubuntu)
```

---

## Fully Pinned Extensions (16)

### 1. infra-tools (14 tools) ✅

**Files Modified**: mise.toml, extension.yaml, install-additional.sh

| Tool       | Version | Source           |
| ---------- | ------- | ---------------- |
| terraform  | 1.14    | mise             |
| kubectl    | 1.35.0  | mise             |
| helm       | 4.1.0   | mise             |
| k9s        | 0.50.18 | mise (asdf)      |
| ansible    | 13.3.0  | apt (documented) |
| pulumi     | 3.219.0 | script           |
| crossplane | 2.2     | script           |
| kubectx    | 0.9.5   | script           |
| kubens     | 0.9.5   | script           |
| kapp       | 0.65.0  | script           |
| ytt        | 0.52.2  | script           |
| kbld       | 0.45.2  | script           |
| vendir     | 0.43.0  | script           |
| imgpkg     | 0.46.0  | script           |

### 2. cloud-tools (7 tools) ✅

**Files Modified**: extension.yaml, install.sh

| Tool     | Version | Install Method         |
| -------- | ------- | ---------------------- |
| aws      | 2.27.41 | Versioned zip download |
| az       | 2.83.0  | pip with version       |
| gcloud   | 555.0.0 | Versioned tarball      |
| aliyun   | 3.2.9   | GitHub release         |
| doctl    | 1.148.0 | GitHub release         |
| flyctl   | 0.4.7   | GitHub release         |
| ibmcloud | 2.41.0  | GitHub release         |

### 3. jvm (7 tools) ✅

**Files Modified**: extension.yaml, install.sh

| Tool      | Version  | Method |
| --------- | -------- | ------ |
| java      | 25 (LTS) | SDKMAN |
| maven     | 3.9.12   | SDKMAN |
| gradle    | 9.3.1    | SDKMAN |
| kotlin    | 2.3.10   | SDKMAN |
| scala     | 3.8.1    | SDKMAN |
| clojure   | 1.12     | mise   |
| leiningen | 2.12     | mise   |

### 4-16. npm-Based Tools ✅

**All synced from mise.toml**:

- nodejs-devtools (5), playwright (1), supabase-cli (1)
- agent-browser, agentic-flow, agentic-qe
- claude-flow-v2, claude-flow-v3, claudeup, claudish
- compahook, openskills, ruvnet-research, pal-mcp-server

---

## Exceptional Cases (11 extensions)

### Semantic Versioning (3) ✅

**Rationale**: Valid semantic versions for release channels

- **rust**: rustc "stable", cargo "stable" (Rust channels)
- **nodejs**: node "lts" (Node.js LTS channel)
- **goose**: "stable" (GitHub stable tag)

### Remote Version Tracking (4) ✅

**Rationale**: MCP servers track remote package versions

- **context7-mcp**: version "remote"
- **jira-mcp**: version "remote"
- **linear-mcp**: version "remote"
- **excalidraw-mcp**: version "remote"

### apt-Managed (4) ✅

**Rationale**: Ubuntu repository versions vary by release

- **docker**: 4 tools (target: 29.2.1, compose 5.0.2)
- **github-cli**: gh (target: 2.87.2)
- **dotnet**: .NET SDK (target: 10.0.2 LTS)
- **ollama**: Ollama (target: 0.15.6)

**All documented with target versions in BOM comments**

---

## Remaining Work (23 extensions)

### Category 1: Bundled Tools (5 extensions)

**Action**: Add documentation comments

- **ruby**: gem, bundle (bundled with ruby 3.4.7)
- **python**: uvx (bundled with uv/python)
- **php**: composer, symfony (PHP 8.4 pinned, these dynamic)
- **nodejs**: npm, npx, pnpm (bundled with node lts)
- **compahook**: node (bundled, compahook itself pinned)

**Note**: These SHOULD remain dynamic (version matches parent runtime)

### Category 2: Need Research (8 extensions)

**Action**: Research latest versions, update install scripts/BOMs

- **haskell** (5 tools: ghcup, ghc, cabal, stack, hls)
- **ai-toolkit** (5 tools: fabric, codex, gemini, droid, grok)
- **monitoring** (3 tools: uv, claude-monitor, claude-usage)
- **agent-manager** (1 tool)
- **claude-code-mux**, **claude-marketplace** (2 tools)
- **mdflow** (1 tool)
- **mise-config**, **ralph**, **spec-kit** (3 tools)

### Category 3: apt-Managed (2 extensions)

**Action**: Add documentation comments (like docker/github-cli)

- **tmux-workspace** (tmux, htop)
- **xfce-ubuntu** (xfce4, xrdp, firefox, mousepad, thunar)

### Category 4: Others (8 extensions)

**Action**: Verify, minimal changes expected

- Python/Ruby secondary tools already verified
- Claude extensions (claude-cli, etc.) - likely npm-based
- MCP servers already handled

---

## Methodology Reference

### Pattern A: Mise with Regular Tools

```bash
# mise.toml
[tools]
go = "1.26"

# Manually sync to extension.yaml bom.tools
# Update version: dynamic → version: "1.26"
```

### Pattern B: Mise with npm Tools

```bash
# mise.toml
[tools]
"npm:package" = "1.2.3"

# Manual sync (audit script doesn't handle npm: prefix)
# Update extension.yaml bom.tools.version to match
```

### Pattern C: Script with GitHub Releases

```bash
# install.sh - Pin version
VERSION="1.2.3"
curl -L "https://github.com/org/tool/releases/download/v${VERSION}/tool.tar.gz"

# extension.yaml - Match version
bom:
  tools:
    - name: tool
      version: 1.2.3  # Pinned in install script
```

### Pattern D: SDKMAN

```bash
# install.sh
install_sdk_tool maven mvn 3.9.12

# extension.yaml
bom:
  tools:
    - name: mvn
      version: 3.9.12  # Pinned in install script
```

### Pattern E: pip

```bash
# install.sh
pip install --user "package==1.2.3"

# extension.yaml
bom:
  tools:
    - name: package
      version: 1.2.3  # Pinned in install script
```

### Pattern F: npm/pnpm

```bash
# install.sh
pnpm add -D package@1.2.3

# extension.yaml
bom:
  tools:
    - name: package
      version: 1.2.3  # Pinned in install script
```

### Pattern X: EXCEPTIONAL - Document Only

```bash
# extension.yaml
bom:
  # EXCEPTIONAL CASE: apt-managed (cannot pin)
  # Target version (as of 2026-02-09): X.Y.Z
  tools:
    - name: tool
      version: dynamic  # apt-managed
```

OR

```yaml
# For bundled tools
bom:
  tools:
    - name: npm
      version: dynamic # Bundled with node (lts)
```

OR

```yaml
# For semantic versions
bom:
  tools:
    - name: rust
      version: stable # Rust release channel
```

---

## Statistics

### Tools by Category

- **Explicitly Pinned**: 60+ tools
- **Semantic Versions**: 6 tools (stable, lts, remote)
- **Exceptional (apt)**: 15+ tools
- **Bundled (should stay dynamic)**: 12+ tools
- **Need Research**: 22 tools

### Files Modified (27 extensions)

- **30 extension.yaml files** updated
- **10 install scripts** modified
- **2 mise.toml files** updated
- **~700 lines of code** changed

---

## Quick Reference Commands

```bash
# Generate full BOM
cargo run -- bom generate

# Check specific extension
cargo run -- bom show infra-tools    # All pinned
cargo run -- bom show docker         # Exceptional (apt)
cargo run -- bom show nodejs         # Mixed (lts + bundled)

# Count remaining dynamic
cargo run -- bom list | grep dynamic | wc -l

# Export for auditing
cargo run -- bom export --format cyclonedx --output sbom.json
cargo run -- bom export --format spdx --output sbom-spdx.json

# Verify specific versions
cargo run -- bom list | grep kubectl  # Should show 1.35.0
cargo run -- bom list | grep aws      # Should show 2.27.41
```

---

## Completion Checklist

### Phase 2B - Remaining Tasks

**Quick Documentation** (1-2 hours):

- [ ] Add bundled tool comments (ruby, python, php, nodejs)
- [ ] Document tmux-workspace as exceptional
- [ ] Document xfce-ubuntu as exceptional

**Research & Update** (3-4 hours):

- [ ] haskell (5 tools)
- [ ] ai-toolkit (5 tools)
- [ ] monitoring (3 tools)
- [ ] Other 5 extensions

**Total Remaining**: ~5-6 hours to complete Phase 2B

---

**Document Status**: FINAL UPDATE FOR SESSION
**Next Session**: Continue with bundled tool documentation + research items
