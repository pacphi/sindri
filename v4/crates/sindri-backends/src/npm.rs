use crate::error::BackendError;
use crate::traits::{binary_available, target_exec, InstallBackend, InstallContext};
use async_trait::async_trait;
use sindri_core::component::Backend;
use sindri_core::platform::Platform;

/// npm install backend.
///
/// Honors [`sindri_core::component::NpmInstallConfig::global`] when the
/// manifest is provided. Without a manifest, defaults to `--global` because
/// that is the historical behavior and what the v3 lockfile contract
/// expects.
pub struct NpmBackend;

#[async_trait]
impl InstallBackend for NpmBackend {
    fn name(&self) -> Backend {
        Backend::Npm
    }

    fn supports(&self, _platform: &Platform) -> bool {
        binary_available("npm")
    }

    async fn install(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        let (pkg, global) = if let Some(manifest) = ctx.manifest {
            if let Some(cfg) = manifest.install.npm.as_ref() {
                (format!("{}@{}", cfg.package, comp.version), cfg.global)
            } else {
                tracing::debug!(
                    "npm: manifest present but no npm install block for {}; \
                     using minimal command",
                    comp.id.to_address()
                );
                (format!("{}@{}", comp.id.name, comp.version), true)
            }
        } else {
            tracing::debug!(
                "npm: manifest not yet plumbed; using minimal command for {}",
                comp.id.to_address()
            );
            (format!("{}@{}", comp.id.name, comp.version), true)
        };
        tracing::info!("npm: installing {} (global={})", pkg, global);
        let mut args: Vec<&str> = vec!["install"];
        if global {
            args.push("-g");
        }
        args.push(&pkg);
        target_exec(ctx.target, "npm", &args).await?;
        Ok(())
    }

    async fn remove(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        tracing::info!("npm: removing {}", comp.id.name);
        target_exec(ctx.target, "npm", &["uninstall", "-g", &comp.id.name]).await?;
        Ok(())
    }

    async fn is_installed(&self, ctx: &InstallContext<'_>) -> bool {
        target_exec(ctx.target, "npm", &["list", "-g", "--json"])
            .await
            .map(|(out, _)| {
                let json: serde_json::Value = serde_json::from_str(&out).unwrap_or_default();
                json.get("dependencies")
                    .and_then(|d| d.get(&ctx.component.id.name))
                    .is_some()
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
                backend: Backend::Npm,
                name: "typescript".into(),
                qualifier: None,
            },
            version: Version::new("5.4.0"),
            backend: Backend::Npm,
            oci_digest: None,
            checksums: HashMap::new(),
            depends_on: vec![],
            manifest: None,
            manifest_digest: None,
        }
    }

    #[tokio::test]
    async fn install_with_manifest_global_true() {
        let yaml = r#"
metadata:
  name: typescript
  version: 5.4.0
  description: x
  license: Apache-2.0
  tags: []
platforms:
  - { os: linux, arch: x86_64 }
install:
  npm:
    package: typescript
    global: true
"#;
        let m: ComponentManifest = serde_yaml::from_str(yaml).unwrap();
        let mock = MockTarget::new();
        let c = comp();
        let ctx = InstallContext::new(&c, Some(&m), &mock);
        NpmBackend.install(&ctx).await.unwrap();
        assert_eq!(
            mock.last_call().as_deref(),
            Some("npm install -g typescript@5.4.0")
        );
    }

    #[tokio::test]
    async fn install_without_manifest_falls_back() {
        let mock = MockTarget::new();
        let c = comp();
        let ctx = InstallContext::new(&c, None, &mock);
        NpmBackend.install(&ctx).await.unwrap();
        assert_eq!(
            mock.last_call().as_deref(),
            Some("npm install -g typescript@5.4.0")
        );
    }
}
