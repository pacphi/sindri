//! Extension verify command

use anyhow::{anyhow, Context, Result};
use sindri_core::types::ExtensionState;
use sindri_extensions::{EventEnvelope, ExtensionEvent, StatusLedger};

use crate::cli::ExtensionVerifyArgs;
use crate::output;

/// Verify installed extensions are working correctly
///
/// Checks that installed extensions have valid extension.yaml files and
/// that their required tools/binaries are available on the system.
///
/// Usage:
/// - `sindri extension verify` - verify all installed extensions
/// - `sindri extension verify python` - verify a specific extension
pub(super) async fn run(args: ExtensionVerifyArgs) -> Result<()> {
    use sindri_extensions::{find_extension_yaml, verify_extension_installed};

    let ledger = StatusLedger::load_default().context("Failed to load status ledger")?;
    let status_map = ledger
        .get_all_latest_status()
        .context("Failed to get extension status")?;

    // Get extensions to verify
    let to_verify: Vec<_> = if let Some(name) = &args.name {
        let status = status_map
            .get(name)
            .filter(|s| s.current_state == ExtensionState::Installed)
            .ok_or_else(|| anyhow!("Extension '{}' is not installed", name))?;
        vec![(name.clone(), status.clone())]
    } else {
        status_map
            .iter()
            .filter(|(_, s)| s.current_state == ExtensionState::Installed)
            .map(|(n, s)| (n.clone(), s.clone()))
            .collect()
    };

    if to_verify.is_empty() {
        output::info("No installed extensions to verify");
        return Ok(());
    }

    output::info(&format!("Verifying {} extension(s)...", to_verify.len()));

    let mut verified = 0;
    let mut failed = 0;

    for (name, status) in &to_verify {
        let version = status.version.clone().unwrap_or_default();

        if let Some(yaml_path) = find_extension_yaml(name, &version) {
            match std::fs::read_to_string(&yaml_path) {
                Ok(content) => {
                    match serde_yaml::from_str::<sindri_core::types::Extension>(&content) {
                        Ok(extension) => {
                            let is_verified = verify_extension_installed(&extension).await;
                            if is_verified {
                                output::success(&format!("{} {} verified", name, version));

                                // Publish ValidationSucceeded event
                                let event = EventEnvelope::new(
                                    name.clone(),
                                    Some(ExtensionState::Installed),
                                    ExtensionState::Installed,
                                    ExtensionEvent::ValidationSucceeded {
                                        extension_name: name.clone(),
                                        version: version.clone(),
                                        validation_type: "manual".to_string(),
                                    },
                                );
                                if let Err(e) = ledger.append(event) {
                                    output::warning(&format!(
                                        "Failed to publish validation event: {}",
                                        e
                                    ));
                                }

                                verified += 1;
                            } else {
                                output::error(&format!("{} {} verification failed", name, version));

                                // Publish ValidationFailed event
                                let event = EventEnvelope::new(
                                    name.clone(),
                                    Some(ExtensionState::Installed),
                                    ExtensionState::Failed,
                                    ExtensionEvent::ValidationFailed {
                                        extension_name: name.clone(),
                                        version: version.clone(),
                                        validation_type: "manual".to_string(),
                                        error_message: "Verification checks failed".to_string(),
                                    },
                                );
                                if let Err(e) = ledger.append(event) {
                                    output::warning(&format!(
                                        "Failed to publish validation event: {}",
                                        e
                                    ));
                                }

                                failed += 1;
                            }
                        }
                        Err(e) => {
                            output::error(&format!("{}: invalid extension.yaml: {}", name, e));
                            failed += 1;
                        }
                    }
                }
                Err(e) => {
                    output::error(&format!("{}: cannot read extension.yaml: {}", name, e));
                    failed += 1;
                }
            }
        } else {
            output::error(&format!("{}: extension.yaml not found", name));
            failed += 1;
        }
    }

    println!();
    output::info(&format!(
        "Verification complete: {} verified, {} failed",
        verified, failed
    ));

    if failed > 0 {
        Err(anyhow!("{} extension(s) failed verification", failed))
    } else {
        Ok(())
    }
}
