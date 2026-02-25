# sindri-core

Core library for the Sindri CLI providing configuration parsing, schema validation, type definitions, and shared utilities used by all other Sindri crates.

## Features

- Configuration file parsing (`sindri.yaml`) with serde deserialization
- JSON Schema validation for configuration files and extension manifests
- Type definitions for extensions, providers, deployments, secrets, and packer configs
- Retry execution engine with policy-based configuration
- Tera template rendering for configuration generation
- Image version resolution interface

## Modules

- `config` - `SindriConfig` parser and `ImageVersionResolver` trait
- `error` - Shared error types and `Result` alias
- `retry` - Retry engine with configurable backoff policies
- `schema` - `SchemaValidator` for JSON Schema validation
- `templates` - Tera-based template rendering utilities
- `types` - Type definitions for extensions, providers, BOM, packer, and secrets
- `utils` - Common utilities including home directory resolution

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
sindri-core = { path = "../sindri-core" }
```

## Part of [Sindri](../../)
