use crate::error::BackendError;
use crate::traits::{binary_available, run_command, InstallBackend};
use sindri_core::component::Backend;
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::Platform;

/// `pipx install` backend — universal Python application installer (ADR-009).
///
/// Until the [`InstallBackend`] trait is reshaped (Wave 2) to receive the
/// declarative [`sindri_core::component::PipxInstallConfig`], this backend
/// only sees the resolved name+version and cannot honor the optional
/// `--python` interpreter override declared in `component.yaml`.
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
        tracing::info!("pipx: uninstalling {}", comp.id.name);
        run_command("pipx", &["uninstall", &comp.id.name])?;
        Ok(())
    }

    fn upgrade(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        tracing::info!("pipx: upgrading {}", comp.id.name);
        run_command("pipx", &["upgrade", &comp.id.name])?;
        Ok(())
    }

    fn is_installed(&self, comp: &ResolvedComponent) -> bool {
        // Best-effort: `pipx list --short` prints `<name> <version>` lines.
        run_command("pipx", &["list", "--short"])
            .map(|(out, _)| {
                out.lines().any(|line| {
                    line.split_whitespace()
                        .next()
                        .is_some_and(|name| name == comp.id.name)
                })
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use sindri_core::component::ComponentManifest;

    #[test]
    fn deserializes_pipx_install_config() {
        let yaml = r#"
metadata:
  name: black
  version: 24.10.0
  description: Python code formatter
  license: MIT
  tags: []
platforms:
  - { os: linux, arch: x86_64 }
install:
  pipx:
    package: black
    version: "24.10.0"
    python: python3.12
"#;
        let manifest: ComponentManifest =
            serde_yaml::from_str(yaml).expect("parse pipx install config");
        let pipx = manifest.install.pipx.expect("pipx install config present");
        assert_eq!(pipx.package, "black");
        assert_eq!(pipx.version.as_deref(), Some("24.10.0"));
        assert_eq!(pipx.python.as_deref(), Some("python3.12"));
    }

    #[test]
    fn pipx_minimal_config_omits_optional_fields() {
        let yaml = r#"
metadata:
  name: poetry
  version: 1.8.0
  description: Python dependency manager
  license: MIT
  tags: []
platforms:
  - { os: linux, arch: x86_64 }
install:
  pipx:
    package: poetry
"#;
        let manifest: ComponentManifest =
            serde_yaml::from_str(yaml).expect("parse pipx install config");
        let pipx = manifest.install.pipx.expect("pipx install config");
        assert_eq!(pipx.package, "poetry");
        assert!(pipx.version.is_none());
        assert!(pipx.python.is_none());
    }
}
