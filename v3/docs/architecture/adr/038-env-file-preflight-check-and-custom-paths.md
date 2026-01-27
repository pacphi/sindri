# ADR 038: .env File Preflight Check and Custom Paths

**Status**: Accepted
**Date**: 2026-01-27
**Deciders**: Core Team
**Related**: [ADR-015: Secrets Resolver](015-secrets-resolver-core-architecture.md), [ADR-016: Vault Integration](016-vault-integration-architecture.md), [ADR-020: S3 Secret Storage](020-s3-encrypted-secret-storage.md)

## Context

Users were experiencing confusing deployment failures when `.env` files were missing or located in unexpected directories. The errors manifested differently across providers:

### Problem 1: Docker Provider Error
```
Error: env file /private/tmp/.env.secrets not found: stat /private/tmp/.env.secrets: no such file or directory
```

**Root Cause**: Docker provider's template referenced `.env.secrets` in `docker-compose.yml` but never created the file before running `docker-compose up`.

### Problem 2: Unclear .env File Location

Users didn't know where to place `.env` files:
- Should it be in the current directory?
- Should it be with `sindri.yaml`?
- What if `sindri.yaml` is in a custom location (`--config` flag)?

### Problem 3: No Custom .env Path Support

Users with non-standard project layouts (e.g., monorepos) couldn't specify custom `.env` file paths:
```
project/
├── config/
│   ├── sindri.yaml
│   └── prod.env       # Can't reference this
├── secrets/
│   └── .env           # Or this
└── src/
```

### Problem 4: Silent Resolution Failures

When secrets couldn't be resolved, users saw cryptic errors from the deployment provider rather than clear feedback about missing `.env` files.

## Decision

We implement a **three-part solution**:

### 1. Provider Secrets Resolution

**All providers must resolve secrets before deployment:**

```rust
// Docker Provider (v3/crates/sindri-providers/src/docker.rs)
async fn resolve_secrets(
    &self,
    config: &SindriConfig,
    custom_env_file: Option<PathBuf>
) -> Result<Option<PathBuf>> {
    // 1. Create ResolutionContext from config directory
    let config_dir = config.config_path.parent()...;
    let context = ResolutionContext::new(config_dir)
        .with_custom_env_file(custom_env_file);

    // 2. Resolve all secrets
    let resolver = SecretResolver::new(context);
    let resolved = resolver.resolve_all(secrets).await?;

    // 3. Write to .env.secrets file
    // 4. Set restrictive permissions (0600)
    // 5. Return path for cleanup
}
```

**Implementation per provider:**
- **Docker**: Write `.env.secrets`, reference in `docker-compose.yml`
- **Fly**: Use `flyctl secrets import` via stdin
- **DevPod**: Populate `containerEnv` in `devcontainer.json`
- **E2B**: Inject `ENV` statements into Dockerfile
- **Kubernetes**: Create `Secret` resource with base64-encoded values

### 2. Preflight Check with User Feedback

**Before deployment, check for `.env` files and provide clear guidance:**

```rust
// v3/crates/sindri/src/commands/deploy.rs
fn check_env_files(
    config: &SindriConfig,
    custom_env_file: Option<&Utf8Path>
) -> Result<()> {
    let config_dir = config.config_path.parent()...;

    // Check for .env.local and .env
    if found_files.is_empty() {
        output::info("No .env files found (this is OK)");
        output::info("Secrets will be loaded from env vars, Vault, S3...");
        output::info("To use .env files, create .env or .env.local");
        output::info("Or use --env-file to specify a custom location");
    } else {
        output::info(&format!(
            "Found environment files: {}",
            found_files.join(", ")
        ));
        output::info("Priority: shell env > .env.local > .env");
    }
}
```

**Output Examples:**

✅ **Success case:**
```
Found environment files in /path/to/project: .env.local, .env
Secrets will be resolved with priority: shell env > .env.local > .env
```

ℹ️ **No files case:**
```
No .env files found in /path/to/project (this is OK)
Secrets will be loaded from environment variables, Vault, S3, or other sources
To use .env files, create .env or .env.local in the config directory
Or use --env-file to specify a custom location
```

⚠️ **Custom file missing:**
```
Custom .env file not found: /path/to/custom.env
Secrets will be loaded from environment variables or other sources
```

### 3. Custom .env File Path Support

**Add `--env-file` flag to deploy command:**

```rust
// v3/crates/sindri/src/cli.rs
#[derive(Args, Debug)]
pub struct DeployArgs {
    // ... existing fields ...

    /// Path to .env file (default: look for .env/.env.local in config directory)
    #[arg(long)]
    pub env_file: Option<Utf8PathBuf>,
}
```

**Pass custom path through to ResolutionContext:**

```rust
// v3/crates/sindri-secrets/src/types.rs
pub struct ResolutionContext {
    pub config_dir: PathBuf,
    pub custom_env_file: Option<PathBuf>,  // NEW
    // ... other fields ...
}

impl ResolutionContext {
    pub fn with_custom_env_file(mut self, path: Option<PathBuf>) -> Self {
        self.custom_env_file = path;
        self
    }
}
```

**Update EnvSource to respect custom path:**

