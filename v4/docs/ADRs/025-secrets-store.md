# ADR-025 — `sindri-secrets` crate: pluggable typed secret store

**Status:** Accepted (Implemented)

---

## Context

Sprint 12 (PR #223) introduced `sindri secrets *` commands and the first
surface area for secret references in `sindri.yaml`.  Two subsequent PRs
created ad-hoc secret handling:

* **PR #235 (Wave 6F)** — added a Vault HTTP client inline inside
  `sindri/src/commands/secrets.rs` as a `TestVault` subcommand.  That
  client is not reusable from any other crate.
* **PR #236 (Wave 6B)** — added OAuth token persistence for cloud targets
  by writing `targets.<name>.auth.token = "plain:<token>"` directly to
  `sindri.yaml`, noting that "migration to a typed secret store can land
  alongside the actual `sindri-secrets` crate."

Inline secrets and `plain:` manifest values have several problems:

1. Secrets written to YAML files end up in version control or world-readable
   config files.
2. There is no consistent abstraction — two different callers handle secrets
   differently.
3. CI/CD environments need a zero-config read path (env vars) without having
   to simulate the file-based flow.

---

## Decision

Introduce a new `sindri-secrets` workspace crate (library) that:

1. Defines a `SecretStore` async trait with four methods:
   `read`, `write`, `delete`, `list`.

2. Provides three built-in backends:
   - **`FileBackend`** — ChaCha20-Poly1305 encrypted file at
     `~/.sindri/secrets.enc`.  The key is derived from a passphrase via
     HKDF-SHA256.  The passphrase is sourced from
     `SINDRI_SECRETS_PASSPHRASE`; falls back to an empty string for CI.
   - **`VaultBackend`** — HashiCorp Vault KV v2 over HTTPS.  Moved from the
     inline implementation in `commands/secrets.rs`.  Configured via
     `VAULT_ADDR` / `VAULT_TOKEN`.
   - **`EnvBackend`** — read-only; resolves `SINDRI_SECRET_<UPPER_NAME>`.
     Intended for CI pipelines.

3. Defines `SecretValue`:
   - Holds `bytes: Vec<u8>` + `description: Option<String>`.
   - Zeroes bytes on `Drop`.
   - `Debug` impl renders `[REDACTED N bytes]` — never the actual secret.

4. Provides a `migrate` module with `maybe_migrate` and
   `resolve_or_delegate`:
   - On read: if a manifest value starts with `plain:`, the token is
     written to the `FileBackend` under the canonical key
     (`targets.<name>.auth.token`) and a `tracing::warn!` is emitted once.
   - The manifest is updated to store `secret:<key>` instead of the raw
     token.
   - Non-plain prefixes (`env:`, `file:`, `cli:`) are delegated back to
     `sindri_targets::auth::AuthValue` unchanged.

5. `sindri/commands/target.rs` `auth` subcommand: when the supplied value
   has a `plain:` prefix, the token is stored in the `FileBackend` and
   the manifest receives `secret:<key>` instead of the inline token.

6. `sindri/commands/secrets.rs` `test-vault` subcommand: delegates first
   to `VaultBackend::from_env()` (HTTP path), then falls back to the vault
   CLI and aws CLI as before.

---

## Consequences

### Positive

* Plain-text secrets no longer land in `sindri.yaml` or version control.
* The `VaultBackend` is reusable from any crate that depends on
  `sindri-secrets`.
* CI workflows can inject secrets as env vars (`SINDRI_SECRET_*`) without
  needing an encrypted file or a Vault instance.
* `SecretValue`'s drop-zeroing and `Debug` masking reduce the risk of
  accidental secret leaks via logs.
* Backwards compatible: legacy `plain:` values are silently migrated on
  first read.

### Negative / Trade-offs

* The `FileBackend` passphrase defaults to an empty string when
  `SINDRI_SECRETS_PASSPHRASE` is unset.  This provides encryption-at-rest
  but with a known key for that configuration.  Users are expected to set
  the passphrase for production use.
* The OS keyring integration (`keyring` crate) is deferred to a follow-up
  PR to avoid adding a cross-platform native dependency now.
* `FileBackend` uses a per-write random nonce; the nonce space is 96 bits
  (ChaCha20-Poly1305), which is safe for the number of writes a CLI tool
  makes.

---

## Alternatives considered

* **`age` encryption** — considered but `chacha20poly1305` + `hkdf` is
  sufficient for local-file encryption without the extra dependency weight
  of the `age` crate's recipient model.
* **OS keyring (`keyring` crate)** — deferred; platform-specific native
  library linkage adds friction to cross-compilation and CI runners.
* **Keeping secrets inline in `sindri.yaml`** — rejected; directly violates
  security principle of least privilege and breaks auditability.

---

## References

* PR #223 — Sprint 12 secrets surface area
* PR #235 — Wave 6F: Vault HTTP client (migrated here)
* PR #236 — Wave 6B: OAuth token storage (migrated here)
* ADR-020 — Unified auth prefixed-value model
