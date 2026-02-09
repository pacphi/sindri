//! BOM test builders for creating BOM-related test fixtures
//!
//! Provides fluent builders for constructing BOM objects with various
//! configurations for testing purposes.

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sindri_core::types::{BomConfig, BomFile, BomFileType, BomSource, BomTool, BomToolType};
use sindri_extensions::bom::{
    BillOfMaterials, BomFormat, BomGenerator, Component, ComponentType, ExtensionBom,
};
use std::collections::HashMap;

// ─── BomToolBuilder ────────────────────────────────────────────────────────

/// Builder for creating BomTool test fixtures (extension.yaml BOM entries)
pub struct BomToolBuilder {
    name: String,
    version: Option<String>,
    source: BomSource,
    tool_type: Option<BomToolType>,
    license: Option<String>,
    homepage: Option<String>,
    download_url: Option<String>,
    purl: Option<String>,
    cpe: Option<String>,
}

impl BomToolBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: Some("1.0.0".to_string()),
            source: BomSource::Mise,
            tool_type: Some(BomToolType::CliTool),
            license: None,
            homepage: None,
            download_url: None,
            purl: None,
            cpe: None,
        }
    }

    /// Create a runtime tool (e.g., python, node)
    pub fn runtime(name: &str, version: &str) -> Self {
        Self::new(name)
            .with_version(version)
            .with_type(BomToolType::Runtime)
    }

    /// Create a CLI tool
    pub fn cli_tool(name: &str, version: &str) -> Self {
        Self::new(name)
            .with_version(version)
            .with_type(BomToolType::CliTool)
    }

    /// Create a package manager tool
    pub fn package_manager(name: &str, version: &str) -> Self {
        Self::new(name)
            .with_version(version)
            .with_type(BomToolType::PackageManager)
    }

    /// Create a tool with dynamic version
    pub fn dynamic(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: Some("dynamic".to_string()),
            source: BomSource::Apt,
            tool_type: Some(BomToolType::CliTool),
            license: None,
            homepage: None,
            download_url: None,
            purl: None,
            cpe: None,
        }
    }

    /// Create a tool with no version
    pub fn unversioned(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: None,
            source: BomSource::Script,
            tool_type: None,
            license: None,
            homepage: None,
            download_url: None,
            purl: None,
            cpe: None,
        }
    }

    pub fn with_version(mut self, version: &str) -> Self {
        self.version = Some(version.to_string());
        self
    }

    pub fn with_source(mut self, source: BomSource) -> Self {
        self.source = source;
        self
    }

    pub fn with_type(mut self, tool_type: BomToolType) -> Self {
        self.tool_type = Some(tool_type);
        self
    }

    pub fn with_license(mut self, license: &str) -> Self {
        self.license = Some(license.to_string());
        self
    }

    pub fn with_homepage(mut self, homepage: &str) -> Self {
        self.homepage = Some(homepage.to_string());
        self
    }

    pub fn with_download_url(mut self, url: &str) -> Self {
        self.download_url = Some(url.to_string());
        self
    }

    pub fn with_purl(mut self, purl: &str) -> Self {
        self.purl = Some(purl.to_string());
        self
    }

    pub fn with_cpe(mut self, cpe: &str) -> Self {
        self.cpe = Some(cpe.to_string());
        self
    }

    pub fn build(self) -> BomTool {
        BomTool {
            name: self.name,
            version: self.version,
            source: self.source,
            r#type: self.tool_type,
            license: self.license,
            homepage: self.homepage,
            download_url: self.download_url,
            checksum: None,
            purl: self.purl,
            cpe: self.cpe,
        }
    }
}

// ─── BomConfigBuilder ──────────────────────────────────────────────────────

/// Builder for creating BomConfig test fixtures
pub struct BomConfigBuilder {
    tools: Vec<BomTool>,
    files: Vec<BomFile>,
}

impl BomConfigBuilder {
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            files: Vec::new(),
        }
    }

    /// Create a BomConfig with a single CLI tool
    pub fn single_tool(name: &str, version: &str) -> Self {
        Self::new().with_tool(BomToolBuilder::cli_tool(name, version).build())
    }

    /// Create a BomConfig mimicking a language extension (runtime + package manager)
    pub fn language(runtime_name: &str, runtime_ver: &str, pkg_name: &str, pkg_ver: &str) -> Self {
        Self::new()
            .with_tool(BomToolBuilder::runtime(runtime_name, runtime_ver).build())
            .with_tool(BomToolBuilder::package_manager(pkg_name, pkg_ver).build())
    }

    /// Create a BomConfig with multiple CLI tools (infra-tools style)
    pub fn multi_tool(tools: Vec<(&str, &str)>) -> Self {
        let mut builder = Self::new();
        for (name, version) in tools {
            builder = builder.with_tool(BomToolBuilder::cli_tool(name, version).build());
        }
        builder
    }

    pub fn with_tool(mut self, tool: BomTool) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn with_file(mut self, path: &str, file_type: BomFileType) -> Self {
        self.files.push(BomFile {
            path: path.to_string(),
            r#type: file_type,
            checksum: None,
        });
        self
    }

    pub fn build(self) -> BomConfig {
        BomConfig {
            tools: self.tools,
            files: self.files,
        }
    }
}

