use sindri_core::component::Backend;
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::Platform;
use crate::error::BackendError;
use crate::traits::InstallBackend;
use crate::brew::BrewBackend;
use crate::binary::BinaryBackend;
use crate::mise::MiseBackend;
use crate::npm::NpmBackend;
use crate::script::ScriptBackend;
use crate::sdkman::SdkmanBackend;
use crate::system_pm::{AptBackend, ApkBackend, DnfBackend, PacmanBackend, ZypperBackend};
use crate::universal::{CargoBackend, GoInstallBackend, PipxBackend};
use crate::winget::{ScoopBackend, WingetBackend};

/// Look up the right backend implementation for a component
pub fn backend_for(backend: &Backend, _platform: &Platform) -> Option<Box<dyn InstallBackend>> {
    match backend {
        Backend::Mise => Some(Box::new(MiseBackend)),
        Backend::Apt => Some(Box::new(AptBackend)),
        Backend::Dnf => Some(Box::new(DnfBackend)),
        Backend::Zypper => Some(Box::new(ZypperBackend)),
        Backend::Pacman => Some(Box::new(PacmanBackend)),
        Backend::Apk => Some(Box::new(ApkBackend)),
        Backend::Npm => Some(Box::new(NpmBackend)),
        Backend::Binary => Some(Box::new(BinaryBackend)),
        Backend::Script => Some(Box::new(ScriptBackend)),
        Backend::Brew => Some(Box::new(BrewBackend)),
        Backend::Winget => Some(Box::new(WingetBackend)),
        Backend::Scoop => Some(Box::new(ScoopBackend)),
        Backend::Pipx => Some(Box::new(PipxBackend)),
        Backend::Cargo => Some(Box::new(CargoBackend)),
        Backend::GoInstall => Some(Box::new(GoInstallBackend)),
        Backend::Sdkman => Some(Box::new(SdkmanBackend)),
        _ => None,
    }
}

/// Install a component using its resolved backend, with fallback messaging
pub fn install_component(comp: &ResolvedComponent, platform: &Platform) -> Result<(), BackendError> {
    let backend = backend_for(&comp.backend, platform).ok_or_else(|| {
        BackendError::Unavailable {
            backend: comp.backend.as_str().to_string(),
        }
    })?;

    if !backend.supports(platform) {
        return Err(BackendError::Unavailable {
            backend: comp.backend.as_str().to_string(),
        });
    }

    // Skip if already at the correct version
    if backend.is_installed(comp) {
        tracing::info!(
            "  {} {} — already installed, skipping",
            comp.id.to_address(),
            comp.version
        );
        return Ok(());
    }

    backend.install(comp)
}
