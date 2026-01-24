# ADR 020: S3-Compatible Encrypted Secret Storage

**Status**: Accepted
**Date**: 2026-01-21
**Related**: [ADR-015: Secrets Resolver](015-secrets-resolver-core-architecture.md), [ADR-019: Phase 5 Integration](019-phase-5-secrets-backup-integration.md)

## Context

Sindri supports three secret sources (env, file, Vault). S3-based secret storage offers distinct advantages for production scenarios:

1. **Team Collaboration**: Shared secrets across distributed teams without Vault infrastructure
2. **CI/CD Integration**: Store secrets in S3 instead of CI platform secret managers
3. **Multi-Environment**: Single S3 bucket with environment-specific prefixes
4. **Secret Rotation**: Centralized updates propagate to all deployments
5. **Cloud-Native**: S3-compatible storage on all major clouds (AWS S3, MinIO, Wasabi, DigitalOcean Spaces)

### Use Cases

**Team Secret Sharing**: Team members pull encrypted secrets from shared S3 bucket instead of Slack/email

**CI/CD Secret Management**: GitHub Actions pulls from S3 instead of GitHub Secrets

**Multi-Environment Secrets**: Bucket structure with `secrets/dev/`, `secrets/staging/`, `secrets/prod/` prefixes

**Secret Rotation**: Update in S3, all deployments get new password on next pull

## Decision

### 1. Encryption Architecture: Envelope Encryption

**ChaCha20-Poly1305 + age encryption**:

```
1. Generate random 256-bit Data Encryption Key (DEK) per secret
2. Encrypt secret value with DEK using ChaCha20-Poly1305
3. Encrypt DEK with Master Key using age encryption
4. Store encrypted DEK + encrypted value in S3
5. Add authenticated metadata (version, timestamp, key ID)
```

**Rationale for ChaCha20-Poly1305**:

- Pure Rust implementation (RustCrypto/AEADs) with NCC Group audit
- Faster than AES-GCM on systems without AES-NI
- Authenticated encryption (AEAD) prevents tampering
- 256-bit keys, 96-bit nonces, 128-bit authentication tags

**Rationale for age encryption**:

- Small explicit keys (age X25519 keypairs)
- No config options, UNIX-style simplicity
- Supports recipient lists (multiple team members)
- Designed for file encryption

### 2. Storage Format

**S3 Key Structure**:

```
secrets/
├── prod/
│   ├── anthropic/
│   │   └── api-key.json
│   └── database/
│       └── password.json
├── staging/
│   └── database/
│       └── password.json
└── dev/
    └── database/
        └── password.json
```

**Secret Metadata (JSON)**:

```json
{
  "version": "1.0",
  "secret_name": "ANTHROPIC_API_KEY",
  "created_at": "2026-01-21T10:30:00Z",
  "updated_at": "2026-01-21T10:30:00Z",
  "encryption": {
    "algorithm": "chacha20poly1305",
    "key_derivation": "age-x25519",
    "recipients": ["age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p"]
  },
  "encrypted_dek": "age-encryption\n-----BEGIN AGE ENCRYPTED FILE-----\n...\n-----END AGE ENCRYPTED FILE-----",
  "encrypted_value": "base64-encoded-ciphertext",
  "nonce": "base64-encoded-nonce",
  "tag": "base64-encoded-auth-tag",
  "metadata": {
    "rotation_count": 3,
    "last_rotated_by": "alice@example.com"
  }
}
```

### 3. Master Key Management

**Three-Tier Strategy**:

**Tier 1: Environment Variable** (Development)

```yaml
secrets:
  backend:
    type: s3
    encryption:
      key_source: env
      key_env: SINDRI_MASTER_KEY
```

**Tier 2: File-Based** (Production)

```yaml
secrets:
  backend:
    type: s3
    encryption:
      key_source: file
      key_file: ~/.sindri/master.key
      permissions: "0600"
```

**Tier 3: AWS KMS** (Enterprise - Future)

```yaml
secrets:
  backend:
    type: s3
    encryption:
      key_source: kms
      kms_key_id: arn:aws:kms:us-east-1:123456789012:key/abc-def
```

