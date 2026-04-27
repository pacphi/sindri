# 2026-04-27 audit delta

Companion to `2026-04-27-implementation-audit.md`. This file tracks status
movement of audit findings as remediation lands. The original audit document
is **never modified**; new wave entries are appended here.

## Wave 3A.1 — Foundation PR (`feat/v4-registry-oci-cosign-foundation`)

### ADR-003 (OCI-only registry distribution)

- **Status:** 🔴 → 🟡 (partial — foundation in PR; live fetch in Wave 3A.2)
- **What landed:**
  - `oci-client` 0.16 added as a workspace dependency (successor to
    `oci-distribution`).
  - `OciRef` parser in `crates/sindri-registry/src/oci_ref.rs` — handles
    bare, `oci://`-prefixed, digest-pinned, and default-registry forms.
  - Content-addressed cache in `crates/sindri-registry/src/cache.rs` —
    `by-digest/<alg>/<aa>/<bbcc…>/{manifest,index,signature}` layout with
    a `refs/<registry>/<encoded-ref>` index.
  - `ResolvedComponent.manifest_digest: Option<String>` lockfile field
    (additive, `#[serde(default)]`, `skip_serializing_if`).
- **What's next (3A.2):**
  - Replace `RegistryClient::fetch_from_source` with `oci-client` manifest
    + blob fetches.
  - Resolver populates `manifest_digest` from the live OCI response.
  - Wiremock-backed integration tests for the OCI fetch path.

### ADR-014 (signed registries via cosign)

- **Status:** 🔴 → 🟡 (partial — trust-key loading; verification deferred)
- **What landed:**
  - `sigstore` 0.13 added with `cosign` + `sigstore-trust-root` features.
  - `CosignVerifier::load_from_trust_dir` in
    `crates/sindri-registry/src/signing.rs` — parses ECDSA P-256 PEM keys
    under `~/.sindri/trust/<registry>/cosign-*.pub` with a stable 8-char
    key id derived from SHA-256 of the SPKI bytes.
  - **Bug fix.** `sindri registry trust` now actually reads the source
    PEM, validates it parses as a P-256 public key, and copies it to
    `~/.sindri/trust/<name>/cosign-<short-key-id>.pub`. The audit flagged
    the previous behaviour (writing a JSON sidecar with the raw signer
    path, never validated) as security theatre.
  - `sindri registry verify <name>` CLI subcommand stub. Exits non-zero
    with an explanatory "deferred to Wave 3A.2" message so callers cannot
    accidentally rely on it.
  - New `RegistryError` variants: `InvalidOciRef`, `SignatureRequired`,
    `SignatureMismatch`, `TrustKeyParseFailed`.
- **What's next (3A.2):**
  - Cosign signature manifest fetch via `oci-client`.
  - Simple-signing payload decode + signature byte verification using the
    loaded `TrustedKey` set.
  - `--insecure` flag on `sindri registry add` (deliberately deferred so
    we don't ship a flag that bypasses verification that doesn't yet
    exist).
  - Strict-policy enforcement: `InstallPolicy::require_signed_registries`
    consulted in `RegistryClient::fetch_index`.
