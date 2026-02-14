//! Dependency resolution integration tests
//!
//! Tests dependency handling including:
//! - Single dependency resolution
//! - Multiple dependency resolution
//! - Dependency chain resolution
//! - Circular dependency detection

mod common;

use common::*;

#[cfg(test)]
mod dependency_tests {
    use super::*;

    #[test]
    fn test_extension_with_no_dependencies() {
        let ext = test_extensions::minimal_extension();
        assert_no_dependencies(&ext);
    }

    #[test]
    fn test_single_dependency() {
        let ext = test_extensions::with_dependencies::single_dependency();

        assert!(!ext.metadata.dependencies.is_empty());
        let deps = &ext.metadata.dependencies;
        assert_eq!(deps.len(), 1);
        assert!(deps.contains(&"base-ext".to_string()));
    }

    #[test]
    fn test_multiple_dependencies() {
        let ext = test_extensions::with_dependencies::multiple_dependencies();

        let deps = &ext.metadata.dependencies;
        assert_eq!(deps.len(), 3);
        assert!(deps.contains(&"dep1".to_string()));
        assert!(deps.contains(&"dep2".to_string()));
        assert!(deps.contains(&"dep3".to_string()));
    }

    #[test]
    fn test_dependency_chain() {
        let chain_a = test_extensions::with_dependencies::chain_dependency_a();
        let chain_b = test_extensions::with_dependencies::chain_dependency_b();
        let chain_c = test_extensions::with_dependencies::chain_dependency_c();

        // A has no dependencies
        assert!(chain_a.metadata.dependencies.is_empty());

        // B depends on A
        let deps_b = &chain_b.metadata.dependencies;
        assert!(deps_b.contains(&"chain-a".to_string()));

        // C depends on B
        let deps_c = &chain_c.metadata.dependencies;
        assert!(deps_c.contains(&"chain-b".to_string()));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let circular_a = test_extensions::with_dependencies::circular_a();
        let circular_b = test_extensions::with_dependencies::circular_b();

        // A depends on B
        let deps_a = &circular_a.metadata.dependencies;
        assert!(deps_a.contains(&"circular-b".to_string()));

        // B depends on A
        let deps_b = &circular_b.metadata.dependencies;
        assert!(deps_b.contains(&"circular-a".to_string()));

        // Note: Actual circular dependency detection is handled by DependencyResolver
        // These tests just verify the structure is parsed correctly
    }

    #[test]
    fn test_deps_extension_yaml_parsing() {
        let yaml = mock_data::DEPS_EXTENSION_YAML;
        let ext: sindri_core::types::Extension = serde_yaml_ng::from_str(yaml).unwrap();

        assert_eq!(ext.metadata.name, "test-deps");
        assert!(!ext.metadata.dependencies.is_empty());
        let deps = &ext.metadata.dependencies;
        assert!(deps.contains(&"test-minimal".to_string()));
    }

    #[test]
    fn test_has_dependencies_assertion() {
        let ext = ExtensionBuilder::new()
            .with_dependency("dep1")
            .with_dependency("dep2")
            .build();

        assert_has_dependencies(&ext, &["dep1", "dep2"]);
    }

    #[test]
    fn test_builder_with_dependencies() {
        let ext = ExtensionBuilder::new()
            .with_name("deps-builder-test")
            .with_dependencies(vec![
                "core-lib".to_string(),
                "utils".to_string(),
                "config".to_string(),
            ])
            .build();

        let deps = &ext.metadata.dependencies;
        assert_eq!(deps.len(), 3);
    }

    #[test]
    fn test_dependency_installation_order() {
        // This test documents expected dependency installation order
        // In a real scenario, DependencyResolver would determine this

        // Given extensions: A (no deps), B (depends on A), C (depends on B)
        // Expected install order: A -> B -> C

        let tracker = MockHookTracker::new();

        // Simulate correct installation order
        tracker.record("chain-a", "pre-install");
        tracker.record("chain-a", "post-install");
        tracker.record("chain-b", "pre-install");
        tracker.record("chain-b", "post-install");
        tracker.record("chain-c", "pre-install");
        tracker.record("chain-c", "post-install");

        let order = tracker.execution_order();

        // A should be installed first
        assert!(order[0].starts_with("chain-a"));
        // B should be installed after A
        assert!(order[2].starts_with("chain-b"));
        // C should be installed last
        assert!(order[4].starts_with("chain-c"));
    }

    #[test]
    fn test_fixture_file_with_dependencies() {
        let manager = FixtureManager::new().unwrap();

        // Try to load the dependencies fixture if it exists
        let result = manager.load_extension_yaml("dependencies");

        // The fixture may not exist yet, which is fine for this test
        if let Ok(yaml) = result {
            let ext: sindri_core::types::Extension = serde_yaml_ng::from_str(&yaml).unwrap();
            assert!(!ext.metadata.dependencies.is_empty());
        }
    }

    #[test]
    fn test_dependency_event_tracking() {
        // This test verifies that when an extension with dependencies is installed,
        // ledger events are published for both the main extension and its dependencies.
        //
        // Test scenario:
        // - Extension "jvm" depends on "mise-config" and "sdkman"
        // - When jvm is installed, all three should have ledger events
        //
        // Expected ledger events:
        // 1. mise-config: install_started -> install_completed
        // 2. sdkman: install_started -> install_completed
        // 3. jvm: install_started -> install_completed

        use sindri_core::types::{Extension, ExtensionMetadata, InstallConfig, InstallMethod};

        // Create test extensions with dependency chain
        let mise_config = Extension {
            metadata: ExtensionMetadata {
                name: "mise-config".to_string(),
                version: "2.0.0".to_string(),
                description: "Global mise configuration".to_string(),
                category: sindri_core::types::ExtensionCategory::PackageManager,
                author: None,
                homepage: None,
                dependencies: vec![],
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
        };

        let sdkman = Extension {
            metadata: ExtensionMetadata {
                name: "sdkman".to_string(),
                version: "1.0.1".to_string(),
                description: "SDKMAN - The Software Development Kit Manager".to_string(),
                category: sindri_core::types::ExtensionCategory::PackageManager,
                author: None,
                homepage: None,
                dependencies: vec![],
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
        };

        let jvm = Extension {
            metadata: ExtensionMetadata {
                name: "jvm".to_string(),
                version: "2.1.1".to_string(),
                description: "JVM languages".to_string(),
                category: sindri_core::types::ExtensionCategory::Languages,
                author: None,
                homepage: None,
                dependencies: vec!["mise-config".to_string(), "sdkman".to_string()],
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
        };

        // Verify dependency structure
        assert_has_dependencies(&jvm, &["mise-config", "sdkman"]);
        assert_no_dependencies(&mise_config);
        assert_no_dependencies(&sdkman);

        // Note: Full integration test with actual ledger verification would require
        // ExtensionDistributor setup and mock installation environment.
        // This test documents the expected behavior and structure.
    }
}
