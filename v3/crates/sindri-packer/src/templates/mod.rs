//! Template registry for Packer HCL2 templates
//!
//! This module provides the template rendering infrastructure for generating
//! cloud-specific Packer HCL2 templates from Sindri configurations.

use anyhow::{Context, Result};
use rust_embed::RustEmbed;
use std::collections::HashMap;
use std::path::PathBuf;
use tera::{Tera, Value};

use sindri_core::types::packer_config::{CloudProvider, PackerConfig};

/// Embedded HCL2 templates
#[derive(RustEmbed)]
#[folder = "src/templates/hcl/"]
struct HclTemplates;

/// Embedded provisioning script templates
#[derive(RustEmbed)]
#[folder = "src/templates/scripts/"]
struct ScriptTemplates;

/// Template registry for Packer HCL2 templates
pub struct TemplateRegistry {
    tera: Tera,
    output_dir: PathBuf,
}

impl TemplateRegistry {
    /// Create a new template registry
    pub fn new() -> Result<Self> {
        let mut tera = Tera::default();

        // Register custom filters
        tera.register_filter("join_quoted", join_quoted_filter);
        tera.register_filter("to_hcl_list", to_hcl_list_filter);
        tera.register_filter("to_hcl_map", to_hcl_map_filter);

        // Load embedded HCL templates
        for file in HclTemplates::iter() {
            if let Some(content) = HclTemplates::get(&file) {
                let template_name = file.to_string();
                let content_str = std::str::from_utf8(content.data.as_ref())
                    .context(format!("Invalid UTF-8 in template: {}", file))?;
                tera.add_raw_template(&template_name, content_str)
                    .context(format!("Failed to add template: {}", file))?;
            }
        }

        // Load embedded script templates
        for file in ScriptTemplates::iter() {
            if let Some(content) = ScriptTemplates::get(&file) {
                let template_name = format!("scripts/{}", file);
                let content_str = std::str::from_utf8(content.data.as_ref())
                    .context(format!("Invalid UTF-8 in script template: {}", file))?;
                tera.add_raw_template(&template_name, content_str)
                    .context(format!("Failed to add script template: {}", file))?;
            }
        }

        let output_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("sindri")
            .join("packer");

        Ok(Self { tera, output_dir })
    }

    /// Get the output directory for generated templates
    pub fn output_dir(&self) -> &PathBuf {
        &self.output_dir
    }

    /// Set a custom output directory
    pub fn set_output_dir(&mut self, dir: PathBuf) {
        self.output_dir = dir;
    }

    /// Render a template with the given context
    pub fn render(&self, template_name: &str, context: &tera::Context) -> Result<String> {
        self.tera
            .render(template_name, context)
            .context(format!("Failed to render template: {}", template_name))
    }

    /// Create a Tera context for AWS
    pub fn create_aws_context(&self, config: &PackerConfig) -> Result<tera::Context> {
        let aws = config
            .aws
            .as_ref()
            .context("AWS configuration required for AWS template")?;

        let mut context = tera::Context::new();

        // Basic image info
        context.insert("image_name", &config.image_name);
        context.insert(
            "description",
            &config.description.clone().unwrap_or_default(),
        );

        // Build configuration
        context.insert("sindri_version", &config.build.sindri_version);
        context.insert("extensions", &config.build.extensions);
        context.insert("profile", &config.build.profile.clone().unwrap_or_default());
        context.insert("ssh_timeout", &config.build.ssh_timeout);

        // AWS-specific
        context.insert("region", &aws.region);
        context.insert("instance_type", &aws.instance_type);
        context.insert("volume_size", &aws.volume_size);
        context.insert("volume_type", &aws.volume_type);
        context.insert("encrypt_boot", &aws.encrypt_boot);

        if let Some(vpc_id) = &aws.vpc_id {
            context.insert("vpc_id", vpc_id);
        }
        if let Some(subnet_id) = &aws.subnet_id {
            context.insert("subnet_id", subnet_id);
        }

        context.insert("ami_regions", &aws.ami_regions);
        context.insert("ami_users", &aws.ami_users);
        context.insert("ami_groups", &aws.ami_groups);

        // Security
        context.insert("cis_hardening", &config.build.security.cis_hardening);
        context.insert("openscap_scan", &config.build.security.openscap_scan);
        context.insert(
            "clean_sensitive_data",
            &config.build.security.clean_sensitive_data,
        );
        context.insert("remove_ssh_keys", &config.build.security.remove_ssh_keys);

        // Provisioning
        context.insert("environment", &config.build.environment);
        context.insert("file_uploads", &config.build.file_uploads);

        if let Some(playbook) = &config.build.ansible_playbook {
            context.insert("ansible_playbook", &playbook.to_string_lossy().to_string());
        }

        // Tags
        let mut tags = config.tags.clone();
        tags.insert("ManagedBy".to_string(), "sindri".to_string());
        tags.insert(
            "SindriVersion".to_string(),
            config.build.sindri_version.clone(),
        );
        context.insert("tags", &tags);

        // GitHub repo for downloads (placeholder)
        context.insert("github_repo", "pacphi/sindri");

        // Checksum types for post-processor
        context.insert("checksum_types", &vec!["sha256"]);

        Ok(context)
    }

