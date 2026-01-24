# ADR 014: SBOM Generation with Industry Standards

**Status**: Accepted
**Date**: 2026-01-21
**Deciders**: Core Team
**Related**: [ADR-008: Extension Type System](008-extension-type-system-yaml-deserialization.md), [ADR-012: Registry and Manifest](012-registry-manifest-dual-state-architecture.md)

## Context

Software Bill of Materials (SBOM) generation is increasingly required for:

1. **Security Compliance**: Know what software is running in production
2. **Vulnerability Tracking**: Identify vulnerable dependencies
3. **License Compliance**: Track open-source licenses
4. **Supply Chain Security**: Verify software provenance
5. **Regulatory Requirements**: FDA, NIST, EU Cyber Resilience Act

Sindri environments include diverse components:

- Base OS packages (Ubuntu apt)
- Runtime environments (nodejs, python, go via mise)
- CLI tools (gh, k9s, lazydocker as binaries)
- npm/pip/cargo packages (application dependencies)
- Custom scripts and configurations

The bash implementation had no SBOM support. Users could manually track installed software, but:

- No machine-readable format
- No version tracking
- No license information
- No vulnerability scanning integration

SBOM Standards:

- **SPDX 2.3**: Linux Foundation standard, comprehensive but complex
- **CycloneDX 1.4**: OWASP standard, security-focused, simpler
- **SWID**: ISO standard, primarily for commercial software

Example SBOM use cases:

- **Security teams**: Scan for CVEs in production environments
- **Compliance teams**: Verify license compatibility
- **DevOps teams**: Track software versions across deployments
- **Audit teams**: Prove what software was running at specific time

## Decision

### Dual Standard Support: CycloneDX and SPDX

We implement **both CycloneDX 1.4 and SPDX 2.3** support, with CycloneDX as default:

**CycloneDX 1.4** (Default):

- Security-focused
- Simpler structure
- Better tool support (Dependency-Track, Grype, Trivy)
- Native vulnerability tracking

**SPDX 2.3** (Optional):

- Comprehensive metadata
- Better license tracking
- Industry standard for legal compliance
- Required by some enterprises

### Component Type Classification

Extensions and their dependencies are classified into SBOM component types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentType {
    Application,    // Full applications (e.g., k9s, lazydocker)
    Framework,      // Runtimes (e.g., nodejs, python, go)
    Library,        // Libraries and packages (e.g., npm packages)
    Container,      // Container images
    OperatingSystem, // Base OS
    Device,         // Hardware (rarely used)
    File,           // Config files, scripts
    Firmware,       // Firmware (rarely used)
}

impl ComponentType {
    /// Infer component type from install method
    pub fn from_install_method(install: &Install) -> Self {
        match install {
            Install::Mise { tool, .. } => {
                // Runtime environments
                if matches!(tool.as_str(), "nodejs" | "python" | "go" | "ruby" | "rust") {
                    ComponentType::Framework
                } else {
                    ComponentType::Application
                }
            }
            Install::Apt { .. } => {
                // System packages - could be apps, libraries, or OS components
                ComponentType::Library
            }
            Install::Binary { .. } => {
                // Pre-compiled executables
                ComponentType::Application
            }
            Install::Npm { .. } => {
                // npm packages are libraries
                ComponentType::Library
            }
            Install::Script { .. } => {
                // Custom scripts
                ComponentType::File
            }
            Install::Hybrid { .. } => {
                // Mixed type - default to application
                ComponentType::Application
            }
        }
    }
}
```

### PURL and CPE Support

Components include Package URL (PURL) and Common Platform Enumeration (CPE) identifiers:

**PURL** (Package URL):

```
pkg:npm/express@4.18.2
pkg:pypi/django@4.2.0
pkg:github/cli/cli@2.40.0
pkg:generic/k9s@0.31.0
```

**CPE** (Common Platform Enumeration):

```
cpe:2.3:a:nodejs:nodejs:20.11.0:*:*:*:*:*:*:*
cpe:2.3:a:python:python:3.12.0:*:*:*:*:*:*:*
```

**Implementation**:

```rust
pub struct ComponentIdentifiers {
    pub purl: Option<String>,
    pub cpe: Option<String>,
}

