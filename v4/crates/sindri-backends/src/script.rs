use std::fs;
use std::path::PathBuf;
use sindri_core::component::Backend;
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::{Os, Platform};
use crate::error::BackendError;
use crate::traits::InstallBackend;

/// Script backend — runs install.sh (bash) or install.ps1 (PowerShell)
pub struct ScriptBackend;

impl InstallBackend for ScriptBackend {
    fn name(&self) -> Backend {
        Backend::Script
    }

    fn supports(&self, _platform: &Platform) -> bool {
        true
    }

    fn install(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        let platform = Platform::current();
        tracing::info!("script: installing {}@{}", comp.id.name, comp.version);

        // The script URL would come from the ComponentManifest.install.script
        // For Sprint 4: check if a local script is available from cache
        let script_path = cached_script_path(&comp.id.name, &platform);
        if !script_path.exists() {
            // Sprint 4 stub: no cached script
            tracing::warn!(
                "script: no cached script for {} on {} — skipping",
                comp.id.name,
                platform.triple()
            );
            return Ok(());
        }

        match platform.os {
            Os::Windows => run_ps1(&script_path, comp),
            _ => run_sh(&script_path, comp),
        }
    }

    fn remove(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        tracing::warn!(
            "script: no standard remove for {} — manual cleanup may be required",
            comp.id.name
        );
        Ok(())
    }

    fn is_installed(&self, _comp: &ResolvedComponent) -> bool {
        false // Script backend can't reliably detect installation state
    }
}

fn run_sh(script: &std::path::Path, comp: &ResolvedComponent) -> Result<(), BackendError> {
    let output = std::process::Command::new("bash")
        .arg(script)
        .env("SINDRI_COMPONENT", &comp.id.name)
        .env("SINDRI_VERSION", comp.version.0.as_str())
        .output()
        .map_err(|e| BackendError::install(&comp.id.name, e.to_string()))?;

    if !output.status.success() {
        return Err(BackendError::install(
            &comp.id.name,
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    Ok(())
}

fn run_ps1(script: &std::path::Path, comp: &ResolvedComponent) -> Result<(), BackendError> {
    let output = std::process::Command::new("pwsh")
        .args(["-NonInteractive", "-File", &script.to_string_lossy()])
        .env("SINDRI_COMPONENT", &comp.id.name)
        .env("SINDRI_VERSION", comp.version.0.as_str())
        .output()
        .map_err(|e| BackendError::install(&comp.id.name, e.to_string()))?;

    if !output.status.success() {
        return Err(BackendError::install(
            &comp.id.name,
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    Ok(())
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
