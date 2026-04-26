use thiserror::Error;

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("Backend {backend} not available on this platform")]
    Unavailable { backend: String },
    #[error("Installation failed for {component}: {detail}")]
    InstallFailed { component: String, detail: String },
    #[error("Removal failed for {component}: {detail}")]
    RemoveFailed { component: String, detail: String },
    #[error("Checksum mismatch for {component}: expected {expected}, got {got}")]
    ChecksumMismatch { component: String, expected: String, got: String },
    #[error("Command failed: {cmd} — {detail}")]
    CommandFailed { cmd: String, detail: String },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl BackendError {
    pub fn install(component: &str, detail: impl Into<String>) -> Self {
        BackendError::InstallFailed {
            component: component.to_string(),
            detail: detail.into(),
        }
    }

    pub fn cmd_failed(cmd: &str, stderr: &str) -> Self {
        BackendError::CommandFailed {
            cmd: cmd.to_string(),
            detail: stderr.to_string(),
        }
    }
}
