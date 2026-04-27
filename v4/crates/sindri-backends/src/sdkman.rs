use crate::error::BackendError;
use crate::traits::{binary_available, InstallBackend, InstallContext};
use async_trait::async_trait;
use sindri_core::component::Backend;
use sindri_core::platform::{Os, Platform};

/// SDKMAN backend — installs JVM ecosystem tools via `sdk install <candidate> <version>`.
///
/// Wave 2A: migrated to the async target-aware [`InstallBackend`] surface.
/// `sdk` is a shell function (not a binary), so we cannot route through the
/// arg-validating [`crate::traits::target_exec`] helper — the command line
/// contains `$SDKMAN_DIR` and embedded quotes that the helper rejects on
/// purpose. Instead we hand-build the `bash -c '...'` line and dispatch it
/// straight through [`sindri_targets::Target::exec`].
pub struct SdkmanBackend;

#[async_trait]
impl InstallBackend for SdkmanBackend {
    fn name(&self) -> Backend {
        Backend::Sdkman
    }

    fn supports(&self, platform: &Platform) -> bool {
        // SDKMAN only runs on Unix (Linux + macOS); not available on Windows.
        !matches!(platform.os, Os::Windows) && binary_available("sdk")
    }

    async fn install(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let candidate = &ctx.component.id.name;
        let version = &ctx.component.version;
        tracing::info!("sdkman: installing {}@{}", candidate, version);
        if ctx.manifest.is_none() {
            tracing::debug!(
                "sdkman: manifest not yet plumbed; using minimal command for {}",
                ctx.component.id.to_address()
            );
        }
        exec_sdk(ctx, &format!("sdk install {} {}", candidate, version))?;
        Ok(())
    }

    async fn remove(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let candidate = &ctx.component.id.name;
        let version = &ctx.component.version;
        tracing::info!("sdkman: removing {}@{}", candidate, version);
        exec_sdk(ctx, &format!("sdk uninstall {} {}", candidate, version))?;
        Ok(())
    }

    async fn is_installed(&self, ctx: &InstallContext<'_>) -> bool {
        let candidate = &ctx.component.id.name;
        let version = &ctx.component.version;
        let probe = format!(
            "sdk current {} 2>/dev/null | grep -q \"{}\"",
            candidate, version
        );
        exec_sdk(ctx, &probe)
            .map(|(stdout, _)| !stdout.is_empty())
            .unwrap_or(false)
    }
}

/// Build the SDKMAN-init-then-run wrapper and dispatch through `Target::exec`.
fn exec_sdk(ctx: &InstallContext<'_>, sdk_cmd: &str) -> Result<(String, String), BackendError> {
    // reason: `sdk` is a bash shell function shipped by SDKMAN, so we must
    // source the init script in the same shell before invoking it. The
    // command line includes `$`, `"`, and `&&` which `target_exec`'s
    // conservative quoter rejects — we therefore call `target.exec`
    // directly. The whole inner script is wrapped in single quotes; we
    // assume `sdk_cmd` is built from internal callers and contains no
    // single quotes (the candidate name + semver version that compose
    // it can never contain `'`).
    debug_assert!(
        !sdk_cmd.contains('\''),
        "sdkman exec_sdk: inner command must not contain single quotes"
    );
    let line = format!(
        "bash -c 'source \"$SDKMAN_DIR/bin/sdkman-init.sh\" && {}'",
        sdk_cmd
    );
    ctx.target.exec(&line, &[]).map_err(BackendError::from)
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
                backend: Backend::Sdkman,
                name: "java".into(),
                qualifier: None,
            },
            version: Version::new("21.0.2-tem"),
            backend: Backend::Sdkman,
            oci_digest: None,
            checksums: HashMap::new(),
            depends_on: vec![],
            manifest: None,
            manifest_digest: None,
        }
    }

    #[tokio::test]
    async fn install_sources_init_script_and_runs_sdk_install() {
        let mock = MockTarget::new();
        let c = comp();
        let ctx = InstallContext::new(&c, None, &mock);
        SdkmanBackend.install(&ctx).await.unwrap();
        let call = mock.last_call().expect("exec called");
        assert!(call.starts_with("bash -c 'source \"$SDKMAN_DIR/bin/sdkman-init.sh\""));
        assert!(call.contains("sdk install java 21.0.2-tem"));
    }

    #[tokio::test]
    async fn remove_invokes_sdk_uninstall() {
        let mock = MockTarget::new();
        let c = comp();
        let ctx = InstallContext::new(&c, None, &mock);
        SdkmanBackend.remove(&ctx).await.unwrap();
        let call = mock.last_call().expect("exec called");
        assert!(call.contains("sdk uninstall java 21.0.2-tem"));
    }
}
