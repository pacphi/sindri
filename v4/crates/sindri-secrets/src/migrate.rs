//! Legacy `plain:` prefix migration helpers (ADR-025 §5).
//!
//! Consumers call [`maybe_migrate`] when reading an auth value from the
//! manifest.  If the value uses the legacy `plain:` prefix the token is
//! moved into the secret store, the manifest value is updated to `secret:<name>`,
//! and a one-time warning is logged.
//!
//! The helper is synchronous-friendly — it returns a `MigrateResult` enum
//! rather than mutating the manifest directly; callers own the write-back.

use crate::{SecretStore, SecretValue, SecretsError};
use tracing::warn;

/// Outcome of a [`maybe_migrate`] call.
#[derive(Debug)]
pub enum MigrateResult {
    /// The value was already a `secret:` reference — nothing to do.
    AlreadyMigrated,
    /// The value used a non-`plain:` prefix (e.g. `env:`, `file:`, `cli:`) —
    /// no migration needed; callers should resolve via [`sindri_targets::auth::AuthValue`].
    NotPlain,
    /// The value used a `plain:` prefix and has been migrated.  The returned
    /// `String` is the new manifest value (`secret:<name>`).
    Migrated { new_manifest_value: String },
}

/// Inspect `raw_value` from the manifest.  If it starts with `plain:`, move
/// the token into `store` under `secret_name`, return the new manifest pointer,
/// and emit a warning.
///
/// # Errors
///
/// Returns [`SecretsError`] only when the store write fails.  In all other
/// cases a `MigrateResult` variant is returned.
pub async fn maybe_migrate(
    store: &dyn SecretStore,
    secret_name: &str,
    raw_value: &str,
) -> Result<MigrateResult, SecretsError> {
    if raw_value.starts_with("secret:") {
        return Ok(MigrateResult::AlreadyMigrated);
    }
    if !raw_value.starts_with("plain:") {
        return Ok(MigrateResult::NotPlain);
    }
    let token = raw_value.strip_prefix("plain:").unwrap_or("");
    warn!(
        key = secret_name,
        "Migrating legacy plain: auth value to secret store. \
         Update sindri.yaml to use `secret:{}` to silence this warning.",
        secret_name,
    );
    let sv = SecretValue::from_plaintext(token)
        .with_description(format!("Migrated from plain: prefix — key {}", secret_name));
    store.write(secret_name, sv).await?;
    Ok(MigrateResult::Migrated {
        new_manifest_value: format!("secret:{}", secret_name),
    })
}

/// Resolve a manifest value that may be a `secret:` pointer, a `plain:` token
/// that needs migration, or a non-secret prefixed value (returned as-is for
/// [`sindri_targets::auth::AuthValue`] to handle).
///
/// Returns `Ok(Some(bytes))` for values the secret store owns, `Ok(None)` for
/// other prefixes, and `Err` when the store read fails.
pub async fn resolve_or_delegate(
    store: &dyn SecretStore,
    secret_name: &str,
    raw_value: &str,
) -> Result<Option<SecretValue>, SecretsError> {
    if let Some(key) = raw_value.strip_prefix("secret:") {
        let sv = store.read(key).await?;
        return Ok(Some(sv));
    }
    // Migrate inline and resolve in one step.
    if raw_value.starts_with("plain:") {
        if let MigrateResult::Migrated { .. } = maybe_migrate(store, secret_name, raw_value).await?
        {
            let sv = store.read(secret_name).await?;
            return Ok(Some(sv));
        }
    }
    Ok(None)
}

// ── tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::FileBackend;

    #[tokio::test]
    async fn migrate_plain_stores_and_returns_pointer() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileBackend::with_path_and_passphrase(
            dir.path().join("secrets.enc"),
            "test-passphrase",
        );
        let result = maybe_migrate(&store, "targets.myapp.auth.token", "plain:mytoken123")
            .await
            .unwrap();
        match result {
            MigrateResult::Migrated { new_manifest_value } => {
                assert_eq!(new_manifest_value, "secret:targets.myapp.auth.token");
            }
            other => panic!("expected Migrated, got {:?}", other),
        }
        // The value must now be retrievable.
        let sv = store.read("targets.myapp.auth.token").await.unwrap();
        assert_eq!(sv.expose_str().unwrap(), "mytoken123");
    }

    #[tokio::test]
    async fn already_migrated_is_a_noop() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileBackend::with_path_and_passphrase(
            dir.path().join("secrets.enc"),
            "test-passphrase",
        );
        let result = maybe_migrate(
            &store,
            "targets.x.auth.token",
            "secret:targets.x.auth.token",
        )
        .await
        .unwrap();
        assert!(matches!(result, MigrateResult::AlreadyMigrated));
    }

    #[tokio::test]
    async fn not_plain_returns_not_plain() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileBackend::with_path_and_passphrase(
            dir.path().join("secrets.enc"),
            "test-passphrase",
        );
        let result = maybe_migrate(&store, "tok", "env:MY_TOKEN").await.unwrap();
        assert!(matches!(result, MigrateResult::NotPlain));
    }
}
