use thiserror::Error;

#[derive(Debug, Error)]
pub enum TargetError {
    #[error("Target '{name}' is not available: {reason}")]
    Unavailable { name: String, reason: String },
    #[error("Command execution failed on target '{target}': {detail}")]
    ExecFailed { target: String, detail: String },
    #[error("Authentication failed for target '{target}': {detail}")]
    AuthFailed { target: String, detail: String },
    #[error("Target '{name}' does not exist — run `sindri target create {name}` first")]
    NotProvisioned { name: String },
    #[error("Prerequisites missing for target '{target}': {detail}")]
    Prerequisites { target: String, detail: String },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
