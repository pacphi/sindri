use crate::error::BackendError;
use crate::traits::{target_exec, InstallBackend, InstallContext};
use async_trait::async_trait;
use sindri_core::component::Backend;
use sindri_core::platform::{Os, Platform};
use std::path::PathBuf;

/// Script backend — runs `install.sh` (bash) or `install.ps1` (PowerShell).
///
/// On the local target the script is read from the per-component cache
/// (`~/.sindri/cache/scripts/<name>.{sh,ps1}`) and invoked via the
/// target's `exec`. Remote targets are not yet supported by this backend
/// (uploading the script is Wave 3+ work).
pub struct ScriptBackend;

#[async_trait]
impl InstallBackend for ScriptBackend {
    fn name(&self) -> Backend {
        Backend::Script
    }

    fn supports(&self, _platform: &Platform) -> bool {
        true
    }

    async fn install(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        let platform = Platform::current();
        tracing::info!("script: installing {}@{}", comp.id.name, comp.version);

        // reason: cached scripts live on the local filesystem; remote target
        // support requires upload + exec which is out of scope for Wave 2A.
        if ctx.target.kind() != "local" {
            return Err(BackendError::InstallFailed {
                component: comp.id.to_address(),
                detail: "script backend on remote targets requires upload+exec; \
                         lands with Wave 3+ remote target work"
                    .into(),
            });
        }

        let script_path = cached_script_path(&comp.id.name, &platform);
        if !script_path.exists() {
            tracing::warn!(
                "script: no cached script for {} on {} — skipping",
                comp.id.name,
                platform.triple()
            );
            return Ok(());
        }

        let script_str = script_path.to_string_lossy().to_string();
        match platform.os {
            Os::Windows => {
                target_exec(
                    ctx.target,
                    "pwsh",
                    &["-NonInteractive", "-File", &script_str],
                )
                .await?;
            }
            _ => {
                target_exec(ctx.target, "bash", &[&script_str]).await?;
            }
        }
        Ok(())
    }

    async fn remove(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        tracing::warn!(
            "script: no standard remove for {} — manual cleanup may be required",
            ctx.component.id.name
        );
        Ok(())
    }

    async fn is_installed(&self, _ctx: &InstallContext<'_>) -> bool {
        // Script backend can't reliably detect installation state.
        false
    }
}

fn cached_script_path(name: &str, platform: &Platform) -> PathBuf {
    let ext = match platform.os {
        Os::Windows => "ps1",
        _ => "sh",
    };
    dirs_next::home_dir()
        .unwrap_or_default()
        .join(".sindri")
        .join("cache")
        .join("scripts")
        .join(format!("{}.{}", name, ext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::component::{Backend, ComponentId};
    use sindri_core::lockfile::ResolvedComponent;
    use sindri_core::version::Version;
    use sindri_targets::error::TargetError;
    use sindri_targets::traits::PrereqCheck;
    use std::collections::HashMap;
    use std::path::Path;

    /// `Target` whose `kind()` reports something other than `"local"`. The
    /// script backend should refuse to operate on it.
    struct RemoteFakeTarget;
    impl sindri_targets::Target for RemoteFakeTarget {
        fn name(&self) -> &str {
            "remote-fake"
        }
        fn kind(&self) -> &str {
            "ssh"
        }
        fn profile(&self) -> Result<sindri_core::platform::TargetProfile, TargetError> {
            unreachable!("not used in this test")
        }
        fn exec(&self, _cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
            unreachable!("script backend must not exec on non-local targets")
        }
        fn upload(&self, _local: &Path, _remote: &str) -> Result<(), TargetError> {
            Ok(())
        }
        fn download(&self, _remote: &str, _local: &Path) -> Result<(), TargetError> {
            Ok(())
        }
        fn check_prerequisites(&self) -> Vec<PrereqCheck> {
            vec![]
        }
    }

    #[tokio::test]
    async fn install_on_remote_target_is_rejected_until_wave_3() {
        let target = RemoteFakeTarget;
        let comp = ResolvedComponent {
            id: ComponentId {
                backend: Backend::Script,
                name: "tool".into(),
                qualifier: None,
            },
            version: Version::new("1.0.0"),
            backend: Backend::Script,
            oci_digest: None,
            checksums: HashMap::new(),
            depends_on: vec![],
            manifest: None,
        };
        let ctx = InstallContext::new(&comp, None, &target);
        let err = ScriptBackend.install(&ctx).await.unwrap_err();
        assert!(matches!(err, BackendError::InstallFailed { .. }));
    }
}
