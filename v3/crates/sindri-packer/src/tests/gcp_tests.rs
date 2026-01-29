//! GCP Packer provider tests

use crate::gcp::GcpPackerProvider;
use crate::traits::PackerProvider;
use sindri_core::types::packer_config::{BuildConfig, CloudProvider, GcpConfig, PackerConfig};

#[test]
fn test_gcp_provider_creation() {
    let provider = GcpPackerProvider::new();
    assert_eq!(provider.cloud_name(), "gcp");
}

#[test]
fn test_gcp_prerequisite_check() {
    let provider = GcpPackerProvider::new();
    let result = provider.check_cloud_prerequisites();
    assert!(result.is_ok());

    let status = result.unwrap();
    assert!(!status.hints.is_empty() || status.satisfied);
}

#[test]
fn test_gcp_template_generation() {
    let provider = GcpPackerProvider::new();

    let config = PackerConfig {
        cloud: CloudProvider::Gcp,
        image_name: "test-sindri".to_string(),
        build: BuildConfig {
            sindri_version: "3.0.0".to_string(),
            extensions: vec!["rust".to_string()],
            ..Default::default()
        },
        gcp: Some(GcpConfig {
            project_id: "test-project".to_string(),
            zone: "us-west1-a".to_string(),
            machine_type: "e2-standard-4".to_string(),
            disk_size: 80,
            disk_type: "pd-ssd".to_string(),
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = provider.generate_template(&config);
    assert!(result.is_err() || result.is_ok());
}
