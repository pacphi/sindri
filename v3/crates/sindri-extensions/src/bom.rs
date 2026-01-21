//! Bill of Materials (BOM) management
//!
//! Tracks installed software components, versions, and dependencies for auditing and compliance.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sindri_core::types::InstallManifest;
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info};

/// Bill of Materials format
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BomFormat {
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// CycloneDX XML format
    CycloneDx,
    /// SPDX format
    Spdx,
}

impl Default for BomFormat {
    fn default() -> Self {
        Self::Json
    }
}

/// Bill of Materials document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillOfMaterials {
    /// BOM schema version
    pub schema_version: String,

    /// Sindri CLI version
    pub cli_version: String,

    /// Generation timestamp
    pub generated_at: DateTime<Utc>,

    /// Sindri configuration name
    pub config_name: String,

    /// Extensions and their components
    pub extensions: Vec<ExtensionBom>,

    /// System-level components
    pub system_components: Vec<Component>,

    /// Total component count
    pub total_components: usize,
}

/// Extension BOM entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionBom {
    /// Extension name
    pub name: String,

    /// Extension version
    pub version: String,

    /// Extension category
    pub category: String,

    /// Installation method
    pub install_method: String,

    /// Installed at timestamp
    pub installed_at: Option<DateTime<Utc>>,

    /// Components provided by this extension
    pub components: Vec<Component>,

    /// Dependencies on other extensions
    pub dependencies: Vec<String>,
}

/// Software component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    /// Component name
    pub name: String,

    /// Component version
    pub version: String,

    /// Component type (tool, library, runtime, etc.)
    pub component_type: ComponentType,

    /// License (if known)
    pub license: Option<String>,

    /// Source URL or package manager reference
    pub source: Option<String>,

    /// Installation path
    pub install_path: Option<String>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Type of software component
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ComponentType {
    /// CLI tool or binary
    Tool,
    /// Programming language runtime
    Runtime,
    /// Library or framework
    Library,
    /// System package
    Package,
    /// Container image
    Image,
    /// Configuration file
    Config,
    /// Other type
    Other,
}

/// BOM generator
pub struct BomGenerator {
    cli_version: String,
    config_name: String,
}

impl BomGenerator {
    /// Create a new BOM generator
    pub fn new(cli_version: String, config_name: String) -> Self {
        Self {
            cli_version,
            config_name,
        }
    }

    /// Generate BOM from installed extensions
    pub fn generate(
        &self,
        extensions: Vec<ExtensionBom>,
        system_components: Vec<Component>,
    ) -> BillOfMaterials {
        let total_components: usize =
            extensions.iter().map(|e| e.components.len()).sum::<usize>() + system_components.len();

        info!(
            "Generated BOM with {} extensions and {} total components",
            extensions.len(),
            total_components
        );

        BillOfMaterials {
            schema_version: "1.0".to_string(),
            cli_version: self.cli_version.clone(),
            generated_at: Utc::now(),
            config_name: self.config_name.clone(),
            extensions,
            system_components,
            total_components,
        }
    }

    /// Generate BOM from manifest and extension definitions
    pub fn generate_from_manifest(
        &self,
        manifest: &InstallManifest,
        registry: &crate::registry::ExtensionRegistry,
    ) -> Result<BillOfMaterials> {
        let mut extension_boms = Vec::new();

        for (name, installed) in &manifest.extensions {
            // Get extension definition
            let ext = registry
                .get_extension(name)
                .ok_or_else(|| anyhow!("Extension not found in registry: {}", name))?;

            // Extract components from extension
            let components = self.extract_components(ext)?;

            let entry = registry
                .get_entry(name)
                .ok_or_else(|| anyhow!("Extension entry not found: {}", name))?;

            extension_boms.push(ExtensionBom {
                name: name.to_string(),
                version: installed.version.clone(),
                category: format!("{:?}", ext.metadata.category).to_lowercase(),
                install_method: format!("{:?}", ext.install.method).to_lowercase(),
                installed_at: Some(installed.installed_at),
                components,
                dependencies: entry.dependencies.clone(),
            });
        }

        // Sort by name
        extension_boms.sort_by(|a, b| a.name.cmp(&b.name));

        // Add system components (mise, docker, etc.)
        let system_components = self.detect_system_components();

        Ok(self.generate(extension_boms, system_components))
    }