impl ComponentIdentifiers {
    /// Generate PURL from install method
    pub fn generate_purl(install: &Install, name: &str, version: &str) -> Option<String> {
        match install {
            Install::Mise { tool, version } => {
                // Map mise tools to PURL types
                let purl_type = match tool.as_str() {
                    "nodejs" => "npm",
                    "python" => "pypi",
                    "go" => "golang",
                    "ruby" => "gem",
                    "rust" => "cargo",
                    _ => return None,
                };
                Some(format!("pkg:{}@{}", purl_type, version))
            }
            Install::Npm { package, version } => {
                let ver = version.as_deref().unwrap_or("latest");
                Some(format!("pkg:npm/{}@{}", package, ver))
            }
            Install::Binary { url, .. } => {
                // Try to extract GitHub repo from URL
                if url.contains("github.com") {
                    if let Some(purl) = Self::parse_github_purl(url, version) {
                        return Some(purl);
                    }
                }
                // Generic PURL
                Some(format!("pkg:generic/{}@{}", name, version))
            }
            _ => Some(format!("pkg:generic/{}@{}", name, version)),
        }
    }

    fn parse_github_purl(url: &str, version: &str) -> Option<String> {
        // Parse GitHub URL: https://github.com/owner/repo/releases/download/v1.2.3/...
        let re = regex::Regex::new(r"github\.com/([^/]+)/([^/]+)").ok()?;
        let caps = re.captures(url)?;
        let owner = caps.get(1)?.as_str();
        let repo = caps.get(2)?.as_str();
        Some(format!("pkg:github/{}/{}@{}", owner, repo, version))
    }

    /// Generate CPE from install method (best effort)
    pub fn generate_cpe(install: &Install, name: &str, version: &str) -> Option<String> {
        match install {
            Install::Mise { tool, version } => {
                // Well-known CPEs for major runtimes
                match tool.as_str() {
                    "nodejs" => Some(format!(
                        "cpe:2.3:a:nodejs:nodejs:{}:*:*:*:*:*:*:*",
                        version
                    )),
                    "python" => Some(format!(
                        "cpe:2.3:a:python:python:{}:*:*:*:*:*:*:*",
                        version
                    )),
                    "go" => Some(format!(
                        "cpe:2.3:a:golang:go:{}:*:*:*:*:*:*:*",
                        version
                    )),
                    _ => None,
                }
            }
            _ => None,  // CPE not available for most components
        }
    }
}
```

### CycloneDX 1.4 Format

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycloneDxBom {
    #[serde(rename = "bomFormat")]
    pub bom_format: String,  // "CycloneDX"

    #[serde(rename = "specVersion")]
    pub spec_version: String,  // "1.4"

    pub version: u32,  // BOM version (increments with updates)

    #[serde(rename = "serialNumber")]
    pub serial_number: String,  // URN UUID

    pub metadata: CycloneDxMetadata,

    pub components: Vec<CycloneDxComponent>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<CycloneDxDependency>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycloneDxMetadata {
    pub timestamp: DateTime<Utc>,

    pub tools: Vec<CycloneDxTool>,

    pub component: CycloneDxComponent,  // The Sindri environment itself
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycloneDxTool {
    pub vendor: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycloneDxComponent {
    #[serde(rename = "type")]
    pub component_type: String,  // "application", "library", "framework", etc.

    pub name: String,
    pub version: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub purl: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpe: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub licenses: Option<Vec<CycloneDxLicense>>,

    #[serde(rename = "bom-ref")]
    pub bom_ref: String,  // Unique identifier for dependencies
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycloneDxLicense {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,  // SPDX license ID

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,  // License name if not SPDX
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycloneDxDependency {
    #[serde(rename = "ref")]
    pub ref_id: String,  // bom-ref of the component

    #[serde(rename = "dependsOn")]
    pub depends_on: Vec<String>,  // List of bom-refs this depends on
}

/// Generate CycloneDX BOM from installed extensions
pub fn generate_cyclonedx_bom(manifest: &Manifest) -> Result<CycloneDxBom> {
    let timestamp = Utc::now();
    let serial_number = format!("urn:uuid:{}", uuid::Uuid::new_v4());

    // Metadata
    let metadata = CycloneDxMetadata {
        timestamp,
        tools: vec![CycloneDxTool {
            vendor: "Sindri".to_string(),
            name: "sindri-cli".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }],
        component: CycloneDxComponent {
            component_type: "application".to_string(),
            name: "sindri-environment".to_string(),
            version: manifest.version.clone(),
            description: Some("Sindri cloud development environment".to_string()),
            purl: None,
            cpe: None,
            licenses: Some(vec![CycloneDxLicense {
                id: Some("MIT".to_string()),
                name: None,
            }]),
            bom_ref: "sindri-environment".to_string(),
        },
    };

    // Convert installed extensions to components
    let components: Vec<CycloneDxComponent> = manifest
        .extensions
        .values()
        .filter(|ext| ext.state == ExtensionState::Installed)
        .map(|ext| {
            let component_type = ComponentType::from_install_method(&ext.install_method)
                .to_string();

            let purl = ComponentIdentifiers::generate_purl(
                &ext.install_method,
                &ext.name,
                &ext.version,
            );

            let cpe = ComponentIdentifiers::generate_cpe(
                &ext.install_method,
                &ext.name,
                &ext.version,
            );

            CycloneDxComponent {
                component_type,
                name: ext.name.clone(),
                version: ext.version.clone(),
                description: None,  // Could fetch from registry
                purl,
                cpe,
                licenses: None,  // Could fetch from extension metadata
                bom_ref: ext.name.clone(),
            }
        })
        .collect();

    // Generate dependency graph
    let dependencies: Vec<CycloneDxDependency> = manifest
        .extensions
        .values()
        .filter(|ext| ext.state == ExtensionState::Installed)
        .map(|ext| CycloneDxDependency {
            ref_id: ext.name.clone(),
            depends_on: ext.dependencies.clone(),
        })
        .collect();

    let bom = CycloneDxBom {
        bom_format: "CycloneDX".to_string(),
        spec_version: "1.4".to_string(),
        version: 1,
        serial_number,
        metadata,
        components,
        dependencies: Some(dependencies),
    };

    Ok(bom)
}
```

