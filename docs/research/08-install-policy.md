# Install Admissibility & Backend Preference

Two distinct concerns that today's v3 conflates — and that v4 should separate cleanly.

- **Admissibility.** Given a component and a target (host OS, arch, user policy), is
  this component _allowed_ to install at all? Yes/no with a reason.
- **Preference.** Given an admissible component with multiple install paths (e.g.,
  macOS can use `brew:gh`, `binary:gh`, or `mise:gh`), which path wins? Ordered
  choice with an explainable ranking.

Both are policy, not mechanism. The install executor (§3 of `07-cross-platform.md`)
just does what it's told — these two layers decide _what_ it should do.

## 1. Admissibility — "can this be installed here?"

Four gates. All must pass. Failure at any gate is a hard resolution error, surfaced
before any network or disk work.

### Gate 1 — Platform eligibility

The component declares a `platforms:` list; the current host must appear in it.
Already covered in `07-cross-platform.md` §4. No negotiation.

### Gate 2 — Policy eligibility (new in v4)

User's or org's configured policy. Ships as `~/.sindri/policy.yaml` (user) overridable
by `./sindri.policy.yaml` (project) and optionally by registry-embedded defaults:

```yaml
apiVersion: sindri.dev/v4
kind: InstallPolicy

licenses:
  allow: [MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, MPL-2.0]
  deny: [GPL-3.0-only, AGPL-3.0-only, BUSL-1.1, proprietary]
  onUnknown: warn # allow | warn | deny

registries:
  require_signed: true
  trust:
    - sindri/core # always trusted
    - acme/internal # signed by acme's cosign key (see §registries config)
  # everything else must be explicitly added to sindri.yaml `registries:`

sources:
  require_checksums: true # every binary download must declare sha256
  require_pinned_versions: true # no ranges in sindri.yaml allowed
  allow_script_backend: prompt # allow | warn | prompt | deny
  allow_privileged: prompt # sudo/apt/system packages require consent
  allow_system_services: false # no systemd/launchd units

network:
  offline: false
  allow_domains: ["*"] # or an allowlist for air-gapped/enterprise
  deny_domains: []

execution:
  require_sandbox: false # if true, scripts must run in a sandbox (§3.4)
```

Policy evaluation is local and deterministic — `sindri resolve` prints the policy
decisions alongside the resolved lockfile so users can see _why_ something was
rejected.

### Gate 3 — Dependency closure

Every transitive dependency (via `dependsOn`) must also pass gates 1 and 2, with
pinned versions that resolve without conflict (see open question §17 —
collection-vs-explicit version conflicts). A failure anywhere in the closure fails
the whole install; `sindri resolve` shows the path (`collection:acme-platform →
mise:python → mise-config` — rejected: license BUSL-1.1).

### Gate 4 — Capability trust

Capabilities like `collision-handling` and `project-init` can reach outside a
component's own scope (write to `.claude/`, run arbitrary commands). Policy can
restrict who's allowed:

```yaml
capabilities:
  trust_sources:
    collision_handling: [sindri/core] # only core can merge shared files
    project_init: [sindri/core, acme/internal]
    mcp_registration: "*" # open to all
    shell_rc_edits: [sindri/core, acme/internal]
```

A component from an untrusted registry declaring `collision-handling` is either
denied, downgraded (rules ignored), or prompted — per policy.

### The admissibility report

Every `sindri resolve` emits an admission report, machine-readable and human-readable:

```
$ sindri resolve
Resolving 14 components...

ADMITTED (12)
  mise:nodejs@22.11.0              license=MIT, signed by sindri/core
  mise:python@3.14.0               license=PSF-2.0, signed by sindri/core
  ...

DENIED (2)
  vendor/closed-source:foo@1.0.0   license=proprietary (policy: licenses.deny)
                                   → to allow: add to policy.licenses.allow or use --allow-license=proprietary
  binary:unpinned-tool@latest      version not pinned (policy: sources.require_pinned_versions)
                                   → to allow: pin version in sindri.yaml

Resolution failed. No changes made.
```

## 2. Preference — "which way do we install it?"

When a component is admissible and exposes multiple install paths on the current
platform, four sources contribute to the ranking. Highest-priority wins; ties broken
deterministically by backend-name alphabetical.

### Priority order (most to least specific)

1. **Per-component user override** in `sindri.yaml`.

   ```yaml
   components:
     gh:
       version: "2.62.0"
       backend: brew # "I want brew, not the default"
   ```

2. **Project-wide user preference** in `sindri.yaml`.

   ```yaml
   preferences:
     backendOrder:
       macos: [brew, mise, binary, script]
       linux: [apt, mise, binary, script]
       windows: [winget, scoop, mise, binary, script]
   ```

   This is the most common shape — "always prefer my OS's native PM, fall back in
   this order."