```rust
// v3/crates/sindri-secrets/src/sources/env.rs
async fn load_env_files(&self, ctx: &ResolutionContext) -> Result<EnvFiles> {
    // If custom env file provided, use only that
    if let Some(custom_path) = &ctx.custom_env_file {
        return self.parse_env_file(custom_path);
    }

    // Otherwise use standard .env.local and .env
    // ...
}
```

## Consequences

### Positive

✅ **Fixes Docker deployment error**: `.env.secrets` is now created before `docker-compose up`

✅ **Clear user feedback**: Preflight check explains where Sindri looks for secrets

✅ **Flexible configuration**: `--env-file` supports non-standard project layouts

✅ **Consistent behavior**: All providers now resolve secrets the same way

✅ **Better DX**: Users immediately understand what's wrong if secrets are missing

✅ **Security maintained**: Custom env files still follow the same security model

### Negative

⚠️ **File secrets not fully supported**: Most providers only support environment variable secrets (file secret support is a future enhancement)

⚠️ **Additional I/O**: Preflight check adds file existence checks (negligible performance impact)

⚠️ **CLI surface area**: Adds another flag for users to learn about

### Neutral

🔄 **Breaking change**: None - existing configurations work unchanged

🔄 **Migration effort**: Zero - feature is additive

## Implementation Details

### Files Modified

1. **v3/crates/sindri/src/cli.rs**: Add `--env-file` flag to `DeployArgs`
2. **v3/crates/sindri/src/commands/deploy.rs**:
   - Add `check_env_files()` preflight check
   - Update deploy to use global `--config` flag
   - Pass `env_file` to providers
3. **v3/crates/sindri-secrets/src/types.rs**: Add `custom_env_file` to `ResolutionContext`
4. **v3/crates/sindri-secrets/src/sources/env.rs**: Respect custom env file path
5. **v3/crates/sindri-providers/src/docker.rs**: Add `resolve_secrets()` and call before compose
6. **v3/crates/sindri-providers/src/fly.rs**: Add `resolve_and_set_secrets()`
7. **v3/crates/sindri-providers/src/devpod.rs**: Add `resolve_secrets()` for containerEnv
8. **v3/crates/sindri-providers/src/e2b.rs**: Add `resolve_secrets()` for ENV statements
9. **v3/crates/sindri-providers/src/kubernetes.rs**: Add `ensure_app_secrets()` for Secret resources
10. **v3/crates/sindri-providers/Cargo.toml**: Add `sindri-secrets` dependency

### Testing Strategy

**Unit tests:**
- `test_check_env_files_with_both_files()`: Detects both .env and .env.local
- `test_check_env_files_with_no_files()`: Handles missing files gracefully
- `test_check_env_files_with_custom_path()`: Uses custom --env-file path
- `test_check_env_files_custom_path_not_found()`: Warns when custom file missing

**Integration tests:**
- Docker: Deploy with secrets, verify `.env.secrets` created
- Fly: Deploy with secrets, verify `flyctl secrets import` called
- DevPod: Deploy with secrets, verify `containerEnv` populated
- E2B: Deploy with secrets, verify `ENV` statements in Dockerfile
- Kubernetes: Deploy with secrets, verify `Secret` resource created

## Examples

### Basic Usage

```bash
# Default behavior - uses .env/.env.local in config directory
sindri deploy

# Custom env file (relative path)
sindri deploy --env-file config/prod.env

# Custom env file (absolute path)
sindri deploy --env-file /secrets/production.env

# Custom config + custom env file
sindri deploy --config /path/to/sindri.yaml --env-file /path/to/.env
```

### sindri.yaml Configuration

```yaml
# No changes required - existing configs work unchanged
secrets:
  - name: ANTHROPIC_API_KEY
    source: env
    required: true

  - name: DATABASE_PASSWORD
    source: vault
    vaultPath: secret/data/myapp
    vaultKey: password
    required: true
```

## Related Work

- **ADR-015**: Established secrets resolver architecture
- **ADR-016**: Vault integration
- **ADR-020**: S3 encrypted storage

## Packer Provider Exception

The Packer provider is **intentionally excluded** from automatic secrets resolution:

**Rationale:**
- Packer builds distributable VM images (not runtime environments)
- Secrets used during build MUST be cleaned before snapshot
- Manual `environment` configuration prevents accidental secret leakage
- Explicit separation between build-time and runtime secrets

**Current approach:** Users manually configure build-time env vars in `packer.build.environment`

**Alternative considered:** Add `build_only: true` flag to secrets, but rejected due to:
- Increased complexity
- Risk of users misunderstanding and baking secrets into images
- Manual configuration makes intent clearer

See [ADR-031](031-packer-vm-provisioning-architecture.md#6-secrets-handling-for-vm-images) for complete Packer secrets architecture.

## Future Enhancements

1. **File Secret Support**: Extend providers to mount file secrets as volumes
2. **Validation Command Enhancement**: Add `sindri secrets validate --env-file` flag
3. **Config Generation**: `sindri config init` could create `.env.template` files
4. **Multiple Env Files**: Support `--env-file` multiple times to load multiple files
5. **Packer Build Secrets** (Deferred): Could add `build_only: true` flag if demand emerges

## References

- Issue: "Docker deployment fails with missing .env.secrets"
- Feature Request: "Support custom .env file paths"
- Documentation: v3/docs/SECRETS_MANAGEMENT.md updated with new features
