use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sindri_core::types::ExtensionState;

/// Extension lifecycle events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExtensionEvent {
    /// Extension installation started
    InstallStarted {
        extension_name: String,
        version: String,
        source: String,
        install_method: String,
    },

    /// Extension installation completed successfully
    InstallCompleted {
        extension_name: String,
        version: String,
        duration_secs: u64,
        components_installed: Vec<String>,
    },

    /// Extension installation failed
    InstallFailed {
        extension_name: String,
        version: String,
        error_message: String,
        retry_count: u32,
        duration_secs: u64,
    },

    /// Extension upgrade started
    UpgradeStarted {
        extension_name: String,
        from_version: String,
        to_version: String,
    },

    /// Extension upgrade completed
    UpgradeCompleted {
        extension_name: String,
        from_version: String,
        to_version: String,
        duration_secs: u64,
    },

    /// Extension upgrade failed
    UpgradeFailed {
        extension_name: String,
        from_version: String,
        to_version: String,
        error_message: String,
        duration_secs: u64,
    },

    /// Extension removal started
    RemoveStarted {
        extension_name: String,
        version: String,
    },

    /// Extension removal completed
    RemoveCompleted {
        extension_name: String,
        version: String,
        duration_secs: u64,
    },

    /// Extension removal failed
    RemoveFailed {
        extension_name: String,
        version: String,
        error_message: String,
        duration_secs: u64,
    },

    /// Extension marked as outdated
    OutdatedDetected {
        extension_name: String,
        current_version: String,
        latest_version: String,
    },

    /// Extension validation succeeded
    ValidationSucceeded {
        extension_name: String,
        version: String,
        validation_type: String,
    },

    /// Extension validation failed
    ValidationFailed {
        extension_name: String,
        version: String,
        validation_type: String,
        error_message: String,
    },
}

/// Event metadata envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// Unique event ID (UUID v4)
    pub event_id: String,

    /// Event timestamp (UTC)
    pub timestamp: DateTime<Utc>,

    /// Extension name (for indexing)
    pub extension_name: String,

    /// CLI version that published event
    pub cli_version: String,

    /// State before event
    pub state_before: Option<ExtensionState>,

    /// State after event
    pub state_after: ExtensionState,

    /// The actual event payload
    pub event: ExtensionEvent,
}

impl EventEnvelope {
    pub fn new(
        extension_name: String,
        state_before: Option<ExtensionState>,
        state_after: ExtensionState,
        event: ExtensionEvent,
    ) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            extension_name,
            cli_version: env!("CARGO_PKG_VERSION").to_string(),
            state_before,
            state_after,
            event,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_started_serialization() {
        let event = ExtensionEvent::InstallStarted {
            extension_name: "python".to_string(),
            version: "3.13.0".to_string(),
            source: "github:pacphi/sindri".to_string(),
            install_method: "Mise".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"install_started"#));
        assert!(json.contains(r#""extension_name":"python"#));

        let deserialized: ExtensionEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_install_completed_serialization() {
        let event = ExtensionEvent::InstallCompleted {
            extension_name: "python".to_string(),
            version: "3.13.0".to_string(),
            duration_secs: 150,
            components_installed: vec!["python".to_string(), "pip".to_string()],
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"install_completed"#));

        let deserialized: ExtensionEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_install_failed_serialization() {
        let event = ExtensionEvent::InstallFailed {
            extension_name: "kubectl".to_string(),
            version: "1.35.0".to_string(),
            error_message: "Network timeout".to_string(),
            retry_count: 0,
            duration_secs: 120,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains(r#""type":"install_failed"#));
        assert!(json.contains(r#""error_message":"Network timeout"#));

        let deserialized: ExtensionEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }

    #[test]
    fn test_event_envelope_creation() {
        let event = ExtensionEvent::InstallStarted {
            extension_name: "python".to_string(),
            version: "3.13.0".to_string(),
            source: "github:pacphi/sindri".to_string(),
            install_method: "Mise".to_string(),
        };

        let envelope = EventEnvelope::new(
            "python".to_string(),
            None,
            ExtensionState::Installing,
            event,
        );

        assert_eq!(envelope.extension_name, "python");
        assert_eq!(envelope.state_after, ExtensionState::Installing);
        assert!(envelope.state_before.is_none());
        assert!(!envelope.event_id.is_empty());
    }

    #[test]
    fn test_event_envelope_serialization() {
        let event = ExtensionEvent::InstallCompleted {
            extension_name: "python".to_string(),
            version: "3.13.0".to_string(),
            duration_secs: 150,
            components_installed: vec!["python".to_string()],
        };

        let envelope = EventEnvelope::new(
            "python".to_string(),
            Some(ExtensionState::Installing),
            ExtensionState::Installed,
            event,
        );

        let json = serde_json::to_string(&envelope).unwrap();
        assert!(json.contains(r#""extension_name":"python"#));
        assert!(json.contains(r#""state_after":"installed"#));

        let deserialized: EventEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(envelope.extension_name, deserialized.extension_name);
        assert_eq!(envelope.state_after, deserialized.state_after);
    }

    #[test]
    fn test_upgrade_events_serialization() {
        let started = ExtensionEvent::UpgradeStarted {
            extension_name: "python".to_string(),
            from_version: "3.12.0".to_string(),
            to_version: "3.13.0".to_string(),
        };

        let json = serde_json::to_string(&started).unwrap();
        assert!(json.contains(r#""type":"upgrade_started"#));
        assert!(json.contains(r#""from_version":"3.12.0"#));
        assert!(json.contains(r#""to_version":"3.13.0"#));

        let deserialized: ExtensionEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(started, deserialized);
    }

    #[test]
    fn test_validation_events_serialization() {
        let succeeded = ExtensionEvent::ValidationSucceeded {
            extension_name: "python".to_string(),
            version: "3.13.0".to_string(),
            validation_type: "post-install".to_string(),
        };

        let json = serde_json::to_string(&succeeded).unwrap();
        assert!(json.contains(r#""type":"validation_succeeded"#));
        assert!(json.contains(r#""validation_type":"post-install"#));

        let deserialized: ExtensionEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(succeeded, deserialized);
    }
}
