/// Unified auth prefixed-value model (ADR-020)
///
/// Values in sindri.yaml look like:
///   `env:MY_TOKEN`             → read from env var
///   `file:~/.token`            → read from file
///   `cli:gh`                   → delegate to gh CLI
///   `secret:vault/path/to/key` → resolve via `sindri-secrets` (Phase 0:
///                                schema-only — actual resolution is wired
///                                up in a later phase per ADR-027 §6)
///   `plain:secret`             → inline string (warned on validate)
use crate::error::TargetError;
use sindri_core::auth::SecretRef;

#[derive(Debug, Clone)]
pub enum AuthValue {
    Env(String),
    File(String),
    Cli(String),
    Plain(String),
    /// Reference to a secret in a backend store (ADR-020 reserved this
    /// variant; ADR-027 §6 / Phase 0 of the auth-aware plan adds the
    /// schema. Resolution is intentionally not wired up yet — see
    /// [`AuthValue::resolve`].
    Secret(SecretRef),
}

impl AuthValue {
    pub fn parse(s: &str) -> Option<Self> {
        if let Some(var) = s.strip_prefix("env:") {
            return Some(AuthValue::Env(var.to_string()));
        }
        if let Some(path) = s.strip_prefix("file:") {
            return Some(AuthValue::File(path.to_string()));
        }
        if let Some(cmd) = s.strip_prefix("cli:") {
            return Some(AuthValue::Cli(cmd.to_string()));
        }
        if let Some(rest) = s.strip_prefix("secret:") {
            // `secret:<backend>/<path>` per ADR-020 / Phase 0 plan §"Files
            // touched". A malformed reference (missing backend or path)
            // is not silently demoted to `plain:` — it surfaces as `None`
            // so callers can report a precise validation error.
            return SecretRef::parse(rest).map(AuthValue::Secret);
        }
        if let Some(val) = s.strip_prefix("plain:") {
            return Some(AuthValue::Plain(val.to_string()));
        }
        // Bare string treated as plain (with warning)
        Some(AuthValue::Plain(s.to_string()))
    }

    /// Resolve to the actual secret string. Never persists to disk.
    pub fn resolve(&self) -> Result<String, TargetError> {
        match self {
            AuthValue::Env(var) => std::env::var(var).map_err(|_| TargetError::AuthFailed {
                target: "(env)".into(),
                detail: format!("env var {} is not set", var),
            }),
            AuthValue::File(path) => {
                // Tilde expansion: only at the beginning of the path, per
                // shell convention. A naïve `replace('~', …)` mangles
                // Windows 8.3 short filenames like `RUNNER~1` that contain
                // a tilde mid-path.
                let expanded = if let Some(rest) = path.strip_prefix("~/") {
                    format!("{}/{}", home_str(), rest)
                } else if path == "~" {
                    home_str()
                } else {
                    path.clone()
                };
                std::fs::read_to_string(&expanded)
                    .map(|s| s.trim().to_string())
                    .map_err(|e| TargetError::AuthFailed {
                        target: "(file)".into(),
                        detail: format!("{}: {}", path, e),
                    })
            }
            AuthValue::Cli(cmd) => {
                let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
                let out = std::process::Command::new(parts[0])
                    .args(
                        parts
                            .get(1)
                            .map(|s| s.split_whitespace().collect::<Vec<_>>())
                            .unwrap_or_default(),
                    )
                    .output()
                    .map_err(|e| TargetError::AuthFailed {
                        target: "(cli)".into(),
                        detail: e.to_string(),
                    })?;
                Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
            }
            AuthValue::Plain(val) => {
                tracing::warn!("Using plain auth value — consider using env: or file: instead");
                Ok(val.clone())
            }
            AuthValue::Secret(r) => {
                // Phase 0 (ADR-026/ADR-027 schema-only) ships the variant
                // and parser without wiring resolution. The sindri-secrets
                // crate (ADR-025) is the eventual resolver; until it lands
                // (Phase 2 of the auth-aware plan), this returns a typed
                // error rather than silently producing an empty string.
                Err(TargetError::AuthFailed {
                    target: "(secret)".into(),
                    detail: format!(
                        "secret backend resolution is not wired yet (ref: {}/{})",
                        r.backend, r.path
                    ),
                })
            }
        }
    }

    pub fn is_plain(&self) -> bool {
        matches!(self, AuthValue::Plain(_))
    }
}

fn home_str() -> String {
    dirs_next::home_dir()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_default()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_secret_ref() {
        let v = AuthValue::parse("secret:vault/secrets/anthropic/prod").unwrap();
        match v {
            AuthValue::Secret(r) => {
                assert_eq!(r.backend, "vault");
                assert_eq!(r.path, "secrets/anthropic/prod");
            }
            other => panic!("expected Secret, got {:?}", other),
        }
    }

    #[test]
    fn parse_secret_ref_rejects_malformed() {
        assert!(AuthValue::parse("secret:nopath").is_none());
        assert!(AuthValue::parse("secret:/missing-backend").is_none());
        assert!(AuthValue::parse("secret:missing-path/").is_none());
    }

    #[test]
    fn parse_existing_prefixes_still_work() {
        assert!(matches!(
            AuthValue::parse("env:GITHUB_TOKEN").unwrap(),
            AuthValue::Env(_)
        ));
        assert!(matches!(
            AuthValue::parse("file:~/.token").unwrap(),
            AuthValue::File(_)
        ));
        assert!(matches!(
            AuthValue::parse("cli:gh auth token").unwrap(),
            AuthValue::Cli(_)
        ));
        assert!(matches!(
            AuthValue::parse("plain:abc").unwrap(),
            AuthValue::Plain(_)
        ));
        // Bare strings still default to Plain.
        assert!(matches!(
            AuthValue::parse("bare-token").unwrap(),
            AuthValue::Plain(_)
        ));
    }

    #[test]
    fn resolve_secret_returns_typed_error() {
        let v = AuthValue::Secret(SecretRef::new("vault", "secrets/x"));
        let err = v.resolve().unwrap_err();
        let msg = format!("{}", err);
        // Must not leak the path verbatim into a "successful" result; we
        // care only that resolution errored.
        assert!(msg.contains("secret"));
    }
}
