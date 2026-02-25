# sindri-image

Container image management for the Sindri CLI. Provides registry queries, semantic version resolution, and image signature verification for OCI-compatible container registries.

## Features

- OCI-compatible container registry queries (GHCR, Docker Hub, and others)
- Semantic version resolution with constraint matching
- Image signature verification via Cosign
- Provenance attestation checking
- SBOM (Software Bill of Materials) fetching and parsing
- Image metadata caching for performance
- Pull policy support (Always, IfNotPresent, Never)
- Bridge adapter for `sindri-core` ImageVersionResolver trait

## Modules

- `registry` - `RegistryClient` for querying OCI container registries
- `resolver` - `VersionResolver` for semantic version constraint resolution
- `types` - Type definitions (`ImageReference`, `ImageManifest`, `Sbom`, `SignatureVerification`, etc.)
- `verify` - `ImageVerifier` for Cosign signature and provenance checks
- `bridge` - `RegistryImageResolver` adapter implementing core traits

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
sindri-image = { path = "../sindri-image" }
```

## Part of [Sindri](../../)
