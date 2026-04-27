use crate::error::BackendError;
use crate::traits::{binary_available, run_command, InstallBackend};
use sindri_core::component::Backend;
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::{Os, Platform};

/// SDKMAN backend — installs JVM ecosystem tools via `sdk install <candidate> <version>`
pub struct SdkmanBackend;

impl InstallBackend for SdkmanBackend {
    fn name(&self) -> Backend {
        Backend::Sdkman
    }

    fn supports(&self, platform: &Platform) -> bool {
        // SDKMAN only runs on Unix (Linux + macOS); not available on Windows
        !matches!(platform.os, Os::Windows) && binary_available("sdk")
    }

    fn install(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        let candidate = &comp.id.name;
        let version = &comp.version;
        tracing::info!("sdkman: installing {}@{}", candidate, version);
        // `sdk install` is a shell function, not a binary — must invoke via bash
        run_command(
            "bash",
            &[
                "-c",
                &format!(
                    r#"source "$SDKMAN_DIR/bin/sdkman-init.sh" && sdk install {} {}"#,
                    candidate, version
                ),
            ],
        )?;
        Ok(())
    }

    fn remove(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        let candidate = &comp.id.name;
        let version = &comp.version;
        tracing::info!("sdkman: removing {}@{}", candidate, version);
        run_command(
            "bash",
            &[
                "-c",
                &format!(
                    r#"source "$SDKMAN_DIR/bin/sdkman-init.sh" && sdk uninstall {} {}"#,
                    candidate, version
                ),
            ],
        )?;
        Ok(())
    }

    fn is_installed(&self, comp: &ResolvedComponent) -> bool {
        let candidate = &comp.id.name;
        let version = &comp.version;
        let check = format!(
            r#"source "$SDKMAN_DIR/bin/sdkman-init.sh" 2>/dev/null && sdk current {candidate} 2>/dev/null | grep -q "{version}""#,
        );
        run_command("bash", &["-c", &check])
            .map(|(stdout, _)| !stdout.is_empty())
            .unwrap_or(false)
    }
}