    /// Extract components from extension definition
    fn extract_components(
        &self,
        extension: &sindri_core::types::Extension,
    ) -> Result<Vec<Component>> {
        let mut components = Vec::new();

        // Extract from validation commands (these are the installed tools)
        for cmd in &extension.validate.commands {
            components.push(Component {
                name: cmd.name.clone(),
                version: "detected".to_string(), // Would need to actually run command to detect
                component_type: ComponentType::Tool,
                license: None,
                source: None,
                install_path: None,
                metadata: HashMap::new(),
            });
        }

        // Extract from BOM config if present (using the BomConfig.tools field)
        if let Some(bom_config) = &extension.bom {
            for tool in &bom_config.tools {
                components.push(Component {
                    name: tool.name.clone(),
                    version: tool
                        .version
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string()),
                    component_type: Self::map_tool_type(&tool.r#type),
                    license: tool.license.clone(),
                    source: tool.homepage.clone().or_else(|| tool.download_url.clone()),
                    install_path: None,
                    metadata: {
                        let mut meta = HashMap::new();
                        if let Some(purl) = &tool.purl {
                            meta.insert("purl".to_string(), purl.clone());
                        }
                        if let Some(cpe) = &tool.cpe {
                            meta.insert("cpe".to_string(), cpe.clone());
                        }
                        meta.insert("source_method".to_string(), format!("{:?}", tool.source));
                        meta
                    },
                });
            }
        }

        Ok(components)
    }

    /// Map BOM tool type to component type
    fn map_tool_type(tool_type: &Option<sindri_core::types::BomToolType>) -> ComponentType {
        match tool_type {
            Some(sindri_core::types::BomToolType::Runtime) => ComponentType::Runtime,
            Some(sindri_core::types::BomToolType::Compiler) => ComponentType::Tool,
            Some(sindri_core::types::BomToolType::PackageManager) => ComponentType::Tool,
            Some(sindri_core::types::BomToolType::CliTool) => ComponentType::Tool,
            Some(sindri_core::types::BomToolType::Library) => ComponentType::Library,
            Some(sindri_core::types::BomToolType::Framework) => ComponentType::Library,
            Some(sindri_core::types::BomToolType::Database) => ComponentType::Package,
            Some(sindri_core::types::BomToolType::Server) => ComponentType::Package,
            Some(sindri_core::types::BomToolType::Utility) => ComponentType::Tool,
            Some(sindri_core::types::BomToolType::Application) => ComponentType::Tool,
            None => ComponentType::Other,
        }
    }

    /// Detect system-level components
    fn detect_system_components(&self) -> Vec<Component> {
        vec![Component {
            name: "sindri-cli".to_string(),
            version: self.cli_version.clone(),
            component_type: ComponentType::Tool,
            license: Some("MIT".to_string()),
            source: Some("github:pacphi/sindri".to_string()),
            install_path: None,
            metadata: HashMap::new(),
        }]
    }

