# ADR 042: Bill of Materials (BOM) Capability Architecture

## Status

Accepted

## Context

Sindri v3 manages 50+ extensions that collectively install hundreds of software components (runtimes, CLI tools, libraries, package managers). Without a centralized inventory system, there is:

1. **No software inventory visibility**: Operators cannot determine what software is installed, at what version, or under what license
2. **No compliance reporting**: Enterprise environments require Software Bill of Materials (SBOM) for security auditing and regulatory compliance (Executive Order 14028, NTIA SBOM guidance)
3. **No version drift detection**: Extension install scripts may install different versions than what the extension.yaml declares
4. **No export to industry standards**: No ability to feed Sindri's component data into vulnerability scanning pipelines (Grype, Trivy, Snyk)

### Industry Standards

The BOM capability must align with established SBOM standards:

- **CycloneDX 1.4** (OWASP): JSON-based component inventory with PURL and license support
- **SPDX 2.3** (Linux Foundation / ISO 5962): Compliance-focused format with download locations and concluded licenses
- **NTIA Minimum Elements**: Supplier, component name, version, unique identifier, dependency relationships, author, and timestamp

### Requirements

**Functional:**

- Generate a complete BOM from installed extensions and their declared tools
- Export to CycloneDX 1.4 and SPDX 2.3 formats
- Support JSON and YAML native export
- Allow per-extension BOM inspection
- Component filtering by type and extension
- Load BOM from file for comparison/auditing

**Non-Functional:**

- BOM generation must complete in < 2 seconds for 50 extensions
- No network access required for generation (offline-capable)
- BOM accuracy: versions in BOM must match what install scripts actually install

## Decision

### Architecture

Implement the BOM system as a layered architecture across three crates:

```
┌─────────────────────────────────────────────────────┐
│  sindri (CLI)                                       │
│  └── commands/bom.rs                                │
│      ├── generate  ─── BomGenerator                 │
│      ├── show      ─── ExtensionBom inspection      │
│      ├── list      ─── Component filtering/display  │
│      └── export    ─── CycloneDX / SPDX / JSON     │
├─────────────────────────────────────────────────────┤
│  sindri-extensions                                  │
│  └── bom.rs                                         │
│      ├── BomGenerator      (generation engine)      │
│      ├── BillOfMaterials   (document model)         │
│      ├── ExtensionBom      (per-extension entry)    │
│      ├── Component         (software component)     │
│      └── export_cyclonedx / export_spdx             │
├─────────────────────────────────────────────────────┤
│  sindri-core                                        │
│  └── types/extension_types.rs                       │
│      ├── BomConfig         (extension.yaml input)   │
│      ├── BomTool           (declared tool)          │
│      ├── BomSource         (mise/apt/npm/pip/...)   │
│      └── BomToolType       (runtime/cli-tool/...)   │
└─────────────────────────────────────────────────────┘
```

### Data Flow

1. **Input**: Extension.yaml `bom` section declares tools with versions, sources, licenses
2. **Enrichment**: Validation commands provide fallback component detection
3. **Generation**: `BomGenerator` combines manifest (installed state) + registry (metadata) + extension definitions (BOM configs)
4. **Output**: `BillOfMaterials` document with extensions, components, and system components
5. **Export**: Serialized to JSON, YAML, CycloneDX 1.4, or SPDX 2.3

### Version Strategy

Three categories of version tracking:

| Category              | Example                     | BOM Value           | Rationale                                           |
| --------------------- | --------------------------- | ------------------- | --------------------------------------------------- |
| **Explicitly Pinned** | kubectl 1.35.0              | `"1.35.0"`          | Exact version installed by script                   |
| **Semantic Channel**  | Rust stable, Node LTS       | `"stable"`, `"lts"` | Intentional tracking of release channels            |
| **Exceptional**       | Docker (apt), npm (bundled) | `"dynamic"`         | Cannot be pinned reliably; documented with comments |

### BOM-to-Install-Script Synchronization

**Critical principle**: The version declared in `extension.yaml` `bom.tools[].version` MUST match what the install script actually installs. This is enforced by:

1. Manual audit and sync during version pinning (Phase 2)
2. Install script variables using the same version strings
3. Verification via `sindri bom show <extension>` post-install

### Multi-Mode Deployment Support

The BOM system supports three deployment modes transparently:

