/// Unified auth prefixed-value model (ADR-020)
///
/// Values in sindri.yaml look like:
///   `env:MY_TOKEN`       → read from env var
///   `file:~/.token`      → read from file
///   `cli:gh`             → delegate to gh CLI
///   `plain:secret`       → inline string (warned on validate)
use crate::error::TargetError;

#[derive(Debug, Clone)]
pub enum AuthValue {
    Env(String),
    File(String),
    Cli(String),
    Plain(String),
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
                let expanded = path.replace('~', &home_str());
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
