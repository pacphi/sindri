# Sindri V2 to V3 Migration Guide

**Version:** 1.0.0
**Created:** 2026-01-24
**Current V2 Version:** 2.2.1
**Target V3 Version:** 3.0.0

---

## Executive Summary

This guide provides step-by-step instructions for migrating from Sindri V2 (Bash-based) to Sindri V3 (Rust-based). V3 is a complete rewrite delivering significant improvements:

| Benefit                | Details                                           |
| ---------------------- | ------------------------------------------------- |
| **Single 12MB binary** | Zero runtime dependencies (replaces bash, yq, jq) |
| **78% code reduction** | 52K Bash lines → 11.2K Rust lines                 |
| **10-50x faster**      | Configuration parsing and validation              |
| **Cross-platform**     | Native binaries for Linux, macOS, Windows         |
| **Self-update**        | Built-in upgrade with compatibility checks        |
| **Enhanced security**  | S3 encrypted secrets, image verification          |

---

## Table of Contents

1. [Pre-Migration Checklist](#pre-migration-checklist)
2. [Breaking Changes](#breaking-changes)
3. [Command Mapping](#command-mapping-v2-to-v3)
4. [Step-by-Step Migration](#step-by-step-migration)
5. [Rollback Procedures](#rollback-procedures)
6. [Post-Migration Validation](#post-migration-validation)
7. [Common Issues & Solutions](#common-issues--solutions)
8. [CI/CD Considerations](#cicd-considerations)

---

## Pre-Migration Checklist

### Inventory Your Environment

- [ ] Document installed extensions:

  ```bash
  ./v2/cli/extension-manager list
  ```

- [ ] Export extension bill of materials:

  ```bash
  ./v2/cli/extension-manager bom > extensions-backup.yaml
  ```

- [ ] List profiles in use:

  ```bash
  ./v2/cli/sindri profiles list
  ```

- [ ] Identify custom extensions in `v2/docker/lib/extensions/`

- [ ] Note custom scripts using V2 CLI commands

- [ ] **Check for VisionFlow extensions** (vf-\* prefixed) - NOT available in V3

### Create Backups

- [ ] Full backup:

  ```bash
  ./v2/cli/sindri backup --profile full --output v2-backup-$(date +%Y%m%d).tar.gz
  ```

- [ ] Configuration backup:

  ```bash
  cp sindri.yaml sindri.yaml.v2.backup
  ```

- [ ] Secrets configuration backup

- [ ] Vault secrets export (if using Vault backend)

### Verify Environment

- [ ] Docker 20.10+: `docker --version`
- [ ] Git installed: `git --version`
- [ ] Provider tools (flyctl, devpod, kubectl) if needed
- [ ] Disk space for V3 binary (~12MB)

### Review Compatibility

- [ ] Read breaking changes section below
- [ ] Check for removed extensions:
  - `claude-flow` v1 → use `claude-flow-v2` or `claude-flow-v3`
  - `claude-auth-with-api-key` → now built-in
  - `ruvnet-aliases` → consolidated into extensions
- [ ] Test V3 in staging before production

---

## Breaking Changes

### CLI Architecture Changes

| Change                          | Impact                             | Migration                 |
| ------------------------------- | ---------------------------------- | ------------------------- |
| CLI rewritten from Bash to Rust | Bash-based extensions incompatible | Use V3 extension format   |
| Single binary distribution      | No longer requires bash, yq, jq    | Download pre-built binary |

### Configuration Changes

| Change                       | Impact                                                              | Migration                        |
| ---------------------------- | ------------------------------------------------------------------- | -------------------------------- |
| Config schema updated to 3.0 | sindri.yaml format changes                                          | Auto-migrated; backup created    |
| Manifest format changed      | ~/.sindri/manifest.yaml requires migration                          | Auto-migrated; backup created    |
| Vault path renamed           | `secrets.provider.vault.path` → `secrets.provider.vault.mount_path` | Update manually or auto-migrated |

### Extension Changes

| Change                          | Impact                         | Migration                           |
| ------------------------------- | ------------------------------ | ----------------------------------- |
| Schema updated to v1.0          | Stricter validation            | Run `sindri extension validate-all` |
| Install method 'manual' removed | Extensions using 'manual' fail | Use 'script' method instead         |
| VisionFlow extensions excluded  | vf-\* only in V2               | Continue using V2 for VisionFlow    |

### Removed Extensions

| Extension                  | Replacement                                           |
| -------------------------- | ----------------------------------------------------- |
| `claude-flow` (v1)         | `claude-flow-v2` (stable) or `claude-flow-v3` (alpha) |
| `claude-auth-with-api-key` | Built-in authentication                               |
| `ruvnet-aliases`           | Consolidated into individual extensions               |

### Removed Features

- CLI tool: `init-claude-flow-agentdb`
- Install method: `manual`
- Extension: `claude-flow` v1
- Extension: `claude-auth-with-api-key`
- Extension: `ruvnet-aliases`

---

## Command Mapping (V2 to V3)

### Deployment Commands

| V2 Command                               | V3 Command       | Notes                 |
| ---------------------------------------- | ---------------- | --------------------- |
| `./v2/cli/sindri deploy --provider <p>`  | `sindri deploy`  | Provider from config  |
| `./v2/cli/sindri destroy --provider <p>` | `sindri destroy` | Added --volumes flag  |
| `./v2/cli/sindri connect`                | `sindri connect` | Added -c/--command    |
| `./v2/cli/sindri status`                 | `sindri status`  | Added --json, --watch |

### Configuration Commands

| V2 Command                        | V3 Command               | Notes                    |
| --------------------------------- | ------------------------ | ------------------------ |
| `./v2/cli/sindri config init`     | `sindri config init`     | Added --profile          |
| `./v2/cli/sindri config validate` | `sindri config validate` | Added --check-extensions |
| `./v2/cli/sindri profiles list`   | `sindri profile list`    | Singular 'profile'       |

### Extension Commands

| V2 Command                                       | V3 Command                          | Notes                     |
| ------------------------------------------------ | ----------------------------------- | ------------------------- |
| `./v2/cli/extension-manager list`                | `sindri extension list`             | Merged into CLI           |
| `./v2/cli/extension-manager install <n>`         | `sindri extension install <n>`      | Added @version, --profile |
| `./v2/cli/extension-manager install-profile <n>` | `sindri profile install <n>`        | Moved to profile          |
| `./v2/cli/extension-manager validate <n>`        | `sindri extension validate <n>`     | Added --file              |
| `./v2/cli/extension-manager status [n]`          | `sindri extension status [n]`       | Added --json              |
| `./v2/cli/extension-manager info <n>`            | `sindri extension info <n>`         | Added --json              |
| `./v2/cli/extension-manager bom [n]`             | `sindri extension list --installed` | Auto BOM                  |

### Secrets Commands

| V2 Command                          | V3 Command                | Notes           |
| ----------------------------------- | ------------------------- | --------------- |
| `./v2/cli/secrets-manager validate` | `sindri secrets validate` | Merged into CLI |
| `./v2/cli/secrets-manager list`     | `sindri secrets list`     | Added --source  |

### Backup/Restore Commands

| V2 Command                             | V3 Command                    | Notes                  |
| -------------------------------------- | ----------------------------- | ---------------------- |
| `./v2/cli/sindri backup --profile <p>` | `sindri backup --profile <p>` | Added --encrypt        |
| `./v2/cli/sindri restore <file>`       | `sindri restore <source>`     | Added S3/HTTPS sources |

### Project Commands

| V2 Command                     | V3 Command                   | Notes            |
| ------------------------------ | ---------------------------- | ---------------- |
| `./v2/cli/new-project <name>`  | `sindri project new <name>`  | Moved to project |
| `./v2/cli/clone-project <url>` | `sindri project clone <url>` | Moved to project |

### New V3-Only Commands

| Command                                               | Description                           |
| ----------------------------------------------------- | ------------------------------------- |
| `sindri version`                                      | Version info with --json              |
| `sindri upgrade`                                      | Self-update with compatibility checks |
| `sindri doctor`                                       | System health with --fix              |
| `sindri k8s create/destroy/list/status`               | Local Kubernetes (kind/k3d)           |
| `sindri image list/inspect/verify/versions`           | Container image management            |
| `sindri extension upgrade/versions/check/rollback`    | Enhanced extension management         |
| `sindri secrets s3 init/push/pull/sync/keygen/rotate` | S3 encrypted secrets                  |

---

## Step-by-Step Migration

### Phase 1: Preparation (Day 1)

1. **Read this entire guide**

2. **Complete pre-migration checklist**

3. **Create comprehensive backup:**

   ```bash
   ./v2/cli/sindri backup --profile full
   ```

4. **Export extension inventory:**

   ```bash
   ./v2/cli/extension-manager bom > extensions.yaml
   ```

5. **Set up V3 test environment**

### Phase 2: V3 Installation (Day 1-2)

1. **Download V3 binary for your platform:**

   ```bash
   # Linux x86_64
   wget https://github.com/pacphi/sindri/releases/latest/download/sindri-linux-x86_64.tar.gz
   tar -xzf sindri-linux-x86_64.tar.gz
   sudo mv sindri /usr/local/bin/

   # Linux aarch64
   wget https://github.com/pacphi/sindri/releases/latest/download/sindri-linux-aarch64.tar.gz

   # macOS Apple Silicon
   wget https://github.com/pacphi/sindri/releases/latest/download/sindri-macos-aarch64.tar.gz

   # macOS Intel
   wget https://github.com/pacphi/sindri/releases/latest/download/sindri-macos-x86_64.tar.gz

   # Windows x86_64
   # Download sindri-windows-x86_64.zip from GitHub releases
   ```

2. **Verify installation:**

   ```bash
   sindri version
   ```

3. **Run system doctor:**
   ```bash
   sindri doctor --all
   ```

### Phase 3: Configuration Migration (Day 2)

1. **Navigate to project:**

   ```bash
   cd <your-project>
   ```

2. **Backup existing config:**

   ```bash
   cp sindri.yaml sindri.yaml.v2.backup
   ```

3. **Run config validation (auto-migrates):**

   ```bash
   sindri config validate
   ```

4. **Review and fix validation errors**

5. **Update sindri.yaml if needed**

### Phase 4: Extension Migration (Day 2-3)

1. **Check extension compatibility:**

   ```bash
   sindri extension list --installed
   ```

2. **Identify incompatible extensions**

3. **Replace removed extensions:**

   ```bash
   # Remove old claude-flow v1
   # Install claude-flow-v2 (stable) or claude-flow-v3 (alpha)
   sindri extension install claude-flow-v3
   ```

4. **Validate all extensions:**

   ```bash
   sindri extension validate-all
   ```

5. **Upgrade compatible extensions:**
   ```bash
   sindri extension upgrade --all
   ```

### Phase 5: Testing (Day 3-5)

1. **Run comprehensive doctor check:**

   ```bash
   sindri doctor --all
   ```

2. **Test deployment dry-run:**

   ```bash
   sindri deploy --dry-run
   ```

3. **Deploy to staging:**

   ```bash
   sindri deploy
   ```

4. **Connect and verify:**

   ```bash
   sindri connect
   ```

5. **Test all critical workflows**

6. **Destroy staging when done:**
   ```bash
   sindri destroy
   ```

### Phase 6: Production Migration (Day 5+)

1. **Schedule maintenance window**

2. **Final backup of production:**

   ```bash
   ./v2/cli/sindri backup --profile full
   ```

3. **Destroy V2 deployment:**

   ```bash
   ./v2/cli/sindri destroy --provider <provider>
   ```

4. **Deploy with V3:**

   ```bash
   sindri deploy
   ```

5. **Validate extensions:**

   ```bash
   sindri extension list --installed
   ```

6. **Test critical workflows**

7. **Monitor for 24-48 hours**

---

## Rollback Procedures

### Quick Rollback (< 5 minutes)

If V3 deployment fails immediately:

```bash
# 1. Stop V3 deployment
sindri destroy --force

# 2. Restore V2 CLI access
cd <sindri-repo> && git checkout v2.2.1

# 3. Restore V2 configuration
cp sindri.yaml.v2.backup sindri.yaml

# 4. Deploy with V2
./v2/cli/sindri deploy --provider <provider>

# 5. Restore backup if needed
./v2/cli/sindri restore v2-backup-*.tar.gz
```

### Full Rollback

For complete V3 removal:

```bash
# 1. Remove V3 binary
sudo rm /usr/local/bin/sindri

# 2. Restore manifest backup
cp ~/.sindri/manifest.yaml.v2.backup ~/.sindri/manifest.yaml

# 3. Checkout V2 version
git checkout v2.2.1

# 4. Deploy with V2
./v2/cli/sindri deploy

# 5. Reinstall extensions
./v2/cli/extension-manager install-profile <profile>
```

### Hybrid Coexistence

V2 and V3 can run side-by-side during transition:

| Version | CLI Access                        | Docker Image               |
| ------- | --------------------------------- | -------------------------- |
| V2      | `./v2/cli/sindri` (relative path) | `ghcr.io/pacphi/sindri:v2` |
| V3      | `sindri` (installed binary)       | `ghcr.io/pacphi/sindri:v3` |

---

## Post-Migration Validation

Run these checks after migration:

- [ ] **CLI version:** `sindri version`
- [ ] **System health:** `sindri doctor --all`
- [ ] **Extensions installed:** `sindri extension list --installed`
- [ ] **Extensions valid:** `sindri extension validate-all`
- [ ] **Deployment cycle:** `sindri deploy` → `sindri connect` → `sindri destroy`
- [ ] **Secrets accessible:** `sindri secrets validate`
- [ ] **Backup/restore works:** `sindri backup` → `sindri restore <file>`
- [ ] **Updates available:** `sindri upgrade --check`

---

## Common Issues & Solutions

### Extension Validation Failures

**Symptom:** Extension validation fails after upgrade

**Cause:** Extension uses deprecated 'manual' install method

**Solution:** Update extension.yaml to use 'script' method

```yaml
# Before (V2)
install:
  method: manual
  commands:
    - echo "Installing..."

# After (V3)
install:
  method: script
  commands:
    - echo "Installing..."
```

### Missing yq or jq Errors

**Symptom:** V2 CLI fails after installing V3

**Cause:** V3 doesn't require yq/jq, but V2 still does

**Solution:** Keep yq and jq installed if using V2 alongside V3

### Manifest Migration Error

**Symptom:** CLI fails to read manifest

**Cause:** Corrupt manifest during migration

**Solution:**

```bash
cp ~/.sindri/manifest.yaml.v2.backup ~/.sindri/manifest.yaml
```

### VisionFlow Extensions Missing

**Symptom:** vf-\* extensions not available in V3

**Cause:** VisionFlow extensions excluded from V3

**Solution:** Continue using V2 for VisionFlow workflows. V2 and V3 can coexist.

### Provider Tools Not Found

**Symptom:** Deploy fails with 'tool not found'

**Cause:** Provider tools (flyctl, devpod) not installed

**Solution:**

```bash
sindri doctor --fix
```

### Slow First Run

**Symptom:** First command takes longer than expected

**Cause:** Initial schema compilation and caching

**Solution:** Normal behavior - subsequent runs will be faster

---

## CI/CD Considerations

### Workflow Triggers

V2 and V3 use separate CI workflows:

| Version | Triggers | Workflow                      |
| ------- | -------- | ----------------------------- |
| V2      | `v2/**`  | `.github/workflows/ci-v2.yml` |
| V3      | `v3/**`  | `.github/workflows/ci-v3.yml` |

### Tag Conventions

| Version | Tag Pattern | Examples       |
| ------- | ----------- | -------------- |
| V2      | `v2.x.x`    | v2.2.1, v2.3.0 |
| V3      | `v3.x.x`    | v3.0.0, v3.1.0 |

### Docker Image Tags

| Version | Tags                                                                     |
| ------- | ------------------------------------------------------------------------ |
| V2      | `ghcr.io/pacphi/sindri:v2`, `:v2.x`, `:v2.x.x`                           |
| V3      | `ghcr.io/pacphi/sindri:v3`, `:v3.x`, `:v3.x.x`, `:latest` (after stable) |

### Binary Artifacts

V3 releases include pre-built binaries:

- `sindri-linux-x86_64.tar.gz`
- `sindri-linux-aarch64.tar.gz`
- `sindri-macos-x86_64.tar.gz`
- `sindri-macos-aarch64.tar.gz`
- `sindri-windows-x86_64.zip`

### Pipeline Updates

Update CI/CD pipelines to use V3 binary:

```yaml
# Before (V2)
- name: Deploy with V2
  run: ./v2/cli/sindri deploy --provider docker

# After (V3)
- name: Install V3 CLI
  run: |
    wget https://github.com/pacphi/sindri/releases/latest/download/sindri-linux-x86_64.tar.gz
    tar -xzf sindri-linux-x86_64.tar.gz
    sudo mv sindri /usr/local/bin/

- name: Deploy with V3
  run: sindri deploy
```

---

## Data Migration Summary

### Automatic Migration

These are auto-migrated on first V3 run:

- sindri.yaml configuration (schema version updated)
- ~/.sindri/manifest.yaml (format migrated)
- Extension metadata (compatibility verified)

### Manual Migration Required

- Custom extension definitions (may need schema updates)
- CI/CD pipeline scripts (command changes)
- Automation scripts (command mapping)
- Documentation references

### No Migration Needed

- User data in workspace volumes
- Git repositories
- Project files
- Secrets stored in Vault
- Backup archives

---

## Timeline Recommendation

| Environment | Recommendation           | Timeline            |
| ----------- | ------------------------ | ------------------- |
| Development | Use V3 for new projects  | Immediate           |
| Staging     | Migrate and validate     | Immediate           |
| Production  | After staging validation | 2-4 weeks           |
| V2 Support  | Maintenance mode         | Security fixes only |

---

## Support Resources

### Documentation

| Document         | Path                             |
| ---------------- | -------------------------------- |
| V3 CLI Reference | `v3/docs/CLI.md`                 |
| V3 Quickstart    | `v3/docs/QUICKSTART.md`          |
| V3 Configuration | `v3/docs/CONFIGURATION.md`       |
| V3 Secrets       | `v3/docs/SECRETS_MANAGEMENT.md`  |
| V3 Doctor        | `v3/docs/DOCTOR.md`              |
| Comparison Guide | `docs/v2-v3-comparison-guide.md` |

### Architecture Decisions

| ADR     | Topic                                 |
| ------- | ------------------------------------- |
| ADR-001 | Rust Migration Workspace Architecture |
| ADR-021 | Bifurcated CI/CD Pipeline             |
| ADR-022 | Self-Update Implementation            |

### Getting Help

- **GitHub Issues:** https://github.com/pacphi/sindri/issues
- **GitHub Discussions:** https://github.com/pacphi/sindri/discussions
- **FAQ:** https://sindri-faq.fly.dev

---

## Appendix: Compatibility Matrix

The `v3/compatibility-matrix.yaml` file defines version mappings:

```yaml
cli_versions:
  "3.0.x":
    extension_schema: "1.0"
    breaking_changes:
      - "CLI rewritten in Rust"
      - "Extension schema updated to v1.0"
      - "Manifest file format changed"
    migration_notes:
      - "Auto-migration with backup"
      - "Run sindri doctor --fix after upgrade"
```

---

_Last updated: 2026-01-24_
_See also: [V2 vs V3 Comparison Guide](v2-v3-comparison-guide.md)_
