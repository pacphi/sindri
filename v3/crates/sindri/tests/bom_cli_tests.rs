//! Integration tests for the BOM CLI commands
//!
//! Tests the BOM generation pipeline end-to-end using fixture extension.yaml files
//! on disk. These tests exercise BomGenerator with real Extension parsing without
//! requiring network access (no registry loading from GitHub).

use sindri_core::types::{
    BomConfig, BomSource, BomTool, BomToolType, CommandValidation, Extension, ExtensionCategory,
    ExtensionMetadata, InstallConfig, InstallMethod, InstalledExtension, ValidateConfig,
};
use sindri_extensions::bom::{BillOfMaterials, BomFormat, BomGenerator};
use sindri_extensions::registry::ExtensionRegistry;
use tempfile::TempDir;

// ─── Helpers ───────────────────────────────────────────────────────────────

/// Create a test extension with BOM config and validation commands
fn create_test_extension(
    name: &str,
    category: ExtensionCategory,
    method: InstallMethod,
    bom_tools: Vec<BomTool>,
    validate_commands: Vec<CommandValidation>,
) -> Extension {
    Extension {
        metadata: ExtensionMetadata {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: format!("{} extension for testing", name),
            category,
            author: Some("Test Author".to_string()),
            homepage: None,
            dependencies: vec![],
        },
        requirements: None,
        install: InstallConfig {
            method,
            mise: None,
            apt: None,
            binary: None,
            npm: None,
            script: None,
        },
        configure: None,
        validate: ValidateConfig {
            commands: validate_commands,
            mise: None,
        },
        remove: None,
        upgrade: None,
        capabilities: None,
        docs: None,
        bom: if bom_tools.is_empty() {
            None
        } else {
            Some(BomConfig {
                tools: bom_tools,
                files: vec![],
            })
        },
    }
}

/// Create a BomTool with common defaults
fn tool(name: &str, version: &str, source: BomSource) -> BomTool {
    BomTool {
        name: name.to_string(),
        version: Some(version.to_string()),
        source,
        r#type: Some(BomToolType::CliTool),
        license: None,
        homepage: None,
        download_url: None,
        checksum: None,
        purl: None,
        cpe: None,
    }
}

fn runtime_tool(name: &str, version: &str, source: BomSource) -> BomTool {
    BomTool {
        name: name.to_string(),
        version: Some(version.to_string()),
        source,
        r#type: Some(BomToolType::Runtime),
        license: Some("MIT".to_string()),
        homepage: None,
        download_url: None,
        checksum: None,
        purl: None,
        cpe: None,
    }
}

/// Write an extension.yaml to a temp dir and register it
#[allow(dead_code)]
fn setup_extension_on_disk(tmp: &TempDir, name: &str, ext: &Extension) -> std::path::PathBuf {
    let ext_dir = tmp.path().join(name);
    std::fs::create_dir_all(&ext_dir).unwrap();

    let yaml = serde_yaml_ng::to_string(ext).expect("serialize extension");
    let path = ext_dir.join("extension.yaml");
    std::fs::write(&path, yaml).unwrap();
    path
}

/// Build a registry with extensions pre-loaded
fn build_registry_with_extensions(extensions: Vec<(&str, Extension)>) -> ExtensionRegistry {
    let mut registry = ExtensionRegistry::new();
    for (name, ext) in extensions {
        // Also register a RegistryEntry
        registry.entries.insert(
            name.to_string(),
            sindri_core::types::RegistryEntry {
                category: format!("{:?}", ext.metadata.category).to_lowercase(),
                description: ext.metadata.description.clone(),
                protected: false,
                dependencies: ext.metadata.dependencies.clone(),
                conflicts: vec![],
                sha256: None,
            },
        );
        registry.extensions.insert(name.to_string(), ext);
    }
    registry
}