// ─── ComponentBuilder ──────────────────────────────────────────────────────

/// Builder for creating Component test fixtures (BOM output components)
pub struct ComponentBuilder {
    name: String,
    version: String,
    component_type: ComponentType,
    license: Option<String>,
    source: Option<String>,
    install_path: Option<String>,
    metadata: HashMap<String, String>,
}

impl ComponentBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            component_type: ComponentType::Tool,
            license: None,
            source: None,
            install_path: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a tool component
    pub fn tool(name: &str, version: &str) -> Self {
        Self::new(name)
            .with_version(version)
            .with_type(ComponentType::Tool)
    }

    /// Create a runtime component
    pub fn runtime(name: &str, version: &str) -> Self {
        Self::new(name)
            .with_version(version)
            .with_type(ComponentType::Runtime)
    }

    /// Create a library component
    pub fn library(name: &str, version: &str) -> Self {
        Self::new(name)
            .with_version(version)
            .with_type(ComponentType::Library)
    }

    /// Create the sindri-cli system component
    pub fn sindri_cli(version: &str) -> Self {
        Self::new("sindri-cli")
            .with_version(version)
            .with_type(ComponentType::Tool)
            .with_license("MIT")
            .with_source("github:pacphi/sindri")
    }

    pub fn with_version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    pub fn with_type(mut self, component_type: ComponentType) -> Self {
        self.component_type = component_type;
        self
    }

    pub fn with_license(mut self, license: &str) -> Self {
        self.license = Some(license.to_string());
        self
    }

    pub fn with_source(mut self, source: &str) -> Self {
        self.source = Some(source.to_string());
        self
    }

    pub fn with_install_path(mut self, path: &str) -> Self {
        self.install_path = Some(path.to_string());
        self
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    pub fn build(self) -> Component {
        Component {
            name: self.name,
            version: self.version,
            component_type: self.component_type,
            license: self.license,
            source: self.source,
            install_path: self.install_path,
            metadata: self.metadata,
        }
    }
}

// ─── ExtensionBomBuilder ───────────────────────────────────────────────────

/// Builder for creating ExtensionBom test fixtures (BOM output extension entries)
pub struct ExtensionBomBuilder {
    name: String,
    version: String,
    category: String,
    install_method: String,
    installed_at: Option<DateTime<Utc>>,
    components: Vec<Component>,
    dependencies: Vec<String>,
}

impl ExtensionBomBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            category: "languages".to_string(),
            install_method: "mise".to_string(),
            installed_at: Some(Utc::now()),
            components: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    /// Create a language extension BOM entry
    pub fn language(name: &str, runtime: &str, version: &str) -> Self {
        Self::new(name)
            .with_category("languages")
            .with_install_method("mise")
            .with_component(ComponentBuilder::runtime(runtime, version).build())
    }

    /// Create a devops extension BOM entry
    pub fn devops(name: &str) -> Self {
        Self::new(name)
            .with_category("devops")
            .with_install_method("script")
    }

    /// Create a Claude extension BOM entry
    pub fn claude(name: &str, version: &str) -> Self {
        Self::new(name)
            .with_version(version)
            .with_category("claude")
            .with_install_method("npm")
    }

    pub fn with_version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    pub fn with_category(mut self, category: &str) -> Self {
        self.category = category.to_string();
        self
    }

    pub fn with_install_method(mut self, method: &str) -> Self {
        self.install_method = method.to_string();
        self
    }

    pub fn with_installed_at(mut self, ts: DateTime<Utc>) -> Self {
        self.installed_at = Some(ts);
        self
    }

    pub fn without_installed_at(mut self) -> Self {
        self.installed_at = None;
        self
    }

    pub fn with_component(mut self, component: Component) -> Self {
        self.components.push(component);
        self
    }

    pub fn with_components(mut self, components: Vec<Component>) -> Self {
        self.components = components;
        self
    }

    pub fn with_dependency(mut self, dep: &str) -> Self {
        self.dependencies.push(dep.to_string());
        self
    }

    pub fn build(self) -> ExtensionBom {
        ExtensionBom {
            name: self.name,
            version: self.version,
            category: self.category,
            install_method: self.install_method,
            installed_at: self.installed_at,
            components: self.components,
            dependencies: self.dependencies,
        }
    }
}

