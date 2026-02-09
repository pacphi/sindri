//! Comprehensive BOM generation tests
//!
//! Tests for the Bill of Materials (BOM) generation, export, and analysis pipeline.

mod common;

use common::bom_builders::*;
use sindri_core::types::{
    BomConfig, BomSource, BomToolType, CommandValidation, Extension, ExtensionCategory,
    ExtensionMetadata, InstallConfig, InstallMethod, ValidateConfig,
};
use sindri_extensions::bom::{
    BillOfMaterials, BomFormat, BomGenerator, Component, ComponentType, ExtensionBom,
};
use tempfile::TempDir;

// ─── Helper: Build a minimal Extension with BOM config ─────────────────────

fn make_extension_with_bom(name: &str, bom_config: BomConfig) -> Extension {
    Extension {
        metadata: ExtensionMetadata {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: format!("Test extension: {}", name),
            category: ExtensionCategory::Languages,
            author: None,
            homepage: None,
            dependencies: vec![],
        },
        requirements: None,
        install: InstallConfig {
            method: InstallMethod::Mise,
            mise: None,
            apt: None,
            binary: None,
            npm: None,
            script: None,
        },
        configure: None,
        validate: ValidateConfig {
            commands: vec![],
            mise: None,
        },
        remove: None,
        upgrade: None,
        capabilities: None,
        docs: None,
        bom: Some(bom_config),
    }
}

fn make_extension_with_validation(name: &str, commands: Vec<CommandValidation>) -> Extension {
    Extension {
        metadata: ExtensionMetadata {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: format!("Test extension: {}", name),
            category: ExtensionCategory::Languages,
            author: None,
            homepage: None,
            dependencies: vec![],
        },
        requirements: None,
        install: InstallConfig {
            method: InstallMethod::Script,
            mise: None,
            apt: None,
            binary: None,
            npm: None,
            script: None,
        },
        configure: None,
        validate: ValidateConfig {
            commands,
            mise: None,
        },
        remove: None,
        upgrade: None,
        capabilities: None,
        docs: None,
        bom: None,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// BomGenerator: generate() tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_generate_empty_bom() {
    let gen = test_bom_generator();
    let bom = gen.generate(vec![], vec![]);

    assert_eq!(bom.schema_version, "1.0");
    assert_eq!(bom.cli_version, "3.0.0-test");
    assert_eq!(bom.config_name, "test-config");
    assert_eq!(bom.extensions.len(), 0);
    assert_eq!(bom.system_components.len(), 0);
    assert_eq!(bom.total_components, 0);
}

#[test]
fn test_generate_with_extensions() {
    let gen = test_bom_generator();
    let ext = ExtensionBomBuilder::language("python", "python", "3.13.0").build();

    let bom = gen.generate(vec![ext], vec![]);

    assert_eq!(bom.extensions.len(), 1);
    assert_eq!(bom.extensions[0].name, "python");
    assert_eq!(bom.total_components, 1);
}

#[test]
fn test_generate_with_system_components() {
    let gen = test_bom_generator();
    let sys = ComponentBuilder::sindri_cli("3.0.0-test").build();

    let bom = gen.generate(vec![], vec![sys]);

    assert_eq!(bom.system_components.len(), 1);
    assert_eq!(bom.system_components[0].name, "sindri-cli");
    assert_eq!(bom.total_components, 1);
}

#[test]
fn test_generate_total_component_count() {
    let gen = test_bom_generator();

    let ext1 = ExtensionBomBuilder::new("ext1")
        .with_component(ComponentBuilder::tool("tool1", "1.0").build())
        .with_component(ComponentBuilder::tool("tool2", "2.0").build())
        .build();

    let ext2 = ExtensionBomBuilder::new("ext2")
        .with_component(ComponentBuilder::runtime("rt1", "1.0").build())
        .build();

    let sys = ComponentBuilder::sindri_cli("3.0.0").build();
    let bom = gen.generate(vec![ext1, ext2], vec![sys]);

    // 2 + 1 + 1 = 4
    assert_eq!(bom.total_components, 4);
}

#[test]
fn test_generate_preserves_extension_order() {
    let gen = test_bom_generator();
    let exts = vec![
        ExtensionBomBuilder::new("alpha").build(),
        ExtensionBomBuilder::new("beta").build(),
        ExtensionBomBuilder::new("gamma").build(),
    ];

    let bom = gen.generate(exts, vec![]);

    assert_eq!(bom.extensions[0].name, "alpha");
    assert_eq!(bom.extensions[1].name, "beta");
    assert_eq!(bom.extensions[2].name, "gamma");
}

#[test]
fn test_generate_timestamp_is_recent() {
    let gen = test_bom_generator();
    let before = chrono::Utc::now();
    let bom = gen.generate(vec![], vec![]);
    let after = chrono::Utc::now();

    assert!(bom.generated_at >= before);
    assert!(bom.generated_at <= after);
}

// ═══════════════════════════════════════════════════════════════════════════
// BomGenerator: extract_components() via BomConfig tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_extract_components_from_bom_config() {
    let bom_config = BomConfigBuilder::language("python", "3.13.0", "pip", "24.0").build();
    let ext = make_extension_with_bom("python", bom_config);

    let bom_config = ext.bom.as_ref().unwrap();

    assert_eq!(bom_config.tools.len(), 2);
    assert_eq!(bom_config.tools[0].name, "python");
    assert_eq!(bom_config.tools[0].version, Some("3.13.0".to_string()));
    assert_eq!(bom_config.tools[1].name, "pip");
}