/// Build a manifest with installed extensions
fn build_manifest(extensions: Vec<(&str, &str)>) -> sindri_core::types::InstallManifest {
    let mut manifest = sindri_core::types::InstallManifest::default();
    for (name, version) in extensions {
        manifest.extensions.insert(
            name.to_string(),
            InstalledExtension {
                version: version.to_string(),
                status_datetime: chrono::Utc::now(),
                source: "test".to_string(),
                state: sindri_core::types::ExtensionState::Installed,
            },
        );
    }
    manifest
}

// ═══════════════════════════════════════════════════════════════════════════
// generate_from_manifest() integration tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_generate_from_manifest_single_extension() {
    let ext = create_test_extension(
        "python",
        ExtensionCategory::Languages,
        InstallMethod::Mise,
        vec![
            runtime_tool("python", "3.13.0", BomSource::Mise),
            tool("uv", "0.9.0", BomSource::Mise),
        ],
        vec![CommandValidation {
            name: "python3".to_string(),
            version_flag: "--version".to_string(),
            expected_pattern: None,
        }],
    );

    let registry = build_registry_with_extensions(vec![("python", ext)]);
    let manifest = build_manifest(vec![("python", "1.0.0")]);

    let gen = BomGenerator::new("3.0.0".to_string(), "test".to_string());
    let bom = gen.generate_from_manifest(&manifest, &registry).unwrap();

    assert_eq!(bom.extensions.len(), 1);
    assert_eq!(bom.extensions[0].name, "python");
    assert_eq!(bom.extensions[0].category, "languages");
    assert_eq!(bom.extensions[0].install_method, "mise");
    // validate command (python3) + BOM tools (python, uv)
    assert_eq!(bom.extensions[0].components.len(), 3);
}

#[test]
fn test_generate_from_manifest_multiple_extensions() {
    let python = create_test_extension(
        "python",
        ExtensionCategory::Languages,
        InstallMethod::Mise,
        vec![runtime_tool("python", "3.13.0", BomSource::Mise)],
        vec![],
    );

    let infra = create_test_extension(
        "infra-tools",
        ExtensionCategory::Devops,
        InstallMethod::Script,
        vec![
            tool("kubectl", "1.35.0", BomSource::Mise),
            tool("helm", "4.1.0", BomSource::Mise),
            tool("terraform", "1.14.0", BomSource::Mise),
        ],
        vec![],
    );

    let claude = create_test_extension(
        "claudeup",
        ExtensionCategory::Claude,
        InstallMethod::Npm,
        vec![tool("claudeup", "1.8.0", BomSource::Npm)],
        vec![],
    );

    let registry = build_registry_with_extensions(vec![
        ("python", python),
        ("infra-tools", infra),
        ("claudeup", claude),
    ]);
    let manifest = build_manifest(vec![
        ("python", "1.0.0"),
        ("infra-tools", "1.0.0"),
        ("claudeup", "1.0.0"),
    ]);

    let gen = BomGenerator::new("3.0.0".to_string(), "production".to_string());
    let bom = gen.generate_from_manifest(&manifest, &registry).unwrap();

    assert_eq!(bom.extensions.len(), 3);
    assert_eq!(bom.config_name, "production");
    // 1 + 3 + 1 = 5 BOM tools + 1 system = 6 total
    assert_eq!(bom.total_components, 6);
}

#[test]
fn test_generate_from_manifest_sorts_by_name() {
    let ext_z = create_test_extension(
        "zsh",
        ExtensionCategory::Languages,
        InstallMethod::Script,
        vec![],
        vec![],
    );
    let ext_a = create_test_extension(
        "ansible",
        ExtensionCategory::Devops,
        InstallMethod::Script,
        vec![],
        vec![],
    );
    let ext_m = create_test_extension(
        "maven",
        ExtensionCategory::Languages,
        InstallMethod::Script,
        vec![],
        vec![],
    );

    let registry =
        build_registry_with_extensions(vec![("zsh", ext_z), ("ansible", ext_a), ("maven", ext_m)]);
    let manifest = build_manifest(vec![
        ("zsh", "1.0.0"),
        ("ansible", "1.0.0"),
        ("maven", "1.0.0"),
    ]);

    let gen = BomGenerator::new("3.0.0".to_string(), "test".to_string());
    let bom = gen.generate_from_manifest(&manifest, &registry).unwrap();

    assert_eq!(bom.extensions[0].name, "ansible");
    assert_eq!(bom.extensions[1].name, "maven");
    assert_eq!(bom.extensions[2].name, "zsh");
}

