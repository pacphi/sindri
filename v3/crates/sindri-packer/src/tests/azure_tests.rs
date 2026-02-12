//! Azure Packer provider tests

use crate::azure::AzurePackerProvider;
use crate::traits::PackerProvider;
use crate::utils;
use sindri_core::types::packer_config::{AzureConfig, BuildConfig, CloudProvider, PackerConfig};

#[test]
fn test_azure_provider_creation() {
    let provider = AzurePackerProvider::new().unwrap();
    assert_eq!(provider.cloud_name(), "azure");
}

#[test]
fn test_azure_prerequisite_check() {
    let provider = AzurePackerProvider::new().unwrap();
    let result = provider.check_cloud_prerequisites();
    assert!(result.is_ok());

    let status = result.unwrap();
    assert!(!status.hints.is_empty() || status.satisfied);
}

#[test]
fn test_azure_template_generation() {
    let provider = AzurePackerProvider::new().unwrap();

    let config = PackerConfig {
        cloud: CloudProvider::Azure,
        image_name: "test-sindri".to_string(),
        description: Some("Test image".to_string()),
        build: BuildConfig {
            sindri_version: "3.0.0".to_string(),
            extensions: vec!["python".to_string()],
            ..Default::default()
        },
        azure: Some(AzureConfig {
            subscription_id: "test-subscription".to_string(),
            resource_group: "test-rg".to_string(),
            location: "westus2".to_string(),
            vm_size: "Standard_D4s_v4".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = provider.generate_template(&config);
    // Templates are not yet created (Phase 2), so this should error
    assert!(
        result.is_err(),
        "Expected template generation to fail until templates are implemented"
    );
}

#[test]
fn test_parse_azure_image_id() {
    let output = r#"
==> azure-arm.sindri: Creating managed image...
    azure-arm.sindri: Resource ID: /subscriptions/abc/resourceGroups/test-rg/providers/Microsoft.Compute/images/sindri-dev-20260125
==> azure-arm.sindri: Image created successfully
    "#;

    let image_id = utils::parse_azure_image_id(output);
    assert!(image_id.is_some());
    assert!(image_id
        .unwrap()
        .contains("/providers/Microsoft.Compute/images/"));
}