#[test]
fn test_extract_components_with_no_bom_config() {
    let ext = make_extension_with_validation(
        "test",
        vec![CommandValidation {
            name: "echo".to_string(),
            version_flag: "--version".to_string(),
            expected_pattern: None,
        }],
    );

    assert!(ext.bom.is_none());
    // Validation commands exist but no BOM - components from validate commands only
    assert_eq!(ext.validate.commands.len(), 1);
}

#[test]
fn test_bom_tool_version_none_defaults_to_unknown() {
    let tool = BomToolBuilder::unversioned("custom").build();
    // When version is None, BOM generation should map to "unknown"
    assert!(tool.version.is_none());
}

#[test]
fn test_bom_tool_with_full_metadata() {
    let tool = BomToolBuilder::cli_tool("kubectl", "1.35.0")
        .with_license("Apache-2.0")
        .with_homepage("https://kubernetes.io")
        .with_purl("pkg:github/kubernetes/kubernetes@1.35.0")
        .with_cpe("cpe:2.3:a:kubernetes:kubectl:1.35.0:*:*:*:*:*:*:*")
        .build();

    assert_eq!(tool.license, Some("Apache-2.0".to_string()));
    assert_eq!(tool.homepage, Some("https://kubernetes.io".to_string()));
    assert!(tool.purl.is_some());
    assert!(tool.cpe.is_some());
}

#[test]
fn test_bom_tool_source_types() {
    let mise_tool = BomToolBuilder::new("go")
        .with_source(BomSource::Mise)
        .build();
    assert_eq!(mise_tool.source, BomSource::Mise);

    let apt_tool = BomToolBuilder::new("docker")
        .with_source(BomSource::Apt)
        .build();
    assert_eq!(apt_tool.source, BomSource::Apt);

    let npm_tool = BomToolBuilder::new("claudeup")
        .with_source(BomSource::Npm)
        .build();
    assert_eq!(npm_tool.source, BomSource::Npm);

    let pip_tool = BomToolBuilder::new("azure-cli")
        .with_source(BomSource::Pip)
        .build();
    assert_eq!(pip_tool.source, BomSource::Pip);

    let bin_tool = BomToolBuilder::new("flyctl")
        .with_source(BomSource::Binary)
        .build();
    assert_eq!(bin_tool.source, BomSource::Binary);

    let script_tool = BomToolBuilder::new("pulumi")
        .with_source(BomSource::Script)
        .build();
    assert_eq!(script_tool.source, BomSource::Script);

    let gh_tool = BomToolBuilder::new("k9s")
        .with_source(BomSource::GithubRelease)
        .build();
    assert_eq!(gh_tool.source, BomSource::GithubRelease);
}

