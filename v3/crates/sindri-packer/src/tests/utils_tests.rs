//! Utility function tests

use crate::utils::*;

#[test]
fn test_sanitize_name() {
    assert_eq!(sanitize_name("My Dev Environment"), "my-dev-environment");
    assert_eq!(sanitize_name("test_name-123"), "test_name-123");
    assert_eq!(sanitize_name("---test---"), "test");
    assert_eq!(sanitize_name("Test@Name#Here"), "test-name-here");
    assert_eq!(sanitize_name("UPPERCASE"), "uppercase");
}

#[test]
fn test_generate_image_name() {
    let name = generate_image_name("sindri-dev");
    assert!(name.starts_with("sindri-dev-"));
    assert!(name.len() > 15); // prefix + dash + timestamp (14 chars)
}

#[test]
fn test_generate_build_id() {
    let id1 = generate_build_id();
    let id2 = generate_build_id();

    assert_eq!(id1.len(), 8);
    assert_eq!(id2.len(), 8);
    assert_ne!(id1, id2); // Should be unique
}

#[test]
fn test_config_hash() {
    use sindri_core::types::packer_config::BuildConfig;

    let config1 = BuildConfig {
        extensions: vec!["python".to_string()],
        ..Default::default()
    };

    let config2 = BuildConfig {
        extensions: vec!["python".to_string()],
        ..Default::default()
    };

    let config3 = BuildConfig {
        extensions: vec!["node".to_string()],
        ..Default::default()
    };

    let hash1 = config_hash(&config1);
    let hash2 = config_hash(&config2);
    let hash3 = config_hash(&config3);

    assert_eq!(hash1, hash2); // Same config should have same hash
    assert_ne!(hash1, hash3); // Different config should have different hash
    assert_eq!(hash1.len(), 8);
}

#[test]
fn test_parse_ami_id() {
    // Test with standard output
    let output1 = "amazon-ebs.sindri: AMI: ami-0123456789abcdef0";
    assert_eq!(
        parse_ami_id(output1),
        Some("ami-0123456789abcdef0".to_string())
    );

    // Test with multiple lines
    let output2 = r#"
==> amazon-ebs.sindri: Creating AMI...
    amazon-ebs.sindri: AMI: ami-0987654321fedcba0
==> Done
    "#;
    assert_eq!(
        parse_ami_id(output2),
        Some("ami-0987654321fedcba0".to_string())
    );

    // Test with no match
    let output3 = "No AMI here";
    assert_eq!(parse_ami_id(output3), None);
}

#[test]
fn test_parse_azure_image_id() {
    let output =
        "/subscriptions/sub-id/resourceGroups/rg/providers/Microsoft.Compute/images/my-image";
    assert!(parse_azure_image_id(output).is_some());

    let no_match = "No Azure image ID here";
    assert!(parse_azure_image_id(no_match).is_none());
}

#[test]
fn test_parse_oci_image_id() {
    let output = "Image OCID: ocid1.image.oc1.iad.aaaaaaaaexample123";
    let parsed = parse_oci_image_id(output);
    assert!(parsed.is_some());
    assert!(parsed.unwrap().starts_with("ocid1.image."));

    let no_match = "No OCI image here";
    assert!(parse_oci_image_id(no_match).is_none());
}

#[test]
fn test_parse_alicloud_image_id() {
    let output = "alicloud-ecs: Image ID: m-bp1234567890";
    let parsed = parse_alicloud_image_id(output);
    assert!(parsed.is_some());
    assert!(parsed.unwrap().starts_with("m-"));

    let no_match = "No Alibaba image here";
    assert!(parse_alicloud_image_id(no_match).is_none());
}
