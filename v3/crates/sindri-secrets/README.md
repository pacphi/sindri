# sindri-secrets

Secrets management system for the Sindri CLI. Provides multi-source secret resolution with encryption, caching, and audit logging.

## Features

- Multi-source resolution: environment variables, files, HashiCorp Vault, S3
- ChaCha20-Poly1305 symmetric encryption for local secrets
- Age envelope encryption for S3-stored secrets
- Memory zeroing via zeroize for sensitive data
- Audit logging for secret access tracking
- Async resolution with caching and retry logic
- Secret validation mode for pre-flight checks
- `.env` file loading via dotenvy
- Path traversal protection and security hardening

## Modules

- `resolver` - `SecretResolver` and `SecretCache` for resolving secrets from multiple sources
- `s3` - S3 backend for encrypted secret storage with age encryption
- `security` - `SecureString` (zeroize-on-drop) and `AuditLog` for access tracking
- `sources` - Source implementations (`EnvSource`, `FileSource`, `VaultSource`)
- `types` - Type definitions (`ResolvedSecret`, `SecretValue`, `VaultSecret`, `TokenMetadata`)

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
sindri-secrets = { path = "../sindri-secrets" }
```

## Part of [Sindri](../../)
