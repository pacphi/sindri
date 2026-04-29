use crate::error::BackendError;
use crate::traits::{InstallBackend, InstallContext};
use async_trait::async_trait;
use sindri_core::component::Backend;
use sindri_core::platform::Platform;
use std::fs;
use std::path::{Path, PathBuf};

/// Direct binary download backend (ADR-010: central platform-matrix resolver).
///
/// Wave 2A: this backend is still a Sprint 4 stub for the actual download
/// path — full URL fetch + sha256 verify lands with OCI manifest plumbing
/// (Wave 3). What changed in Wave 2A is the trait surface: it is now async
/// and target-aware, so when the download is implemented it can stream the
/// asset onto a remote target via [`sindri_targets::Target::upload`].
pub struct BinaryBackend;

#[async_trait]
impl InstallBackend for BinaryBackend {
    fn name(&self) -> Backend {
        Backend::Binary
    }

    fn supports(&self, _platform: &Platform) -> bool {
        true // available on all platforms; individual components list their platforms
    }

    async fn install(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        tracing::info!("binary: installing {}@{}", comp.id.name, comp.version);

        if comp.checksums.is_empty() {
            tracing::warn!(
                "binary: no checksums for {} — install skipped (run sindri registry fetch-checksums)",
                comp.id.name
            );
            return Ok(());
        }

        // For local target use the host platform; for remote targets we
        // would query target.profile() — that path will be exercised once
        // the actual download lands (Wave 3).
        let platform = Platform::current();
        let platform_key = format!(
            "{}-{}",
            platform_os_str(&platform),
            platform_arch_str(&platform)
        );

        let expected_checksum = comp.checksums.get(&platform_key).ok_or_else(|| {
            BackendError::install(
                &comp.id.name,
                format!("no checksum for platform {}", platform_key),
            )
        })?;

        // Sprint 4 stub: just log the checksum verification step.
        tracing::info!(
            "binary: would verify sha256 {} for {}",
            expected_checksum,
            platform_key
        );

        Ok(())
    }

    async fn remove(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        // reason: binary remove inspects/mutates the local filesystem; remote
        // remove requires Target::exec("rm ...") and lands with Wave 3+.
        if ctx.target.kind() != "local" {
            return Err(BackendError::RemoveFailed {
                component: ctx.component.id.to_address(),
                detail: "binary remove on remote targets lands with Wave 3+ remote work".into(),
            });
        }
        let install_path = expand_install_path("~/.local/bin", &ctx.component.id.name);
        if install_path.exists() {
            fs::remove_file(&install_path)?;
            tracing::info!("binary: removed {}", install_path.display());
        }
        Ok(())
    }

    async fn is_installed(&self, ctx: &InstallContext<'_>) -> bool {
        if ctx.target.kind() != "local" {
            return false; // conservative; full impl arrives with Wave 3
        }
        let path = expand_install_path("~/.local/bin", &ctx.component.id.name);
        path.exists()
    }
}

fn expand_install_path(template: &str, name: &str) -> PathBuf {
    let base = template.replace('~', &dirs_next_home());
    Path::new(&base).join(name).to_path_buf()
}

fn dirs_next_home() -> String {
    sindri_core::paths::home_dir()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn platform_os_str(p: &Platform) -> &'static str {
    match p.os {
        sindri_core::platform::Os::Linux => "linux",
        sindri_core::platform::Os::Macos => "macos",
        sindri_core::platform::Os::Windows => "windows",
    }
}

fn platform_arch_str(p: &Platform) -> &'static str {
    match p.arch {
        sindri_core::platform::Arch::X86_64 => "x86_64",
        sindri_core::platform::Arch::Aarch64 => "aarch64",
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

    #[tokio::test]
    async fn install_without_checksums_returns_ok_and_does_not_exec() {
        let mock = MockTarget::new();
        let c = ResolvedComponent {
            id: ComponentId {
                backend: Backend::Binary,
                name: "ripgrep".into(),
                qualifier: None,
            },
            version: Version::new("14.1.0"),
            backend: Backend::Binary,
            oci_digest: None,
            checksums: HashMap::new(),
            depends_on: vec![],
            manifest: None,
            manifest_digest: None,
            component_digest: None,
            platforms: None,
        };
        let ctx = InstallContext::new(&c, None, &mock);
        // Empty-checksum path: backend logs a warn and returns Ok without
        // invoking `target.exec` (download is Wave 3 work).
        BinaryBackend.install(&ctx).await.unwrap();
        assert!(mock.last_call().is_none());
    }
}
