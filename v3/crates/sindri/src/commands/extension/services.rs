//! Extension services command
//!
//! Manages background services registered by extensions with a `service:` block.
//! Services are stored as executable scripts in ~/.sindri/services/<name>.sh.

use anyhow::{anyhow, Result};
use std::path::PathBuf;

use crate::cli::{ExtensionServicesArgs, ServicesAction};
use crate::output;
use crate::utils::get_home_dir;

/// Get the services directory (~/.sindri/services)
fn get_services_dir() -> Result<PathBuf> {
    Ok(get_home_dir()?.join(".sindri").join("services"))
}

/// List all registered services and their status
async fn list_services() -> Result<()> {
    let services_dir = get_services_dir()?;

    if !services_dir.exists() {
        output::info("No extension services registered");
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(&services_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "sh").unwrap_or(false))
        .collect();

    if entries.is_empty() {
        output::info("No extension services registered");
        return Ok(());
    }

    entries.sort_by_key(|e| e.file_name());

    output::header("Extension Services");

    let home = get_home_dir()?;
    let sindri_dir = home.join(".sindri");

    for entry in &entries {
        let name = entry
            .path()
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Check PID file to determine running status
        let pid_file = sindri_dir.join(format!("{}.pid", name));
        let status = if pid_file.exists() {
            if let Ok(pid_str) = std::fs::read_to_string(&pid_file) {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    // Check if process is actually running
                    let check = tokio::process::Command::new("kill")
                        .arg("-0")
                        .arg(pid.to_string())
                        .output()
                        .await;
                    match check {
                        Ok(out) if out.status.success() => format!("running (PID {})", pid),
                        _ => "stopped (stale PID)".to_string(),
                    }
                } else {
                    "stopped (invalid PID)".to_string()
                }
            } else {
                "stopped (unreadable PID)".to_string()
            }
        } else {
            "stopped".to_string()
        };

        output::kv(&name, &status);
    }

    Ok(())
}

/// Start a specific service by name
async fn start_service(name: &str) -> Result<()> {
    let services_dir = get_services_dir()?;
    let script = services_dir.join(format!("{}.sh", name));

    if !script.exists() {
        return Err(anyhow!(
            "No service script found for '{}'. Is the extension installed with a service: block?",
            name
        ));
    }

    output::info(&format!("Starting service: {}", name));

    let result = tokio::process::Command::new("bash")
        .arg(&script)
        .output()
        .await?;

    if result.status.success() {
        output::success(&format!("Service '{}' started", name));
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        let msg = if !stdout.is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        output::error(&format!("Failed to start service '{}': {}", name, msg));
    }

    Ok(())
}

/// Stop a specific service by name
async fn stop_service(name: &str) -> Result<()> {
    let home = get_home_dir()?;
    let pid_file = home.join(".sindri").join(format!("{}.pid", name));

    if !pid_file.exists() {
        output::info(&format!("Service '{}' is not running (no PID file)", name));
        return Ok(());
    }

    let pid_str = std::fs::read_to_string(&pid_file)?;
    let pid: u32 = pid_str
        .trim()
        .parse()
        .map_err(|_| anyhow!("Invalid PID in {}", pid_file.display()))?;

    output::info(&format!("Stopping service '{}' (PID {})...", name, pid));

    // Send SIGTERM
    let term_result = tokio::process::Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .output()
        .await;

    if let Ok(out) = &term_result {
        if !out.status.success() {
            // Process already dead
            std::fs::remove_file(&pid_file).ok();
            output::info(&format!("Service '{}' was already stopped", name));
            return Ok(());
        }
    }

    // Wait up to 10 seconds for graceful shutdown
    for _ in 0..10 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let check = tokio::process::Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .output()
            .await;
        if let Ok(out) = check {
            if !out.status.success() {
                // Process has exited
                std::fs::remove_file(&pid_file).ok();
                output::success(&format!("Service '{}' stopped", name));
                return Ok(());
            }
        }
    }

    // SIGKILL as last resort
    output::warning(&format!(
        "Service '{}' did not stop gracefully, sending SIGKILL",
        name
    ));
    let _ = tokio::process::Command::new("kill")
        .arg("-KILL")
        .arg(pid.to_string())
        .output()
        .await;

    std::fs::remove_file(&pid_file).ok();
    output::success(&format!("Service '{}' killed", name));

    Ok(())
}

/// Main entry point for services subcommand
pub(super) async fn run(args: ExtensionServicesArgs) -> Result<()> {
    match args.action {
        None => list_services().await,
        Some(ServicesAction::Start(arg)) => start_service(&arg.name).await,
        Some(ServicesAction::Stop(arg)) => stop_service(&arg.name).await,
        Some(ServicesAction::Restart(arg)) => {
            stop_service(&arg.name).await?;
            start_service(&arg.name).await
        }
    }
}