// ═══════════════════════════════════════════════════════════════════════════
// ComponentType mapping tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_component_type_tool() {
    let comp = ComponentBuilder::tool("kubectl", "1.35.0").build();
    assert_eq!(comp.component_type, ComponentType::Tool);
}

#[test]
fn test_component_type_runtime() {
    let comp = ComponentBuilder::runtime("python", "3.13.0").build();
    assert_eq!(comp.component_type, ComponentType::Runtime);
}

#[test]
fn test_component_type_library() {
    let comp = ComponentBuilder::library("libssl", "3.0").build();
    assert_eq!(comp.component_type, ComponentType::Library);
}

#[test]
fn test_component_types_are_distinct() {
    assert_ne!(ComponentType::Tool, ComponentType::Runtime);
    assert_ne!(ComponentType::Runtime, ComponentType::Library);
    assert_ne!(ComponentType::Library, ComponentType::Package);
    assert_ne!(ComponentType::Package, ComponentType::Image);
    assert_ne!(ComponentType::Image, ComponentType::Config);
    assert_ne!(ComponentType::Config, ComponentType::Other);
}

// ═══════════════════════════════════════════════════════════════════════════
// BillOfMaterials: summary() tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_summary_empty_bom() {
    let bom = BillOfMaterialsBuilder::empty()
        .without_system_components()
        .build();
    let summary = bom.summary();

    assert_eq!(summary.total_extensions, 0);
    assert_eq!(summary.total_components, 0);
    assert!(summary.components_by_type.is_empty());
    assert!(summary.extensions_by_category.is_empty());
}

#[test]
fn test_summary_counts_by_type() {
    let bom = BillOfMaterialsBuilder::new()
        .without_system_components()
        .with_extension(
            ExtensionBomBuilder::new("test")
                .with_component(ComponentBuilder::tool("t1", "1.0").build())
                .with_component(ComponentBuilder::tool("t2", "2.0").build())
                .with_component(ComponentBuilder::runtime("r1", "1.0").build())
                .with_component(ComponentBuilder::library("l1", "1.0").build())
                .build(),
        )
        .build();

    let summary = bom.summary();

    assert_eq!(summary.components_by_type[&ComponentType::Tool], 2);
    assert_eq!(summary.components_by_type[&ComponentType::Runtime], 1);
    assert_eq!(summary.components_by_type[&ComponentType::Library], 1);
}

#[test]
fn test_summary_counts_by_category() {
    let bom = BillOfMaterialsBuilder::new()
        .without_system_components()
        .with_extension(
            ExtensionBomBuilder::new("py")
                .with_category("languages")
                .build(),
        )
        .with_extension(
            ExtensionBomBuilder::new("go")
                .with_category("languages")
                .build(),
        )
        .with_extension(
            ExtensionBomBuilder::new("infra")
                .with_category("devops")
                .build(),
        )
        .build();

    let summary = bom.summary();

    assert_eq!(summary.extensions_by_category["languages"], 2);
    assert_eq!(summary.extensions_by_category["devops"], 1);
}

#[test]
fn test_summary_includes_system_components() {
    let bom = BillOfMaterialsBuilder::new().build(); // Has sindri-cli
    let summary = bom.summary();

    // sindri-cli is a Tool
    assert!(summary
        .components_by_type
        .contains_key(&ComponentType::Tool));
}

#[test]
fn test_summary_realistic_bom() {
    let bom = BillOfMaterialsBuilder::realistic().build();
    let summary = bom.summary();

    assert_eq!(summary.total_extensions, 4);
    // python(2) + nodejs(2) + infra-tools(3) + claudeup(1) + system(1) = 9
    assert_eq!(summary.total_components, 9);
}