#[test]
fn test_generate_from_manifest_includes_system_components() {
    let ext = create_test_extension(
        "test",
        ExtensionCategory::Testing,
        InstallMethod::Script,
        vec![],
        vec![],
    );

    let registry = build_registry_with_extensions(vec![("test", ext)]);
    let manifest = build_manifest(vec![("test", "1.0.0")]);

    let gen = BomGenerator::new("3.0.0".to_string(), "test".to_string());
    let bom = gen.generate_from_manifest(&manifest, &registry).unwrap();

    assert!(!bom.system_components.is_empty());
    assert_eq!(bom.system_components[0].name, "sindri-cli");
    assert_eq!(bom.system_components[0].version, "3.0.0");
}

#[test]
fn test_generate_from_manifest_preserves_dependencies() {
    let mut devtools = create_test_extension(
        "nodejs-devtools",
        ExtensionCategory::Languages,
        InstallMethod::Mise,
        vec![tool("typescript", "5.9.0", BomSource::Npm)],
        vec![],
    );
    devtools.metadata.dependencies = vec!["nodejs".to_string()];

    let nodejs = create_test_extension(
        "nodejs",
        ExtensionCategory::Languages,
        InstallMethod::Mise,
        vec![runtime_tool("node", "22.0.0", BomSource::Mise)],
        vec![],
    );

    let mut registry =
        build_registry_with_extensions(vec![("nodejs", nodejs), ("nodejs-devtools", devtools)]);
    // Update the registry entry to reflect dependency
    registry
        .entries
        .get_mut("nodejs-devtools")
        .unwrap()
        .dependencies = vec!["nodejs".to_string()];

    let manifest = build_manifest(vec![("nodejs", "1.0.0"), ("nodejs-devtools", "1.0.0")]);

    let gen = BomGenerator::new("3.0.0".to_string(), "test".to_string());
    let bom = gen.generate_from_manifest(&manifest, &registry).unwrap();

    let devtools_bom = bom
        .extensions
        .iter()
        .find(|e| e.name == "nodejs-devtools")
        .unwrap();
    assert_eq!(devtools_bom.dependencies, vec!["nodejs".to_string()]);
}

#[test]
fn test_generate_from_manifest_missing_extension_errors() {
    let registry = build_registry_with_extensions(vec![]);
    let manifest = build_manifest(vec![("nonexistent", "1.0.0")]);

    let gen = BomGenerator::new("3.0.0".to_string(), "test".to_string());
    let result = gen.generate_from_manifest(&manifest, &registry);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

// ═══════════════════════════════════════════════════════════════════════════
// File export integration tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_export_json_integration() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("bom.json");

    let ext = create_test_extension(
        "golang",
        ExtensionCategory::Languages,
        InstallMethod::Mise,
        vec![
            runtime_tool("go", "1.26.0", BomSource::Mise),
            tool("golangci-lint", "1.64.0", BomSource::GithubRelease),
        ],
        vec![],
    );

    let registry = build_registry_with_extensions(vec![("golang", ext)]);
    let manifest = build_manifest(vec![("golang", "1.0.0")]);

    let gen = BomGenerator::new("3.0.0".to_string(), "test".to_string());
    let bom = gen.generate_from_manifest(&manifest, &registry).unwrap();
    gen.write_bom(&bom, &output, BomFormat::Json).unwrap();

    // Verify file contents
    let content = std::fs::read_to_string(&output).unwrap();
    let parsed: BillOfMaterials = serde_json::from_str(&content).unwrap();

    assert_eq!(parsed.extensions.len(), 1);
    assert_eq!(parsed.extensions[0].name, "golang");
    assert_eq!(parsed.extensions[0].components.len(), 2);
}

