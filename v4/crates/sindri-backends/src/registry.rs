use crate::binary::BinaryBackend;
use crate::brew::BrewBackend;
use crate::cargo::CargoBackend;
use crate::error::BackendError;
use crate::go_install::GoInstallBackend;
use crate::mise::MiseBackend;
use crate::npm::NpmBackend;
use crate::pipx::PipxBackend;
use crate::script::ScriptBackend;
use crate::sdkman::SdkmanBackend;
use crate::system_pm::{ApkBackend, AptBackend, DnfBackend, PacmanBackend, ZypperBackend};
use crate::traits::{InstallBackend, InstallContext};
use crate::winget::{ScoopBackend, WingetBackend};
use sindri_core::component::{Backend, ComponentManifest};
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::Platform;
use sindri_targets::Target;

/// Look up the right backend implementation for a component.
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

/// Install a component using its resolved backend, dispatching all shell
/// invocations through `target` (Wave 2A, ADR-017).
///
/// `manifest` is `Option<&ComponentManifest>` because OCI manifest fetch is
/// not wired in until Wave 3 — when it is `None`, individual backends fall
/// back to a minimal `name@version` invocation and emit a `tracing::debug!`.
pub async fn install_component(
    comp: &ResolvedComponent,
    manifest: Option<&ComponentManifest>,
    target: &dyn Target,
) -> Result<(), BackendError> {
    let platform = Platform::current();
    let backend =
        backend_for(&comp.backend, &platform).ok_or_else(|| BackendError::Unavailable {
            backend: comp.backend.as_str().to_string(),
        })?;

    if !backend.supports(&platform) {
        return Err(BackendError::Unavailable {
            backend: comp.backend.as_str().to_string(),
        });
    }

    let ctx = InstallContext::new(comp, manifest, target);

    if backend.is_installed(&ctx).await {
        tracing::info!(
            "  {} {} — already installed, skipping",
            comp.id.to_address(),
            comp.version
        );
        return Ok(());
    }

    backend.install(&ctx).await
}