// ═══════════════════════════════════════════════════════════════════════════
// JSON/YAML serialization roundtrip tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_json_serialization_roundtrip() {
    let bom = BillOfMaterialsBuilder::realistic().build();

    let json = serde_json::to_string_pretty(&bom).expect("JSON serialize");
    let deserialized: BillOfMaterials = serde_json::from_str(&json).expect("JSON deserialize");

    assert_eq!(deserialized.cli_version, bom.cli_version);
    assert_eq!(deserialized.extensions.len(), bom.extensions.len());
    assert_eq!(deserialized.total_components, bom.total_components);
}

#[test]
fn test_yaml_serialization_roundtrip() {
    let bom = BillOfMaterialsBuilder::realistic().build();

    let yaml = serde_yaml::to_string(&bom).expect("YAML serialize");
    let deserialized: BillOfMaterials = serde_yaml::from_str(&yaml).expect("YAML deserialize");

    assert_eq!(deserialized.cli_version, bom.cli_version);
    assert_eq!(deserialized.extensions.len(), bom.extensions.len());
    assert_eq!(deserialized.total_components, bom.total_components);
}

#[test]
fn test_json_contains_expected_fields() {
    let bom = BillOfMaterialsBuilder::single_language("python", "python", "3.13.0").build();
    let json = serde_json::to_string_pretty(&bom).expect("JSON serialize");

    assert!(json.contains("\"schema_version\""));
    assert!(json.contains("\"cli_version\""));
    assert!(json.contains("\"generated_at\""));
    assert!(json.contains("\"config_name\""));
    assert!(json.contains("\"extensions\""));
    assert!(json.contains("\"system_components\""));
    assert!(json.contains("\"total_components\""));
    assert!(json.contains("\"python\""));
}

#[test]
fn test_component_json_roundtrip() {
    let comp = ComponentBuilder::tool("kubectl", "1.35.0")
        .with_license("Apache-2.0")
        .with_source("https://kubernetes.io")
        .with_metadata("purl", "pkg:github/kubernetes/kubectl@1.35.0")
        .build();

    let json = serde_json::to_string(&comp).expect("serialize");
    let back: Component = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(back.name, "kubectl");
    assert_eq!(back.version, "1.35.0");
    assert_eq!(back.license, Some("Apache-2.0".to_string()));
    assert_eq!(
        back.metadata["purl"],
        "pkg:github/kubernetes/kubectl@1.35.0"
    );
}

#[test]
fn test_extension_bom_json_roundtrip() {
    let ext = ExtensionBomBuilder::language("python", "python", "3.13.0")
        .with_dependency("mise-config")
        .build();

    let json = serde_json::to_string(&ext).expect("serialize");
    let back: ExtensionBom = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(back.name, "python");
    assert_eq!(back.category, "languages");
    assert_eq!(back.dependencies, vec!["mise-config".to_string()]);
}

// ═══════════════════════════════════════════════════════════════════════════
// BomGenerator: write_bom / load_bom file I/O tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_write_and_load_json_bom() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("bom.json");

    let gen = test_bom_generator();
    let bom = BillOfMaterialsBuilder::realistic().build();

    gen.write_bom(&bom, &path, BomFormat::Json)
        .expect("write JSON");
    assert!(path.exists());

    let loaded = BomGenerator::load_bom(&path, BomFormat::Json).expect("load JSON");
    assert_eq!(loaded.extensions.len(), bom.extensions.len());
    assert_eq!(loaded.total_components, bom.total_components);
}

#[test]
fn test_write_and_load_yaml_bom() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("bom.yaml");

    let gen = test_bom_generator();
    let bom = BillOfMaterialsBuilder::single_language("golang", "go", "1.25.0").build();

    gen.write_bom(&bom, &path, BomFormat::Yaml)
        .expect("write YAML");
    assert!(path.exists());

    let loaded = BomGenerator::load_bom(&path, BomFormat::Yaml).expect("load YAML");
    assert_eq!(loaded.extensions.len(), 1);
    assert_eq!(loaded.extensions[0].name, "golang");
}

#[test]
fn test_write_creates_parent_directories() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("nested").join("dir").join("bom.json");

    let gen = test_bom_generator();
    let bom = BillOfMaterialsBuilder::empty().build();

    gen.write_bom(&bom, &path, BomFormat::Json).expect("write");
    assert!(path.exists());
}

