use sindri_core::component::Backend;
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::Platform;
use crate::error::BackendError;
use crate::traits::{InstallBackend, binary_available, run_command};

/// npm global install backend
pub struct NpmBackend;

impl InstallBackend for NpmBackend {
    fn name(&self) -> Backend {
        Backend::Npm
    }

    fn supports(&self, _platform: &Platform) -> bool {
        binary_available("npm")
    }

    fn install(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        let pkg = format!("{}@{}", comp.id.name, comp.version);
        tracing::info!("npm: installing {}", pkg);
        run_command("npm", &["install", "-g", &pkg])?;
        Ok(())
    }

    fn remove(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        tracing::info!("npm: removing {}", comp.id.name);
        run_command("npm", &["uninstall", "-g", &comp.id.name])?;
        Ok(())
    }

    fn is_installed(&self, comp: &ResolvedComponent) -> bool {
        run_command("npm", &["list", "-g", "--json"])
            .map(|(out, _)| {
                let json: serde_json::Value = serde_json::from_str(&out).unwrap_or_default();
                json.get("dependencies")
                    .and_then(|d| d.get(&comp.id.name))
                    .is_some()
            })
            .unwrap_or(false)
    }
}
