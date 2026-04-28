use crate::error::BackendError;
use crate::traits::{binary_available, target_exec, InstallBackend, InstallContext};
use async_trait::async_trait;
use sindri_core::component::Backend;
use sindri_core::platform::{Os, Platform};

/// Homebrew backend — macOS (and opt-in Linux) (ADR-009).
///
/// Honors [`sindri_core::component::BrewInstallConfig::tap`] when present:
/// runs `brew tap <tap>` before `brew install`. Without a manifest, only
/// the bare `brew install <name>` form is issued.
pub struct BrewBackend;

#[async_trait]
impl InstallBackend for BrewBackend {
    fn name(&self) -> Backend {
        Backend::Brew
    }

    fn supports(&self, platform: &Platform) -> bool {
        matches!(platform.os, Os::Macos) && binary_available("brew")
    }

    async fn install(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        let (pkg, tap) = if let Some(manifest) = ctx.manifest {
            if let Some(cfg) = manifest.install.brew.as_ref() {
                (cfg.package.clone(), cfg.tap.clone())
            } else {
                tracing::debug!(
                    "brew: manifest present but no brew install block for {}; \
                     using minimal command",
                    comp.id.to_address()
                );
                (comp.id.name.clone(), None)
            }
        } else {
            tracing::debug!(
                "brew: manifest not yet plumbed; using minimal command for {}",
                comp.id.to_address()
            );
            (comp.id.name.clone(), None)
        };

        if let Some(tap) = tap.as_deref() {
            tracing::info!("brew: tapping {}", tap);
            target_exec(ctx.target, "brew", &["tap", tap]).await?;
        }
        tracing::info!("brew: installing {}", pkg);
        target_exec(ctx.target, "brew", &["install", &pkg]).await?;
        Ok(())
    }

    async fn remove(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        tracing::info!("brew: removing {}", comp.id.name);
        target_exec(ctx.target, "brew", &["uninstall", &comp.id.name]).await?;
        Ok(())
    }

    async fn upgrade(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        tracing::info!("brew: upgrading {}", comp.id.name);
        target_exec(ctx.target, "brew", &["upgrade", &comp.id.name]).await?;
        Ok(())
    }

    async fn is_installed(&self, ctx: &InstallContext<'_>) -> bool {
        target_exec(
            ctx.target,
            "brew",
            &["list", "--versions", &ctx.component.id.name],
        )
        .await
        .map(|(out, _)| !out.trim().is_empty())
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
                backend: Backend::Brew,
                name: "ripgrep".into(),
                qualifier: None,
            },
            version: Version::new("14.1.0"),
            backend: Backend::Brew,
            oci_digest: None,
            checksums: HashMap::new(),
            depends_on: vec![],
            manifest: None,
            manifest_digest: None,
            component_digest: None,
        }
    }

    #[tokio::test]
    async fn install_with_tap_runs_tap_then_install() {
        let yaml = r#"
metadata:
  name: ripgrep
  version: 14.1.0
  description: x
  license: MIT
  tags: []
platforms:
  - { os: macos, arch: aarch64 }
install:
  brew:
    package: ripgrep
    tap: someorg/sometap
"#;
        let m: ComponentManifest = serde_yaml::from_str(yaml).unwrap();
        let mock = MockTarget::new();
        let c = comp();
        let ctx = InstallContext::new(&c, Some(&m), &mock);
        BrewBackend.install(&ctx).await.unwrap();
        let calls = mock.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0], "brew tap someorg/sometap");
        assert_eq!(calls[1], "brew install ripgrep");
    }

    #[tokio::test]
    async fn install_without_manifest_falls_back() {
        let mock = MockTarget::new();
        let c = comp();
        let ctx = InstallContext::new(&c, None, &mock);
        BrewBackend.install(&ctx).await.unwrap();
        assert_eq!(mock.last_call().as_deref(), Some("brew install ripgrep"));
    }
}
