//! [`EnvBackend`] — read-only secret store backed by environment variables.
//!
//! Secret names are upper-cased and have any non-alphanumeric characters
//! replaced with `_`, then looked up as `SINDRI_SECRET_<NAME>`.
//!
//! Example: `"targets.fly1.auth.token"` → `SINDRI_SECRET_TARGETS_FLY1_AUTH_TOKEN`.
//!
//! This backend is **read-only** and suitable for CI/CD pipelines where
//! secrets are injected as environment variables.

use crate::{SecretStore, SecretValue, SecretsError};
use async_trait::async_trait;

/// Read-only secret store that resolves `SINDRI_SECRET_<NORMALISED_NAME>`.
///
/// `write`, `delete`, and `list` return [`SecretsError::Unsupported`].
#[derive(Debug, Default, Clone)]
pub struct EnvBackend;

impl EnvBackend {
    pub fn new() -> Self {
        Self
    }

    /// Convert a secret name to the corresponding env-var name.
    ///
    /// The name is upper-cased; every character that is not `A-Z`, `0-9`
    /// is replaced with `_`.
    pub fn env_var_name(name: &str) -> String {
        let normalised: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .collect();
        format!("SINDRI_SECRET_{}", normalised)
    }
}

#[async_trait]
impl SecretStore for EnvBackend {
    async fn read(&self, name: &str) -> Result<SecretValue, SecretsError> {
        let var = Self::env_var_name(name);
        std::env::var(&var)
            .map(|v| SecretValue::from_plaintext(&v))
            .map_err(|_| SecretsError::EnvVarMissing {
                name: name.to_string(),
            })
    }

    async fn write(&self, name: &str, _value: SecretValue) -> Result<(), SecretsError> {
        Err(SecretsError::Unsupported(format!(
            "EnvBackend is read-only; cannot write secret '{}'",
            name
        )))
    }

    async fn delete(&self, name: &str) -> Result<(), SecretsError> {
        Err(SecretsError::Unsupported(format!(
            "EnvBackend is read-only; cannot delete secret '{}'",
            name
        )))
    }

    async fn list(&self) -> Result<Vec<String>, SecretsError> {
        Err(SecretsError::Unsupported(
            "EnvBackend cannot enumerate secrets".into(),
        ))
    }
}

// ── tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_var_name_normalises_dots_and_hyphens() {
        assert_eq!(
            EnvBackend::env_var_name("targets.fly1.auth.token"),
            "SINDRI_SECRET_TARGETS_FLY1_AUTH_TOKEN"
        );
        assert_eq!(EnvBackend::env_var_name("my-key"), "SINDRI_SECRET_MY_KEY");
    }

    #[tokio::test]
    async fn read_present_env_var() {
        let backend = EnvBackend::new();
        let var = "SINDRI_SECRET_ENV_BACKEND_READ_TEST";
        std::env::set_var(var, "hunter2");
        // The secret name we look up must map to this var.
        let result = backend.read("env_backend_read_test").await;
        std::env::remove_var(var);
        let sv = result.expect("should have resolved");
        assert_eq!(sv.expose_str().unwrap(), "hunter2");
    }

    #[tokio::test]
    async fn read_missing_env_var_returns_error() {
        let backend = EnvBackend::new();
        let var = "SINDRI_SECRET_ENV_BACKEND_MISSING_XYZ";
        std::env::remove_var(var);
        let result = backend.read("env_backend_missing_xyz").await;
        assert!(
            matches!(result, Err(SecretsError::EnvVarMissing { .. })),
            "unexpected result: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn write_returns_unsupported() {
        let backend = EnvBackend::new();
        let result = backend.write("any", SecretValue::from_plaintext("x")).await;
        assert!(matches!(result, Err(SecretsError::Unsupported(_))));
    }

    #[tokio::test]
    async fn delete_returns_unsupported() {
        let backend = EnvBackend::new();
        let result = backend.delete("any").await;
        assert!(matches!(result, Err(SecretsError::Unsupported(_))));
    }

    #[tokio::test]
    async fn list_returns_unsupported() {
        let backend = EnvBackend::new();
        let result = backend.list().await;
        assert!(matches!(result, Err(SecretsError::Unsupported(_))));
    }
}
