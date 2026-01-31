//! Build lifecycle integration tests
//!
//! Tests the complete Packer build lifecycle including:
//! - Template generation
//! - Build validation
//! - Multi-cloud builds
//! - Error handling

mod common;

use common::*;

#[cfg(test)]
mod build_lifecycle {
    use super::*;

    #[test]
    fn test_mock_aws_provider_basic() {
        let provider = MockAwsProvider::new();

        // Add test images
        provider.add_image(MockImage::new("ami-123", "sindri-v3-ubuntu"));
        provider.add_image(MockImage::new("ami-456", "sindri-v3-debian"));

        // List all images
        let images = provider.list_images(None);
        assert_eq!(images.len(), 2);

        // Filter by name
        let ubuntu_images = provider.list_images(Some("ubuntu"));
        assert_eq!(ubuntu_images.len(), 1);
        assert_eq!(ubuntu_images[0].id, "ami-123");
    }

    #[test]
    fn test_mock_build_workflow() {
        let provider = MockAwsProvider::new();

        // Set up expected build result
        provider.set_build_result(
            "sindri-anthropic-dev",
            MockBuildResult::success("ami-newbuild", 300),
        );

        // Execute build
        let result = provider.build("sindri-anthropic-dev");

        assert_build_success(&result);
        assert_eq!(result.image_id, Some("ami-newbuild".to_string()));
        assert_build_duration(&result, 600);

        // Verify API was called
        assert!(provider.was_called("build"));
    }

    #[test]
    fn test_mock_build_failure_handling() {
        let provider = MockAwsProvider::new();

        // Set up expected failure
        provider.set_build_result(
            "failing-config",
            MockBuildResult::failure("Insufficient permissions", 30),
        );

        // Execute build
        let result = provider.build("failing-config");

        assert_build_failure(&result);
        assert!(result.error.unwrap().contains("permissions"));
    }

