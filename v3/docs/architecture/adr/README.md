# Architecture Decision Records (ADRs)

This directory contains Architecture Decision Records documenting key architectural decisions for the Sindri CLI Rust migration (v3.0.0).

## Quick Reference

| ADR                                                      | Title                                      | Phase | Status   |
| -------------------------------------------------------- | ------------------------------------------ | ----- | -------- |
| [001](001-rust-migration-workspace-architecture.md)      | Rust Migration Workspace Architecture      | 1     | Accepted |
| [002](002-provider-abstraction-layer.md)                 | Provider Abstraction Layer                 | 2     | Accepted |
| [003](003-template-based-configuration.md)               | Template-Based Configuration               | 2     | Accepted |
| [004](004-async-runtime-command-execution.md)            | Async Runtime Command Execution            | 2     | Accepted |
| [005](005-provider-specific-implementations.md)          | Provider-Specific Implementations          | 3     | Accepted |
| [006](006-template-refactoring-consistency.md)           | Template Refactoring Consistency           | 3     | Accepted |
| [007](007-phases-2-3-completion.md)                      | Phases 2-3 Completion                      | 3     | Accepted |
| [008](008-extension-type-system-yaml-deserialization.md) | Extension Type System YAML Deserialization | 4     | Accepted |
| [009](009-dependency-resolution-dag-topological-sort.md) | Dependency Resolution DAG Topological Sort | 4     | Accepted |
| [010](010-github-extension-distribution.md)              | GitHub Extension Distribution              | 4     | Accepted |
| [011](011-multi-method-extension-installation.md)        | Multi-Method Extension Installation        | 4     | Accepted |
| [012](012-registry-manifest-dual-state-architecture.md)  | Registry Manifest Dual-State Architecture  | 4     | Accepted |
| [013](013-schema-validation-strategy.md)                 | Schema Validation Strategy                 | 4     | Accepted |
| [014](014-sbom-generation-industry-standards.md)         | SBOM Generation Industry Standards         | 4     | Accepted |
| [015](015-secrets-resolver-core-architecture.md)         | Secrets Resolver Core Architecture         | 5     | Proposed |
| [016](016-vault-integration-architecture.md)             | Vault Integration Architecture             | 5     | Proposed |
| [017](017-backup-system-architecture.md)                 | Backup System Architecture                 | 5     | Accepted |
| [018](018-restore-system-architecture.md)                | Restore System Architecture                | 5     | Accepted |
| [019](019-phase-5-secrets-backup-integration.md)         | Phase 5 Integration Strategy               | 5     | Accepted |
| [020](020-s3-encrypted-secret-storage.md)                | S3 Encrypted Secret Storage                | 5     | Proposed |
| [021](021-bifurcated-ci-cd-v2-v3.md)                     | Bifurcated CI/CD Pipeline for v2 and v3    | 6     | Accepted |
| [022](022-phase-6-self-update-implementation.md)         | Phase 6 Self-Update Implementation         | 6     | Accepted |
| [023](023-phase-7-project-management-architecture.md)    | Phase 7 Project Management Architecture    | 7     | Accepted |
| [024](024-template-based-project-scaffolding.md)         | Template-Based Project Scaffolding         | 7     | Accepted |
| [025](025-git-operations-repository-management.md)       | Git Operations and Repository Management   | 7     | Accepted |
| [026](026-extension-version-lifecycle-management.md)     | Extension Version Lifecycle Management     | 4     | Accepted |

## By Phase

### Phase 1: Foundation (Weeks 1-3)

- **ADR-001**: Workspace structure and crate organization

### Phase 2: Provider Framework (Weeks 4-6)

- **ADR-002**: Provider trait definition and abstraction
- **ADR-003**: Tera template-based configuration generation
- **ADR-004**: Async/await patterns with tokio runtime

### Phase 3: Additional Providers (Weeks 7-10)

- **ADR-005**: Fly.io, DevPod, E2B, Kubernetes implementations
- **ADR-006**: Template consistency across providers
- **ADR-007**: Phase 2-3 completion summary

### Phase 4: Extension System (Weeks 11-14)

- **ADR-008**: Type system for YAML deserialization
- **ADR-009**: DAG-based dependency resolution
- **ADR-010**: GitHub release-based distribution
- **ADR-011**: Multi-method installation (mise, apt, binary, npm, script, hybrid)
- **ADR-012**: Registry vs manifest dual-state architecture
- **ADR-013**: Three-level schema validation strategy
- **ADR-014**: SBOM generation with SPDX/CycloneDX
- **ADR-026**: Extension version lifecycle (versions, rollback, history tracking)

### Phase 5: Secrets & Backup (Weeks 15-17) ✨ **COMPLETE**

- **ADR-015**: Secrets resolver core with async multi-source resolution
- **ADR-016**: HashiCorp Vault integration (vaultrs, token renewal)
- **ADR-017**: Backup system (3 profiles, tar.gz streaming)
- **ADR-018**: Restore system (3 modes, atomic rollback)
- **ADR-019**: Phase 5 integration strategy and timeline
- **ADR-020**: S3 encrypted secret storage (ChaCha20-Poly1305 + age)

### Phase 6: CI/CD & Self-Update (Weeks 18-19)

- **ADR-021**: Bifurcated CI/CD pipeline for v2 and v3 parallel development
- **ADR-022**: Self-update implementation (auto-rollback, extension compatibility blocking, update caching)

### Phase 7: Project Management (Weeks 20-21)

- **ADR-023**: Project management architecture (`sindri new` and `sindri clone`)
- **ADR-024**: Template-based project scaffolding (YAML-driven, Tera templates, type detection)
- **ADR-025**: Git operations and repository management (git2, fork workflow, feature branches)

## ADR Statistics

- **Total ADRs**: 26
- **Total Lines**: 12,000+ lines
- **Total Size**: ~370KB
- **Phases Covered**: 1-7 (of 8)
- **Implementation Status**: Phases 1-7 complete

## Key Architectural Themes

### Type Safety

- Compile-time guarantees via Rust type system
- Serde-based YAML/JSON deserialization
- Strong typing prevents category errors

### Async/Await

- Tokio runtime for async I/O
- Non-blocking network operations
- Parallel task execution

### Security

- Memory zeroing (zeroize)
- Client-side encryption
- Audit logging
- Path validation

### Provider Abstraction

- Trait-based polymorphism
- Provider-agnostic core logic
- Clean separation of concerns

### Testing

- Comprehensive unit tests
- Integration test strategy
- Mock-based testing for external services

## Related Documentation

- [Rust Migration Plan](../../planning/rust-cli-migration-v3.md)

## Contributing

When adding new ADRs:

1. Use sequential numbering (next: 027)
2. Follow template structure (Context → Decision → Consequences)
3. Include implementation details and code examples
4. Link to related ADRs
5. Update this README with the new entry
6. Mark status (Proposed → Accepted → Deprecated → Superseded)

## ADR Status Definitions

- **Proposed**: Under review, not yet implemented
- **Accepted**: Approved and being/been implemented
- **Deprecated**: No longer recommended
- **Superseded**: Replaced by newer ADR
