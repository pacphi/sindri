use crate::error::BackendError;
use crate::traits::{binary_available, target_exec, InstallBackend, InstallContext};
use async_trait::async_trait;
use sindri_core::component::Backend;
use sindri_core::platform::Platform;

/// `cargo install` backend — universal Rust toolchain (ADR-009).
///
/// When `ctx.manifest` is provided, the backend honors the declarative
/// [`sindri_core::component::CargoInstallConfig`]: `--features`, `--git`,
/// and `--locked` (default true). Without a manifest it falls back to the
/// minimal `cargo install <name> --version <v> --locked` form and emits a
/// `tracing::debug!` so the gap is observable.
pub struct CargoBackend;

#[async_trait]
impl InstallBackend for CargoBackend {
    fn name(&self) -> Backend {
        Backend::Cargo
    }

    fn supports(&self, _platform: &Platform) -> bool {
        binary_available("cargo")
    }

    async fn install(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        tracing::info!("cargo: installing {}@{}", comp.id.name, comp.version);

        // Build args. Owned strings, then slice as &[&str] for target_exec.
        let mut args_owned: Vec<String> = vec!["install".into()];
        let features_joined: Option<String>;

        if let Some(manifest) = ctx.manifest {
            if let Some(cfg) = manifest.install.cargo.as_ref() {
                args_owned.push(cfg.crate_name.clone());
                if let Some(v) = cfg.version.as_ref() {
                    args_owned.push("--version".into());
                    args_owned.push(v.clone());
                } else {
                    args_owned.push("--version".into());
                    args_owned.push(comp.version.0.clone());
                }
                if cfg.locked {
                    args_owned.push("--locked".into());
                }
                if !cfg.features.is_empty() {
                    args_owned.push("--features".into());
                    features_joined = Some(cfg.features.join(","));
                    args_owned.push(features_joined.clone().unwrap());
                }
                if let Some(git) = cfg.git.as_ref() {
                    args_owned.push("--git".into());
                    args_owned.push(git.clone());
                }
            } else {
                tracing::debug!(
                    "cargo: manifest present but no cargo install block for {}; \
                     using minimal command",
                    comp.id.to_address()
                );
                push_minimal(&mut args_owned, &comp.id.name, &comp.version.0);
            }
        } else {
            tracing::debug!(
                "cargo: manifest not yet plumbed; using minimal command for {}",
                comp.id.to_address()
            );
            push_minimal(&mut args_owned, &comp.id.name, &comp.version.0);
        }

        let args: Vec<&str> = args_owned.iter().map(|s| s.as_str()).collect();
        target_exec(ctx.target, "cargo", &args).await?;
        Ok(())
    }

    async fn remove(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        tracing::info!("cargo: uninstalling {}", comp.id.name);
        target_exec(ctx.target, "cargo", &["uninstall", &comp.id.name]).await?;
        Ok(())
    }

    async fn is_installed(&self, ctx: &InstallContext<'_>) -> bool {
        // Best-effort: `cargo install --list` lists installed binaries.
        target_exec(ctx.target, "cargo", &["install", "--list"])
            .await
            .map(|(out, _)| {
                out.lines()
                    .any(|line| line.starts_with(&format!("{} v", ctx.component.id.name)))
            })
            .unwrap_or(false)
    }
}

fn push_minimal(args: &mut Vec<String>, name: &str, version: &str) {
    args.push(name.to_string());
    args.push("--version".into());
    args.push(version.to_string());
    args.push("--locked".into());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::test_support::MockTarget;
    use sindri_core::component::{ComponentId, ComponentManifest};
    use sindri_core::lockfile::ResolvedComponent;
    use sindri_core::version::Version;
    use std::collections::HashMap;

    fn comp() -> ResolvedComponent {
        ResolvedComponent {
            id: ComponentId {
                backend: Backend::Cargo,
                name: "ripgrep".into(),
                qualifier: None,
            },
            version: Version::new("14.1.0"),
            backend: Backend::Cargo,
            oci_digest: None,
            checksums: HashMap::new(),
            depends_on: vec![],
            manifest: None,
            manifest_digest: None,
            component_digest: None,
        }
    }

    fn manifest_yaml(yaml: &str) -> ComponentManifest {
        serde_yaml::from_str(yaml).expect("parse manifest")
    }

    #[tokio::test]
    async fn install_with_manifest_features_renders_full_command() {
        let m = manifest_yaml(
            r#"
metadata:
  name: ripgrep
  version: 14.1.0
  description: x
  license: MIT
  tags: []
platforms:
  - { os: linux, arch: x86_64 }
install:
  cargo:
    crate: ripgrep
    version: "14.1.0"
    features: [pcre2, simd]
    locked: true
"#,
        );
        let mock = MockTarget::new();
        let c = comp();
        let ctx = InstallContext::new(&c, Some(&m), &mock);
        CargoBackend.install(&ctx).await.unwrap();
        let call = mock.last_call().unwrap();
        assert!(call.contains("cargo install ripgrep"));
        assert!(call.contains("--version 14.1.0"));
        assert!(call.contains("--locked"));
        assert!(call.contains("--features pcre2,simd"));
    }

    #[tokio::test]
    async fn install_without_manifest_falls_back_to_minimal_command() {
        let mock = MockTarget::new();
        let c = comp();
        let ctx = InstallContext::new(&c, None, &mock);
        CargoBackend.install(&ctx).await.unwrap();
        let call = mock.last_call().unwrap();
        assert_eq!(call, "cargo install ripgrep --version 14.1.0 --locked");
    }

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
