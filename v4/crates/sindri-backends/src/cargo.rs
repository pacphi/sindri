use crate::error::BackendError;
use crate::traits::{binary_available, run_command, InstallBackend};
use sindri_core::component::Backend;
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::Platform;

/// `cargo install` backend — universal Rust toolchain (ADR-009).
///
/// Note: until the [`InstallBackend`] trait is reshaped (Wave 2) to accept the
/// declarative [`sindri_core::component::CargoInstallConfig`], this backend can
/// only see a [`ResolvedComponent`] and therefore cannot wire `--features` or
/// `--git`. It always passes `--locked` for reproducibility and uses
/// `--version` from the resolved component.
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
            &[
                "install",
                &comp.id.name,
                "--version",
                &comp.version.0,
                "--locked",
            ],
        )?;
        Ok(())
    }

    fn remove(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        tracing::info!("cargo: uninstalling {}", comp.id.name);
        run_command("cargo", &["uninstall", &comp.id.name])?;
        Ok(())
    }

    fn is_installed(&self, comp: &ResolvedComponent) -> bool {
        // Best-effort: `cargo install --list` lists installed binaries.
        // Each crate appears as `name vX.Y.Z:` followed by indented binary names.
        run_command("cargo", &["install", "--list"])
            .map(|(out, _)| {
                out.lines()
                    .any(|line| line.starts_with(&format!("{} v", comp.id.name)))
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use sindri_core::component::ComponentManifest;

    #[test]
    fn deserializes_cargo_install_config() {
        let yaml = r#"
metadata:
  name: ripgrep
  version: 14.1.0
  description: Fast recursive search
  license: MIT
  tags: []
platforms:
  - { os: linux, arch: x86_64 }
install:
  cargo:
    crate: ripgrep
    version: "14.1.0"
    features:
      - pcre2
    locked: true
"#;
        let manifest: ComponentManifest =
            serde_yaml::from_str(yaml).expect("parse cargo install config");
        let cargo = manifest
            .install
            .cargo
            .expect("cargo install config present");
        assert_eq!(cargo.crate_name, "ripgrep");
        assert_eq!(cargo.version.as_deref(), Some("14.1.0"));
        assert_eq!(cargo.features, vec!["pcre2".to_string()]);
        assert!(cargo.locked);
        assert!(cargo.git.is_none());
    }

    #[test]
    fn cargo_locked_defaults_to_true_when_omitted() {
        let yaml = r#"
metadata:
  name: cargo-edit
  version: 0.12.0
  description: Cargo subcommand
  license: MIT
  tags: []
platforms:
  - { os: linux, arch: x86_64 }
install:
  cargo:
    crate: cargo-edit
"#;
        let manifest: ComponentManifest =
            serde_yaml::from_str(yaml).expect("parse cargo install config");
        let cargo = manifest.install.cargo.expect("cargo install config");
        assert_eq!(cargo.crate_name, "cargo-edit");
        assert!(cargo.locked, "locked should default to true");
        assert!(cargo.features.is_empty());
    }
}