### SPDX 2.3 Format (Optional)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpdxDocument {
    #[serde(rename = "spdxVersion")]
    pub spdx_version: String,  // "SPDX-2.3"

    #[serde(rename = "dataLicense")]
    pub data_license: String,  // "CC0-1.0"

    #[serde(rename = "SPDXID")]
    pub spdx_id: String,  // "SPDXRef-DOCUMENT"

    pub name: String,
    pub namespace: String,  // Unique URI

    #[serde(rename = "creationInfo")]
    pub creation_info: SpdxCreationInfo,

    pub packages: Vec<SpdxPackage>,

    pub relationships: Vec<SpdxRelationship>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpdxCreationInfo {
    pub created: DateTime<Utc>,
    pub creators: Vec<String>,
    #[serde(rename = "licenseListVersion")]
    pub license_list_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpdxPackage {
    #[serde(rename = "SPDXID")]
    pub spdx_id: String,

    pub name: String,
    pub version: String,

    #[serde(rename = "filesAnalyzed")]
    pub files_analyzed: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(rename = "downloadLocation")]
    pub download_location: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,

    #[serde(rename = "licenseConcluded")]
    pub license_concluded: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub externalRefs: Option<Vec<SpdxExternalRef>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpdxExternalRef {
    #[serde(rename = "referenceCategory")]
    pub reference_category: String,  // "PACKAGE-MANAGER", "SECURITY"

    #[serde(rename = "referenceType")]
    pub reference_type: String,  // "purl", "cpe23Type"

    #[serde(rename = "referenceLocator")]
    pub reference_locator: String,  // The actual PURL or CPE
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpdxRelationship {
    #[serde(rename = "spdxElementId")]
    pub spdx_element_id: String,

    #[serde(rename = "relationshipType")]
    pub relationship_type: String,  // "DEPENDS_ON", "CONTAINS"

    #[serde(rename = "relatedSpdxElement")]
    pub related_spdx_element: String,
}
```

### Multiple Export Formats

```rust
pub enum SbomFormat {
    CycloneDxJson,
    CycloneDxXml,
    SpdxJson,
    SpdxRdf,
}

pub fn export_sbom(manifest: &Manifest, format: SbomFormat) -> Result<String> {
    match format {
        SbomFormat::CycloneDxJson => {
            let bom = generate_cyclonedx_bom(manifest)?;
            Ok(serde_json::to_string_pretty(&bom)?)
        }
        SbomFormat::CycloneDxXml => {
            let bom = generate_cyclonedx_bom(manifest)?;
            Ok(quick_xml::se::to_string(&bom)?)
        }
        SbomFormat::SpdxJson => {
            let doc = generate_spdx_document(manifest)?;
            Ok(serde_json::to_string_pretty(&doc)?)
        }
        SbomFormat::SpdxRdf => {
            let doc = generate_spdx_document(manifest)?;
            Ok(generate_spdx_rdf(&doc)?)
        }
    }
}
```

### CLI Integration

```bash
# Generate CycloneDX SBOM (default)
sindri sbom generate

# Generate SPDX SBOM
sindri sbom generate --format spdx

# Export to file
sindri sbom generate --output sbom.json

# Export with specific format
sindri sbom generate --format cyclonedx-xml --output sbom.xml

# Validate SBOM
sindri sbom validate sbom.json

# Compare with previous SBOM
sindri sbom diff sbom-old.json sbom-new.json
```

## Consequences

### Positive

1. **Industry Standards**: CycloneDX and SPDX are widely adopted
2. **Security Integration**: Tools like Grype, Trivy can scan CycloneDX SBOMs
3. **Compliance Ready**: Meets regulatory requirements (NIST, FDA, EU CRA)
4. **Machine Readable**: JSON/XML formats enable automation
5. **Dependency Tracking**: Full dependency graph included
6. **Vulnerability Scanning**: PURL enables CVE lookup
7. **License Compliance**: Track open-source licenses
8. **Version Control**: SBOM can be committed to git for history
9. **Audit Trail**: Timestamp and tool information included
10. **Flexibility**: Multiple export formats for different tools

### Negative

1. **Complexity**: SBOM generation adds ~1000 lines of code
2. **Maintenance**: Must keep up with SBOM standard evolution
3. **Completeness**: Hard to get 100% accurate component metadata
4. **License Detection**: Manual license specification required
5. **CPE Availability**: Not all components have CPEs
6. **File Size**: SBOM can be 100KB+ for large environments
7. **Update Frequency**: SBOM must be regenerated on every extension change

### Neutral

1. **Format Choice**: CycloneDX vs SPDX depends on use case
2. **Storage**: SBOM can be stored in git or artifact registry
3. **Integration**: Some CI/CD tools require specific SBOM formats

## Alternatives Considered

### 1. Custom SBOM Format Only

**Description**: Create Sindri-specific SBOM format instead of industry standards.

**Pros**:

- Full control over format
- Simpler implementation
- Smaller file size

**Cons**:

- No tool support (scanners, validators)
- Not recognized by compliance frameworks
- Reinventing the wheel
- No ecosystem integration

**Rejected**: Industry standards provide ecosystem benefits.

### 2. CycloneDX Only (No SPDX)

**Description**: Support only CycloneDX, drop SPDX support.

**Pros**:

- Simpler implementation (one format)
- CycloneDX is more modern
- Better security tool support

**Cons**:

- Some enterprises require SPDX
- SPDX better for legal compliance
- Limits flexibility

**Partially Adopted**: CycloneDX is default, SPDX is optional.

### 3. SPDX Only (No CycloneDX)

**Description**: Support only SPDX, drop CycloneDX support.

**Pros**:

- SPDX is older, more established
- Better license tracking
- Linux Foundation standard

**Cons**:

- Worse security tool support
- More complex format
- Slower ecosystem adoption

**Rejected**: CycloneDX has better security tooling.

### 4. No PURL/CPE Support

**Description**: Skip PURL and CPE generation, only include name/version.

**Pros**:

- Simpler implementation
- No identifier parsing logic

**Cons**:

- Limits vulnerability scanning
- Worse tool integration
- Missing critical metadata

**Rejected**: PURL/CPE are essential for security scanning.

### 5. Runtime Dependency Scanning

**Description**: Scan running processes to generate SBOM instead of using manifest.

**Pros**:

- Captures all running software
- No need for manifest tracking
- Discovers transitive dependencies

**Cons**:

- Complex implementation (process scanning)
- Inconsistent results (depends on what's running)
- Misses installed-but-not-running components
- Security concerns (process enumeration)

**Rejected**: Manifest-based approach is more reliable and consistent.

## Compliance

- ✅ CycloneDX 1.4 support
- ✅ SPDX 2.3 support
- ✅ PURL generation for all component types
- ✅ CPE generation for major runtimes
- ✅ Component type classification
- ✅ Dependency graph tracking
- ✅ Multiple export formats (JSON, XML)
- ✅ CLI integration with generate/validate/diff commands
- ✅ 100% test coverage for SBOM generation

## Notes

SBOM generation is increasingly required by regulations:

- **NIST SSDF**: Secure Software Development Framework requires SBOMs
- **FDA**: Medical device software must include SBOMs
- **EU Cyber Resilience Act**: CE marking requires software transparency
- **US Executive Order 14028**: Federal software requires SBOMs

The choice to support both CycloneDX and SPDX provides maximum flexibility for different compliance needs. CycloneDX is better for security use cases, SPDX is better for legal/licensing use cases.

PURL and CPE generation is best-effort - not all components have well-defined identifiers. We prioritize common cases (npm, pypi, github) and fall back to generic PURLs.

Future enhancements:

- Automatic license detection from package metadata
- Integration with vulnerability databases (NVD, OSV)
- SBOM diff visualization (what changed between versions)
- SBOM signing for integrity verification

## Related Decisions

- [ADR-008: Extension Type System](008-extension-type-system-yaml-deserialization.md) - Component metadata
- [ADR-011: Multi-Method Installation](011-multi-method-extension-installation.md) - Component types from install methods
- [ADR-012: Registry and Manifest Architecture](012-registry-manifest-dual-state-architecture.md) - Source of installed components
- [ADR-013: Schema Validation](013-schema-validation-strategy.md) - Validates SBOM structure
