use crate::error::BackendError;
use crate::traits::{binary_available, target_exec, InstallBackend, InstallContext};
use async_trait::async_trait;
use sindri_core::component::Backend;
use sindri_core::platform::Platform;

/// `mise` — version manager backend (cross-platform, ADR-009).
pub struct MiseBackend;

#[async_trait]
impl InstallBackend for MiseBackend {
    fn name(&self) -> Backend {
        Backend::Mise
    }

    fn supports(&self, _platform: &Platform) -> bool {
        binary_available("mise")
    }

    async fn install(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        let tool = format!("{}@{}", comp.id.name, comp.version);
        tracing::info!("mise: installing {}", tool);
        target_exec(ctx.target, "mise", &["install", &tool]).await?;
        Ok(())
    }

    async fn remove(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        let tool = format!("{}@{}", comp.id.name, comp.version);
        tracing::info!("mise: removing {}", tool);
        target_exec(ctx.target, "mise", &["uninstall", &tool]).await?;
        Ok(())
    }

    async fn is_installed(&self, ctx: &InstallContext<'_>) -> bool {
        target_exec(ctx.target, "mise", &["which", &ctx.component.id.name])
            .await
            .map(|(stdout, _)| !stdout.trim().is_empty())
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

    fn comp() -> ResolvedComponent {
        ResolvedComponent {
            id: ComponentId {
                backend: Backend::Mise,
                name: "node".into(),
                qualifier: None,
            },
            version: Version::new("20.0.0"),
            backend: Backend::Mise,
            oci_digest: None,
            checksums: HashMap::new(),
            depends_on: vec![],
        }
    }

    #[tokio::test]
    async fn install_dispatches_correct_mise_command() {
        let mock = MockTarget::new();
        let c = comp();
        let ctx = InstallContext::new(&c, None, &mock);
        MiseBackend.install(&ctx).await.unwrap();
        assert_eq!(
            mock.last_call().as_deref(),
            Some("mise install node@20.0.0")
        );
    }
}