3. **Sindri built-in defaults.** Ship a sensible order per OS that matches what most
   users would pick. Suggested:

   | OS      | Default order                                                                                |
   | ------- | -------------------------------------------------------------------------------------------- |
   | macOS   | `brew > mise > pipx/npm/cargo/go-install > binary > script`                                  |
   | Linux   | `mise > apt/dnf/zypper/pacman/apk (by distro) > pipx/npm/cargo/go-install > binary > script` |
   | Windows | `winget > scoop > mise > pipx/npm/cargo/go-install > binary > script (ps1)`                  |

   Rationale: native PMs give best integration (uninstall, upgrade, GUI visibility);
   `mise` is preferred for language runtimes where version-switching matters; the
   typed ecosystem backends (`pipx`, `npm`, `cargo`, `go-install`) are preferred over
   raw `binary` because they give upgrade paths; `script` is last because it's opaque.

4. **Component-declared preference.** Component authors know which backend actually
   works best for their tool:
   ```yaml
   install:
     preferences:
       macos:   [brew, binary]     # brew tap is official; binary as fallback
       linux:   [binary]           # upstream only ships binaries
       windows: [scoop, binary]    # scoop bucket exists; winget does not
     default:
       binary: { ... }
     overrides:
       macos:  { brew:  { package: gh } }
       linux:  { apt:   { packages: [gh] } }     # applies when user chose apt
       windows:{ scoop: { bucket: main, package: gh } }
   ```
   This acts as a _filter and a hint_: the component tells Sindri which backends are
   actually known to work. User preferences intersect with it.

### Resolution algorithm

```
given: component C, platform P, user prefs U, component prefs K, sindri defaults D
let admissible_backends = K.overrides[P].keys() ∪ { D if C.install.default exists }
let candidate_order =
    filter( U.per_component[C].backend     if set,  admissible_backends ) ||
    filter( U.preferences.backendOrder[P], admissible_backends )        ||
    filter( K.preferences[P],              admissible_backends )        ||
    filter( D[P],                           admissible_backends )
pick candidate_order[0]
if none: error "no installable backend on {P}"
```

Every step of this is surfaced by `sindri resolve --explain`:

```
$ sindri resolve --explain gh
gh@2.62.0 on macos-aarch64
  candidates: brew, binary                (from component K)
  user pref:  [brew, mise, binary, script] (from sindri.yaml)
  chosen:     brew
  install:    brew install gh
```

### What happens when preferences disagree across the closure

Suppose user prefers `brew` globally, `collection:anthropic-dev` pulls `mise:python`
explicitly (as an opinionated choice: the collection author _wants_ mise here).

**Recommendation:** an explicit `backend:name` in a `dependsOn` or `sindri.yaml`
components entry is a _pin_, not a _hint_. It wins over the generic preference chain.
This matches how collections become meaningful — a collection can guarantee "you get
Python via mise, not via brew" because mixing PMs for languages creates painful
version-switching confusion.

## 3. Other policy dimensions worth encoding

### 3.1 Scope of effect

Components can touch different blast radii. Policy should classify each before
admitting it:

| Scope                 | Example                                           | Default policy                      |
| --------------------- | ------------------------------------------------- | ----------------------------------- |
| **User-local**        | `mise:nodejs` installs to `~/.mise/`              | allow                               |
| **User-dotfiles**     | edits to `~/.zshrc`, `~/.config/...`              | allow, announce                     |
| **Project-local**     | writes to `./` (project init)                     | allow with collision handling       |
| **System-privileged** | `apt install docker-ce`, services, kernel modules | prompt by default                   |
| **Global shared**     | `~/.claude/` (shared across projects)             | prompt, require `shared` capability |

Tie this to the existing collision-handling `path-scope` mechanism. Extend
policy.yaml:

```yaml
scopes:
  user_local: allow
  user_dotfiles: allow
  project_local: allow
  system_privileged: prompt
  global_shared: prompt
```

### 3.2 Source provenance

For `binary:` backends, record where the bits came from. Policy can restrict:

```yaml
binary_sources:
  allow:
    - github-release
    - custom-https:sindri.dev/cdn/*
    - custom-https:ghcr.io/*
  deny:
    - http:* # refuse non-TLS
    - custom-https:*.my-shady-cdn.biz
```

Every binary download ends up in the generated SBOM with its source URL, resolved
digest, and signature (if any) — so provenance is auditable after the fact, not just
at install time.

### 3.3 License reporting, not just gating

License policy shouldn't be a blunt yes/no if users want nuance. Support four
actions:

- `allow` — silent.
- `warn` — install, but print a warning at resolve time.
- `prompt` — interactive consent (batch mode defaults to deny).
- `deny` — hard fail.

