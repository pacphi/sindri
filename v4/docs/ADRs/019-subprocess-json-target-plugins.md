# ADR-019: Subprocess-JSON Target Plugin Protocol for v4.0

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

v3 required editing 4–5 files and cutting a CLI release to add a new provider. The
community cannot add providers; only sindri-dev can. This bottleneck will slow adoption
as more cloud platforms emerge (Modal, Replit, Lambda Labs, Azure Container Apps, etc.).

Two extensibility approaches were evaluated:

- **Option B (WASM):** Target plugins as WASM modules loaded dynamically, signed and
  distributed as OCI artifacts. Cleanest isolation; most complex to implement.
- **Option C (subprocess-JSON):** Target plugins as binaries on `$PATH` named
  `sindri-target-<name>`, speaking a stable JSON-over-stdio protocol (same pattern as
  `terraform-provider-*`, `kubectl` plugins, `gh` extensions).

## Decision

**Subprocess-JSON for v4.0.** WASM deferred to v4.1+.

Open question Q31 resolved.

### Protocol specification

A target plugin is a binary named `sindri-target-<name>` that speaks JSON over stdio:

**Request (from Sindri CLI → plugin):**

```json
{"method": "profile", "params": {}}
{"method": "plan", "params": {"lock": {...}}}
{"method": "create", "params": {"infra": {...}}}
{"method": "exec", "params": {"cmd": "node --version", "cwd": "/workspace"}}
```

**Response (from plugin → Sindri CLI):**

```json
{"result": {"os": "linux", "arch": "x86_64", "capabilities": {...}}}
{"result": {"actions": [...]}}
{"result": {"status": "created", "details": {...}}}
{"result": {"stdout": "v22.11.0", "stderr": "", "exit_code": 0}}
{"error": {"code": "PREREQ_MISSING", "message": "modal CLI not found"}}
```

The protocol is versioned via a `"sindri_protocol_version": "v4"` field in the plugin's
`metadata` response.

### Plugin discovery

Sindri searches `$PATH` for binaries matching `sindri-target-*` on startup. Users install
a community target plugin by placing the binary on `$PATH`:

```bash
sindri target plugin install oci://ghcr.io/myorg/sindri-target-modal:1.0
# downloads binary, places in ~/.sindri/bin/sindri-target-modal, adds to PATH
sindri target plugin trust modal --signer cosign:key=...
```

### Security model

- Plugins are binaries running as the user. No additional sandboxing in v4.0.
- `sindri target plugin trust` records the cosign signature of the plugin binary.
  Installing an unsigned plugin requires `--no-verify` and is logged.
- The subprocess protocol does not give plugins access to Sindri's internal state
  beyond what is passed in the `params` object.

### WASM path forward (v4.1+)

The JSON-over-stdio ABI is intentionally simple so migrating to WASM host-guest ABI
later is feasible: the message shapes are the same; only the transport changes.

## Consequences

**Positive**

- Community can ship new targets without a CLI release.
- Same pattern as well-established tools (`terraform-provider-*`, `kubectl` plugins).
- Simple to implement in v4.0; proven ABI evolution path.

**Negative / Risks**

- Subprocess plugins are less sandboxed than WASM. Acceptable for v4.0 where users
  opt-in explicitly.
- ABI versioning requires discipline. Mitigation: protocol version field in every
  message; CLI rejects plugins with incompatible protocol versions.

## References

- Research: `12-provider-targets.md` §10, `05-open-questions.md` Q31
