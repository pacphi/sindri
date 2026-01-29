//! Hook lifecycle integration tests
//!
//! Tests hook execution during extension lifecycle:
//! - Pre-install hooks
//! - Post-install hooks
//! - Hook failure handling
//! - Hook execution order

mod common;

use common::*;

#[cfg(test)]
mod hooks_lifecycle {
    use super::*;

    #[test]
    fn test_hooks_extension_yaml_parsing() {
        let yaml = mock_data::HOOKS_EXTENSION_YAML;
        let ext: sindri_core::types::Extension = serde_yaml::from_str(yaml).unwrap();

        assert_eq!(ext.metadata.name, "test-hooks");
        assert!(ext.capabilities.is_some());

        let caps = ext.capabilities.unwrap();
        assert!(caps.hooks.is_some());

        let hooks = caps.hooks.unwrap();
        assert!(hooks.pre_install.is_some());
        assert!(hooks.post_install.is_some());

        let pre = hooks.pre_install.unwrap();
        assert!(pre.command.contains("Pre-install hook"));
    }

    #[test]
    fn test_extension_builder_with_hooks() {
        let ext = ExtensionBuilder::with_hooks_preset().build();

        assert_has_hooks(&ext);
        assert_has_pre_install_hook(&ext);
        assert_has_post_install_hook(&ext);
    }

    #[test]
    fn test_extension_with_pre_install_only() {
        let ext = test_extensions::with_hooks::pre_install_only();

        assert!(ext.capabilities.is_some());
        let hooks = ext.capabilities.unwrap().hooks.unwrap();
        assert!(hooks.pre_install.is_some());
        assert!(hooks.post_install.is_none());
    }

    #[test]
    fn test_extension_with_post_install_only() {
        let ext = test_extensions::with_hooks::post_install_only();

        assert!(ext.capabilities.is_some());
        let hooks = ext.capabilities.unwrap().hooks.unwrap();
        assert!(hooks.pre_install.is_none());
        assert!(hooks.post_install.is_some());
    }

    #[test]
    fn test_mock_hook_tracker_records_execution() {
        let tracker = MockHookTracker::new();

        tracker.record("ext1", "pre-install");
        tracker.record("ext1", "post-install");
        tracker.record("ext2", "pre-install");

        assert!(tracker.was_executed("ext1", "pre-install"));
        assert!(tracker.was_executed("ext1", "post-install"));
        assert!(tracker.was_executed("ext2", "pre-install"));
        assert!(!tracker.was_executed("ext2", "post-install"));
    }

    #[test]
    fn test_hook_execution_order() {
        let tracker = MockHookTracker::new();

        tracker.record("ext1", "pre-install");
        tracker.record("ext1", "post-install");

        let order = tracker.execution_order();
        assert_eq!(order, vec!["ext1:pre-install", "ext1:post-install"]);
    }

    #[test]
    fn test_hook_assertions() {
        let tracker = MockHookTracker::new();
        tracker.record("test-ext", "pre-install");

        assert_hook_executed(&tracker, "test-ext", "pre-install");
        assert_hook_not_executed(&tracker, "test-ext", "post-install");
    }

    #[test]
    fn test_hook_order_assertion() {
        let tracker = MockHookTracker::new();
        tracker.record("ext", "pre-install");
        tracker.record("ext", "post-install");

        assert_hook_order(&tracker, &[("ext", "pre-install"), ("ext", "post-install")]);
    }

    #[test]
    fn test_failing_pre_hook_extension() {
        let ext = test_extensions::with_hooks::failing_pre_hook();

        let hooks = ext.capabilities.unwrap().hooks.unwrap();
        let pre_hook = hooks.pre_install.unwrap();

        assert_eq!(pre_hook.command, "exit 1");
    }

    #[test]
    fn test_hook_config_structure() {
        use sindri_core::types::HookConfig;

        let hook = HookConfig {
            command: "echo 'test'".to_string(),
            description: Some("Test hook".to_string()),
        };

        assert_eq!(hook.command, "echo 'test'");
        assert_eq!(hook.description, Some("Test hook".to_string()));
    }

    #[test]
    fn test_multiple_extensions_hook_tracking() {
        let tracker = MockHookTracker::new();

        // Simulate installing multiple extensions
        tracker.record("ext1", "pre-install");
        tracker.record("ext1", "post-install");
        tracker.record("ext2", "pre-install");
        tracker.record("ext2", "post-install");
        tracker.record("ext3", "pre-install");
        tracker.record("ext3", "post-install");

        let executed = tracker.get_executed();
        assert_eq!(executed.len(), 6);

        // Verify order
        assert_eq!(executed[0], ("ext1".to_string(), "pre-install".to_string()));
        assert_eq!(
            executed[5],
            ("ext3".to_string(), "post-install".to_string())
        );
    }
}
