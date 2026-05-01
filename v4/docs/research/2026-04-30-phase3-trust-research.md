# Phase 3 Trust Research — Industry Best Practices for Supply-Chain Trust Decisions

Date: 2026-04-30
Author: research pass for Sindri v4 Phase 3 (trust + signing reconciliation)
Audience: Sindri maintainers, with bias toward "security-conscious infra CLI, pre-1.0, move-fast"

---

## Q1 — Embedded vs. on-disk trust keys, and key rotation

### Findings

**Cosign / Sigstore — embedded TUF root, fetched updates.**
`cosign initialize` ships with an embedded Sigstore TUF root: per the cosign docs, "the current trusted Sigstore TUF root is embedded inside cosign at the time of release." Updates are pulled from `tuf-repo-cdn.sigstore.dev` and persisted to `$HOME/.sigstore/root/`. Operators can override with `--root <file|url>` and `--mirror <gcs|http|file:>`. Source: cosign `doc/cosign_initialize.md` (sigstore/cosign on GitHub) and Sigstore docs at `docs.sigstore.dev/cosign/system_config/initialize/`. This is the canonical embedded-then-fetch-updates pattern.

**TUF spec — chained root rotation with overlap.**
The TUF specification (theupdateframework.github.io/specification/latest) requires clients to ship "a good, trusted copy" of root metadata out-of-band. Each new root version `N+1` must be signed by both a threshold of keys from version `N` AND a threshold from `N+1` itself ("dual-threshold"). Crucially, "an application will sign the new root.json file with both the new and old root keys" during transition, so old clients can still validate. Versions are strictly incremental (N+1 only, no skipping), and clients walk the chain. This is the textbook formal model for what Sindri's `EmbeddedKey[]` array is approximating informally.

**Sigstore root-signing ceremony — 4-month live cadence, 5 keyholders, threshold majority.**
Per the Sigstore blog ("A New Kind of Trust Root") and the `sigstore/root-signing` repo: live ceremonies run every 4 months, 5 keyholders from different orgs (Red Hat, NYU, Purdue, Google), each holder serves ~1.5 years before rotation. Threshold is a majority. This is the operational model behind the spec.

**Helm — explicitly no embedded trust roots, BYO PGP.**
From `helm.sh/docs/topics/provenance/`: "We don't want to be 'the certificate authority' for all chart signers." Verification uses `~/.gnupg/pubring.gpg`; users specify `--key` and `--keyring`. Helm deliberately punts trust bootstrapping to the user. Result: almost nobody actually verifies Helm charts, which is widely cited as a supply-chain weakness.

**Apt/Debian — embedded keyring shipped via OS package.**
Debian's initial archive trust ships in the `debian-archive-keyring` package (installed via the bootstrap installer), then lives at `/usr/share/keyrings/debian-archive-keyring.gpg`. Modern entries reference it via `Signed-By: /usr/share/keyrings/...` in the deb822 sources file format (Debian Wiki SourcesList; Arch man page `sources.list.5`). Keyring rotation rides the OS upgrade cycle — old keys remain in the keyring during transition releases.

**kubectl, gh, terraform, cargo, npm, pip — no embedded signing roots.**
None of the major package-CLI cousins ship embedded signing keys for their default registries. Cargo, npm, pip rely on TLS to the registry host. `terraform init` does verify provider signatures, but the trust anchor is the HashiCorp registry's TLS cert, not embedded keys. `rustup-init` uses TLS plus a published GPG signature on the install script (the sig file lives next to the script; users must fetch and verify out-of-band).

### Synthesis

The field is split along two clusters. Cluster A (high-assurance: cosign/sigstore, TUF, Debian, Notary): embed an initial root in the binary or OS package, fetch updates over an authenticated channel, support rotation with overlap. Cluster B (TLS-only: cargo, npm, pip, helm, terraform-providers): rely on TLS plus optional out-of-band signature checks, accepting that first-fetch is TOFU. Sigstore/cosign explicitly chose Cluster A and is the de-facto template for new infra CLIs. The TUF model — embed N most recent root keys, accept signatures from any of them, use threshold + version chaining — is the durable answer; ad-hoc `EmbeddedKey[]` arrays are a simplified flavor of the same idea.