#[test]
fn test_load_nonexistent_file_fails() {
    let result = BomGenerator::load_bom(
        std::path::Path::new("/nonexistent/bom.json"),
        BomFormat::Json,
    );
    assert!(result.is_err());
}

#[test]
fn test_load_cyclonedx_not_supported() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("bom.cdx.json");
    std::fs::write(&path, "{}").unwrap();

    let result = BomGenerator::load_bom(&path, BomFormat::CycloneDx);
    assert!(result.is_err());
}

#[test]
fn test_load_spdx_not_supported() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("bom.spdx.json");
    std::fs::write(&path, "{}").unwrap();

    let result = BomGenerator::load_bom(&path, BomFormat::Spdx);
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════════════════
// CycloneDX export tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_cyclonedx_export_structure() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("sbom.cdx.json");

    let gen = test_bom_generator();
    let bom = BillOfMaterialsBuilder::realistic().build();

    gen.write_bom(&bom, &path, BomFormat::CycloneDx)
        .expect("write CycloneDX");

    let content = std::fs::read_to_string(&path).expect("read file");
    let cdx: serde_json::Value = serde_json::from_str(&content).expect("parse JSON");

    assert_eq!(cdx["bomFormat"], "CycloneDX");
    assert_eq!(cdx["specVersion"], "1.4");
    assert_eq!(cdx["version"], 1);
    assert!(cdx["metadata"]["timestamp"].is_string());
    assert!(cdx["components"].is_array());
}

#[test]
fn test_cyclonedx_contains_tool_metadata() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("sbom.cdx.json");

    let gen = test_bom_generator();
    let bom = BillOfMaterialsBuilder::empty().build(); // Has sindri-cli system component

    gen.write_bom(&bom, &path, BomFormat::CycloneDx)
        .expect("write");

    let content = std::fs::read_to_string(&path).expect("read");
    let cdx: serde_json::Value = serde_json::from_str(&content).expect("parse");

    let tools = &cdx["metadata"]["tools"];
    assert!(tools.is_array());
    assert_eq!(tools[0]["name"], "sindri");
}

#[test]
fn test_cyclonedx_component_types() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("sbom.cdx.json");

    let gen = test_bom_generator();
    let bom = BillOfMaterialsBuilder::new()
        .without_system_components()
        .with_extension(
            ExtensionBomBuilder::new("test")
                .with_component(ComponentBuilder::tool("t1", "1.0").build())
                .with_component(ComponentBuilder::runtime("r1", "1.0").build())
                .with_component(ComponentBuilder::library("l1", "1.0").build())
                .build(),
        )
        .build();

    gen.write_bom(&bom, &path, BomFormat::CycloneDx)
        .expect("write");

    let content = std::fs::read_to_string(&path).expect("read");
    let cdx: serde_json::Value = serde_json::from_str(&content).expect("parse");

    let components = cdx["components"].as_array().unwrap();
    assert_eq!(components.len(), 3);

    // Tool -> "application"
    assert_eq!(components[0]["type"], "application");
    // Runtime -> "library"
    assert_eq!(components[1]["type"], "library");
    // Library -> "library"
    assert_eq!(components[2]["type"], "library");
}

#[test]
fn test_cyclonedx_includes_licenses() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("sbom.cdx.json");

    let gen = test_bom_generator();
    let bom = BillOfMaterialsBuilder::new()
        .without_system_components()
        .with_extension(
            ExtensionBomBuilder::new("test")
                .with_component(
                    ComponentBuilder::tool("kubectl", "1.35.0")
                        .with_license("Apache-2.0")
                        .build(),
                )
                .build(),
        )
        .build();

    gen.write_bom(&bom, &path, BomFormat::CycloneDx)
        .expect("write");

    let content = std::fs::read_to_string(&path).expect("read");
    let cdx: serde_json::Value = serde_json::from_str(&content).expect("parse");

    let licenses = &cdx["components"][0]["licenses"];
    assert!(licenses.is_array());
    assert_eq!(licenses[0]["license"]["id"], "Apache-2.0");
}

