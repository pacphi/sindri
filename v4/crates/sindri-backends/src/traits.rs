use crate::error::BackendError;
use sindri_core::component::Backend;
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::Platform;

/// The unified install backend trait (Sprint 4, ADR-002)
pub trait InstallBackend: Send + Sync {
    fn name(&self) -> Backend;

    /// Returns true if this backend can operate on the given platform
    fn supports(&self, platform: &Platform) -> bool;

    /// Install a resolved component
    fn install(&self, comp: &ResolvedComponent) -> Result<(), BackendError>;

    /// Remove a resolved component
    fn remove(&self, comp: &ResolvedComponent) -> Result<(), BackendError>;

    /// Upgrade a resolved component to the version in `comp`
    fn upgrade(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        // Default: remove then re-install
        self.remove(comp)?;
        self.install(comp)
    }

    /// Check whether the component is already installed at the expected version
    fn is_installed(&self, comp: &ResolvedComponent) -> bool;
}

/// Run a command, capture output, return (stdout, stderr) or an error
pub fn run_command(program: &str, args: &[&str]) -> Result<(String, String), BackendError> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .map_err(|e| BackendError::CommandFailed {
            cmd: format!("{} {}", program, args.join(" ")),
            detail: e.to_string(),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(BackendError::cmd_failed(program, &stderr));
    }

    Ok((stdout, stderr))
}

/// Check if a binary is available in PATH
pub fn binary_available(name: &str) -> bool {
    which(name).is_some()
}

fn which(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths).find_map(|dir| {
            let candidate = dir.join(name);
            if candidate.is_file() {
                Some(candidate)
            } else {
                None
            }
        })
    })
}