### Eighth-grade explanation

When you install Sindri, your computer needs to know which "first-party" registry signatures to trust. There are two ways to teach it. The first way: ship a copy of the registry's signing key inside the Sindri binary itself, like a bookstore handing you a sealed envelope at the counter. As long as you trust the bookstore's binary (downloaded over HTTPS, ideally itself signed), you trust what's in the envelope. The second way: make the user run `sindri registry trust ...` after install, which is like getting handed an envelope through the mail — fine if the mail is reliable, but the very first letter is the most attackable.

Cosign and Debian both use the sealed-envelope approach, because it removes the most dangerous moment (the first download). Helm picked the mail approach and most users skip verification entirely as a result. Cargo and npm don't have the envelope at all; they trust the post office (TLS).

For rotation: imagine the bookstore changes its seal every year, but accepts the old seal for 3 months while customers update. That's TUF's overlap window. The concrete decision: when key v2 is added, Sindri keeps v1 in the embedded array for one minor release (or one quarter), accepts signatures from either, and warns when v1 is used. After the window, v1 is removed in a release.

A user-visible decision looks like: today, the user types `sindri registry trust sindri-core --signer cosign:key=...` after install. With embedded keys, that step disappears entirely for the first-party registry — `sindri install foo` works on a fresh box.

---

## Q2 — Plugin sandbox / unverified-trust marker conventions

### Findings