    /// Create a Tera context for Azure
    pub fn create_azure_context(&self, config: &PackerConfig) -> Result<tera::Context> {
        let azure = config
            .azure
            .as_ref()
            .context("Azure configuration required for Azure template")?;

        let mut context = tera::Context::new();

        // Basic image info
        context.insert("image_name", &config.image_name);
        context.insert(
            "description",
            &config.description.clone().unwrap_or_default(),
        );

        // Build configuration
        context.insert("sindri_version", &config.build.sindri_version);
        context.insert("extensions", &config.build.extensions);
        context.insert("profile", &config.build.profile.clone().unwrap_or_default());
        context.insert("ssh_timeout", &config.build.ssh_timeout);

        // Azure-specific
        context.insert("subscription_id", &azure.subscription_id);
        context.insert("resource_group", &azure.resource_group);
        context.insert("location", &azure.location);
        context.insert("vm_size", &azure.vm_size);
        context.insert("os_disk_size_gb", &azure.os_disk_size_gb);
        context.insert("storage_account_type", &azure.storage_account_type);

        if let Some(gallery) = &azure.gallery {
            context.insert("gallery_name", &gallery.gallery_name);
            context.insert("gallery_image_name", &gallery.image_name);
            context.insert("gallery_image_version", &gallery.image_version);
            context.insert("replication_regions", &gallery.replication_regions);
        }

        // Security
        context.insert("cis_hardening", &config.build.security.cis_hardening);
        context.insert(
            "clean_sensitive_data",
            &config.build.security.clean_sensitive_data,
        );
        context.insert("remove_ssh_keys", &config.build.security.remove_ssh_keys);

        // Provisioning
        context.insert("environment", &config.build.environment);
        context.insert("file_uploads", &config.build.file_uploads);

        // Tags
        let mut tags = config.tags.clone();
        tags.insert("ManagedBy".to_string(), "sindri".to_string());
        context.insert("tags", &tags);

        context.insert("github_repo", "pacphi/sindri");

        Ok(context)
    }

    /// Create a Tera context for GCP
    pub fn create_gcp_context(&self, config: &PackerConfig) -> Result<tera::Context> {
        let gcp = config
            .gcp
            .as_ref()
            .context("GCP configuration required for GCP template")?;

        let mut context = tera::Context::new();

        // Basic image info
        context.insert("image_name", &config.image_name);
        context.insert(
            "description",
            &config.description.clone().unwrap_or_default(),
        );

        // Build configuration
        context.insert("sindri_version", &config.build.sindri_version);
        context.insert("extensions", &config.build.extensions);
        context.insert("profile", &config.build.profile.clone().unwrap_or_default());
        context.insert("ssh_timeout", &config.build.ssh_timeout);

        // GCP-specific
        context.insert("project_id", &gcp.project_id);
        context.insert("zone", &gcp.zone);
        context.insert("machine_type", &gcp.machine_type);
        context.insert("disk_size", &gcp.disk_size);
        context.insert("disk_type", &gcp.disk_type);
        context.insert("enable_secure_boot", &gcp.enable_secure_boot);

        if let Some(family) = &gcp.image_family {
            context.insert("image_family", family);
        }
        if let Some(network) = &gcp.network {
            context.insert("network", network);
        }
        if let Some(subnetwork) = &gcp.subnetwork {
            context.insert("subnetwork", subnetwork);
        }

        // Security
        context.insert("cis_hardening", &config.build.security.cis_hardening);
        context.insert(
            "clean_sensitive_data",
            &config.build.security.clean_sensitive_data,
        );
        context.insert("remove_ssh_keys", &config.build.security.remove_ssh_keys);

        // Provisioning
        context.insert("environment", &config.build.environment);
        context.insert("file_uploads", &config.build.file_uploads);

        // Labels (GCP uses labels instead of tags)
        let mut labels = config.tags.clone();
        labels.insert("managed-by".to_string(), "sindri".to_string());
        context.insert("labels", &labels);

        context.insert("github_repo", "pacphi/sindri");

        Ok(context)
    }