    /// Write BOM to file in specified format
    pub fn write_bom(&self, bom: &BillOfMaterials, path: &Path, format: BomFormat) -> Result<()> {
        let content = match format {
            BomFormat::Json => serde_json::to_string_pretty(bom)?,
            BomFormat::Yaml => serde_yaml::to_string(bom)?,
            BomFormat::CycloneDx => self.export_cyclonedx(bom)?,
            BomFormat::Spdx => self.export_spdx(bom)?,
        };

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, content)?;
        debug!("Wrote BOM to {:?} in {:?} format", path, format);
        Ok(())
    }

    /// Export to CycloneDX format (JSON)
    fn export_cyclonedx(&self, bom: &BillOfMaterials) -> Result<String> {
        #[derive(Serialize)]
        struct CycloneDxBom {
            #[serde(rename = "bomFormat")]
            bom_format: String,
            #[serde(rename = "specVersion")]
            spec_version: String,
            version: u32,
            metadata: CycloneDxMetadata,
            components: Vec<CycloneDxComponent>,
        }

        #[derive(Serialize)]
        struct CycloneDxMetadata {
            timestamp: String,
            tools: Vec<CycloneDxTool>,
        }

        #[derive(Serialize)]
        struct CycloneDxTool {
            name: String,
            version: String,
        }

        #[derive(Serialize)]
        struct CycloneDxComponent {
            #[serde(rename = "type")]
            component_type: String,
            name: String,
            version: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            purl: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            licenses: Option<Vec<CycloneDxLicense>>,
        }

        #[derive(Serialize)]
        struct CycloneDxLicense {
            license: CycloneDxLicenseId,
        }

        #[derive(Serialize)]
        struct CycloneDxLicenseId {
            id: String,
        }

        let mut components = Vec::new();

        // Add system components
        for comp in &bom.system_components {
            components.push(CycloneDxComponent {
                component_type: Self::component_type_to_cyclonedx(&comp.component_type),
                name: comp.name.clone(),
                version: comp.version.clone(),
                purl: comp.metadata.get("purl").cloned(),
                licenses: comp.license.as_ref().map(|lic| {
                    vec![CycloneDxLicense {
                        license: CycloneDxLicenseId { id: lic.clone() },
                    }]
                }),
            });
        }

        // Add extension components
        for ext in &bom.extensions {
            for comp in &ext.components {
                components.push(CycloneDxComponent {
                    component_type: Self::component_type_to_cyclonedx(&comp.component_type),
                    name: comp.name.clone(),
                    version: comp.version.clone(),
                    purl: comp.metadata.get("purl").cloned(),
                    licenses: comp.license.as_ref().map(|lic| {
                        vec![CycloneDxLicense {
                            license: CycloneDxLicenseId { id: lic.clone() },
                        }]
                    }),
                });
            }
        }

        let cdx_bom = CycloneDxBom {
            bom_format: "CycloneDX".to_string(),
            spec_version: "1.4".to_string(),
            version: 1,
            metadata: CycloneDxMetadata {
                timestamp: bom.generated_at.to_rfc3339(),
                tools: vec![CycloneDxTool {
                    name: "sindri".to_string(),
                    version: bom.cli_version.clone(),
                }],
            },
            components,
        };

        Ok(serde_json::to_string_pretty(&cdx_bom)?)
    }

    /// Export to SPDX format (JSON)
    fn export_spdx(&self, bom: &BillOfMaterials) -> Result<String> {
        #[derive(Serialize)]
        struct SpdxDocument {
            #[serde(rename = "spdxVersion")]
            spdx_version: String,
            #[serde(rename = "dataLicense")]
            data_license: String,
            #[serde(rename = "SPDXID")]
            spdx_id: String,
            name: String,
            #[serde(rename = "documentNamespace")]
            document_namespace: String,
            #[serde(rename = "creationInfo")]
            creation_info: SpdxCreationInfo,
            packages: Vec<SpdxPackage>,
        }

        #[derive(Serialize)]
        struct SpdxCreationInfo {
            created: String,
            creators: Vec<String>,
        }

        #[derive(Serialize)]
        struct SpdxPackage {
            #[serde(rename = "SPDXID")]
            spdx_id: String,
            name: String,
            #[serde(rename = "versionInfo")]
            version_info: String,
            #[serde(rename = "downloadLocation")]
            download_location: String,
            #[serde(rename = "licenseConcluded")]
            license_concluded: String,
        }

        let mut packages = Vec::new();
        let mut idx = 0;

        // Add system components
        for comp in &bom.system_components {
            packages.push(SpdxPackage {
                spdx_id: format!("SPDXRef-Package-{}", idx),
                name: comp.name.clone(),
                version_info: comp.version.clone(),
                download_location: comp
                    .source
                    .clone()
                    .unwrap_or_else(|| "NOASSERTION".to_string()),
                license_concluded: comp
                    .license
                    .clone()
                    .unwrap_or_else(|| "NOASSERTION".to_string()),
            });
            idx += 1;
        }

        // Add extension components
        for ext in &bom.extensions {
            for comp in &ext.components {
                packages.push(SpdxPackage {
                    spdx_id: format!("SPDXRef-Package-{}", idx),
                    name: comp.name.clone(),
                    version_info: comp.version.clone(),
                    download_location: comp
                        .source
                        .clone()
                        .unwrap_or_else(|| "NOASSERTION".to_string()),
                    license_concluded: comp
                        .license
                        .clone()
                        .unwrap_or_else(|| "NOASSERTION".to_string()),
                });
                idx += 1;
            }
        }

        let spdx_doc = SpdxDocument {
            spdx_version: "SPDX-2.3".to_string(),
            data_license: "CC0-1.0".to_string(),
            spdx_id: "SPDXRef-DOCUMENT".to_string(),
            name: format!("Sindri BOM - {}", bom.config_name),
            document_namespace: format!("https://sindri.dev/spdx/{}", uuid::Uuid::new_v4()),
            creation_info: SpdxCreationInfo {
                created: bom.generated_at.to_rfc3339(),
                creators: vec![format!("Tool: sindri-{}", bom.cli_version)],
            },
            packages,
        };

        Ok(serde_json::to_string_pretty(&spdx_doc)?)
    }

    /// Convert ComponentType to CycloneDX component type string
    fn component_type_to_cyclonedx(comp_type: &ComponentType) -> String {
        match comp_type {
            ComponentType::Tool => "application".to_string(),
            ComponentType::Runtime => "library".to_string(),
            ComponentType::Library => "library".to_string(),
            ComponentType::Package => "library".to_string(),
            ComponentType::Image => "container".to_string(),
            ComponentType::Config => "file".to_string(),
            ComponentType::Other => "library".to_string(),
        }
    }

    /// Load BOM from file
    pub fn load_bom(path: &Path, format: BomFormat) -> Result<BillOfMaterials> {
        let content = std::fs::read_to_string(path)?;

        let bom = match format {
            BomFormat::Json => serde_json::from_str(&content)?,
            BomFormat::Yaml => serde_yaml::from_str(&content)?,
            BomFormat::CycloneDx | BomFormat::Spdx => {
                return Err(anyhow!("Format {:?} not yet supported for loading", format));
            }
        };

        debug!("Loaded BOM from {:?}", path);
        Ok(bom)
    }
}

