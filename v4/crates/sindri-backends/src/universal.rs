use sindri_core::component::Backend;
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::Platform;
use crate::error::BackendError;
use crate::traits::{InstallBackend, binary_available, run_command};

/// pipx backend — all platforms (ADR-009)
pub struct PipxBackend;

impl InstallBackend for PipxBackend {
    fn name(&self) -> Backend {
        Backend::Pipx
    }

    fn supports(&self, _platform: &Platform) -> bool {
        binary_available("pipx")
    }

    fn install(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        let pkg = format!("{}=={}", comp.id.name, comp.version);
        tracing::info!("pipx: installing {}", pkg);
        run_command("pipx", &["install", &pkg])?;
        Ok(())
    }

    fn remove(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        run_command("pipx", &["uninstall", &comp.id.name])?;
        Ok(())
    }

    fn upgrade(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        run_command("pipx", &["upgrade", &comp.id.name])?;
        Ok(())
    }

    fn is_installed(&self, comp: &ResolvedComponent) -> bool {
        run_command("pipx", &["list", "--short"])
            .map(|(out, _)| out.contains(&comp.id.name))
            .unwrap_or(false)
    }
}

/// cargo install backend — all platforms (ADR-009)
pub struct CargoBackend;

impl InstallBackend for CargoBackend {
    fn name(&self) -> Backend {
        Backend::Cargo
    }

    fn supports(&self, _platform: &Platform) -> bool {
        binary_available("cargo")
    }

    fn install(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        tracing::info!("cargo: installing {}@{}", comp.id.name, comp.version);
        run_command(
            "cargo",
            &["install", &comp.id.name, "--version", &comp.version.0],
        )?;
        Ok(())
    }

    fn remove(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        run_command("cargo", &["uninstall", &comp.id.name])?;
        Ok(())
    }

    fn is_installed(&self, comp: &ResolvedComponent) -> bool {
        run_command("cargo", &["install", "--list"])
            .map(|(out, _)| out.contains(&comp.id.name))
            .unwrap_or(false)
    }
}

/// go install backend — all platforms (ADR-009)
pub struct GoInstallBackend;

impl InstallBackend for GoInstallBackend {
    fn name(&self) -> Backend {
        Backend::GoInstall
    }

    fn supports(&self, _platform: &Platform) -> bool {
        binary_available("go")
    }

    fn install(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        // go install expects module@version format
        let module_at_ver = format!("{}@{}", comp.id.name, comp.version);
        tracing::info!("go: installing {}", module_at_ver);
        run_command("go", &["install", &module_at_ver])?;
        Ok(())
    }

    fn remove(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        // go doesn't have a formal uninstall; remove from GOPATH/bin
        let bin_name = comp.id.name.split('/').last().unwrap_or(&comp.id.name);
        let gopath = std::env::var("GOPATH")
            .unwrap_or_else(|_| {
                dirs_next::home_dir()
                    .unwrap_or_default()
                    .join("go")
                    .to_string_lossy()
                    .to_string()
            });
        let bin_path = std::path::Path::new(&gopath).join("bin").join(bin_name);
        if bin_path.exists() {
            std::fs::remove_file(&bin_path)?;
        }
        Ok(())
    }

    fn is_installed(&self, comp: &ResolvedComponent) -> bool {
        let bin_name = comp.id.name.split('/').last().unwrap_or(&comp.id.name);
        crate::traits::binary_available(bin_name)
    }
}