### 4. Secret Resolution Priority

Updated resolution order:

```
1. Shell environment variables
2. .env.local
3. .env
4. fromFile
5. S3 (if configured)
6. Vault (if configured)
```

**Configuration**:

```yaml
secrets:
  backend:
    type: s3
    bucket: my-sindri-secrets
    region: us-east-1
    endpoint: https://s3.amazonaws.com # Optional for S3-compatible
    prefix: secrets/prod/
    encryption:
      algorithm: chacha20poly1305
      key_source: file
      key_file: ~/.sindri/master.key
    cache:
      enabled: true
      ttl_seconds: 3600
      path: ~/.sindri/cache/secrets/

  secrets:
    - name: ANTHROPIC_API_KEY
      source: s3
      s3_path: anthropic/api-key
      required: true

    - name: DATABASE_PASSWORD
      source: s3
      s3_path: database/password
      fallback: env
      required: true
```

### 5. CLI Commands

**Initialize S3 Backend**:

```bash
sindri secrets s3 init --bucket my-secrets --region us-east-1

# Output:
# ✓ Generated master key: ~/.sindri/master.key
# ✓ Public key: age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p
# ✓ Created S3 bucket: my-secrets
# ✓ Enabled versioning and encryption (SSE-S3)
```

**Push Secret**:

```bash
sindri secrets s3 push ANTHROPIC_API_KEY
sindri secrets s3 push DATABASE_PASSWORD --value "super_secret_123"
sindri secrets s3 push TLS_CERT --from-file ./certs/tls.crt

# Output:
# ✓ Encrypted ANTHROPIC_API_KEY with ChaCha20-Poly1305
# ✓ Uploaded to s3://my-secrets/secrets/prod/anthropic/api-key.json
```

**Pull Secret**:

```bash
sindri secrets s3 pull ANTHROPIC_API_KEY
eval $(sindri secrets s3 pull ANTHROPIC_API_KEY --export)
sindri secrets s3 pull TLS_CERT --output ./certs/tls.crt

# Output:
# ✓ Downloaded from s3://my-secrets/secrets/prod/database/password.json
# ✓ Decrypted using master key
# DATABASE_PASSWORD=super_secret_123
```

**Sync Secrets**:

```bash
sindri secrets s3 sync

# Output:
# ✓ ANTHROPIC_API_KEY: up-to-date (cached 15m ago)
# ↓ DATABASE_PASSWORD: fetching from S3
# ✓ DATABASE_PASSWORD: decrypted and cached
```

**Key Management**:

```bash
sindri secrets s3 keygen --output ~/.sindri/new-master.key
sindri secrets s3 rotate --new-key ~/.sindri/new-master.key
sindri secrets s3 add-recipient ANTHROPIC_API_KEY --recipient age1abc...
sindri secrets s3 recipients ANTHROPIC_API_KEY
```

**Version Control**:

```bash
sindri secrets s3 list
sindri secrets s3 history ANTHROPIC_API_KEY
sindri secrets s3 rollback ANTHROPIC_API_KEY --version v4
```

### 6. Rust Implementation

**Type Definitions**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3SecretBackend {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub prefix: String,
    pub encryption: S3EncryptionConfig,
    pub cache: Option<S3CacheConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3EncryptionConfig {
    pub algorithm: EncryptionAlgorithm,
    pub key_source: KeySource,
    pub key_env: Option<String>,
    pub key_file: Option<String>,
}

#[derive(Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum KeySource {
    Env,
    File,
    Kms,  // Future
}
```

**S3 Backend**:

```rust
pub struct S3Backend {
    client: Client,
    bucket: String,
    prefix: String,
}

impl S3Backend {
    pub async fn get_secret(&self, s3_path: &str) -> Result<Vec<u8>> {
        let key = format!("{}{}", self.prefix, s3_path);
        let resp = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await?;
        let body = resp.body.collect().await?;
        Ok(body.into_bytes().to_vec())
    }

