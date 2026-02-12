//! OCI Packer provider tests

use crate::oci::OciPackerProvider;
use crate::traits::PackerProvider;
use crate::utils;
use sindri_core::types::packer_config::{BuildConfig, CloudProvider, OciConfig, PackerConfig};

#[test]
fn test_oci_provider_creation() {
    let provider = OciPackerProvider::new();
    assert_eq!(provider.cloud_name(), "oci");
}

#[test]
fn test_oci_prerequisite_check() {
    let provider = OciPackerProvider::new();
    let result = provider.check_cloud_prerequisites();
    assert!(result.is_ok());

    let status = result.unwrap();
    assert!(!status.hints.is_empty() || status.satisfied);
}

#[test]
fn test_oci_template_generation() {
    let provider = OciPackerProvider::new();

    let config = PackerConfig {
        cloud: CloudProvider::Oci,
        image_name: "test-sindri".to_string(),
        build: BuildConfig {
            sindri_version: "3.0.0".to_string(),
            ..Default::default()
        },
        oci: Some(OciConfig {
            compartment_ocid: "ocid1.compartment.oc1..test".to_string(),
            availability_domain: "US-ASHBURN-AD-1".to_string(),
            shape: "VM.Standard.E4.Flex".to_string(),
            subnet_ocid: "ocid1.subnet.oc1..test".to_string(),
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
fn test_parse_oci_image_id() {
    let output = r#"
==> oracle-oci.sindri: Creating custom image...
    oracle-oci.sindri: Image OCID: ocid1.image.oc1.iad.aaaaaaaaexampleocid
==> oracle-oci.sindri: Image created successfully
    "#;

    let image_id = utils::parse_oci_image_id(output);
    assert!(image_id.is_some());
    assert!(image_id.unwrap().starts_with("ocid1.image."));
}
