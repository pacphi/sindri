# ADR-010: Central Platform-Matrix Resolver for Binary Asset Selection

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

v3's `InstallMethod::Binary` allows extensions to download GitHub release assets, but
each extension author writes their own `uname -m` logic. There is no canonical mapping
from OS/arch detection strings to backend-native asset names. The result: inconsistent
behaviour across extensions and silent failures when an asset is missing.

## Decision

Introduce a `platform_matrix` module in `sindri-core` that:

1. Detects OS and arch once (at CLI startup or at resolve time).
2. Normalizes to canonical `{os}-{arch}` identifiers:
   - `linux-x86_64`, `linux-aarch64`
   - `macos-aarch64`
   - `windows-x86_64`, `windows-aarch64`
3. Is the single source of truth for all backend asset-selection logic.

Component authors use structured `assets:` maps (see ADR-009). The platform-matrix
module looks up `assets["{os}-{arch}"]` and returns the resolved asset name / URL.

If no entry exists for the detected platform, the resolver fails with:

```
Error: component gh@2.62.0 does not support platform macos-aarch64
  Supported platforms: linux-x86_64, linux-aarch64, windows-x86_64
  → Use a different component or submit a PR to add macos-aarch64 support
```

No more "installed but broken" surprises.

## Consequences

**Positive**

- Authors write platform keys once; Sindri resolves automatically.
- Detection logic is shared, tested, and maintained in one place.

**Negative / Risks**

- Authors must enumerate all target platforms explicitly. This is a feature, not a bug:
  undeclared platforms fail loudly rather than silently.

## References

- Research: `07-cross-platform.md` §2.3
