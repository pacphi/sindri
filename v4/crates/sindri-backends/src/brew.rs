use sindri_core::component::Backend;
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::{Os, Platform};
use crate::error::BackendError;
use crate::traits::{InstallBackend, binary_available, run_command};

/// Homebrew backend — macOS (and opt-in Linux) (ADR-009)
pub struct BrewBackend;

impl InstallBackend for BrewBackend {
    fn name(&self) -> Backend {
        Backend::Brew
    }

    fn supports(&self, platform: &Platform) -> bool {
        matches!(platform.os, Os::Macos) && binary_available("brew")
    }

    fn install(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        tracing::info!("brew: installing {}", comp.id.name);
        run_command("brew", &["install", &comp.id.name])?;
        Ok(())
    }

    fn remove(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        tracing::info!("brew: removing {}", comp.id.name);
        run_command("brew", &["uninstall", &comp.id.name])?;
        Ok(())
    }

    fn upgrade(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        tracing::info!("brew: upgrading {}", comp.id.name);
        run_command("brew", &["upgrade", &comp.id.name])?;
        Ok(())
    }

    fn is_installed(&self, comp: &ResolvedComponent) -> bool {
        run_command("brew", &["list", "--versions", &comp.id.name])
            .map(|(out, _)| !out.trim().is_empty())
            .unwrap_or(false)
    }
}
