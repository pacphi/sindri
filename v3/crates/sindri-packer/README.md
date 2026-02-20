# sindri-packer

HashiCorp Packer provider for Sindri, enabling multi-cloud VM image building and deployment. Generates HCL2 templates and orchestrates Packer to produce machine images across five cloud platforms.

## Features

- Multi-cloud VM image building (AWS, Azure, GCP, OCI, Alibaba Cloud)
- HCL2 template generation via Tera rendering
- Parallel multi-cloud builds
- Cloud prerequisite validation (credentials, CLI tools, quotas)
- Embedded template assets via rust-embed
- Configurable error handling behavior (cleanup, abort, ask)
- Feature-gated cloud integration tests

## Modules

- `aws` - AWS EC2 AMI builder via `amazon-ebs`
- `azure` - Azure managed images with Shared Image Gallery support
- `gcp` - GCP Compute Engine image builder via `googlecompute`
- `oci` - Oracle Cloud Infrastructure custom image builder
- `alibaba` - Alibaba Cloud ECS custom image builder
- `templates` - Embedded HCL2 template management
- `traits` - `PackerProvider` trait, `BuildOptions`, `BuildResult`, `ValidationResult`
- `utils` - Packer installation detection

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
sindri-packer = { path = "../sindri-packer" }
```

## Part of [Sindri](../../)
