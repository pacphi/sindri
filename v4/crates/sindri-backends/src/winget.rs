use crate::error::BackendError;
use crate::traits::{binary_available, target_exec, InstallBackend, InstallContext};
use async_trait::async_trait;
use sindri_core::component::Backend;
use sindri_core::platform::{Os, Platform};

/// winget backend — Windows only (ADR-009).
///
/// Wave 2A: migrated to the async target-aware [`InstallBackend`] surface.
/// The minimal `winget install --exact --id <name>` form is used in all
/// cases; declarative options (e.g. `--source`, `--scope`) are deferred to
/// Wave 2C [`sindri_core::component::WingetInstallConfig`] expansion.
pub struct WingetBackend;

#[async_trait]
impl InstallBackend for WingetBackend {
    fn name(&self) -> Backend {
        Backend::Winget
    }

    fn supports(&self, platform: &Platform) -> bool {
        matches!(platform.os, Os::Windows) && binary_available("winget")
    }

    async fn install(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        tracing::info!("winget: installing {}", comp.id.name);
        if ctx.manifest.is_none() {
            tracing::debug!(
                "winget: manifest not yet plumbed; using minimal command for {}",
                comp.id.to_address()
            );
        }
        target_exec(
            ctx.target,
            "winget",
            &["install", "--exact", "--id", &comp.id.name, "-e"],
        )
        .await?;
        Ok(())
    }

    async fn remove(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        target_exec(
            ctx.target,
            "winget",
            &["uninstall", "--exact", "--id", &comp.id.name],
        )
        .await?;
        Ok(())
    }

    async fn is_installed(&self, ctx: &InstallContext<'_>) -> bool {
        let name = ctx.component.id.name.clone();
        target_exec(ctx.target, "winget", &["list", "--exact", "--id", &name])
            .await
            .map(|(out, _)| out.contains(&name))
            .unwrap_or(false)
    }
}

/// Scoop backend — Windows only (ADR-009).
pub struct ScoopBackend;

#[async_trait]
impl InstallBackend for ScoopBackend {
    fn name(&self) -> Backend {
        Backend::Scoop
    }

    fn supports(&self, platform: &Platform) -> bool {
        matches!(platform.os, Os::Windows) && binary_available("scoop")
    }

    async fn install(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        tracing::info!("scoop: installing {}", comp.id.name);
        if ctx.manifest.is_none() {
            tracing::debug!(
                "scoop: manifest not yet plumbed; using minimal command for {}",
                comp.id.to_address()
            );
        }
        target_exec(ctx.target, "scoop", &["install", &comp.id.name]).await?;
        Ok(())
    }

    async fn remove(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        target_exec(ctx.target, "scoop", &["uninstall", &ctx.component.id.name]).await?;
        Ok(())
    }

    async fn is_installed(&self, ctx: &InstallContext<'_>) -> bool {
        let name = ctx.component.id.name.clone();
        target_exec(ctx.target, "scoop", &["list"])
            .await
            .map(|(out, _)| out.contains(&name))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::test_support::MockTarget;
    use sindri_core::component::ComponentId;
    use sindri_core::lockfile::ResolvedComponent;
    use sindri_core::version::Version;
    use std::collections::HashMap;

    fn comp(backend: Backend, name: &str) -> ResolvedComponent {
        ResolvedComponent {
            id: ComponentId {
                backend: backend.clone(),
                name: name.into(),
                qualifier: None,
            },
            version: Version::new("1.0.0"),
            backend,
            oci_digest: None,
            checksums: HashMap::new(),
            depends_on: vec![],
            manifest: None,
        }
    }

    #[tokio::test]
    async fn winget_install_dispatches_minimal_command_without_manifest() {
        let mock = MockTarget::new();
        let c = comp(Backend::Winget, "Microsoft.PowerToys");
        let ctx = InstallContext::new(&c, None, &mock);
        WingetBackend.install(&ctx).await.unwrap();
        assert_eq!(
            mock.last_call().as_deref(),
            Some("winget install --exact --id Microsoft.PowerToys -e")
        );
    }

    #[tokio::test]
    async fn winget_remove_dispatches_uninstall() {
        let mock = MockTarget::new();
        let c = comp(Backend::Winget, "Microsoft.PowerToys");
        let ctx = InstallContext::new(&c, None, &mock);
        WingetBackend.remove(&ctx).await.unwrap();
        assert_eq!(
            mock.last_call().as_deref(),
            Some("winget uninstall --exact --id Microsoft.PowerToys")
        );
    }

    #[tokio::test]
    async fn scoop_install_dispatches_minimal_command_without_manifest() {
        let mock = MockTarget::new();
        let c = comp(Backend::Scoop, "ripgrep");
        let ctx = InstallContext::new(&c, None, &mock);
        ScoopBackend.install(&ctx).await.unwrap();
        assert_eq!(mock.last_call().as_deref(), Some("scoop install ripgrep"));
    }

    #[tokio::test]
    async fn scoop_remove_dispatches_uninstall() {
        let mock = MockTarget::new();
        let c = comp(Backend::Scoop, "ripgrep");
        let ctx = InstallContext::new(&c, None, &mock);
        ScoopBackend.remove(&ctx).await.unwrap();
        assert_eq!(mock.last_call().as_deref(), Some("scoop uninstall ripgrep"));
    }
}
