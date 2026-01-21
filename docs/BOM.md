# Bill of Materials (BOM) System

Sindri's extension system includes comprehensive Bill of Materials (BOM) tracking for all installed software. This enables security auditing, compliance, vulnerability scanning, and reproducible environments.

## Overview

The BOM system automatically tracks:

- Software name and version
- Installation source (mise, apt, npm, binary, script)
- Software type (runtime, compiler, cli-tool, etc.)
- License information
- Homepage and download URLs
- Package URLs (PURL) for SBOM compatibility
- Common Platform Enumeration (CPE) for vulnerability scanning

## Usage

### View Complete BOM

Show BOM for all installed extensions:

```bash
./v2/cli/extension-manager bom
```

### View Extension-Specific BOM

Show BOM for a single extension:

```bash
./v2/cli/extension-manager bom nodejs
```

### Export Formats

#### YAML (default)

```bash
./v2/cli/extension-manager bom --format yaml
```

#### JSON

```bash
./v2/cli/extension-manager bom --format json
```

#### CSV

```bash
./v2/cli/extension-manager bom --format csv > bom.csv
```

#### CycloneDX SBOM

Industry-standard SBOM format:

```bash
./v2/cli/extension-manager bom --format cyclonedx > sbom.cdx.json
```

#### SPDX SBOM

Software Package Data Exchange format:

```bash
./v2/cli/extension-manager bom --format spdx > sbom.spdx
```

### Regenerate BOMs

Force regeneration of all BOMs:

```bash
./v2/cli/extension-manager bom-regenerate
```

## BOM Storage

BOMs are stored in `/workspace/.system/bom/`:

- `<extension-name>.bom.yaml` - Individual extension BOMs
- `complete.bom.yaml` - Aggregate BOM for all extensions

## Declaring BOM in Extensions

Extensions can explicitly declare their BOM in `extension.yaml`:

### Example: Explicit BOM Declaration

```yaml
metadata:
  name: nodejs
  version: 1.0.0
  description: Node.js LTS via mise
  category: language

install:
  method: mise
  mise:
    configFile: mise.toml

# Explicit BOM declaration
bom:
  tools:
    - name: node
      version: dynamic # Resolved at runtime
      source: mise
      type: runtime
      license: MIT
      homepage: https://nodejs.org
      purl: pkg:generic/nodejs

    - name: npm
      version: dynamic
      source: mise
      type: package-manager
      license: Artistic-2.0
      homepage: https://www.npmjs.com
      purl: pkg:npm/npm

  files:
    - path: .config/mise/conf.d/nodejs.toml
      type: config
```

### BOM Tool Properties

- `name` (required): Tool/package name
- `version`: Version number or "dynamic" if resolved at runtime
- `source` (required): Installation method (mise, apt, npm, binary, script)
- `type`: Software type (runtime, compiler, package-manager, cli-tool, library, framework, database, server, utility)
- `license`: Software license (SPDX identifier)
- `homepage`: Project homepage URL
- `downloadUrl`: Direct download URL (for binaries)
- `purl`: Package URL for SBOM compatibility
- `cpe`: Common Platform Enumeration for vulnerability scanning
- `checksum`: File integrity verification
  - `algorithm`: sha256, sha512, md5
  - `value`: Checksum value

### Automatic BOM Discovery

If no explicit `bom` section is provided, the system automatically derives the BOM from the install method:

- **mise**: Parses `mise.toml` for tools
- **apt**: Uses package list from `install.apt.packages`
- **npm**: Uses package list from `install.npm.packages`
- **binary**: Uses binary downloads from `install.binary.downloads`
- **script/hybrid**: Requires explicit BOM declaration

## Version Resolution

Dynamic versions are resolved at installation time:

1. If `version: dynamic` is specified, the system attempts to resolve the actual version
2. Common version flags are tried: `--version`, `-v`, `version`
3. Version strings are parsed using regex: `\d+\.\d+\.\d+`
4. If resolution fails, version is set to "unknown"

## BOM Output Formats

### YAML Format

```yaml
# Bill of Materials for extension: nodejs
# Generated: 2025-11-21T10:30:00Z

extension:
  name: nodejs
  version: 1.0.0
  category: language
  description: "Node.js LTS via mise"
  installed: 2025-11-21T09:00:00Z

software:
  - name: node
    version: 22.0.0
    source: mise
    type: runtime
    license: MIT
    homepage: https://nodejs.org
    purl: pkg:generic/nodejs

  - name: npm
    version: 10.0.0
    source: mise
    type: package-manager
    license: Artistic-2.0
    homepage: https://www.npmjs.com
    purl: pkg:npm/npm

files: []
```

