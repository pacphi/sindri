//! AWS Packer provider tests

use crate::aws::AwsPackerProvider;
use crate::traits::PackerProvider;
use crate::utils;
use sindri_core::types::packer_config::{AwsConfig, BuildConfig, CloudProvider, PackerConfig};

#[test]
fn test_aws_provider_creation() {
    let provider = AwsPackerProvider::new();
    assert_eq!(provider.cloud_name(), "aws");
}

#[test]
fn test_aws_prerequisite_check() {
    let provider = AwsPackerProvider::new();
    let result = provider.check_cloud_prerequisites();
    assert!(result.is_ok());

    let status = result.unwrap();
    // Packer and AWS CLI may or may not be installed
    assert!(!status.hints.is_empty() || status.satisfied);
}

#[test]
fn test_aws_template_generation() {
    let provider = AwsPackerProvider::new();

    let config = PackerConfig {
        cloud: CloudProvider::Aws,
        image_name: "test-sindri".to_string(),
        description: Some("Test image".to_string()),
        build: BuildConfig {
            sindri_version: "3.0.0".to_string(),
            extensions: vec!["python".to_string(), "node".to_string()],
            profile: Some("anthropic-dev".to_string()),
            ..Default::default()
        },
        aws: Some(AwsConfig {
            region: "us-west-2".to_string(),
            instance_type: "t3.large".to_string(),
            volume_size: 80,
            volume_type: "gp3".to_string(),
            encrypt_boot: true,
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
fn test_parse_ami_id() {
    let output = r#"
==> amazon-ebs.sindri: Creating AMI from instance i-1234567890abcdef0
    amazon-ebs.sindri: AMI: ami-0123456789abcdef0
==> amazon-ebs.sindri: Waiting for AMI to become ready...
    "#;

    let ami_id = utils::parse_ami_id(output);
    assert_eq!(ami_id, Some("ami-0123456789abcdef0".to_string()));
}

#[test]
fn test_parse_ami_id_no_match() {
    let output = "No AMI found in output";
    let ami_id = utils::parse_ami_id(output);
    assert_eq!(ami_id, None);
}

#[tokio::test]
#[ignore] // Requires AWS credentials
async fn test_aws_list_images() {
    let provider = AwsPackerProvider::new();

    let config = PackerConfig {
        cloud: CloudProvider::Aws,
        image_name: "sindri".to_string(),
        aws: Some(AwsConfig::default()),
        ..Default::default()
    };

    let result = provider.list_images(&config).await;
    // Requires AWS credentials which are not available in test environment
    assert!(
        result.is_err(),
        "Expected list_images to fail without AWS credentials"
    );
}