Plus `onUnknown:` for components with missing/ambiguous license declarations.
Registry CI should refuse to publish components without a declared license, which
makes `onUnknown: deny` a safe default for the core registry.

### 3.4 Script sandboxing (stretch)

`script:` backend is the biggest policy headache — it's arbitrary code. Options:

- **Status quo.** Run scripts as the user. Fast and simple; zero protection.
- **Network allowlist.** Wrap scripts in a process with a pf/iptables rule limiting
  outbound connections to domains declared in `requirements.domains`. Requires a
  helper; OS-specific; moderate effort.
- **Full sandbox.** Run scripts in Landlock (Linux), Seatbelt/App Sandbox (macOS),
  AppContainer (Windows). High effort, meaningful protection.

**Recommendation:** v4.0 ships with network-allowlist wrapping (`requirements.domains`
is already declared per-component; make it enforceable). Full sandboxing is a v4.1+
stretch — track demand.

## 4. Concrete recommendations for v4

1. **Introduce `sindri-policy` as a first-class subsystem** alongside `sindri-resolver`
   in the Rust workspace. Single crate, serde-deserialized `InstallPolicy`, functions
   `admit(component, platform, policy) -> AdmissionResult` and
   `choose_backend(component, platform, user_prefs, defaults) -> BackendChoice`.

2. **Ship three built-in policy presets** for quick adoption:
   - `default` — permissive home-lab mode (what most users want).
   - `strict` — pinned-only, signed-registries-only, license allowlist, deny script,
     deny privileged. Good for CI and enterprise onboarding.
   - `offline` — adds `network.offline: true`, requires all components cached
     locally, denies any network-backed source.
     `sindri policy use <preset>` writes to `~/.sindri/policy.yaml`.

3. **Make `preferences.backendOrder` the single configurable knob** for most users.
   Almost every preference question reduces to "on my OS, what do I reach for first?"
   This is the 80% solution — everything else (per-component overrides, component
   author preferences) is for the 20%.

4. **Admission errors are structured.** Every denial carries a machine-readable code
   (`ADM_LICENSE_DENIED`, `ADM_UNSIGNED_REGISTRY`, `ADM_PLATFORM_UNSUPPORTED`, etc.)
   so IDE/console integrations can surface actionable fixes, not just strings.

5. **`sindri resolve --explain` is a first-class command**, not a flag footnote. It
   takes an optional component name and shows the full admission + backend-choice
   trace. Devx matters here — users will wonder "why did it pick brew?" and the
   answer must be one command away.

6. **Publish-time validation in registries.** The registry CI tool (`sindri registry
lint`) enforces:
   - every component has a `platforms:` list;
   - every listed platform has an `install.default` or an override that works on it;
   - every component has a declared SPDX license;
   - every `binary:` asset has a sha256;
   - no component from a non-core registry declares `capabilities.collision-handling`
     that writes outside `{component-name}/**` without an explicit `:shared` path
     (tie-in with open question §10).

7. **Users can always force.** `sindri install --allow-license foo-license`,
   `sindri install --backend script`, `sindri install --allow-unsigned` — but every
   override is logged to the StatusLedger with timestamp and user, so audit later is
   possible. No "just run this curl-pipe-sh" mental shortcut.

## 5. What stays out of v4.0

- **Full script sandboxing** (Landlock/Seatbelt/AppContainer) — stretch.
- **Cryptographic capability attestation** (SLSA L3+, in-toto attestations) — we
  track the digests and signatures; formal attestation chains are a v4.1+ ask.
- **User-facing GUI policy editor** — out of scope for the CLI; console team can
  build it later against the structured `policy.yaml`.
- **Per-component risk scoring** (e.g., CVE feed integration) — out of scope;
  separate project.

## 6. Open questions added to §05

25. **Default admission strictness.** Ship `default` preset as permissive (current
    lean) or as `strict`? Strict is safer but breaks the "install and it works"
    onboarding story. Leaning: `default`, with a prominent nudge to switch to
    `strict` for CI and production environments.

26. **Component-declared preferences vs user preferences — which wins on a tie?**
    Proposed: user project-level `backendOrder` beats component-declared preference.
    Component author hints are the floor, not the ceiling.

27. **License data source.** Do we trust the SPDX identifier declared in
    `component.yaml`, or require registry CI to cross-check with an upstream source
    (e.g., scancode)? Trusting the declaration keeps the registry CI fast; cross-
    checking is safer but slower.

28. **"Forced" overrides — audit trail format.** Should forced `--allow-*` flags
    require a justification string (`--allow-license=proprietary --reason "vendor
contract SA-2342"`)? Enterprise users will probably want this; home users will
    find it friction. Leaning: optional flag, hard-required if the policy has
    `audit.require_justification: true`.