    /// Create a Tera context for OCI
    pub fn create_oci_context(&self, config: &PackerConfig) -> Result<tera::Context> {
        let oci = config
            .oci
            .as_ref()
            .context("OCI configuration required for OCI template")?;

        let mut context = tera::Context::new();

        // Basic image info
        context.insert("image_name", &config.image_name);
        context.insert(
            "description",
            &config.description.clone().unwrap_or_default(),
        );

        // Build configuration
        context.insert("sindri_version", &config.build.sindri_version);
        context.insert("extensions", &config.build.extensions);
        context.insert("profile", &config.build.profile.clone().unwrap_or_default());
        context.insert("ssh_timeout", &config.build.ssh_timeout);

        // OCI-specific
        context.insert("compartment_ocid", &oci.compartment_ocid);
        context.insert("availability_domain", &oci.availability_domain);
        context.insert("shape", &oci.shape);
        context.insert("subnet_ocid", &oci.subnet_ocid);
        context.insert("boot_volume_size_gb", &oci.boot_volume_size_gb);

        if let Some(shape_config) = &oci.shape_config {
            context.insert("ocpus", &shape_config.ocpus);
            context.insert("memory_in_gbs", &shape_config.memory_in_gbs);
        }

        // Security
        context.insert("cis_hardening", &config.build.security.cis_hardening);
        context.insert(
            "clean_sensitive_data",
            &config.build.security.clean_sensitive_data,
        );
        context.insert("remove_ssh_keys", &config.build.security.remove_ssh_keys);

        // Provisioning
        context.insert("environment", &config.build.environment);
        context.insert("file_uploads", &config.build.file_uploads);

        // Freeform tags
        let mut freeform_tags = config.tags.clone();
        freeform_tags.insert("ManagedBy".to_string(), "sindri".to_string());
        context.insert("freeform_tags", &freeform_tags);

        context.insert("github_repo", "pacphi/sindri");

        Ok(context)
    }

    /// Create a Tera context for Alibaba
    pub fn create_alibaba_context(&self, config: &PackerConfig) -> Result<tera::Context> {
        let alibaba = config
            .alibaba
            .as_ref()
            .context("Alibaba configuration required for Alibaba template")?;

        let mut context = tera::Context::new();

        // Basic image info
        context.insert("image_name", &config.image_name);
        context.insert(
            "description",
            &config.description.clone().unwrap_or_default(),
        );

        // Build configuration
        context.insert("sindri_version", &config.build.sindri_version);
        context.insert("extensions", &config.build.extensions);
        context.insert("profile", &config.build.profile.clone().unwrap_or_default());
        context.insert("ssh_timeout", &config.build.ssh_timeout);

        // Alibaba-specific
        context.insert("region", &alibaba.region);
        context.insert("instance_type", &alibaba.instance_type);
        context.insert("system_disk_size_gb", &alibaba.system_disk_size_gb);
        context.insert("system_disk_category", &alibaba.system_disk_category);

        if let Some(vswitch_id) = &alibaba.vswitch_id {
            context.insert("vswitch_id", vswitch_id);
        }

        // Security
        context.insert("cis_hardening", &config.build.security.cis_hardening);
        context.insert(
            "clean_sensitive_data",
            &config.build.security.clean_sensitive_data,
        );
        context.insert("remove_ssh_keys", &config.build.security.remove_ssh_keys);

        // Provisioning
        context.insert("environment", &config.build.environment);
        context.insert("file_uploads", &config.build.file_uploads);

        // Tags
        let mut tags = config.tags.clone();
        tags.insert("ManagedBy".to_string(), "sindri".to_string());
        context.insert("tags", &tags);

        context.insert("github_repo", "pacphi/sindri");

        Ok(context)
    }