// ─── BillOfMaterialsBuilder ───────────────────────────────────────────────

/// Builder for creating complete BillOfMaterials test fixtures
pub struct BillOfMaterialsBuilder {
    schema_version: String,
    cli_version: String,
    generated_at: DateTime<Utc>,
    config_name: String,
    extensions: Vec<ExtensionBom>,
    system_components: Vec<Component>,
}

impl BillOfMaterialsBuilder {
    pub fn new() -> Self {
        Self {
            schema_version: "1.0".to_string(),
            cli_version: "3.0.0".to_string(),
            generated_at: Utc::now(),
            config_name: "test-config".to_string(),
            extensions: Vec::new(),
            system_components: vec![ComponentBuilder::sindri_cli("3.0.0").build()],
        }
    }

    /// Create a minimal BOM with no extensions
    pub fn empty() -> Self {
        Self::new()
    }

    /// Create a BOM with a single language extension
    pub fn single_language(name: &str, runtime: &str, version: &str) -> Self {
        Self::new().with_extension(ExtensionBomBuilder::language(name, runtime, version).build())
    }

    /// Create a realistic multi-extension BOM
    pub fn realistic() -> Self {
        Self::new()
            .with_extension(
                ExtensionBomBuilder::language("python", "python", "3.13.0")
                    .with_component(
                        ComponentBuilder::tool("uv", "0.9.0")
                            .with_license("Apache-2.0")
                            .build(),
                    )
                    .build(),
            )
            .with_extension(
                ExtensionBomBuilder::language("nodejs", "node", "22.0.0")
                    .with_component(
                        ComponentBuilder::tool("npm", "10.0.0")
                            .with_license("Artistic-2.0")
                            .build(),
                    )
                    .build(),
            )
            .with_extension(
                ExtensionBomBuilder::devops("infra-tools")
                    .with_component(ComponentBuilder::tool("kubectl", "1.35.0").build())
                    .with_component(ComponentBuilder::tool("helm", "4.1.0").build())
                    .with_component(ComponentBuilder::tool("terraform", "1.14.0").build())
                    .build(),
            )
            .with_extension(
                ExtensionBomBuilder::claude("claudeup", "1.8.0")
                    .with_component(ComponentBuilder::tool("claudeup", "1.8.0").build())
                    .build(),
            )
    }

    pub fn with_cli_version(mut self, version: &str) -> Self {
        self.cli_version = version.to_string();
        // Also update system component
        self.system_components = vec![ComponentBuilder::sindri_cli(version).build()];
        self
    }

    pub fn with_config_name(mut self, name: &str) -> Self {
        self.config_name = name.to_string();
        self
    }

    pub fn with_extension(mut self, ext: ExtensionBom) -> Self {
        self.extensions.push(ext);
        self
    }

    pub fn with_system_component(mut self, component: Component) -> Self {
        self.system_components.push(component);
        self
    }

    pub fn without_system_components(mut self) -> Self {
        self.system_components.clear();
        self
    }

    pub fn build(self) -> BillOfMaterials {
        let total_components: usize = self
            .extensions
            .iter()
            .map(|e| e.components.len())
            .sum::<usize>()
            + self.system_components.len();

        BillOfMaterials {
            schema_version: self.schema_version,
            cli_version: self.cli_version,
            generated_at: self.generated_at,
            config_name: self.config_name,
            extensions: self.extensions,
            system_components: self.system_components,
            total_components,
        }
    }
}

// ─── Helper: BomGenerator factory ──────────────────────────────────────────