#[test]
fn test_export_yaml_integration() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("bom.yaml");

    let ext = create_test_extension(
        "rust",
        ExtensionCategory::Languages,
        InstallMethod::Mise,
        vec![
            runtime_tool("rustc", "stable", BomSource::Mise),
            tool("cargo", "stable", BomSource::Mise),
        ],
        vec![],
    );

    let registry = build_registry_with_extensions(vec![("rust", ext)]);
    let manifest = build_manifest(vec![("rust", "1.0.0")]);

    let gen = BomGenerator::new("3.0.0".to_string(), "test".to_string());
    let bom = gen.generate_from_manifest(&manifest, &registry).unwrap();
    gen.write_bom(&bom, &output, BomFormat::Yaml).unwrap();

    let content = std::fs::read_to_string(&output).unwrap();
    let parsed: BillOfMaterials = serde_yaml_ng::from_str(&content).unwrap();

    assert_eq!(parsed.extensions[0].name, "rust");
}

#[test]
fn test_export_cyclonedx_integration() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("sbom.cdx.json");

    let ext = create_test_extension(
        "jvm",
        ExtensionCategory::Languages,
        InstallMethod::Script,
        vec![
            runtime_tool("java", "25", BomSource::Script),
            tool("mvn", "3.9.12", BomSource::Script),
            tool("gradle", "9.3.1", BomSource::Script),
        ],
        vec![],
    );

    let registry = build_registry_with_extensions(vec![("jvm", ext)]);
    let manifest = build_manifest(vec![("jvm", "1.0.0")]);

    let gen = BomGenerator::new("3.0.0".to_string(), "test".to_string());
    let bom = gen.generate_from_manifest(&manifest, &registry).unwrap();
    gen.write_bom(&bom, &output, BomFormat::CycloneDx).unwrap();

    let content = std::fs::read_to_string(&output).unwrap();
    let cdx: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(cdx["bomFormat"], "CycloneDX");
    assert_eq!(cdx["specVersion"], "1.4");
    // 3 BOM tools + 1 system = 4 components
    assert_eq!(cdx["components"].as_array().unwrap().len(), 4);
}

#[test]
fn test_export_spdx_integration() {
    let tmp = TempDir::new().unwrap();
    let output = tmp.path().join("sbom.spdx.json");

    let ext = create_test_extension(
        "cloud-tools",
        ExtensionCategory::Cloud,
        InstallMethod::Script,
        vec![
            tool("aws", "2.27.41", BomSource::Script),
            tool("az", "2.83.0", BomSource::Pip),
            tool("gcloud", "555.0.0", BomSource::Script),
        ],
        vec![],
    );

    let registry = build_registry_with_extensions(vec![("cloud-tools", ext)]);
    let manifest = build_manifest(vec![("cloud-tools", "1.0.0")]);

    let gen = BomGenerator::new("3.0.0".to_string(), "test".to_string());
    let bom = gen.generate_from_manifest(&manifest, &registry).unwrap();
    gen.write_bom(&bom, &output, BomFormat::Spdx).unwrap();

    let content = std::fs::read_to_string(&output).unwrap();
    let spdx: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(spdx["spdxVersion"], "SPDX-2.3");
    // 3 BOM tools + 1 system = 4 packages
    assert_eq!(spdx["packages"].as_array().unwrap().len(), 4);
}

// ═══════════════════════════════════════════════════════════════════════════
// Component extraction integration tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_bom_tools_with_licenses_and_metadata() {
    let ext = create_test_extension(
        "kubectl-ext",
        ExtensionCategory::Devops,
        InstallMethod::Binary,
        vec![BomTool {
            name: "kubectl".to_string(),
            version: Some("1.35.0".to_string()),
            source: BomSource::Binary,
            r#type: Some(BomToolType::CliTool),
            license: Some("Apache-2.0".to_string()),
            homepage: Some("https://kubernetes.io".to_string()),
            download_url: None,
            checksum: None,
            purl: Some("pkg:github/kubernetes/kubectl@1.35.0".to_string()),
            cpe: Some("cpe:2.3:a:kubernetes:kubectl:1.35.0".to_string()),
        }],
        vec![],
    );

    let registry = build_registry_with_extensions(vec![("kubectl-ext", ext)]);
    let manifest = build_manifest(vec![("kubectl-ext", "1.0.0")]);

    let gen = BomGenerator::new("3.0.0".to_string(), "test".to_string());
    let bom = gen.generate_from_manifest(&manifest, &registry).unwrap();

    let kubectl_ext = &bom.extensions[0];
    let kubectl_comp = &kubectl_ext.components[0];
    assert_eq!(kubectl_comp.name, "kubectl");
    assert_eq!(kubectl_comp.version, "1.35.0");
    assert_eq!(kubectl_comp.license, Some("Apache-2.0".to_string()));
    assert!(kubectl_comp.metadata.contains_key("purl"));
    assert!(kubectl_comp.metadata.contains_key("cpe"));
}

