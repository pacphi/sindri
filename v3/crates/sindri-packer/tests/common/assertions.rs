//! Assertion helpers for Packer testing
//!
//! Provides specialized assertions for validating Packer builds,
//! template generation, and cloud provider operations.

use std::path::Path;

use super::mock_cloud::MockBuildResult;

/// Assert that a build was successful
pub fn assert_build_success(result: &MockBuildResult) {
    assert!(result.success, "Build failed: {:?}", result.error);
    assert!(
        result.image_id.is_some(),
        "Build succeeded but no image ID was returned"
    );
}

/// Assert that a build failed
pub fn assert_build_failure(result: &MockBuildResult) {
    assert!(
        !result.success,
        "Build unexpectedly succeeded with image ID: {:?}",
        result.image_id
    );
}

/// Assert that a build completed within expected duration
pub fn assert_build_duration(result: &MockBuildResult, max_seconds: u64) {
    assert!(
        result.duration_seconds <= max_seconds,
        "Build took {}s, exceeding expected {}s",
        result.duration_seconds,
        max_seconds
    );
}

/// Assert that a template contains expected content
pub fn assert_template_contains(template: &str, expected: &str) {
    assert!(
        template.contains(expected),
        "Template does not contain expected content.\nExpected: {}\nTemplate:\n{}",
        expected,
        template
    );
}

/// Assert that a template is valid HCL2 (basic syntax check)
pub fn assert_valid_hcl2(template: &str) {
    // Basic HCL2 structure checks
    assert!(
        template.contains("packer {") || template.contains("source \""),
        "Template does not appear to be valid HCL2"
    );
}

/// Assert that a template has the expected cloud provider builder
pub fn assert_has_builder(template: &str, cloud: &str) {
    let builder_pattern = match cloud {
        "aws" => "amazon-ebs",
        "azure" => "azure-arm",
        "gcp" => "googlecompute",
        "oci" => "oracle-oci",
        "alibaba" => "alicloud-ecs",
        _ => cloud,
    };

    assert!(
        template.contains(builder_pattern),
        "Template does not contain {} builder.\nTemplate:\n{}",
        builder_pattern,
        template
    );
}

/// Assert that a template has required AWS fields
pub fn assert_aws_template_valid(template: &str) {
    assert_has_builder(template, "aws");
    assert_template_contains(template, "region");
    assert_template_contains(template, "source_ami");
    assert_template_contains(template, "instance_type");
}

/// Assert that a template has required Azure fields
pub fn assert_azure_template_valid(template: &str) {
    assert_has_builder(template, "azure");
    assert_template_contains(template, "subscription_id");
    assert_template_contains(template, "resource_group_name");
}

/// Assert that a template has required GCP fields
pub fn assert_gcp_template_valid(template: &str) {
    assert_has_builder(template, "gcp");
    assert_template_contains(template, "project_id");
    assert_template_contains(template, "zone");
}

/// Assert that a template has required OCI (Oracle Cloud) fields
pub fn assert_oci_template_valid(template: &str) {
    assert_has_builder(template, "oci");
    assert_template_contains(template, "compartment_ocid");
    assert_template_contains(template, "availability_domain");
    assert_template_contains(template, "base_image_ocid");
}

/// Assert that a template has required Alibaba Cloud fields
pub fn assert_alibaba_template_valid(template: &str) {
    assert_has_builder(template, "alibaba");
    assert_template_contains(template, "region");
    assert_template_contains(template, "source_image");
    assert_template_contains(template, "instance_type");
}

/// Assert that an image has expected tags
pub fn assert_image_tags(
    tags: &std::collections::HashMap<String, String>,
    expected: &[(&str, &str)],
) {
    for (key, value) in expected {
        let actual = tags.get(*key);
        assert!(actual.is_some(), "Image missing expected tag: {}", key);
        assert_eq!(
            actual.unwrap(),
            *value,
            "Tag '{}' has wrong value. Expected: {}, Got: {}",
            key,
            value,
            actual.unwrap()
        );
    }
}

/// Assert that a file exists and is valid HCL2
#[allow(dead_code)]
pub fn assert_hcl2_file_valid(path: impl AsRef<Path>) {
    let path = path.as_ref();
    assert!(path.exists(), "HCL2 file does not exist: {:?}", path);

    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read HCL2 file {:?}: {}", path, e));

    assert_valid_hcl2(&content);
}