    /// Create a context for the specified cloud
    pub fn create_context(&self, config: &PackerConfig) -> Result<tera::Context> {
        match config.cloud {
            CloudProvider::Aws => self.create_aws_context(config),
            CloudProvider::Azure => self.create_azure_context(config),
            CloudProvider::Gcp => self.create_gcp_context(config),
            CloudProvider::Oci => self.create_oci_context(config),
            CloudProvider::Alibaba => self.create_alibaba_context(config),
        }
    }

    /// Get the template name for the specified cloud
    pub fn template_name(cloud: CloudProvider) -> &'static str {
        match cloud {
            CloudProvider::Aws => "aws.pkr.hcl.tera",
            CloudProvider::Azure => "azure.pkr.hcl.tera",
            CloudProvider::Gcp => "gcp.pkr.hcl.tera",
            CloudProvider::Oci => "oci.pkr.hcl.tera",
            CloudProvider::Alibaba => "alibaba.pkr.hcl.tera",
        }
    }

    /// Render provisioning scripts
    pub fn render_scripts(&self, config: &PackerConfig) -> Result<HashMap<String, String>> {
        let context = self.create_context(config)?;
        let mut scripts = HashMap::new();

        let script_names = ["init.sh", "install-sindri.sh", "cleanup.sh"];

        for name in script_names {
            let template_name = format!("scripts/{}.tera", name);
            if self.tera.get_template_names().any(|n| n == template_name) {
                let content = self.render(&template_name, &context)?;
                scripts.insert(name.to_string(), content);
            }
        }

        // Add security hardening script if enabled
        if config.build.security.cis_hardening {
            let template_name = "scripts/security-hardening.sh.tera";
            if self.tera.get_template_names().any(|n| n == template_name) {
                let content = self.render(template_name, &context)?;
                scripts.insert("security-hardening.sh".to_string(), content);
            }
        }

        Ok(scripts)
    }
}

/// Custom filter to join strings with quotes
fn join_quoted_filter(value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
    let arr = value
        .as_array()
        .ok_or_else(|| tera::Error::msg("Expected array"))?;
    let sep = args.get("sep").and_then(|v| v.as_str()).unwrap_or(", ");

    let quoted: Vec<String> = arr
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| format!("\"{}\"", s))
        .collect();

    Ok(Value::String(quoted.join(sep)))
}

/// Custom filter to convert array to HCL list syntax
fn to_hcl_list_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    let arr = value
        .as_array()
        .ok_or_else(|| tera::Error::msg("Expected array"))?;

    if arr.is_empty() {
        return Ok(Value::String("[]".to_string()));
    }

    let items: Vec<String> = arr
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| format!("\"{}\"", s))
        .collect();

    Ok(Value::String(format!("[{}]", items.join(", "))))
}

/// Custom filter to convert HashMap to HCL map syntax
fn to_hcl_map_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    let obj = value
        .as_object()
        .ok_or_else(|| tera::Error::msg("Expected object"))?;

    if obj.is_empty() {
        return Ok(Value::String("{}".to_string()));
    }

    let items: Vec<String> = obj
        .iter()
        .map(|(k, v)| {
            let v_str = match v {
                Value::String(s) => format!("\"{}\"", s),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => format!("\"{}\"", v),
            };
            format!("    {} = {}", k, v_str)
        })
        .collect();

    Ok(Value::String(format!("{{\n{}\n  }}", items.join("\n"))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_registry_creation() {
        let registry = TemplateRegistry::new();
        assert!(registry.is_ok());
    }

    #[test]
    fn test_template_names() {
        assert_eq!(
            TemplateRegistry::template_name(CloudProvider::Aws),
            "aws.pkr.hcl.tera"
        );
        assert_eq!(
            TemplateRegistry::template_name(CloudProvider::Azure),
            "azure.pkr.hcl.tera"
        );
        assert_eq!(
            TemplateRegistry::template_name(CloudProvider::Gcp),
            "gcp.pkr.hcl.tera"
        );
    }
}
