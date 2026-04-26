use sindri_core::component::Backend;
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::{Os, Platform};
use crate::error::BackendError;
use crate::traits::{InstallBackend, binary_available, run_command};

/// winget backend — Windows only (ADR-009)
pub struct WingetBackend;

impl InstallBackend for WingetBackend {
    fn name(&self) -> Backend {
        Backend::Winget
    }

    fn supports(&self, platform: &Platform) -> bool {
        matches!(platform.os, Os::Windows) && binary_available("winget")
    }

    fn install(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        tracing::info!("winget: installing {}", comp.id.name);
        run_command("winget", &["install", "--exact", "--id", &comp.id.name, "-e"])?;
        Ok(())
    }

    fn remove(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        run_command("winget", &["uninstall", "--exact", "--id", &comp.id.name])?;
        Ok(())
    }

    fn is_installed(&self, comp: &ResolvedComponent) -> bool {
        run_command("winget", &["list", "--exact", "--id", &comp.id.name])
            .map(|(out, _)| out.contains(&comp.id.name))
            .unwrap_or(false)
    }
}

/// Scoop backend — Windows only (ADR-009)
pub struct ScoopBackend;

impl InstallBackend for ScoopBackend {
    fn name(&self) -> Backend {
        Backend::Scoop
    }

    fn supports(&self, platform: &Platform) -> bool {
        matches!(platform.os, Os::Windows) && binary_available("scoop")
    }

    fn install(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        tracing::info!("scoop: installing {}", comp.id.name);
        run_command("scoop", &["install", &comp.id.name])?;
        Ok(())
    }

    fn remove(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        run_command("scoop", &["uninstall", &comp.id.name])?;
        Ok(())
    }

    fn is_installed(&self, comp: &ResolvedComponent) -> bool {
        run_command("scoop", &["list"])
            .map(|(out, _)| out.contains(&comp.id.name))
            .unwrap_or(false)
    }
}