#[test]
fn test_validation_commands_generate_detected_components() {
    let ext = create_test_extension(
        "test-ext",
        ExtensionCategory::Testing,
        InstallMethod::Script,
        vec![], // No BOM tools
        vec![
            CommandValidation {
                name: "echo".to_string(),
                version_flag: "--version".to_string(),
                expected_pattern: None,
            },
            CommandValidation {
                name: "test-tool".to_string(),
                version_flag: "--version".to_string(),
                expected_pattern: Some(r"\d+\.\d+".to_string()),
            },
        ],
    );

    let registry = build_registry_with_extensions(vec![("test-ext", ext)]);
    let manifest = build_manifest(vec![("test-ext", "1.0.0")]);

    let gen = BomGenerator::new("3.0.0".to_string(), "test".to_string());
    let bom = gen.generate_from_manifest(&manifest, &registry).unwrap();

    // Validation commands generate components with "detected" version
    let ext_bom = &bom.extensions[0];
    assert_eq!(ext_bom.components.len(), 2);
    assert_eq!(ext_bom.components[0].version, "detected");
    assert_eq!(ext_bom.components[1].version, "detected");
}

#[test]
fn test_extension_without_bom_uses_validation_only() {
    let ext = create_test_extension(
        "basic",
        ExtensionCategory::Testing,
        InstallMethod::Script,
        vec![], // No BOM config
        vec![CommandValidation {
            name: "basic-tool".to_string(),
            version_flag: "--version".to_string(),
            expected_pattern: None,
        }],
    );

    let registry = build_registry_with_extensions(vec![("basic", ext)]);
    let manifest = build_manifest(vec![("basic", "1.0.0")]);

    let gen = BomGenerator::new("3.0.0".to_string(), "test".to_string());
    let bom = gen.generate_from_manifest(&manifest, &registry).unwrap();

    assert_eq!(bom.extensions[0].components.len(), 1);
    assert_eq!(bom.extensions[0].components[0].name, "basic-tool");
}