#[test]
fn test_cyclonedx_component_count_matches() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("sbom.cdx.json");

    let gen = test_bom_generator();
    let bom = BillOfMaterialsBuilder::realistic().build();
    let expected_count = bom.total_components;

    gen.write_bom(&bom, &path, BomFormat::CycloneDx)
        .expect("write");

    let content = std::fs::read_to_string(&path).expect("read");
    let cdx: serde_json::Value = serde_json::from_str(&content).expect("parse");

    let components = cdx["components"].as_array().unwrap();
    assert_eq!(components.len(), expected_count);
}

// ═══════════════════════════════════════════════════════════════════════════
// SPDX export tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_spdx_export_structure() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("sbom.spdx.json");

    let gen = test_bom_generator();
    let bom = BillOfMaterialsBuilder::realistic().build();

    gen.write_bom(&bom, &path, BomFormat::Spdx)
        .expect("write SPDX");

    let content = std::fs::read_to_string(&path).expect("read");
    let spdx: serde_json::Value = serde_json::from_str(&content).expect("parse");

    assert_eq!(spdx["spdxVersion"], "SPDX-2.3");
    assert_eq!(spdx["dataLicense"], "CC0-1.0");
    assert_eq!(spdx["SPDXID"], "SPDXRef-DOCUMENT");
    assert!(spdx["documentNamespace"]
        .as_str()
        .unwrap()
        .starts_with("https://sindri.dev/spdx/"));
    assert!(spdx["packages"].is_array());
}

#[test]
fn test_spdx_creation_info() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("sbom.spdx.json");

    let gen = test_bom_generator();
    let bom = BillOfMaterialsBuilder::empty().build();

    gen.write_bom(&bom, &path, BomFormat::Spdx).expect("write");

    let content = std::fs::read_to_string(&path).expect("read");
    let spdx: serde_json::Value = serde_json::from_str(&content).expect("parse");

    assert!(spdx["creationInfo"]["created"].is_string());
    let creators = spdx["creationInfo"]["creators"].as_array().unwrap();
    assert!(creators[0].as_str().unwrap().starts_with("Tool: sindri-"));
}

#[test]
fn test_spdx_package_ids_are_sequential() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("sbom.spdx.json");

    let gen = test_bom_generator();
    let bom = BillOfMaterialsBuilder::realistic().build();

    gen.write_bom(&bom, &path, BomFormat::Spdx).expect("write");

    let content = std::fs::read_to_string(&path).expect("read");
    let spdx: serde_json::Value = serde_json::from_str(&content).expect("parse");

    let packages = spdx["packages"].as_array().unwrap();
    for (idx, pkg) in packages.iter().enumerate() {
        assert_eq!(
            pkg["SPDXID"],
            format!("SPDXRef-Package-{}", idx),
            "Package {} has wrong SPDXID",
            idx
        );
    }
}

#[test]
fn test_spdx_no_assertion_for_missing_fields() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("sbom.spdx.json");

    let gen = test_bom_generator();
    let bom = BillOfMaterialsBuilder::new()
        .without_system_components()
        .with_extension(
            ExtensionBomBuilder::new("test")
                .with_component(ComponentBuilder::new("no-license-tool").build())
                .build(),
        )
        .build();

    gen.write_bom(&bom, &path, BomFormat::Spdx).expect("write");

    let content = std::fs::read_to_string(&path).expect("read");
    let spdx: serde_json::Value = serde_json::from_str(&content).expect("parse");

    let pkg = &spdx["packages"][0];
    assert_eq!(pkg["licenseConcluded"], "NOASSERTION");
    assert_eq!(pkg["downloadLocation"], "NOASSERTION");
}

#[test]
fn test_spdx_package_count_matches() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("sbom.spdx.json");

    let gen = test_bom_generator();
    let bom = BillOfMaterialsBuilder::realistic().build();
    let expected_count = bom.total_components;

    gen.write_bom(&bom, &path, BomFormat::Spdx).expect("write");

    let content = std::fs::read_to_string(&path).expect("read");
    let spdx: serde_json::Value = serde_json::from_str(&content).expect("parse");

    let packages = spdx["packages"].as_array().unwrap();
    assert_eq!(packages.len(), expected_count);
}