    pub async fn put_secret(&self, s3_path: &str, data: Vec<u8>) -> Result<String> {
        let key = format!("{}{}", self.prefix, s3_path);
        let resp = self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(data.into())
            .server_side_encryption(ServerSideEncryption::Aes256)
            .send()
            .await?;
        Ok(resp.version_id.unwrap_or_default())
    }
}
```

**Encryption**:

```rust
pub struct SecretEncryptor {
    master_key: x25519::Identity,
}

impl SecretEncryptor {
    pub fn encrypt_secret(
        &self,
        secret_value: &str,
        recipients: &[x25519::Recipient],
    ) -> Result<S3SecretMetadata> {
        // Generate random DEK
        let dek = ChaCha20Poly1305::generate_key(&mut OsRng);
        let cipher = ChaCha20Poly1305::new(&dek);

        // Generate random nonce
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

        // Encrypt secret value with DEK
        let ciphertext = cipher.encrypt(&nonce, secret_value.as_bytes())?;

        // Encrypt DEK with age
        let encrypted_dek = self.encrypt_dek_with_age(&dek, recipients)?;

        // Extract auth tag
        let (encrypted_value, tag) = ciphertext.split_at(ciphertext.len() - 16);

        Ok(S3SecretMetadata {
            encrypted_dek,
            encrypted_value: base64::encode(encrypted_value),
            nonce: base64::encode(nonce),
            tag: base64::encode(tag),
            // ... metadata
        })
    }

    pub fn decrypt_secret(&self, metadata: &S3SecretMetadata) -> Result<String> {
        // Decrypt DEK
        let dek_bytes = self.decrypt_dek_with_age(&metadata.encrypted_dek)?;
        let dek = Key::from_slice(&dek_bytes);
        let cipher = ChaCha20Poly1305::new(dek);

        // Decode components
        let encrypted_value = base64::decode(&metadata.encrypted_value)?;
        let nonce = Nonce::from_slice(&base64::decode(&metadata.nonce)?);
        let tag = base64::decode(&metadata.tag)?;

        // Reconstruct ciphertext
        let mut ciphertext = encrypted_value;
        ciphertext.extend_from_slice(&tag);

        // Decrypt
        let plaintext = cipher.decrypt(nonce, ciphertext.as_ref())?;
        Ok(String::from_utf8(plaintext)?)
    }
}
```

**S3 Resolver**:

```rust
pub struct S3SecretResolver {
    backend: S3Backend,
    encryptor: SecretEncryptor,
    cache: Option<SecretCache>,
}

impl S3SecretResolver {
    pub async fn resolve(&self, s3_path: &str) -> Result<String> {
        // Check cache
        if let Some(cache) = &self.cache {
            if let Some(cached) = cache.get(s3_path)? {
                return Ok(cached);
            }
        }

        // Fetch from S3
        let data = self.backend.get_secret(s3_path).await?;
        let metadata: S3SecretMetadata = serde_json::from_slice(&data)?;

        // Decrypt
        let secret_value = self.encryptor.decrypt_secret(&metadata)?;

        // Cache
        if let Some(cache) = &self.cache {
            cache.set(s3_path, &secret_value)?;
        }

        Ok(secret_value)
    }