// ═══════════════════════════════════════════════════════════════════════════
// Realistic scenario tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_realistic_multi_extension_bom_pipeline() {
    // Simulate a realistic Sindri deployment with multiple extension types
    let python = create_test_extension(
        "python",
        ExtensionCategory::Languages,
        InstallMethod::Mise,
        vec![
            runtime_tool("python", "3.13.0", BomSource::Mise),
            tool("uv", "0.9.0", BomSource::Mise),
        ],
        vec![CommandValidation {
            name: "python3".to_string(),
            version_flag: "--version".to_string(),
            expected_pattern: Some(r"Python 3\.\d+".to_string()),
        }],
    );

    let infra = create_test_extension(
        "infra-tools",
        ExtensionCategory::Devops,
        InstallMethod::Script,
        vec![
            tool("kubectl", "1.35.0", BomSource::Mise),
            tool("helm", "4.1.0", BomSource::Mise),
            tool("terraform", "1.14.0", BomSource::Mise),
            tool("pulumi", "3.219.0", BomSource::Script),
        ],
        vec![],
    );

    let docker = create_test_extension(
        "docker",
        ExtensionCategory::Devops,
        InstallMethod::Apt,
        vec![BomTool {
            name: "docker".to_string(),
            version: Some("dynamic".to_string()),
            source: BomSource::Apt,
            r#type: Some(BomToolType::Application),
            license: Some("Apache-2.0".to_string()),
            homepage: None,
            download_url: None,
            checksum: None,
            purl: None,
            cpe: None,
        }],
        vec![],
    );

    let registry = build_registry_with_extensions(vec![
        ("python", python),
        ("infra-tools", infra),
        ("docker", docker),
    ]);
    let manifest = build_manifest(vec![
        ("python", "1.0.0"),
        ("infra-tools", "1.0.0"),
        ("docker", "1.0.0"),
    ]);

    let gen = BomGenerator::new("3.0.0-beta.6".to_string(), "my-workspace".to_string());
    let bom = gen.generate_from_manifest(&manifest, &registry).unwrap();

    // Verify overall structure
    assert_eq!(bom.cli_version, "3.0.0-beta.6");
    assert_eq!(bom.config_name, "my-workspace");
    assert_eq!(bom.extensions.len(), 3);

    // Verify sorted order
    assert_eq!(bom.extensions[0].name, "docker");
    assert_eq!(bom.extensions[1].name, "infra-tools");
    assert_eq!(bom.extensions[2].name, "python");

    // Verify system components
    assert_eq!(bom.system_components[0].name, "sindri-cli");
    assert_eq!(bom.system_components[0].version, "3.0.0-beta.6");

    // Export to all formats
    let tmp = TempDir::new().unwrap();
    for (format, filename) in [
        (BomFormat::Json, "bom.json"),
        (BomFormat::Yaml, "bom.yaml"),
        (BomFormat::CycloneDx, "sbom.cdx.json"),
        (BomFormat::Spdx, "sbom.spdx.json"),
    ] {
        let path = tmp.path().join(filename);
        gen.write_bom(&bom, &path, format)
            .unwrap_or_else(|e| panic!("Failed to write {}: {}", filename, e));
        assert!(path.exists(), "File {} should exist", filename);
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.is_empty(), "File {} should not be empty", filename);
    }

    // Verify summary
    let summary = bom.summary();
    assert_eq!(summary.total_extensions, 3);
    assert!(summary.extensions_by_category.contains_key("devops"));
    assert!(summary.extensions_by_category.contains_key("languages"));
}

#[test]
fn test_bom_roundtrip_preserves_data() {
    let ext = create_test_extension(
        "roundtrip-test",
        ExtensionCategory::Testing,
        InstallMethod::Script,
        vec![BomTool {
            name: "test-tool".to_string(),
            version: Some("1.2.3".to_string()),
            source: BomSource::Script,
            r#type: Some(BomToolType::CliTool),
            license: Some("MIT".to_string()),
            homepage: Some("https://example.com".to_string()),
            download_url: Some("https://example.com/download".to_string()),
            checksum: None,
            purl: Some("pkg:generic/test-tool@1.2.3".to_string()),
            cpe: None,
        }],
        vec![],
    );

    let registry = build_registry_with_extensions(vec![("roundtrip-test", ext)]);
    let manifest = build_manifest(vec![("roundtrip-test", "1.0.0")]);

    let gen = BomGenerator::new("3.0.0".to_string(), "test".to_string());
    let bom = gen.generate_from_manifest(&manifest, &registry).unwrap();

    // Write JSON, load it back, compare
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("bom.json");
    gen.write_bom(&bom, &path, BomFormat::Json).unwrap();

    let loaded = BomGenerator::load_bom(&path, BomFormat::Json).unwrap();

    assert_eq!(loaded.cli_version, bom.cli_version);
    assert_eq!(loaded.extensions.len(), bom.extensions.len());
    assert_eq!(loaded.total_components, bom.total_components);

    let orig_comp = &bom.extensions[0].components[0];
    let loaded_comp = &loaded.extensions[0].components[0];
    assert_eq!(loaded_comp.name, orig_comp.name);
    assert_eq!(loaded_comp.version, orig_comp.version);
    assert_eq!(loaded_comp.license, orig_comp.license);
}