#[test]
fn test_spdx_document_name_includes_config() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("sbom.spdx.json");

    let gen = BomGenerator::new("3.0.0".to_string(), "my-workspace".to_string());
    let bom = gen.generate(vec![], vec![]);

    gen.write_bom(&bom, &path, BomFormat::Spdx).expect("write");

    let content = std::fs::read_to_string(&path).expect("read");
    let spdx: serde_json::Value = serde_json::from_str(&content).expect("parse");

    assert!(spdx["name"].as_str().unwrap().contains("my-workspace"));
}

// ═══════════════════════════════════════════════════════════════════════════
// BomFormat tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bom_format_default_is_json() {
    let format: BomFormat = Default::default();
    assert_eq!(format, BomFormat::Json);
}

#[test]
fn test_bom_format_equality() {
    assert_eq!(BomFormat::Json, BomFormat::Json);
    assert_eq!(BomFormat::Yaml, BomFormat::Yaml);
    assert_ne!(BomFormat::Json, BomFormat::Yaml);
    assert_ne!(BomFormat::CycloneDx, BomFormat::Spdx);
}

#[test]
fn test_bom_format_serialization() {
    let json_str = serde_json::to_string(&BomFormat::Json).unwrap();
    assert_eq!(json_str, "\"json\"");

    let yaml_str = serde_json::to_string(&BomFormat::Yaml).unwrap();
    assert_eq!(yaml_str, "\"yaml\"");

    let cdx_str = serde_json::to_string(&BomFormat::CycloneDx).unwrap();
    assert_eq!(cdx_str, "\"cyclonedx\"");

    let spdx_str = serde_json::to_string(&BomFormat::Spdx).unwrap();
    assert_eq!(spdx_str, "\"spdx\"");
}

// ═══════════════════════════════════════════════════════════════════════════
// ExtensionBom structure tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_extension_bom_with_dependencies() {
    let ext = ExtensionBomBuilder::new("nodejs-devtools")
        .with_dependency("nodejs")
        .with_component(ComponentBuilder::tool("typescript", "5.9.0").build())
        .with_component(ComponentBuilder::tool("eslint", "9.0.0").build())
        .build();

    assert_eq!(ext.dependencies.len(), 1);
    assert_eq!(ext.dependencies[0], "nodejs");
    assert_eq!(ext.components.len(), 2);
}

#[test]
fn test_extension_bom_categories() {
    let lang = ExtensionBomBuilder::new("py")
        .with_category("languages")
        .build();
    let devops = ExtensionBomBuilder::new("infra")
        .with_category("devops")
        .build();
    let claude = ExtensionBomBuilder::new("cl")
        .with_category("claude")
        .build();

    assert_eq!(lang.category, "languages");
    assert_eq!(devops.category, "devops");
    assert_eq!(claude.category, "claude");
}

#[test]
fn test_extension_bom_install_methods() {
    let mise = ExtensionBomBuilder::new("e1")
        .with_install_method("mise")
        .build();
    let script = ExtensionBomBuilder::new("e2")
        .with_install_method("script")
        .build();
    let npm = ExtensionBomBuilder::new("e3")
        .with_install_method("npm")
        .build();
    let apt = ExtensionBomBuilder::new("e4")
        .with_install_method("apt")
        .build();

    assert_eq!(mise.install_method, "mise");
    assert_eq!(script.install_method, "script");
    assert_eq!(npm.install_method, "npm");
    assert_eq!(apt.install_method, "apt");
}

// ═══════════════════════════════════════════════════════════════════════════
// Edge case tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_empty_extension_name() {
    let ext = ExtensionBomBuilder::new("").build();
    assert_eq!(ext.name, "");
}

#[test]
fn test_extension_with_many_components() {
    let mut builder = ExtensionBomBuilder::devops("infra-tools");
    for i in 0..20 {
        builder = builder.with_component(
            ComponentBuilder::tool(&format!("tool-{}", i), &format!("{}.0.0", i)).build(),
        );
    }
    let ext = builder.build();
    assert_eq!(ext.components.len(), 20);
}

