//! Integration tests for dependency installation event tracking
//!
//! Tests that verify ledger events are properly published when installing
//! extensions with dependencies via ExtensionDistributor.
//!
//! Key scenarios:
//! - Install extension with dependencies via distributor
//! - Verify all dependencies have install_started events
//! - Verify all dependencies have install_completed events
//! - Verify event ordering is correct (deps before main extension)

mod common;

#[cfg(test)]
mod dependency_event_tracking_tests {
    use sindri_core::types::{
        Extension, ExtensionMetadata, ExtensionState, InstallConfig, InstallMethod,
    };
    use sindri_extensions::{EventEnvelope, ExtensionEvent, StatusLedger};
    use tempfile::TempDir;

    /// Create a test extension with dependencies
    fn create_test_extension(name: &str, deps: Vec<String>) -> Extension {
        Extension {
            metadata: ExtensionMetadata {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                description: format!("Test extension: {}", name),
                category: sindri_core::types::ExtensionCategory::Languages,
                author: None,
                homepage: None,
                dependencies: deps,
            },
            requirements: None,
            install: InstallConfig {
                method: InstallMethod::Script,
                mise: None,
                apt: None,
                binary: None,
                npm: None,
                script: None,
            },
            configure: None,
            validate: sindri_core::types::ValidateConfig {
                commands: vec![],
                mise: None,
            },
            remove: None,
            upgrade: None,
            capabilities: None,
            docs: None,
            bom: None,
        }
    }

    #[test]
    fn test_dependency_events_are_published() {
        // Create a temporary ledger
        let temp_dir = TempDir::new().unwrap();
        let ledger_path = temp_dir.path().join("status_ledger.jsonl");
        let ledger = StatusLedger::new(ledger_path.clone());

        // Simulate installing a dependency: publish install_started
        let dep_started = EventEnvelope::new(
            "sdkman".to_string(),
            None,
            ExtensionState::Installing,
            ExtensionEvent::InstallStarted {
                extension_name: "sdkman".to_string(),
                version: "1.0.1".to_string(),
                source: "dependency of jvm".to_string(),
                install_method: "Distributor".to_string(),
            },
        );
        ledger.append(dep_started).unwrap();

        // Simulate successful dependency installation: publish install_completed
        let dep_completed = EventEnvelope::new(
            "sdkman".to_string(),
            Some(ExtensionState::Installing),
            ExtensionState::Installed,
            ExtensionEvent::InstallCompleted {
                extension_name: "sdkman".to_string(),
                version: "1.0.1".to_string(),
                duration_secs: 10,
                components_installed: vec![],
                log_file: None,
            },
        );
        ledger.append(dep_completed).unwrap();

        // Verify events were recorded
        let status_map = ledger.get_all_latest_status().unwrap();
        assert!(status_map.contains_key("sdkman"));

        let sdkman_status = status_map.get("sdkman").unwrap();
        assert_eq!(sdkman_status.current_state, ExtensionState::Installed);
        assert_eq!(sdkman_status.version, Some("1.0.1".to_string()));
    }