**Terraform `dev_overrides` — banner warning on every command.**
`~/.terraformrc` `dev_overrides` block disables version + checksum verification. Terraform prints (per HashiCorp issue #27481, verified):
> "Warning: Provider development overrides are in effect. The following provider development overrides are set in the CLI configuration: [...] The behavior may therefore not match any released version of the provider and applying changes may cause the state to become incompatible with published releases."
This warning appears on `terraform apply` (issue #27481 was filed because it didn't appear on `plan`). The marker is a config-file block, not a per-binary attribute; the audit trail is the `.terraformrc` file itself.

**kubectl plugins — zero verification, documented as user-risk.**
Kubernetes docs (`kubernetes.io/docs/tasks/extend-kubectl/kubectl-plugins/`) explicitly warn: "Kubectl plugins available via the Krew plugin index are not audited for security." Any executable on `PATH` named `kubectl-*` runs. No prompts, no markers, no sandbox. This is the baseline "no trust at all" model.

**VS Code Workspace Trust — per-folder trust state in user settings.**
Per `code.visualstudio.com/docs/editing/workspaces/workspace-trust`, trust decisions are stored centrally in user settings (not per-folder dotfiles, to prevent the folder itself from re-trusting itself). Untrusted workspaces enter "Restricted Mode" — extensions opt-in via `extensions.supportUntrustedWorkspaces`. Trust prompt appears once per new folder; banner remains visible while restricted.

**macOS Gatekeeper — `com.apple.quarantine` xattr.**
Files downloaded by browsers/curl receive an extended attribute `com.apple.quarantine` that triggers Gatekeeper inspection. Removed with `xattr -d com.apple.quarantine <path>`. Per macOS 15+ (per MacRumors threads), unsigned apps cannot be launched at all without explicit override in System Settings > Privacy & Security. The marker is on the filesystem object, not in a config file.

**Browsers (Chrome dev-mode extensions) — persistent banner.**
Chrome shows a "Disable developer mode extensions" banner every launch when `chrome://extensions` has dev-mode unpacked extensions loaded. The marker is per-extension state inside the browser profile.

### Synthesis

There are two viable patterns: (1) **per-invocation banner** (Terraform, Chrome) — every command prints the warning, even if noisy; (2) **filesystem marker** (macOS quarantine) — the file itself carries metadata, removable but visible to `ls`-equivalents. The strongest practice combines both: a config-level marker (so it's auditable in `git diff`) plus a banner on every relevant command. kubectl is the cautionary tale: no markers, no warnings, well-known supply-chain hole.

For Sindri, the most natural fit is Terraform's pattern: store the `--insecure` decision in `sindri.yaml` (or a sibling file like `.sindri/insecure-plugins.yaml`) as a typed list with timestamps and rationale field, and print a banner on every `sindri apply` while any plugin is in that list.

### Eighth-grade explanation

If a plugin is signed, Sindri can prove it came from who it says. With `--insecure`, Sindri runs the plugin anyway. The question is: how loud should the warning be, and where does the "I accepted this risk" decision live?

Three options, in order of loudness. (a) Quiet: just run, log once. Risky — three weeks later nobody remembers they bypassed verification. (b) File marker: write a line to `.sindri/insecure-plugins.yaml` with the plugin name and a "why" note. Now `git diff` shows it, code review can catch it, and anyone running `sindri status` sees it. (c) Every-invocation banner: in addition to (b), print a yellow box on every `sindri apply` listing which plugins are unverified. Terraform does (b)+(c). kubectl does (a) and worse — there's no marker at all, which is widely regarded as a mistake.

A user-visible decision looks like: `sindri plugin add ./my-dev-plugin --insecure --reason "local debugging of #1234"` writes a line to a `.sindri/insecure-plugins.yaml` file, and from then on every `sindri apply` prints "WARNING: 1 plugin running without signature verification: my-dev-plugin (added 2026-04-30, reason: local debugging of #1234)".

---

## Q3 — `<verb> add` URL handling: heuristic vs. explicit type

### Findings

**Helm — split commands by source type.**
Per the Helm docs (`helm.sh/docs/topics/registries/`), `helm repo add` works for HTTP-based chart repositories *only*. OCI registries do not use `helm repo add` at all — users address them inline (`helm install foo oci://...`) after `helm registry login`. Helm explicitly does not sniff scheme; it routes by command. Quote: "the overhead of adding and updating repositories with repo add and repo update is not needed anymore" for OCI.

**Cargo — scheme prefix to disambiguate.**
Per `doc.rust-lang.org/cargo/reference/registries.html`: "If the registry index URL starts with `sparse+`, Cargo uses the sparse protocol. Otherwise Cargo uses the git protocol." Different source kinds (registry, git, path, directory, local-registry) live in distinct config tables — `[registries.foo]` vs `[source.foo]` with kind-specific keys. Cargo never sniffs; the table key declares the type.

**Apt — explicit type word at the start.**
Sources entries begin with `deb` or `deb-src` (or in deb822 format, `Types: deb deb-src`). The URL scheme (http, https, ftp, file, cdrom) is parsed but doesn't disambiguate the source *type* — that's always explicit (Debian Wiki SourcesList).

**Brew — heuristic with two-arg fallback.**
Per `docs.brew.sh/Taps`: `brew tap user/repo` defaults to GitHub (`https://github.com/<user>/homebrew-<repo>`); the two-arg form `brew tap user/repo <URL>` accepts "any location and any protocol that Git can handle." So brew sniffs for the common case but lets the user be explicit. This works because brew has exactly one source type underneath (a git repo).

**Go GOPROXY — comma-separated with sentinel keywords.**
`go env -w GOPROXY=https://proxy.golang.org,direct,off` mixes URLs with reserved words `direct` and `off`. Not really sniffing — keywords are explicit, URLs are URLs.

**gh auth login — hostname sniffing with default.**
Defaults to `github.com` if no host given; explicit `--hostname ghe.example.com` for Enterprise.

### Synthesis

Most CLIs do **not** sniff URLs for ambiguous source-type detection. The dominant patterns are: (1) **scheme-as-discriminator** (cargo's `sparse+`, OCI's `oci://`); (2) **separate command per type** (helm's repo add vs. registry login); (3) **explicit type word** (apt's `deb`/`deb-src`). Brew sniffs because its underlying type is unambiguous (always git). Where sniffing exists, it's narrow — usually a default for the common case with an explicit-override flag.

For Sindri with four source types (`oci`, `local-path`, `git`, `local-oci`), pure URL sniffing is risky: `oci://...` clearly means oci; `https://github.com/...` could mean `git` or could mean an OCI registry hosted there; `/path/to/dir` could be `local-path` or `local-oci` depending on whether it contains an OCI image layout. The cargo/apt-style "scheme prefix or explicit `--type`" pattern is the safest middle ground.

### Eighth-grade explanation

You're standing at a luggage carousel and someone hands you a bag. The tag says only "John". You ask: which John? In the real world, you'd ask for clarification. CLIs face the same problem when a user types a URL — `git+https://github.com/foo` clearly says "git", but bare `/home/me/foo` could be three things.

Tools that guess wrong cause confusing errors hours later when the user can't figure out why their "git source" is being treated as a directory. Most tools therefore refuse to guess: cargo demands a `sparse+` prefix or a separate config block per type; apt demands a `deb` keyword. Brew guesses, but only because it has exactly one type underneath.

A user-visible decision: should `sindri registry add core oci://ghcr.io/sindri/core` work? Yes — `oci://` is unambiguous. Should `sindri registry add core ./my-stuff` work? Probably yes if `./my-stuff/oci-layout` exists (it's a `local-oci`), else as `local-path`. Should `sindri registry add core https://github.com/sindri/core.git` work? Force `--type git` here; the `.git` suffix is convention but not contract. Print the inferred type before writing config: "Detected source type: oci (from oci:// scheme). Add --type to override."

---

## Q4 — CLI consistency: optional vs. mandatory args after registration

### Findings

**Docker — registered host inferred from image string.**
`docker pull nginx` resolves to Docker Hub by default. `docker pull myregistry.local:5000/foo` parses the host out of the image string. Credentials in `~/.docker/config.json` are keyed by hostname and applied automatically. There's no "registered name vs URL" — there's only the hostname embedded in the image reference.

**Helm — name-only after `repo add`, all-update by default.**
`helm repo update` (no args) updates all known repos; `helm repo update <name>` targets one. Once added, `helm install foo bitnami/nginx` uses just the registered name. Forcing `--url` on every command would be unergonomic; Helm does not.

**gh — name resolution with hierarchy.**
`gh repo view` with no arg uses the current dir's git remote. `gh repo view foo` infers `<user>/foo`. `gh repo view org/foo` is explicit. URL works too. Falls back gracefully and prints the resolved repo before acting.

**kubectl — context name only.**
`kubectl --context prod` references a context registered in `~/.kube/config`. There's no `--context-url` ad-hoc form; if you want a one-off cluster, you write a kubeconfig and point at it with `KUBECONFIG=...`. Strict registered-name model.

**git remote — both forms supported.**
`git remote show origin` (registered name) and `git ls-remote https://...` (ad-hoc URL) coexist for different verbs. `git fetch <url>` is allowed but unusual.

**Terraform — backend config split between block and CLI.**
`terraform init -backend-config=key=value` overlays explicit values onto the registered backend block in code. Mixed mode is normal.

**Cargo — name only for registered registries.**
Once `[registries.my-reg]` is in config, dependencies say `registry = "my-reg"` — no URL inline. URL-only ad-hoc isn't supported in `Cargo.toml` for alternative registries.

### Synthesis

Strong field consensus: **once a name is registered, the name is the primary handle; an inline URL is either unsupported (cargo, kubectl) or a power-user override (git, terraform)**. Forcing the URL on every command after registration is anti-pattern. The clear convention: `<verb> <name>` looks up registered config; `<verb> --url ...` (or positional URL where unambiguous) supports the unregistered case; if both are given, explicit wins and a warning is printed. Errors when a name isn't registered should suggest `registry add` ("Registry 'foo' not found. Run `sindri registry add foo <url>` first, or pass `--url` to verify ad-hoc.").

### Eighth-grade explanation

When you save a Wi-Fi network, you don't type the password every time you connect — you type the network name. CLIs work the same way. Once `sindri registry add core oci://ghcr.io/sindri/core` is run, the registry name `core` should be enough.

The trap is requiring the URL anyway "for safety". This is what happens when a CLI grows feature-by-feature without a consistency pass. The user runs `sindri registry verify core --url oci://ghcr.io/sindri/core`, gets annoyed, and either writes a wrapper script (which becomes a maintenance burden) or assumes the tool is broken.

The right rule: registered name is the primary, URL is a fallback for the unregistered case, and if a name isn't found the error tells the user how to register or how to use the URL form. A user-visible decision: `sindri registry verify core` should work; `sindri registry verify core --url oci://...` should also work (override); `sindri registry verify https://...` (positional URL where the name slot was expected) should also work because `://` makes the intent unambiguous.

---

## Recommendation cheat sheet

| Question | Option A (conservative) | Option B (middle) | Option C (move-fast) | What most peers do | Recommended for Sindri |
|---|---|---|---|---|---|
| Q1: trust key bootstrap + rotation | Require manual `registry trust` for all registries (today's behavior) | Embed only `sindri-core` key, single-key, no rotation | **Embed `EmbeddedKey[]` for `sindri-core` with N=2 overlap, fetched updates optional later** | Cosign/Debian embed; helm/cargo/npm don't | **C** — embedded array with rotation overlap; matches cosign/TUF; eliminates first-fetch attack on first-party registry; defer fetched-updates (TUF mirror) until post-1.0 |
| Q2: `--insecure` plugin marker | Disallow `--insecure` entirely | **File marker only (`.sindri/insecure-plugins.yaml`) + one-time warn** | File marker + every-invocation banner (Terraform pattern) | Terraform: file + banner; kubectl: nothing | **B → C** — start with file marker + one-time warn for velocity; promote to every-invocation banner before 1.0. File must include `reason` field |
| Q3: `registry add` URL handling | Mandatory `--type` flag always | **Scheme-prefix sniff (`oci://`, `git+https://`, path heuristic) with `--type` override and printed inferred type** | Full URL sniffing including `.git` suffix, GitHub host detection, etc. | Cargo: scheme prefix; apt: explicit type; brew: sniff (single type only) | **B** — sniff when scheme is unambiguous (`oci://`, `git+`, absolute path with `oci-layout` file); require `--type` for `https://...` ambiguity; always print the resolved type before writing |
| Q4: name vs URL after registration | Always require `--url` even after add | **Name primary, `--url` optional override, positional URL accepted when contains `://`** | Sniff from any input including raw hostnames | Docker, helm, kubectl, cargo: name primary | **B** — universal field consensus; clear error when name not registered ("run `sindri registry add` or pass `--url`") |

---

## Sources

- Cosign initialize: https://github.com/sigstore/cosign/blob/main/doc/cosign_initialize.md
- TUF spec: https://theupdateframework.github.io/specification/latest/
- Sigstore root rotation: https://blog.sigstore.dev/a-new-kind-of-trust-root-f11eeeed92ef/ ; https://github.com/sigstore/root-signing
- Helm provenance: https://helm.sh/docs/topics/provenance/
- Helm OCI registries: https://helm.sh/docs/topics/registries/
- Terraform dev_overrides: https://developer.hashicorp.com/terraform/cli/config/config-file ; https://github.com/hashicorp/terraform/issues/27481
- kubectl plugins: https://kubernetes.io/docs/tasks/extend-kubectl/kubectl-plugins/
- VS Code workspace trust: https://code.visualstudio.com/docs/editing/workspaces/workspace-trust
- macOS quarantine: https://book.hacktricks.xyz/macos-hardening/macos-security-and-privilege-escalation/macos-security-protections/macos-gatekeeper
- Debian sources.list: https://wiki.debian.org/SourcesList
- Cargo registries: https://doc.rust-lang.org/cargo/reference/registries.html
- Brew taps: https://docs.brew.sh/Taps
- Docker pull/login: https://docs.docker.com/reference/cli/docker/image/pull/ ; https://docs.docker.com/reference/cli/docker/login/
- gh CLI: https://cli.github.com/manual/gh_repo_view