    #[test]
    fn test_mock_image_deletion() {
        let provider = MockAwsProvider::new();

        provider.add_image(MockImage::new("ami-to-delete", "old-image"));
        provider.add_image(MockImage::new("ami-keep", "current-image"));

        // Delete one image
        let deleted = provider.delete_image("ami-to-delete");
        assert!(deleted);

        // Verify only one image remains
        let images = provider.list_images(None);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].id, "ami-keep");

        // Verify delete was called
        assert!(provider.was_called("delete_image"));
    }

    #[test]
    fn test_mock_image_with_tags() {
        let image = MockImage::new("ami-tagged", "sindri-tagged")
            .with_tag("Project", "Sindri")
            .with_tag("Version", "3.0.0")
            .with_tag("Environment", "production");

        assert_image_tags(
            &image.tags,
            &[
                ("Project", "Sindri"),
                ("Version", "3.0.0"),
                ("Environment", "production"),
            ],
        );
    }

    #[test]
    fn test_mock_image_multi_region() {
        let image = MockImage::new("ami-multiregion", "sindri-global").with_regions(vec![
            "us-east-1",
            "us-west-2",
            "eu-west-1",
            "ap-southeast-1",
        ]);

        assert_eq!(image.regions.len(), 4);
        assert!(image.regions.contains(&"us-east-1".to_string()));
        assert!(image.regions.contains(&"ap-southeast-1".to_string()));
    }

    #[test]
    fn test_mock_template_renderer() {
        let renderer = MockTemplateRenderer::new();

        // Set up template
        renderer.set_template(
            "aws",
            r#"
packer {
  required_plugins {
    amazon = {
      source  = "github.com/hashicorp/amazon"
      version = "~> 1.3"
    }
  }
}

source "amazon-ebs" "sindri" {
  region        = "us-west-2"
  source_ami    = "ami-base123"
  instance_type = "t3.large"
}
"#,
        );

        // Render template
        let result = renderer.render("aws", &std::collections::HashMap::new());
        assert!(result.is_some());

        let template = result.unwrap();
        assert_template_contains(&template, "amazon-ebs");
        assert_template_contains(&template, "region");
        assert_has_builder(&template, "aws");
    }

    #[test]
    fn test_gcp_provider_mock() {
        let provider = MockGcpProvider::new();

        provider.add_image(MockImage::new("sindri-v3-image", "sindri-ubuntu-24"));

        let images = provider.list_images();
        assert_eq!(images.len(), 1);
        assert!(provider
            .get_api_calls()
            .contains(&"list_images()".to_string()));
    }

    #[test]
    fn test_azure_provider_mock() {
        let provider = MockAzureProvider::new();

        provider.add_image(MockImage::new(
            "/subscriptions/xxx/images/sindri-v3",
            "sindri-ubuntu",
        ));

        let images = provider.list_images();
        assert_eq!(images.len(), 1);
    }

    #[test]
    fn test_oci_provider_mock() {
        let provider = MockOciProvider::new();

        provider.add_image(MockImage::new("ocid1.image.oc1..xxx", "sindri-v3-ubuntu"));

        let images = provider.list_images();
        assert_eq!(images.len(), 1);
        assert!(provider
            .get_api_calls()
            .contains(&"list_images()".to_string()));
    }

    #[test]
    fn test_alibaba_provider_mock() {
        let provider = MockAlibabaProvider::new();

        provider.add_image(MockImage::new("m-uf6xxx", "sindri-v3-ubuntu"));

        let images = provider.list_images();
        assert_eq!(images.len(), 1);
        assert!(provider
            .get_api_calls()
            .contains(&"list_images()".to_string()));
    }

    #[test]
    fn test_template_validation_aws() {
        let template = r#"
packer {
  required_plugins {
    amazon = {
      source  = "github.com/hashicorp/amazon"
      version = "~> 1.3"
    }
  }
}

source "amazon-ebs" "sindri" {
  region        = "us-west-2"
  source_ami    = "ami-0abcdef1234567890"
  instance_type = "t3.large"
  ami_name      = "sindri-v3-{{timestamp}}"

  tags = {
    Name        = "Sindri V3"
    Environment = "production"
  }
}

build {
  sources = ["source.amazon-ebs.sindri"]
}
"#;

        assert_aws_template_valid(template);
        assert_valid_hcl2(template);
    }

    #[test]
    fn test_template_validation_gcp() {
        let template = r#"
source "googlecompute" "sindri" {
  project_id   = "sindri-project"
  zone         = "us-central1-a"
  source_image = "ubuntu-2404-noble-amd64-v20240101"
  image_name   = "sindri-v3-{{timestamp}}"
}

build {
  sources = ["source.googlecompute.sindri"]
}
"#;

        assert_gcp_template_valid(template);
        assert_valid_hcl2(template);
    }

    #[test]
    fn test_template_validation_oci() {
        let template = r#"
packer {
  required_plugins {
    oracle = {
      source  = "github.com/hashicorp/oracle"
      version = "~> 1.0"
    }
  }
}

source "oracle-oci" "sindri" {
  compartment_ocid    = "ocid1.compartment.oc1..aaaaaaaaexample"
  availability_domain = "Uocm:PHX-AD-1"
  base_image_ocid     = "ocid1.image.oc1.phx.aaaaaaaaexample"
  shape               = "VM.Standard.E4.Flex"
  image_name          = "sindri-v3-{{timestamp}}"

  shape_config {
    ocpus         = 4
    memory_in_gbs = 16
  }
}

build {
  sources = ["source.oracle-oci.sindri"]
}
"#;

        assert_oci_template_valid(template);
        assert_valid_hcl2(template);
    }

    #[test]
    fn test_template_validation_alibaba() {
        let template = r#"
packer {
  required_plugins {
    alicloud = {
      source  = "github.com/hashicorp/alicloud"
      version = "~> 1.1"
    }
  }
}

source "alicloud-ecs" "sindri" {
  region           = "cn-hangzhou"
  source_image     = "ubuntu_24_04_x64_20G_alibase_20240101.vhd"
  instance_type    = "ecs.g6.xlarge"
  image_name       = "sindri-v3-{{timestamp}}"
  internet_charge_type = "PayByTraffic"

  system_disk_mapping {
    disk_category = "cloud_essd"
    disk_size     = 40
  }
}

build {
  sources = ["source.alicloud-ecs.sindri"]
}
"#;

        assert_alibaba_template_valid(template);
        assert_valid_hcl2(template);
    }

    #[test]
    fn test_template_validation_azure() {
        let template = r#"
packer {
  required_plugins {
    azure = {
      source  = "github.com/hashicorp/azure"
      version = "~> 2.1"
    }
  }
}

source "azure-arm" "sindri" {
  subscription_id               = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
  resource_group_name           = "sindri-images-rg"
  managed_image_name            = "sindri-v3-{{timestamp}}"
  managed_image_resource_group_name = "sindri-images-rg"
  location                      = "westus2"
  vm_size                       = "Standard_D4s_v4"
  os_type                       = "Linux"

  image_publisher = "Canonical"
  image_offer     = "0001-com-ubuntu-server-noble"
  image_sku       = "24_04-lts"
}

build {
  sources = ["source.azure-arm.sindri"]
}
"#;

        assert_azure_template_valid(template);
        assert_valid_hcl2(template);
    }

    #[test]
    fn test_provider_api_call_tracking() {
        let provider = MockAwsProvider::new();

        // Make several API calls
        provider.list_images(None);
        provider.list_images(Some("sindri"));
        provider.build("test-config");
        provider.delete_image("ami-123");

        let calls = provider.get_api_calls();
        assert_eq!(calls.len(), 4);
        assert!(calls[0].contains("list_images"));
        assert!(calls[2].contains("build"));
        assert!(calls[3].contains("delete_image"));
    }

    #[test]
    fn test_provider_reset() {
        let provider = MockAwsProvider::new();

        provider.add_image(MockImage::new("ami-1", "test-1"));
        provider.list_images(None);

        // Reset
        provider.reset();

        // Should be empty
        let images = provider.list_images(None);
        assert!(images.is_empty());

        // API calls should also be reset (only the new list_images call)
        let calls = provider.get_api_calls();
        assert_eq!(calls.len(), 1);
    }
}