#[test]
fn test_bom_with_many_extensions() {
    let mut builder = BillOfMaterialsBuilder::new().without_system_components();
    for i in 0..50 {
        builder = builder.with_extension(
            ExtensionBomBuilder::new(&format!("ext-{}", i))
                .with_component(ComponentBuilder::tool(&format!("tool-{}", i), "1.0").build())
                .build(),
        );
    }
    let bom = builder.build();

    assert_eq!(bom.extensions.len(), 50);
    assert_eq!(bom.total_components, 50);
}

#[test]
fn test_component_metadata_arbitrary_keys() {
    let comp = ComponentBuilder::new("test")
        .with_metadata("purl", "pkg:npm/test@1.0")
        .with_metadata("cpe", "cpe:2.3:a:test:test:1.0")
        .with_metadata("source_method", "Mise")
        .with_metadata("custom_key", "custom_value")
        .build();

    assert_eq!(comp.metadata.len(), 4);
    assert_eq!(comp.metadata["purl"], "pkg:npm/test@1.0");
    assert_eq!(comp.metadata["custom_key"], "custom_value");
}

#[test]
fn test_version_strings_preserved_exactly() {
    // Semantic versions
    let tool1 = BomToolBuilder::new("t").with_version("1.2.3").build();
    assert_eq!(tool1.version, Some("1.2.3".to_string()));

    // Semantic channels
    let tool2 = BomToolBuilder::new("t").with_version("stable").build();
    assert_eq!(tool2.version, Some("stable".to_string()));

    // LTS channels
    let tool3 = BomToolBuilder::new("t").with_version("lts").build();
    assert_eq!(tool3.version, Some("lts".to_string()));

    // Dynamic
    let tool4 = BomToolBuilder::new("t").with_version("dynamic").build();
    assert_eq!(tool4.version, Some("dynamic".to_string()));

    // Remote
    let tool5 = BomToolBuilder::new("t").with_version("remote").build();
    assert_eq!(tool5.version, Some("remote".to_string()));

    // Pre-release
    let tool6 = BomToolBuilder::new("t").with_version("3.0.0-alpha").build();
    assert_eq!(tool6.version, Some("3.0.0-alpha".to_string()));
}

// ═══════════════════════════════════════════════════════════════════════════
// BomConfig (extension.yaml input) tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bom_config_empty() {
    let config = BomConfigBuilder::new().build();
    assert!(config.tools.is_empty());
    assert!(config.files.is_empty());
}

#[test]
fn test_bom_config_with_files() {
    use sindri_core::types::BomFileType;

    let config = BomConfigBuilder::new()
        .with_tool(BomToolBuilder::cli_tool("tool1", "1.0").build())
        .with_file("~/.config/tool1.yaml", BomFileType::Config)
        .with_file("/usr/local/bin/tool1", BomFileType::Binary)
        .build();

    assert_eq!(config.tools.len(), 1);
    assert_eq!(config.files.len(), 2);
    assert_eq!(config.files[0].r#type, BomFileType::Config);
    assert_eq!(config.files[1].r#type, BomFileType::Binary);
}

#[test]
fn test_bom_tool_types_all_variants() {
    let types_and_expected = vec![
        (BomToolType::Runtime, "Runtime"),
        (BomToolType::Compiler, "Compiler"),
        (BomToolType::PackageManager, "PackageManager"),
        (BomToolType::CliTool, "CliTool"),
        (BomToolType::Library, "Library"),
        (BomToolType::Framework, "Framework"),
        (BomToolType::Database, "Database"),
        (BomToolType::Server, "Server"),
        (BomToolType::Utility, "Utility"),
        (BomToolType::Application, "Application"),
    ];

    for (tool_type, expected_name) in types_and_expected {
        let tool = BomToolBuilder::new("t").with_type(tool_type).build();
        assert!(
            format!("{:?}", tool.r#type).contains(expected_name),
            "Expected {:?} to contain {}",
            tool.r#type,
            expected_name
        );
    }
}