| Mode            | Path Pattern                                           | Use Case                |
| --------------- | ------------------------------------------------------ | ----------------------- |
| **Development** | `v3/extensions/{name}/extension.yaml`                  | Source tree development |
| **Bundled**     | `/opt/sindri/extensions/{name}/extension.yaml`         | Container images        |
| **Downloaded**  | `~/.sindri/extensions/{name}/{version}/extension.yaml` | User installations      |

### CLI Commands

```
sindri bom generate [--json] [--detect-versions]
sindri bom show <extension> [--json]
sindri bom list [extension] [--component-type <type>] [--json]
sindri bom export --format <json|yaml|cyclonedx|spdx> --output <path> [--force]
```

### Extension Schema Integration

The `bom` section in `extension.yaml` is the primary source of truth:

```yaml
bom:
  tools:
    - name: kubectl
      version: "1.35.0"
      source: mise
      type: cli-tool
      license: Apache-2.0
      homepage: https://kubernetes.io
      purl: "pkg:github/kubernetes/kubectl@1.35.0"
  files:
    - path: ~/.kube/config
      type: config
```

Supported `source` values: `mise`, `apt`, `npm`, `pip`, `binary`, `script`, `github-release`

Supported `type` values: `runtime`, `compiler`, `package-manager`, `cli-tool`, `library`, `framework`, `database`, `server`, `utility`, `application`

## Consequences

### Positive

- **Compliance ready**: CycloneDX and SPDX exports integrate directly with vulnerability scanners (Grype, Trivy, Snyk) and compliance platforms
- **Full visibility**: Operators can inventory all installed software across all extensions with a single command
- **Audit trail**: BOM timestamps and CLI version tracking enable point-in-time auditing
- **Version accuracy**: BOM-to-install-script synchronization ensures reported versions match reality
- **Extensible**: New export formats can be added without changing the core model
- **Offline-capable**: BOM generation requires no network access after initial extension installation

### Negative

- **Manual sync burden**: Version pinning requires manual synchronization between install scripts and extension.yaml BOM entries; this is inherently error-prone and will need periodic auditing
- **No runtime detection**: BOM versions reflect declared versions, not necessarily what is actually running; the `--detect-versions` flag provides limited runtime detection via mise.toml parsing
- **Exceptional cases**: Some tools (apt-managed, bundled runtimes) cannot be precisely version-pinned, requiring documentation rather than exact versions

### Neutral

- **Schema evolution**: The `bom` section is optional in extension.yaml; extensions without BOM configs fall back to validation command detection (generating "detected" version placeholders)
- **No signature verification**: BOM integrity verification (checksums, signing) is not implemented in this phase but the `checksum` field exists in the schema for future use

## Implementation Notes

### Testing Strategy

- **Unit tests** (`sindri-extensions/tests/bom_generation_tests.rs`): 50+ tests covering BOM generation, component extraction, type mapping, export formats, serialization roundtrips
- **Integration tests** (`sindri/tests/bom_cli_tests.rs`): 15+ tests exercising the full BomGenerator pipeline with fixture extensions and manifest data
- **Test builders** (`tests/common/bom_builders.rs`): Fluent builders for BomTool, BomConfig, Component, ExtensionBom, and BillOfMaterials

### Performance

BOM generation for 50 extensions with 200+ components completes in < 1 second on modern hardware. No async I/O is needed for generation; async is only used for optional mise.toml version detection.

## References

### Code

- `v3/crates/sindri/src/commands/bom.rs` - CLI command implementation
- `v3/crates/sindri-extensions/src/bom.rs` - BOM engine (generator, export, types)
- `v3/crates/sindri-core/src/types/extension_types.rs` - BomConfig, BomTool types
- `v3/schemas/extension.schema.json` - Extension schema with BOM section

### Standards

- [CycloneDX 1.4 Specification](https://cyclonedx.org/specification/overview/)
- [SPDX 2.3 Specification](https://spdx.github.io/spdx-spec/v2.3/)
- [NTIA SBOM Minimum Elements](https://www.ntia.gov/files/ntia/publications/sbom_minimum_elements_report.pdf)
- [Executive Order 14028 - Improving the Nation's Cybersecurity](https://www.whitehouse.gov/briefing-room/presidential-actions/2021/05/12/executive-order-on-improving-the-nations-cybersecurity/)

### Related ADRs

- ADR-032: Extension Configure Processing (post-install integration point)

## Decision Date

2026-02-09

## Authors

Sindri Core Team