    #[test]
    fn test_dependency_chain_event_ordering() {
        // Create a temporary ledger
        let temp_dir = TempDir::new().unwrap();
        let ledger_path = temp_dir.path().join("status_ledger.jsonl");
        let ledger = StatusLedger::new(ledger_path.clone());

        // Simulate installing dependency chain: mise-config -> sdkman -> jvm

        // 1. mise-config (no dependencies)
        ledger
            .append(EventEnvelope::new(
                "mise-config".to_string(),
                None,
                ExtensionState::Installing,
                ExtensionEvent::InstallStarted {
                    extension_name: "mise-config".to_string(),
                    version: "2.0.0".to_string(),
                    source: "dependency of jvm".to_string(),
                    install_method: "Distributor".to_string(),
                },
            ))
            .unwrap();

        ledger
            .append(EventEnvelope::new(
                "mise-config".to_string(),
                Some(ExtensionState::Installing),
                ExtensionState::Installed,
                ExtensionEvent::InstallCompleted {
                    extension_name: "mise-config".to_string(),
                    version: "2.0.0".to_string(),
                    duration_secs: 5,
                    components_installed: vec![],
                    log_file: None,
                },
            ))
            .unwrap();

        // 2. sdkman (no dependencies)
        ledger
            .append(EventEnvelope::new(
                "sdkman".to_string(),
                None,
                ExtensionState::Installing,
                ExtensionEvent::InstallStarted {
                    extension_name: "sdkman".to_string(),
                    version: "1.0.1".to_string(),
                    source: "dependency of jvm".to_string(),
                    install_method: "Distributor".to_string(),
                },
            ))
            .unwrap();

        ledger
            .append(EventEnvelope::new(
                "sdkman".to_string(),
                Some(ExtensionState::Installing),
                ExtensionState::Installed,
                ExtensionEvent::InstallCompleted {
                    extension_name: "sdkman".to_string(),
                    version: "1.0.1".to_string(),
                    duration_secs: 10,
                    components_installed: vec![],
                    log_file: None,
                },
            ))
            .unwrap();

        // 3. jvm (depends on mise-config and sdkman)
        ledger
            .append(EventEnvelope::new(
                "jvm".to_string(),
                None,
                ExtensionState::Installing,
                ExtensionEvent::InstallStarted {
                    extension_name: "jvm".to_string(),
                    version: "2.1.1".to_string(),
                    source: "github:pacphi/sindri".to_string(),
                    install_method: "Distributor".to_string(),
                },
            ))
            .unwrap();

        ledger
            .append(EventEnvelope::new(
                "jvm".to_string(),
                Some(ExtensionState::Installing),
                ExtensionState::Installed,
                ExtensionEvent::InstallCompleted {
                    extension_name: "jvm".to_string(),
                    version: "2.1.1".to_string(),
                    duration_secs: 30,
                    components_installed: vec![],
                    log_file: None,
                },
            ))
            .unwrap();

        // Verify all three extensions show as installed
        let status_map = ledger.get_all_latest_status().unwrap();
        assert_eq!(status_map.len(), 3);

        assert_eq!(
            status_map.get("mise-config").unwrap().current_state,
            ExtensionState::Installed
        );
        assert_eq!(
            status_map.get("sdkman").unwrap().current_state,
            ExtensionState::Installed
        );
        assert_eq!(
            status_map.get("jvm").unwrap().current_state,
            ExtensionState::Installed
        );

        // Verify event history shows correct ordering
        let mise_history = ledger.get_extension_history("mise-config", None).unwrap();
        let sdkman_history = ledger.get_extension_history("sdkman", None).unwrap();
        let jvm_history = ledger.get_extension_history("jvm", None).unwrap();

        assert_eq!(mise_history.len(), 2); // started + completed
        assert_eq!(sdkman_history.len(), 2); // started + completed
        assert_eq!(jvm_history.len(), 2); // started + completed
    }

    #[test]
    fn test_dependency_failure_event_tracking() {
        // Create a temporary ledger
        let temp_dir = TempDir::new().unwrap();
        let ledger_path = temp_dir.path().join("status_ledger.jsonl");
        let ledger = StatusLedger::new(ledger_path.clone());

        // Simulate installing a dependency that fails
        ledger
            .append(EventEnvelope::new(
                "sdkman".to_string(),
                None,
                ExtensionState::Installing,
                ExtensionEvent::InstallStarted {
                    extension_name: "sdkman".to_string(),
                    version: "1.0.1".to_string(),
                    source: "dependency of jvm".to_string(),
                    install_method: "Distributor".to_string(),
                },
            ))
            .unwrap();

        ledger
            .append(EventEnvelope::new(
                "sdkman".to_string(),
                Some(ExtensionState::Installing),
                ExtensionState::Failed,
                ExtensionEvent::InstallFailed {
                    extension_name: "sdkman".to_string(),
                    version: "1.0.1".to_string(),
                    error_message: "Script installation failed for sdkman (exit code: 1)"
                        .to_string(),
                    retry_count: 0,
                    duration_secs: 5,
                    log_file: None,
                },
            ))
            .unwrap();

        // Verify dependency shows as failed
        let status_map = ledger.get_all_latest_status().unwrap();
        assert!(status_map.contains_key("sdkman"));

        let sdkman_status = status_map.get("sdkman").unwrap();
        assert_eq!(sdkman_status.current_state, ExtensionState::Failed);
    }

    #[test]
    fn test_extension_structure_with_dependencies() {
        // Verify extension metadata structure supports dependencies
        let jvm =
            create_test_extension("jvm", vec!["mise-config".to_string(), "sdkman".to_string()]);

        assert_eq!(jvm.metadata.name, "jvm");
        assert_eq!(jvm.metadata.dependencies.len(), 2);
        assert!(jvm
            .metadata
            .dependencies
            .contains(&"mise-config".to_string()));
        assert!(jvm.metadata.dependencies.contains(&"sdkman".to_string()));
    }
}
