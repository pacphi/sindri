# Sindri — `v4` branch (Rust, redesigned)

v4 is the next-generation Rust implementation, promoted from `research/v4` during
the April 2026 reorg. Different crate layout from v3:

```
v4/
├─ crates/                  # core sindri crates
├─ registry-core/           # registry abstraction (new in v4)
├─ renovate-plugin/         # automated dependency updates
├─ tools/                   # auxiliary build tooling
├─ docs/
└─ schemas/
```

## Stack

- Rust workspace, stable toolchain.
- Plugin-oriented architecture (renovate-plugin, registry-core).

## Build & test

```bash
cd v4 && cargo build --workspace
cd v4 && cargo test --workspace
cd v4 && cargo clippy --workspace --all-targets -- -D warnings
cd v4 && cargo fmt --all --check
```

## Standards

- Zero clippy warnings.
- Format on save (`cargo fmt`).
- Document public APIs with `///` doc comments.

## CI

Workflows live on `main` (`.github/workflows/ci-v4.yml`, `release-v4.yml`) and
trigger on push/PR to `v4` or on `v4.*.*` tags.

## GitNexus

Optional. To index v4 locally:

```bash
npx gitnexus analyze --embeddings
```

`.gitnexus/` is **gitignored** — never commit it.

## Relationship to v3

v3 remains active in parallel. v4 is not a strict superset; some features were
removed or redesigned. See `v4/docs/MIGRATION_FROM_V3.md` once authored.
