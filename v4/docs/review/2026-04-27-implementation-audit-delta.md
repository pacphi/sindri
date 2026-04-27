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

## Wave 3A.2 — Live fetch + verify (`feat/v4-registry-oci-cosign-live`)

### ADR-003 (OCI-only registry distribution)

- **Status:** 🟡 → 🟢 (live OCI fetch operational)
- **What landed:**
  - `RegistryClient::fetch_index` and `refresh_index` now perform real OCI
    Distribution Spec pulls via `oci-client::Client::{pull_manifest,
    pull_blob}`. The legacy `reqwest::get($URL/index.yaml)` shim is gone.
  - Anonymous auth is the default; `~/.docker/config.json` is parsed for
    basic-auth credentials when present (`parse_docker_config_auth`).
    `oci-client` handles the `Www-Authenticate: Bearer` token exchange
    transparently from there.
  - Layer media type negotiation:
    `application/vnd.sindri.registry.index.v1+yaml` is the canonical
    in-band form for `index.yaml` blobs. Tar+gzip layers and unknown media
    types fail loudly with `RegistryError::UnsupportedMediaType` rather
    than silently misbehaving.
  - The content-addressed cache (`by-digest/sha256/<aa>/<bbcc…>/index.yaml`
    + `refs/<registry>/<encoded-ref>` digest pointer) is now populated on
    every successful fetch alongside the legacy
    `<registry>/index.yaml` path that the resolver still reads.
  - `RegistryCache::any_digest_for_registry` exposes the linked digest to
    the resolver so lockfile entries can carry it.
  - `ResolvedComponent.manifest_digest` is now populated with the
    *registry-level* OCI digest. Per-component manifest digests (each
    component carrying its own per-blob digest) are explicitly deferred
    to Wave 5 / SBOM work.
- **What's still deferred:**
  - Tar+gzip layer extraction for registries that bundle `index.yaml`
    inside a tarball.
  - Wiremock-backed mock-server tests for the full pull flow
    (TODO(wave-3a.3) marker in `tests/oci_integration.rs`). The bearer-
    token handshake balloons the mock-server setup; a live integration
    test is gated behind `--features live-oci-tests --ignored` instead.

### ADR-014 (signed registries via cosign)

- **Status:** 🟡 → 🟢 (key-based cosign verification operational)
- **What landed:**
  - `CosignVerifier::verify_payload` — pure-function verifier over already-
    fetched bytes. Asserts
    `critical.image.docker-manifest-digest == expected_digest`, then walks
    the trusted-key set looking for one whose P-256 ECDSA verification
    succeeds over the canonical simple-signing payload bytes.
  - `CosignVerifier::verify_registry_signature` — fetches the cosign
    signature manifest at `<repo>:sha256-<hex>.sig` via `oci-client`,
    extracts the simple-signing layer + base64 signature annotation, then
    delegates to `verify_payload`.
  - `RegistryClient::with_verifier` + `with_insecure` plumbing.
    `RegistryClient::fetch_index` now invokes verification before handing
    the index back to the caller, gated by the `InstallPolicy::
    require_signed_registries` flag and the `--insecure` escape hatch.
  - `--insecure` flag on `sindri registry refresh`. Loud `tracing::warn!`
    on use; rejected with `RegistryError::InsecureForbiddenByPolicy` when
    strict mode is active.
  - `sindri registry verify <name> --url <oci-ref>` is no longer a stub —
    it runs the full verification flow and prints either
    `Verified registry '<name>': signed by trusted key <key-id>` or a
    typed `RegistryError`.
  - Test coverage:
    - `verify_succeeds_with_test_signature_against_trusted_key`
    - `verify_fails_with_wrong_payload_digest`
    - `verify_fails_with_wrong_key`
    - `strict_policy_no_keys_fails`
    - `permissive_policy_no_keys_warns_only`
    - `cosign_signature_tag_round_trip`
    - `client::tests::registry_local_protocol_unaffected`
- **What's still deferred (v4.1):**
  - **Keyless OIDC** (Fulcio + Rekor verification of cosign signatures
    that carry a transient certificate instead of a long-lived key). The
    sigstore 0.13 helpers cover this but pull in `tough` and a lot of
    additional networking; we explicitly chose key-based verification
    first per ADR-014's "trust model" section.
  - Per-component cosign signatures (each component blob signed
    independently). Out of scope until the SBOM work in Wave 5 wires
    component manifest digests through the lockfile.

## Wave 4C — Sprint 12 hardening (`feat/v4-doctor-secrets-backup`)

### Sprint 12 verbs (`doctor --fix`, `secrets *`, `backup` / `restore`)

- **Status:** 🔴 → 🟢
- **What landed:**
  - `crates/sindri/src/commands/doctor.rs` rewritten as a typed
    [`HealthCheck`] registry. Initial fixable checks: `~/.sindri/`,
    `~/.sindri/trust/`, `~/.sindri/cache/registries/`,
    `~/.cargo/bin` on `PATH` via guarded shell-rc block. Stale-lockfile
    detection is suggestion-only (never auto-resolves).
  - New `--fix`, `--dry-run`, and `--json` flags on `sindri doctor`.
    `--fix` and `--dry-run` are mutually exclusive at the clap layer.
  - Shell-rc remediation reuses the `# sindri:auto`-marker idempotent
    pattern from `sindri-extensions::configure` (PR #215). The doctor
    block uses a distinct `# sindri:auto path` marker so it does not
    collide with the per-component env-fragment block.
  - New `crates/sindri/src/commands/secrets.rs`: `validate`, `list`,
    `test-vault`, `encode-file`, and `s3 {get,put,list}`. `secrets list`
    and `secrets validate` never print the resolved secret value.
  - **S3 backend simplification.** Rather than depend on `aws-sdk-s3`,
    `secrets s3 *` shells out to the `aws` CLI. Documented in the PR
    body. Argv builders are pure functions (`s3_list_argv`, …) so unit
    tests assert command shape without invoking aws.
  - `BomManifest` gained an optional `secrets: HashMap<String,String>`
    field (additive, `#[serde(default)]`) to back `secrets validate`.
  - New `crates/sindri/src/commands/backup.rs`: `sindri backup`
    produces `sindri-backup-<iso8601>.tar.gz` containing project
    files (`sindri.yaml`, `sindri.policy.yaml`, `sindri.lock`,
    `sindri.<target>.lock`), `~/.sindri/ledger.jsonl`,
    `~/.sindri/{trust,plugins,history}/`, and optionally
    `~/.sindri/cache/registries/` under `--include-cache`.
  - `sindri restore` honours default-deny overwrite (`--force` to
    override) and rejects archives containing absolute paths or `..`
    traversal entries before any extraction.
  - `flate2`, `tar`, `base64`, and `chrono` added as workspace
    dependencies.
  - Wired into `main.rs` as `Doctor`, `Secrets { … }`, `Backup`, and
    `Restore` subcommands.
  - Tests added (all passing): 5 in `doctor::tests`, 6 in
    `secrets::tests`, 4 in `backup::tests`.
- **What's deferred:**
  - Full HashiCorp Vault protocol-level health checks (today: shell out
    to `vault status`).
  - `sindri doctor --components` runs validate commands from the
    lockfile (Sprint 12.2 backlog item; flag exists but is reserved).
  - Compression alternatives (zstd) for backup; `flate2` is the
    initial choice for portability.
