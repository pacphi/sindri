# ADR 001: Rust Migration and Workspace Architecture

**Status**: Accepted
**Date**: 2026-01-21
**Deciders**: Core Team
**Related**: [Rust Migration Plan](../../planning/rust-cli-migration-v3.md)

## Context

The Sindri CLI was originally implemented as ~52,000 lines of bash scripts across multiple adapters and utilities. This architecture presented several challenges:

1. **Maintainability**: Large bash scripts are difficult to refactor and test
2. **Type Safety**: No compile-time guarantees, runtime errors common
3. **Distribution**: Required users to have bash, yq, jq, and other dependencies
4. **Performance**: Parsing YAML repeatedly, slow subprocess spawning
5. **Error Handling**: Inconsistent error propagation across scripts
6. **Testing**: Limited unit testing capabilities for bash
7. **Binary Distribution**: No single portable executable

The goal was to migrate to Rust 1.92.0 while maintaining 100% feature parity with the bash implementation.

## Decision

### Workspace Architecture

We adopt a **multi-crate workspace** structure with clear separation of concerns:

```
sindri-rs/
├── Cargo.toml                    # Workspace root
└── crates/
    ├── sindri/                   # Main CLI binary (entry point)
    ├── sindri-core/              # Core types, config, schemas
    ├── sindri-providers/         # Provider adapters (Docker, Fly, etc.)
    ├── sindri-extensions/        # Extension system
    ├── sindri-secrets/           # Secrets management
    └── sindri-update/            # Self-update framework
```

### Crate Responsibilities

**sindri** (bin crate)

- CLI argument parsing with clap derive
- Command dispatch (version, config, deploy, etc.)
- User-facing output formatting
- Dependencies: clap, indicatif, colored

**sindri-core** (lib)

- Type definitions matching JSON schemas
- Config loading and validation
- Schema validation with jsonschema
- Embedded schemas via rust-embed
- Zero async dependencies

**sindri-providers** (lib)

- Provider trait definition
- 5 provider implementations (Docker, Fly, DevPod, E2B, Kubernetes)
- Template rendering with Tera
- Async operations with tokio

**sindri-extensions** (lib)

- Extension registry and dependency resolution
- YAML-based declarative executor
- Validation framework
- DAG-based topological sort

**sindri-secrets** (lib)

- Multi-source secret resolution (env, file, vault)
- Provider-specific injection (Docker env_file, Fly secrets, etc.)

**sindri-update** (lib)

- GitHub releases API integration
- Self-update with rollback
- Version compatibility checking

### Version and Metadata

- **Rust Version**: 1.92.0 (standardized via rust-toolchain.toml)
- **Edition**: 2021
- **Target Version**: 3.0.0 (semver major bump from bash 2.2.1)
- **Repository**: https://github.com/pacphi/sindri

### Build Configuration

- Custom build.rs in sindri crate for version info (GIT_SHA, BUILD_DATE)
- Environment variables set at build time
- Embedded schemas compiled into binary
- Release builds optimized with LTO

## Consequences

### Positive

1. **Code Reduction**: 78% reduction (52K bash → 11.2K Rust)
2. **Type Safety**: Compile-time guarantees via Rust's type system
3. **Single Binary**: 12MB executable with zero runtime dependencies
4. **Performance**: ~10-50x faster config parsing and validation
5. **Testing**: 28 unit tests with 100% pass rate
6. **Maintainability**: Clear module boundaries, easy to refactor
7. **Distribution**: Pre-built binaries for multiple platforms
8. **Error Messages**: Structured error handling with anyhow/thiserror
9. **Documentation**: Doc comments generate API docs automatically

### Negative

1. **Learning Curve**: Team needs Rust expertise
2. **Build Time**: 1-2 minutes vs instant bash script changes
3. **Binary Size**: 12MB vs ~50KB of bash scripts
4. **Initial Migration**: ~4 weeks of development time
5. **Cross-compilation**: Requires additional CI setup

### Neutral

1. **Dependencies**: 30+ Rust crates vs 5 CLI tools (bash, yq, jq, etc.)
2. **Debugging**: Different toolchain (lldb/gdb vs bash -x)
3. **Async Runtime**: Adds complexity but enables concurrent operations

## Compliance

- ✅ Rust 1.92.0 standardization
- ✅ Edition 2021
- ✅ Semver versioning
- ✅ MIT license inheritance
- ✅ GitHub repository configured

## Notes

The workspace pattern allows independent versioning of crates if needed in the future, though we currently use workspace-level version inheritance for simplicity.

The decision to use rust-embed for schemas enables offline operation and guarantees schema availability at runtime without file I/O.

Build metadata (git SHA, build date) is captured at compile time to support debugging and version tracking.

## Related Decisions

- [ADR-002: Provider Abstraction Layer](002-provider-abstraction-layer.md)
- [ADR-003: Template-Based Configuration](003-template-based-configuration.md)
- [ADR-004: Async Runtime](004-async-runtime-command-execution.md)
