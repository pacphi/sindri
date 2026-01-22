# ADR 019: Phase 5 - Secrets, Backup, and Restore Integration Strategy

**Status**: Accepted
**Date**: 2026-01-21
**Related**: [Rust Migration Plan](../../planning/active/rust-cli-migration-v3.md), [ADR-015](015-secrets-resolver-core-architecture.md), [ADR-016](016-vault-integration-architecture.md), [ADR-017](017-backup-system-architecture.md), [ADR-018](018-restore-system-architecture.md)

## Context

Phase 5 implements the final operational subsystems: secrets management and backup/restore functionality. These systems interact with all other components and must integrate seamlessly.

**Current State**:

- Bash implementation: `cli/secrets-manager` (824 lines), `cli/backup-restore` (1,101 lines)
- Rust skeleton: `sindri-secrets` crate exists (294 lines)
- Phase 4 complete: Extension system operational (6,744 lines)

**Integration Challenges**:

1. Secrets must integrate with all 5 providers
2. Backup/restore must handle secrets securely
3. System markers must NEVER be restored
4. Cross-system error handling patterns
5. Security: memory zeroing, audit logging

## Decision

### Overall Architecture: Phased Implementation

**Week 15**: Core secrets resolver (env + file sources)
**Week 16**: Vault + S3 encryption + backup system
**Week 17**: S3 CLI + restore + integration

### Crate Structure

#### 1. `sindri-secrets` Crate

**Location**: `crates/sindri-secrets/`

```
sindri-secrets/
├── src/
│   ├── lib.rs
│   ├── resolver.rs
│   ├── sources/
│   │   ├── mod.rs
│   │   ├── env.rs
│   │   ├── file.rs
│   │   ├── vault.rs
│   │   └── s3.rs          # NEW (Week 16-17)
│   ├── injection/
│   │   ├── mod.rs
│   │   ├── docker.rs
│   │   ├── fly.rs
│   │   ├── devpod.rs
│   │   ├── e2b.rs
│   │   └── kubernetes.rs
│   ├── validation.rs
│   ├── security.rs
│   └── types.rs
```

**Dependencies**:

```toml
sindri-core = { workspace = true }
anyhow = { workspace = true }
tokio = { workspace = true }
reqwest = { workspace = true }
serde = { workspace = true }
shellexpand = "3.1"
zeroize = "1.7"
base64 = "0.21"
# Vault
vaultrs = "0.7"
# S3
aws-sdk-s3 = "1.60"
chacha20poly1305 = "0.10"
age = "0.10"
```

#### 2. `sindri-backup` Crate (New)

**Location**: `crates/sindri-backup/`

```
sindri-backup/
├── src/
│   ├── lib.rs
│   ├── profile.rs
│   ├── archive.rs
│   ├── backup/
│   │   ├── mod.rs
│   │   ├── local.rs
│   │   ├── docker.rs
│   │   └── fly.rs
│   ├── restore/
│   │   ├── mod.rs
│   │   ├── modes.rs
│   │   ├── analysis.rs
│   │   └── markers.rs
│   ├── encryption.rs
│   ├── compression.rs
│   └── progress.rs
```

**Dependencies**:

```toml
sindri-core = { workspace = true }
tar = { workspace = true }
flate2 = { workspace = true }
aws-sdk-s3 = "1.10"
sha2 = "0.10"
walkdir = "2.4"
age = "0.10"
```

#### 3. CLI Commands

**Location**: `crates/sindri/src/commands/`

- `secrets.rs` (~600 lines): validate, list, test-vault, encode-file
- `secrets/s3.rs` (~400 lines): init, push, pull, sync, keygen, rotate
- `backup.rs` (~500 lines): backup command
- `restore.rs` (~500 lines): restore command

### Implementation Phases

#### Week 15: Secrets Resolver Core

**Deliverables**:

- `SecretResolver` with env/file sources (sync operations)
- `SecureString` wrapper with zeroization
- CLI: `secrets validate`, `secrets list`

**Tasks**:

1. Implement `sources/env.rs` (env vars, .env parsing, fromFile)
2. Implement `sources/file.rs` (path validation, base64)
3. Implement `security.rs` (SecureString, sanitization)
4. Implement `resolver.rs` (orchestration, error handling)
5. CLI commands

**Estimated LOC**: ~800-1,000 lines

#### Week 16: Vault + S3 Encryption + Backup

**Deliverables**:

- Vault integration (async)
- S3 encryption core (ChaCha20-Poly1305)
- Backup system with profiles
- S3 upload support

**Tasks**:

1. Implement `sources/vault.rs` (HTTP client, token renewal)
2. Make resolver fully async
3. Implement `sources/s3/encryption.rs` (envelope encryption)
4. Implement `sources/s3/backend.rs` (S3 client)
5. Implement `sindri-backup` crate (profiles, archive)
6. CLI: `backup`, `secrets test-vault`

**Estimated LOC**: ~1,200-1,500 lines

#### Week 17: S3 CLI + Restore + Integration

