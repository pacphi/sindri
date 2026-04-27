use crate::error::BackendError;
use crate::traits::{binary_available, target_exec, InstallBackend, InstallContext};
use async_trait::async_trait;
use sindri_core::component::Backend;
use sindri_core::platform::Platform;

/// `pipx install` backend — universal Python application installer (ADR-009).
///
/// Honors the declarative [`sindri_core::component::PipxInstallConfig`]
/// when `ctx.manifest` is provided (in particular `--python <python>`).
/// Falls back to `pipx install <name>==<version>` when the manifest is
/// absent.
pub struct PipxBackend;

#[async_trait]
impl InstallBackend for PipxBackend {
    fn name(&self) -> Backend {
        Backend::Pipx
    }

    fn supports(&self, _platform: &Platform) -> bool {
        binary_available("pipx")
    }

    async fn install(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        let mut args_owned: Vec<String> = vec!["install".into()];

        if let Some(manifest) = ctx.manifest {
            if let Some(cfg) = manifest.install.pipx.as_ref() {
                let pkg = match cfg.version.as_ref() {
                    Some(v) => format!("{}=={}", cfg.package, v),
                    None => format!("{}=={}", cfg.package, comp.version),
                };
                args_owned.push(pkg);
                if let Some(py) = cfg.python.as_ref() {
                    args_owned.push("--python".into());
                    args_owned.push(py.clone());
                }
            } else {
                tracing::debug!(
                    "pipx: manifest present but no pipx install block for {}; \
                     using minimal command",
                    comp.id.to_address()
                );
                args_owned.push(format!("{}=={}", comp.id.name, comp.version));
            }
        } else {
            tracing::debug!(
                "pipx: manifest not yet plumbed; using minimal command for {}",
                comp.id.to_address()
            );
            args_owned.push(format!("{}=={}", comp.id.name, comp.version));
        }

        let args: Vec<&str> = args_owned.iter().map(|s| s.as_str()).collect();
        tracing::info!("pipx: installing {}", &args_owned[1]);
        target_exec(ctx.target, "pipx", &args).await?;
        Ok(())
    }

    async fn remove(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        tracing::info!("pipx: uninstalling {}", comp.id.name);
        target_exec(ctx.target, "pipx", &["uninstall", &comp.id.name]).await?;
        Ok(())
    }

    async fn upgrade(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        tracing::info!("pipx: upgrading {}", comp.id.name);
        target_exec(ctx.target, "pipx", &["upgrade", &comp.id.name]).await?;
        Ok(())
    }

    async fn is_installed(&self, ctx: &InstallContext<'_>) -> bool {
        target_exec(ctx.target, "pipx", &["list", "--short"])
            .await
            .map(|(out, _)| {
                out.lines().any(|line| {
                    line.split_whitespace()
                        .next()
                        .is_some_and(|name| name == ctx.component.id.name)
                })
            })
            .unwrap_or(false)
    }
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
                backend: Backend::Pipx,
                name: "black".into(),
                qualifier: None,
            },
            version: Version::new("24.10.0"),
            backend: Backend::Pipx,
            oci_digest: None,
            checksums: HashMap::new(),
            depends_on: vec![],
            manifest: None,
            manifest_digest: None,
        }
    }

    #[tokio::test]
    async fn install_with_manifest_python_renders_python_flag() {
        let yaml = r#"
metadata:
  name: black
  version: 24.10.0
  description: x
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
        let m: ComponentManifest = serde_yaml::from_str(yaml).unwrap();
        let mock = MockTarget::new();
        let c = comp();
        let ctx = InstallContext::new(&c, Some(&m), &mock);
        PipxBackend.install(&ctx).await.unwrap();
        let call = mock.last_call().unwrap();
        assert!(call.contains("pipx install black==24.10.0"));
        assert!(call.contains("--python python3.12"));
    }

    #[tokio::test]
    async fn install_without_manifest_falls_back() {
        let mock = MockTarget::new();
        let c = comp();
        let ctx = InstallContext::new(&c, None, &mock);
        PipxBackend.install(&ctx).await.unwrap();
        assert_eq!(
            mock.last_call().as_deref(),
            Some("pipx install black==24.10.0")
        );
    }

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
