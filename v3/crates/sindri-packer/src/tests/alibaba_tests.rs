//! Alibaba Cloud Packer provider tests

use crate::alibaba::AlibabaPackerProvider;
use crate::traits::PackerProvider;
use crate::utils;
use sindri_core::types::packer_config::{AlibabaConfig, BuildConfig, CloudProvider, PackerConfig};

#[test]
fn test_alibaba_provider_creation() {
    let provider = AlibabaPackerProvider::new().unwrap();
    assert_eq!(provider.cloud_name(), "alibaba");
}

#[test]
fn test_alibaba_prerequisite_check() {
    let provider = AlibabaPackerProvider::new().unwrap();
    let result = provider.check_cloud_prerequisites();
    assert!(result.is_ok());

    let status = result.unwrap();
    assert!(!status.hints.is_empty() || status.satisfied);
}

#[test]
fn test_alibaba_template_generation() {
    let provider = AlibabaPackerProvider::new().unwrap();

    let config = PackerConfig {
        cloud: CloudProvider::Alibaba,
        image_name: "test-sindri".to_string(),
        build: BuildConfig {
            sindri_version: "3.0.0".to_string(),
            ..Default::default()
        },
        alibaba: Some(AlibabaConfig {
            region: "cn-hangzhou".to_string(),
            instance_type: "ecs.g6.xlarge".to_string(),
            system_disk_size_gb: 80,
            system_disk_category: "cloud_essd".to_string(),
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
fn test_parse_alicloud_image_id() {
    let output = r#"
==> alicloud-ecs.sindri: Creating custom image...
    alicloud-ecs.sindri: Image ID: m-bp1234567890abcdef
==> alicloud-ecs.sindri: Image created successfully
    "#;

    let image_id = utils::parse_alicloud_image_id(output);
    assert!(image_id.is_some());
    assert!(image_id.unwrap().starts_with("m-"));
}