**Deliverables**:

- S3 CLI commands
- Restore system
- Provider integration
- Integration tests

**Tasks**:

1. Implement `sources/s3/resolver.rs` (resolution + caching)
2. Implement S3 CLI (init, push, pull, sync)
3. Implement restore system (modes, markers, atomic)
4. Implement secret injection (all providers)
5. Integration tests

**Estimated LOC**: ~1,000-1,200 lines

### Key Design Decisions

#### 1. Should Secrets Resolution Be Async?

**Decision**: **Yes, async is required**

**Rationale**:

- Vault requires network I/O
- S3 requires network I/O
- Parallel resolution improves performance
- Future sources may require async

#### 2. How Do Backup/Restore Handle Secrets?

**Decision**: **Three-tier approach based on profile**

- **User-Data**: Never includes shell history, .env files
- **Standard**: Excludes .env.local, includes .env
- **Full**: Includes all files with warning

**Encryption Option**: `sindri backup --encrypt --output backup.tar.gz.age`

#### 3. Testing Strategy for Vault and S3?

**Decision**: **Mock for unit tests, optional real instance for integration**

- Unit tests: Use `mockall` to mock clients
- Integration tests: Opt-in with env vars (VAULT_ADDR, AWS credentials)
- CI: Unit tests always, integration tests weekly

#### 4. Idempotency for Backup/Restore?

**Decision**: **Checksum verification + deterministic tar creation**

- Backups: Deterministic tar order, SHA256 checksums
- Restores: Safe mode naturally idempotent, full mode not idempotent by design

### Security Considerations

**1. Secret Handling in Memory**:

```rust
use zeroize::{Zeroize, Zeroizing};

pub struct SecureString(Zeroizing<String>);
// Automatically zeros memory on drop
```

**2. Audit Logging**:

```rust
pub struct AuditLogger;

impl AuditLogger {
    pub async fn log_secret_resolution(&self, event: SecretEvent) {
        // Never log actual values
        let entry = json!({
            "timestamp": Utc::now(),
            "event": "secret_resolved",
            "name": event.name,
            "source": event.source,
            "success": event.success,
        });
    }
}
```

**3. Sanitization**:

```rust
pub fn sanitize_error(err: &Error) -> String {
    let msg = err.to_string();
    // Redact tokens, keys, passwords
    // Redact base64-looking strings
    msg
}
```

### Migration from Bash

**Parity Checklist**:

- ✅ Environment variable resolution
- ✅ File secret resolution
- ✅ Vault KV v2 integration
- ✅ Three backup profiles
- ✅ Three restore modes
- ✅ S3 upload/download
- ✅ System marker protection
- ✅ All CLI commands

**New Features**:

- Async secret resolution (parallel)
- Type-safe secret handling
- S3 encrypted secret storage
- Memory-safe secret handling (zeroization)
- Audit logging

### Total Estimated LOC

**Phase 5 Total**: ~5,000-5,700 lines

- `sindri-secrets`: ~2,000-2,400 lines (with S3)
- `sindri-backup`: ~1,200-1,500 lines
- CLI commands: ~1,800-2,000 lines

**Comparison to Bash**: 1,925 lines → ~5,000 lines (2.6x increase)
**Reason**: Type definitions, error handling, S3 encryption, tests included

## Consequences

### Positive

1. **Unified Secrets Management**: Single resolver across all providers
2. **Security First**: Zeroization, audit logging, sanitized errors
3. **Type Safety**: Compile-time guarantees
4. **Performance**: Async parallel resolution
5. **S3 Encrypted Storage**: Team collaboration, cloud-native
6. **Comprehensive Testing**: 80%+ coverage target
7. **100% Parity**: All bash features preserved
8. **Backward Compatible**: Same config format

### Negative

1. **Complexity**: Async secrets add overhead
2. **Testing Burden**: Vault/S3 mocking requires setup
3. **Binary Size**: +2-3MB for dependencies
4. **S3 Dependency**: Requires S3-compatible storage

## Testing Strategy

**Unit Tests**: 80%+ coverage

- Env resolution with mocked env vars
- File resolution with temporary files
- Vault mocking with `mockall`
- S3 encryption roundtrip tests

**Integration Tests**: Opt-in

- Local MinIO for S3 tests
- Vault dev server
- Real AWS S3 (weekly)

**E2E Tests**:

- Full backup/restore cycle
- Cross-version compatibility
- Provider integration

## Related Decisions

- [ADR-001: Workspace Architecture](001-rust-migration-workspace-architecture.md)
- [ADR-004: Async Runtime](004-async-runtime-command-execution.md)
- [ADR-015: Secrets Resolver](015-secrets-resolver-core-architecture.md)
- [ADR-016: Vault Integration](016-vault-integration-architecture.md)
- [ADR-017: Backup System](017-backup-system-architecture.md)
- [ADR-018: Restore System](018-restore-system-architecture.md)
- [ADR-020: S3 Encrypted Storage](020-s3-encrypted-secret-storage.md)