### Aggregate BOM Format

```yaml
# Aggregate Bill of Materials
# Generated: 2025-11-21T10:30:00Z
# Sindri Version: 1.0.0

extensions:
  - extension:
      name: nodejs
      version: 1.0.0
      category: language
    software:
      - name: node
        version: 22.0.0
        source: mise
        type: runtime

  - extension:
      name: python
      version: 1.0.0
      category: language
    software:
      - name: python
        version: 3.13.0
        source: mise
        type: runtime
```

### CSV Format

```csv
Extension,Software,Version,Source,Type,License
nodejs,node,22.0.0,mise,runtime,MIT
nodejs,npm,10.0.0,mise,package-manager,Artistic-2.0
python,python,3.13.0,mise,runtime,PSF-2.0
docker,docker,27.0.0,apt,server,Apache-2.0
```

### CycloneDX SBOM Format

```json
{
  "bomFormat": "CycloneDX",
  "specVersion": "1.4",
  "version": 1,
  "metadata": {
    "timestamp": "2025-11-21T10:30:00Z",
    "component": {
      "type": "application",
      "name": "sindri-workspace",
      "version": "1.0.0"
    }
  },
  "components": [
    {
      "type": "library",
      "name": "node",
      "version": "22.0.0",
      "licenses": [
        {
          "license": {
            "id": "MIT"
          }
        }
      ]
    }
  ]
}
```

### SPDX Format

```text
SPDXVersion: SPDX-2.3
DataLicense: CC0-1.0
SPDXID: SPDXRef-DOCUMENT
DocumentName: Sindri-Workspace-BOM
DocumentNamespace: https://sindri.dev/spdxdocs/workspace-1732186200
Creator: Tool: sindri-extension-manager

# Packages

PackageName: node
SPDXID: SPDXRef-Package-1
PackageVersion: 22.0.0
PackageDownloadLocation: NOASSERTION
FilesAnalyzed: false
PackageLicenseConcluded: MIT
PackageLicenseDeclared: MIT
PackageCopyrightText: NOASSERTION
```

## Use Cases

### Security Auditing

Export BOM and scan for vulnerabilities:

```bash
# Export as CycloneDX
./v2/cli/extension-manager bom --format cyclonedx > sbom.cdx.json

# Scan with dependency-track, grype, or other SBOM scanners
grype sbom:sbom.cdx.json
```

### Compliance

Generate SPDX SBOM for compliance requirements:

```bash
./v2/cli/extension-manager bom --format spdx > sbom.spdx
```

### Environment Reproducibility

Export complete BOM to recreate exact environment:

```bash
./v2/cli/extension-manager bom --format yaml > environment-bom.yaml
```

### License Tracking

Export CSV for license compliance review:

```bash
./v2/cli/extension-manager bom --format csv | grep -v "^Extension" | cut -d',' -f6 | sort -u
```

## Integration with CI/CD

### GitHub Actions Example

```yaml
- name: Generate SBOM
  run: |
    ./cli/extension-manager bom --format cyclonedx > sbom.cdx.json

- name: Upload SBOM Artifact
  uses: actions/upload-artifact@v3
  with:
    name: sbom
    path: sbom.cdx.json
```

## Best Practices

1. **Explicit BOM Declarations**: Prefer explicit `bom` sections in extension.yaml for accuracy
2. **Include License Information**: Always specify licenses for compliance
3. **Use PURLs**: Include Package URLs for SBOM tool compatibility
4. **Version Pinning**: Use specific versions instead of "dynamic" when possible
5. **Regular Regeneration**: Run `bom-regenerate` after software updates
6. **SBOM Scanning**: Integrate with vulnerability scanners like Grype, Trivy, or Dependency-Track

## Schema Validation

BOMs are validated against `extension.schema.json`. All BOM declarations must conform to:

- `tools[]` array with required `name` and `source`
- `files[]` array with required `path` and `type`
- Valid enum values for `source`, `type`, `checksum.algorithm`

## Architecture

BOM generation is handled by the `bom.sh` module:

- **generate_extension_bom()** - Generate BOM for single extension
- **generate_aggregate_bom()** - Generate complete BOM
- **resolve_dynamic_versions()** - Resolve version numbers at runtime
- **export_bom_format()** - Export to various formats
- **derive_bom_from_install()** - Auto-discover BOM from install method

BOMs are automatically generated:

1. During extension installation (via `executor.sh`)
2. On demand via `extension-manager bom` command
3. During `bom-regenerate` command

## Related Documentation

- [Extension Development](./EXTENSIONS.md)
- [Extension Schema](../v2/docker/lib/schemas/extension.schema.json)
- [Security Best Practices](./SECURITY.md)