/// Assert that packer is installed
#[allow(dead_code)]
pub fn assert_packer_installed() {
    let output = std::process::Command::new("which").arg("packer").output();

    match output {
        Ok(o) => assert!(o.status.success(), "Packer is not installed or not in PATH"),
        Err(_) => panic!("Failed to check for packer installation"),
    }
}

/// Assert that a cloud CLI is installed
#[allow(dead_code)]
pub fn assert_cloud_cli_installed(cloud: &str) {
    let cmd = match cloud {
        "aws" => "aws",
        "azure" => "az",
        "gcp" => "gcloud",
        "oci" => "oci",
        "alibaba" => "aliyun",
        _ => cloud,
    };

    let output = std::process::Command::new("which").arg(cmd).output();

    match output {
        Ok(o) => {
            if !o.status.success() {
                eprintln!("Warning: {} CLI not installed", cloud);
            }
        }
        Err(_) => eprintln!("Warning: Could not check {} CLI installation", cloud),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_assert_build_success() {
        let result = MockBuildResult::success("ami-123", 60);
        assert_build_success(&result);
    }

    #[test]
    #[should_panic(expected = "Build failed")]
    fn test_assert_build_success_panics() {
        let result = MockBuildResult::failure("error", 10);
        assert_build_success(&result);
    }

    #[test]
    fn test_assert_build_failure() {
        let result = MockBuildResult::failure("error", 10);
        assert_build_failure(&result);
    }

    #[test]
    fn test_assert_template_contains() {
        let template = "packer { version = \"1.0\" }";
        assert_template_contains(template, "packer");
        assert_template_contains(template, "version");
    }

    #[test]
    fn test_assert_has_builder() {
        let aws_template = r#"source "amazon-ebs" "test" { }"#;
        assert_has_builder(aws_template, "aws");

        let gcp_template = r#"source "googlecompute" "test" { }"#;
        assert_has_builder(gcp_template, "gcp");

        let oci_template = r#"source "oracle-oci" "test" { }"#;
        assert_has_builder(oci_template, "oci");

        let alibaba_template = r#"source "alicloud-ecs" "test" { }"#;
        assert_has_builder(alibaba_template, "alibaba");

        let azure_template = r#"source "azure-arm" "test" { }"#;
        assert_has_builder(azure_template, "azure");
    }

    #[test]
    fn test_assert_image_tags() {
        let mut tags = HashMap::new();
        tags.insert("Name".to_string(), "sindri-vm".to_string());
        tags.insert("Environment".to_string(), "test".to_string());

        assert_image_tags(&tags, &[("Name", "sindri-vm"), ("Environment", "test")]);
    }

    #[test]
    fn test_assert_build_duration() {
        let result = MockBuildResult::success("ami-123", 60);
        assert_build_duration(&result, 120);
    }

    #[test]
    #[should_panic(expected = "exceeding expected")]
    fn test_assert_build_duration_exceeds() {
        let result = MockBuildResult::success("ami-123", 200);
        assert_build_duration(&result, 100);
    }

    #[test]
    fn test_assert_aws_template_valid() {
        let template = r#"
source "amazon-ebs" "test" {
  region        = "us-west-2"
  source_ami    = "ami-0123456789abcdef0"
  instance_type = "t3.large"
}
"#;
        assert_aws_template_valid(template);
    }

    #[test]
    fn test_assert_gcp_template_valid() {
        let template = r#"
source "googlecompute" "test" {
  project_id = "test-project"
  zone       = "us-central1-a"
}
"#;
        assert_gcp_template_valid(template);
    }

    #[test]
    fn test_assert_oci_template_valid() {
        let template = r#"
source "oracle-oci" "test" {
  compartment_ocid    = "ocid1.compartment.oc1..example"
  availability_domain = "Uocm:PHX-AD-1"
  base_image_ocid     = "ocid1.image.oc1.phx.example"
}
"#;
        assert_oci_template_valid(template);
    }

    #[test]
    fn test_assert_alibaba_template_valid() {
        let template = r#"
source "alicloud-ecs" "test" {
  region        = "cn-hangzhou"
  source_image  = "ubuntu_24_04_x64_20G_alibase.vhd"
  instance_type = "ecs.g6.xlarge"
}
"#;
        assert_alibaba_template_valid(template);
    }

    #[test]
    fn test_assert_azure_template_valid() {
        let template = r#"
source "azure-arm" "test" {
  subscription_id     = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
  resource_group_name = "test-rg"
}
"#;
        assert_azure_template_valid(template);
    }
}