/// Create a BomGenerator with default test values
pub fn test_bom_generator() -> BomGenerator {
    BomGenerator::new("3.0.0-test".to_string(), "test-config".to_string())
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bom_tool_builder_default() {
        let tool = BomToolBuilder::new("kubectl").build();
        assert_eq!(tool.name, "kubectl");
        assert_eq!(tool.version, Some("1.0.0".to_string()));
        assert_eq!(tool.source, BomSource::Mise);
    }

    #[test]
    fn test_bom_tool_builder_runtime() {
        let tool = BomToolBuilder::runtime("python", "3.13.0").build();
        assert_eq!(tool.name, "python");
        assert_eq!(tool.version, Some("3.13.0".to_string()));
        assert_eq!(tool.r#type, Some(BomToolType::Runtime));
    }

    #[test]
    fn test_bom_tool_builder_dynamic() {
        let tool = BomToolBuilder::dynamic("docker").build();
        assert_eq!(tool.version, Some("dynamic".to_string()));
        assert_eq!(tool.source, BomSource::Apt);
    }

    #[test]
    fn test_bom_tool_builder_unversioned() {
        let tool = BomToolBuilder::unversioned("custom-tool").build();
        assert!(tool.version.is_none());
        assert!(tool.r#type.is_none());
    }

    #[test]
    fn test_bom_config_builder_single_tool() {
        let config = BomConfigBuilder::single_tool("helm", "4.1.0").build();
        assert_eq!(config.tools.len(), 1);
        assert_eq!(config.tools[0].name, "helm");
    }

    #[test]
    fn test_bom_config_builder_language() {
        let config = BomConfigBuilder::language("python", "3.13", "pip", "24.0").build();
        assert_eq!(config.tools.len(), 2);
        assert_eq!(config.tools[0].r#type, Some(BomToolType::Runtime));
        assert_eq!(config.tools[1].r#type, Some(BomToolType::PackageManager));
    }

    #[test]
    fn test_bom_config_builder_multi_tool() {
        let config = BomConfigBuilder::multi_tool(vec![
            ("kubectl", "1.35.0"),
            ("helm", "4.1.0"),
            ("terraform", "1.14"),
        ])
        .build();
        assert_eq!(config.tools.len(), 3);
    }

    #[test]
    fn test_component_builder_default() {
        let comp = ComponentBuilder::new("test").build();
        assert_eq!(comp.name, "test");
        assert_eq!(comp.component_type, ComponentType::Tool);
    }

    #[test]
    fn test_component_builder_runtime() {
        let comp = ComponentBuilder::runtime("python", "3.13.0").build();
        assert_eq!(comp.component_type, ComponentType::Runtime);
        assert_eq!(comp.version, "3.13.0");
    }

    #[test]
    fn test_component_builder_sindri_cli() {
        let comp = ComponentBuilder::sindri_cli("3.0.0").build();
        assert_eq!(comp.name, "sindri-cli");
        assert_eq!(comp.license, Some("MIT".to_string()));
        assert_eq!(comp.source, Some("github:pacphi/sindri".to_string()));
    }

    #[test]
    fn test_extension_bom_builder_default() {
        let ext = ExtensionBomBuilder::new("test-ext").build();
        assert_eq!(ext.name, "test-ext");
        assert_eq!(ext.category, "languages");
        assert!(ext.installed_at.is_some());
    }

    #[test]
    fn test_extension_bom_builder_language() {
        let ext = ExtensionBomBuilder::language("python", "python", "3.13.0").build();
        assert_eq!(ext.name, "python");
        assert_eq!(ext.install_method, "mise");
        assert_eq!(ext.components.len(), 1);
        assert_eq!(ext.components[0].component_type, ComponentType::Runtime);
    }

    #[test]
    fn test_extension_bom_builder_with_dependencies() {
        let ext = ExtensionBomBuilder::new("nodejs-devtools")
            .with_dependency("nodejs")
            .build();
        assert_eq!(ext.dependencies, vec!["nodejs".to_string()]);
    }

    #[test]
    fn test_bill_of_materials_builder_empty() {
        let bom = BillOfMaterialsBuilder::empty().build();
        assert_eq!(bom.extensions.len(), 0);
        assert_eq!(bom.system_components.len(), 1); // sindri-cli
        assert_eq!(bom.total_components, 1);
    }

    #[test]
    fn test_bill_of_materials_builder_realistic() {
        let bom = BillOfMaterialsBuilder::realistic().build();
        assert_eq!(bom.extensions.len(), 4);
        // python(2) + nodejs(2) + infra-tools(3) + claudeup(1) + system(1) = 9
        assert_eq!(bom.total_components, 9);
    }

    #[test]
    fn test_bill_of_materials_builder_total_count() {
        let bom = BillOfMaterialsBuilder::new()
            .with_extension(
                ExtensionBomBuilder::new("ext1")
                    .with_component(ComponentBuilder::tool("tool1", "1.0").build())
                    .with_component(ComponentBuilder::tool("tool2", "2.0").build())
                    .build(),
            )
            .build();
        // 2 extension components + 1 system (sindri-cli) = 3
        assert_eq!(bom.total_components, 3);
    }

    #[test]
    fn test_bill_of_materials_without_system() {
        let bom = BillOfMaterialsBuilder::new()
            .without_system_components()
            .build();
        assert_eq!(bom.system_components.len(), 0);
        assert_eq!(bom.total_components, 0);
    }
}