    pub async fn push(
        &self,
        name: &str,
        value: &str,
        s3_path: &str,
        recipients: &[x25519::Recipient]
    ) -> Result<String> {
        // Encrypt
        let metadata = self.encryptor.encrypt_secret(value, recipients)?;

        // Upload to S3
        let json_data = serde_json::to_vec_pretty(&metadata)?;
        let version_id = self.backend.put_secret(s3_path, json_data).await?;

        // Invalidate cache
        if let Some(cache) = &self.cache {
            cache.invalidate(s3_path)?;
        }

        Ok(version_id)
    }
}
```

### 7. Security Model

**Defense in Depth**:

```
Layer 1: S3 Server-Side Encryption (SSE-S3)
Layer 2: Client-Side Encryption (ChaCha20-Poly1305)
Layer 3: Master Key Encryption (age)
Layer 4: IAM/Bucket Policies
Layer 5: TLS in Transit
```

**Security Properties**:

1. **Zero-Knowledge**: S3 provider never sees plaintext
2. **Forward Secrecy**: Rotating master key doesn't compromise old secrets
3. **Authenticated Encryption**: AEAD prevents tampering
4. **Audit Trail**: S3 versioning tracks all changes
5. **Multi-Party**: age recipients enable team access

### 8. Key Rotation

**Scenario 1: Master Key Rotation**:

```bash
sindri secrets s3 keygen --output ~/.sindri/new-master.key
sindri secrets s3 rotate --new-key ~/.sindri/new-master.key --add-only
sindri secrets s3 validate --key ~/.sindri/new-master.key
sindri secrets s3 rotate --new-key ~/.sindri/new-master.key --remove-old
```

**Scenario 2: Secret Value Rotation**:

```bash
NEW_PASSWORD=$(openssl rand -base64 32)
psql -c "ALTER USER app_user WITH PASSWORD '$NEW_PASSWORD';"
sindri secrets s3 push DATABASE_PASSWORD --value "$NEW_PASSWORD"
```

### 9. Migration Guide

**Phase 1: Setup**:

```bash
sindri secrets s3 init --bucket my-team-secrets --region us-east-1
```

**Phase 2: Migrate Secrets**:

```bash
sindri secrets s3 push-from-env .env.local
# Or individually
sindri secrets s3 push ANTHROPIC_API_KEY
```

**Phase 3: Update sindri.yaml**:

```yaml
secrets:
  backend:
    type: s3
    bucket: my-team-secrets
    # ...

  secrets:
    - name: ANTHROPIC_API_KEY
      source: s3
      s3_path: anthropic/api-key
      fallback: env
      required: true
```

**Phase 4: Team Onboarding**:

```bash
# Receive master key securely
# Save to ~/.sindri/master.key (0600)
sindri secrets s3 sync
sindri deploy
```

### 10. Performance

**Benchmarks** (target):

```
Encrypt 1KB secret:     <1ms
Decrypt 1KB secret:     <1ms
S3 upload (1KB):        50-100ms
S3 download (1KB):      50-100ms
Cache lookup:           <0.1ms
Sync 10 secrets:        <2s
```

## Consequences

### Positive

1. **Team Collaboration**: Secure sharing without Vault
2. **Cloud Agnostic**: AWS S3, MinIO, Wasabi, DigitalOcean
3. **Zero-Knowledge**: Provider never sees plaintext
4. **Audit Trail**: S3 versioning tracks changes
5. **Disaster Recovery**: S3 as authoritative backup
6. **Offline Capable**: Cache enables offline work
7. **Key Rotation**: Easy with age recipients
8. **CI/CD Friendly**: Single bucket replaces per-project secrets
9. **Cost Effective**: Pennies per month

### Negative

1. **Complexity**: ~2000 lines of encryption/S3 code
2. **Key Management**: Teams must distribute master keys
3. **S3 Dependency**: Requires S3-compatible storage
4. **Network Latency**: 50-100ms vs local .env
5. **Operational Burden**: Monitor S3 availability
6. **Bootstrap Problem**: Initial key distribution manual

## Implementation

**Week 16**: Core Encryption

- ChaCha20-Poly1305 + age integration
- Unit tests for encrypt/decrypt
- Benchmarks

**Week 17**: S3 Backend and CLI

- S3Backend (get/put/list)
- S3SecretResolver
- CLI commands (init, push, pull, sync)
- Key management (keygen, rotate)
- Integration tests (MinIO)

## Dependencies

```toml
# S3
aws-sdk-s3 = "1.60"
aws-config = "1.5"

# Encryption
chacha20poly1305 = "0.10"
age = "0.10"
rand = "0.8"

# Existing
serde = "1.0"
serde_json = "1.0"
base64 = "0.22"
```

## Related Decisions

- [ADR-015: Secrets Resolver](015-secrets-resolver-core-architecture.md)
- [ADR-016: Vault Integration](016-vault-integration-architecture.md)
- [ADR-019: Phase 5 Integration](019-phase-5-secrets-backup-integration.md)