/// BOM summary for display
pub struct BomSummary {
    pub total_extensions: usize,
    pub total_components: usize,
    pub components_by_type: HashMap<ComponentType, usize>,
    pub extensions_by_category: HashMap<String, usize>,
}

impl BillOfMaterials {
    /// Generate a summary of the BOM
    pub fn summary(&self) -> BomSummary {
        let mut components_by_type: HashMap<ComponentType, usize> = HashMap::new();
        let mut extensions_by_category: HashMap<String, usize> = HashMap::new();

        // Count components by type
        for ext in &self.extensions {
            for component in &ext.components {
                *components_by_type
                    .entry(component.component_type.clone())
                    .or_insert(0) += 1;
            }
            *extensions_by_category
                .entry(ext.category.clone())
                .or_insert(0) += 1;
        }

        for component in &self.system_components {
            *components_by_type
                .entry(component.component_type.clone())
                .or_insert(0) += 1;
        }

        BomSummary {
            total_extensions: self.extensions.len(),
            total_components: self.total_components,
            components_by_type,
            extensions_by_category,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bom_creation() {
        let generator = BomGenerator::new("3.0.0".to_string(), "test-config".to_string());

        let bom = generator.generate(vec![], vec![]);

        assert_eq!(bom.cli_version, "3.0.0");
        assert_eq!(bom.config_name, "test-config");
        assert_eq!(bom.total_components, 0);
    }

    #[test]
    fn test_component_counting() {
        let generator = BomGenerator::new("3.0.0".to_string(), "test-config".to_string());

        let ext_bom = ExtensionBom {
            name: "python".to_string(),
            version: "1.0.0".to_string(),
            category: "language".to_string(),
            install_method: "mise".to_string(),
            installed_at: None,
            components: vec![Component {
                name: "python".to_string(),
                version: "3.13.0".to_string(),
                component_type: ComponentType::Runtime,
                license: Some("PSF".to_string()),
                source: None,
                install_path: None,
                metadata: HashMap::new(),
            }],
            dependencies: vec![],
        };

        let bom = generator.generate(vec![ext_bom], vec![]);

        assert_eq!(bom.total_components, 1);
        assert_eq!(bom.extensions.len(), 1);
    }
}
