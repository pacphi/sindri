use crate::error::BackendError;
use crate::traits::InstallBackend;
use sindri_core::component::Backend;
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::Platform;
use std::fs;
use std::path::{Path, PathBuf};

/// Direct binary download backend (ADR-010: central platform-matrix resolver)
pub struct BinaryBackend;

impl InstallBackend for BinaryBackend {
    fn name(&self) -> Backend {
        Backend::Binary
    }

    fn supports(&self, _platform: &Platform) -> bool {
        true // available on all platforms; individual components list their platforms
    }

    fn install(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        tracing::info!("binary: installing {}@{}", comp.id.name, comp.version);

        // The oci_digest field in Sprint 3 holds the OCI ref (url template in real impl)
        // For Sprint 4: stub that verifies the checksum map is non-empty, then no-op
        // Full download + verify in Sprint 6 hardening
        if comp.checksums.is_empty() {
            tracing::warn!(
                "binary: no checksums for {} — install skipped (run sindri registry fetch-checksums)",
                comp.id.name
            );
            return Ok(());
        }

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

        // Sprint 4 stub: just log the checksum verification step
        tracing::info!(
            "binary: would verify sha256 {} for {}",
            expected_checksum,
            platform_key
        );

        Ok(())
    }

    fn remove(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        // Determine install path and remove the binary
        let install_path = expand_install_path("~/.local/bin", &comp.id.name);
        if install_path.exists() {
            fs::remove_file(&install_path)?;
            tracing::info!("binary: removed {}", install_path.display());
        }
        Ok(())
    }

    fn is_installed(&self, comp: &ResolvedComponent) -> bool {
        let path = expand_install_path("~/.local/bin", &comp.id.name);
        path.exists()
    }
}

fn expand_install_path(template: &str, name: &str) -> PathBuf {
    let base = template.replace('~', &dirs_next_home());
    Path::new(&base).join(name).to_path_buf()
}

fn dirs_next_home() -> String {
    dirs_next::home_dir()
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
