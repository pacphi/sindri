use sindri_core::component::Backend;
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::Platform;
use crate::error::BackendError;
use crate::traits::{InstallBackend, binary_available, run_command};

/// mise — version manager backend (cross-platform, ADR-009)
pub struct MiseBackend;

impl InstallBackend for MiseBackend {
    fn name(&self) -> Backend {
        Backend::Mise
    }

    fn supports(&self, _platform: &Platform) -> bool {
        binary_available("mise")
    }

    fn install(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        let tool = format!("{}@{}", comp.id.name, comp.version);
        tracing::info!("mise: installing {}", tool);
        run_command("mise", &["install", &tool])?;
        Ok(())
    }

    fn remove(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        let tool = format!("{}@{}", comp.id.name, comp.version);
        tracing::info!("mise: removing {}", tool);
        run_command("mise", &["uninstall", &tool])?;
        Ok(())
    }

    fn is_installed(&self, comp: &ResolvedComponent) -> bool {
        let _tool = format!("{}@{}", comp.id.name, comp.version);
        run_command("mise", &["which", &comp.id.name])
            .map(|(stdout, _)| !stdout.trim().is_empty())
            .unwrap_or(false)
    }
}
