# ADR 031: Packer VM Provisioning Architecture

**Status**: Accepted
**Date**: 2026-01-25
**Deciders**: Core Team
**Related**: [ADR-002: Provider Abstraction Layer](002-provider-abstraction-layer.md), [ADR-003: Template-Based Configuration](003-template-based-configuration.md)

## Context

Sindri v3 supports deployment to containerized environments (Docker, Fly.io, E2B, Kubernetes) but lacked native VM image building capabilities. Users needed to:

1. Build custom VM images for cloud providers (AWS AMI, Azure Managed Images, GCP Compute Images)
2. Pre-configure development environments with Sindri and extensions
3. Deploy from pre-built images for faster startup times
4. Apply security hardening (CIS benchmarks) consistently across clouds

HashiCorp Packer is the industry standard for multi-cloud image building, but integrating it required architectural decisions about:

- How to fit VM provisioning into Sindri's provider abstraction
- Template rendering for HCL2 across five cloud providers
- CLI command structure for image lifecycle management
- Security and compliance integration

## Decision

### 1. Unified Packer Provider with Cloud Selection

Rather than creating five separate providers (aws-vm, azure-vm, etc.), we implemented a **single `packer` provider** with a `cloud` attribute:

```yaml
provider: packer
packer:
  cloud: aws # aws | azure | gcp | oci | alibaba
  region: us-west-2
  instance_type: t3.large
```

**Rationale**: This mirrors DevPod's multi-backend approach and keeps the provider list manageable while allowing cloud-specific configuration.

### 2. PackerProvider Trait Extension

Extended the provider abstraction with image-building operations:

```rust
#[async_trait]
pub trait PackerProvider: Send + Sync {
    // Identity
    fn cloud_name(&self) -> &'static str;

    // Template generation
    fn generate_template(&self, config: &PackerConfig) -> Result<String>;

    // Image lifecycle
    async fn build_image(&self, config: &PackerConfig, opts: BuildOptions) -> Result<BuildResult>;
    async fn validate_template(&self, config: &PackerConfig) -> Result<ValidationResult>;
    async fn list_images(&self, config: &PackerConfig) -> Result<Vec<ImageInfo>>;
    async fn delete_image(&self, config: &PackerConfig, image_id: &str) -> Result<()>;
    async fn get_image(&self, config: &PackerConfig, image_id: &str) -> Result<ImageInfo>;

    // Deployment from images
    async fn deploy_from_image(&self, image_id: &str, config: &PackerConfig) -> Result<DeployFromImageResult>;
    async fn find_cached_image(&self, config: &PackerConfig) -> Result<Option<String>>;

    // Prerequisites
    fn check_cloud_prerequisites(&self) -> Result<CloudPrerequisiteStatus>;
}
```

### 3. Tera-Based HCL2 Template System

Used rust-embed + Tera for cloud-specific template rendering:

```
sindri-packer/src/templates/
├── hcl/
│   ├── aws.pkr.hcl.tera
│   ├── azure.pkr.hcl.tera
│   ├── gcp.pkr.hcl.tera
│   ├── oci.pkr.hcl.tera
│   └── alibaba.pkr.hcl.tera
└── scripts/
    ├── init.sh.tera
    ├── install-sindri.sh.tera
    ├── cleanup.sh.tera
    └── security-hardening.sh.tera
```

**Key Decision**: Escape Packer's `{{ .Path }}` syntax within Tera templates using `{% raw %}...{% endraw %}` blocks to avoid parsing conflicts.

### 4. CLI Command Structure

Added `sindri packer` subcommands mirroring the trait:

```
sindri packer build     # Build VM image
sindri packer validate  # Validate template
sindri packer list      # List images
sindri packer delete    # Delete image
sindri packer deploy    # Deploy from pre-built image
sindri packer doctor    # Check prerequisites
sindri packer init      # Generate template files
```

### 5. Security Integration

Embedded security hardening directly into provisioning scripts:

- **CIS Benchmark Hardening**: SSH configuration, password policies, audit system, AppArmor
- **OpenSCAP Scanning**: Compliance scanning with multiple profile support (CIS Level 1/2, STIG)
- **InSpec Testing**: Post-build compliance verification via GitHub Actions
- **Sensitive Data Cleanup**: Remove SSH keys, cloud metadata, bash history before snapshotting

## Consequences

### Positive

1. **Unified Experience**: Same `sindri deploy/connect/status/destroy` workflow works with VM images
2. **Multi-Cloud Parity**: Consistent provisioning across AWS, Azure, GCP, OCI, Alibaba
3. **Pre-built Image Support**: `image_id` parameter allows deploying from cached images
4. **Security by Default**: CIS hardening and compliance scanning built into the pipeline
5. **CI/CD Ready**: GitHub Actions workflows for automated image building and testing

### Negative

1. **Template Complexity**: Maintaining five HCL2 templates with cloud-specific syntax
2. **Build Time**: VM image builds take 10-30 minutes vs seconds for containers
3. **Cloud CLI Dependencies**: Requires AWS CLI, Azure CLI, gcloud, OCI CLI, or aliyun CLI

### Mitigations

- **Template Testing**: Each template includes validation in CI
- **Image Caching**: `find_cached_image()` avoids unnecessary rebuilds
- **Doctor Command**: Comprehensive prerequisite checking before builds

## Implementation

### Crate Structure

```
crates/sindri-packer/
├── src/
│   ├── lib.rs           # Public exports, factory function
│   ├── traits.rs        # PackerProvider trait definition
│   ├── aws.rs           # AWS EC2 AMI provider
│   ├── azure.rs         # Azure Managed Images provider
│   ├── gcp.rs           # GCP Compute Engine provider
│   ├── oci.rs           # Oracle Cloud provider
│   ├── alibaba.rs       # Alibaba Cloud provider
│   ├── templates/       # Embedded HCL2 + script templates
│   └── utils.rs         # Packer CLI helpers
└── Cargo.toml
```

### Configuration Schema

```rust
pub struct PackerConfig {
    pub cloud: CloudProvider,
    pub image_name: String,
    pub description: Option<String>,
    pub build: BuildConfig,
    pub aws: Option<AwsConfig>,
    pub azure: Option<AzureConfig>,
    pub gcp: Option<GcpConfig>,
    pub oci: Option<OciConfig>,
    pub alibaba: Option<AlibabaConfig>,
}

pub struct BuildConfig {
    pub sindri_version: String,
    pub extensions: Vec<String>,
    pub profile: Option<String>,
    pub security: SecurityConfig,
}
```

## References

- [HashiCorp Packer Documentation](https://developer.hashicorp.com/packer/docs)
- [CIS Benchmarks](https://www.cisecurity.org/cis-benchmarks)
- [OpenSCAP](https://www.open-scap.org/)
- [Chef InSpec](https://docs.chef.io/inspec/)
