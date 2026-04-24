# Cross-Platform & Cross-Architecture Support

**The goal.** v4 runs well on macOS (Apple Silicon), Windows (x86_64 + ARM64),
and Linux (x86_64 + aarch64 across the major distros), with every backend pulling its
weight on every platform where that backend makes sense.

**On macOS Intel:** explicitly out of scope for v4. Apple stopped shipping Intel Macs
in 2023; macOS 26 (Tahoe, 2025) is the last version to support Intel, with security-
only updates through ~2028. The other arch variants (Linux aarch64, Windows aarch64,
Apple Silicon) have a much longer runway. Intel Mac users should use the Docker
image or stay on v3.

**The reality today.** Sindri v3 is essentially a Linux-container-provisioning tool
with a CLI that happens to build for other hosts. This doc maps today's state onto a
v4 gap list.

## 1. v3 today, summarized

### CLI host builds

| Host    | Arch                    | v3 status                                      |
| ------- | ----------------------- | ---------------------------------------------- |
| Linux   | x86_64                  | ✅ built + tested (musl static)                |
| Linux   | aarch64                 | ✅ cross-compiled, not tested in CI            |
| macOS   | Apple Silicon (aarch64) | ✅ built natively                              |
| macOS   | Intel (x86_64)          | ⛔ out of scope for v4 (Apple-deprecated host) |
| Windows | x86_64                  | ✅ built, **never tested** (no Windows CI job) |
| Windows | aarch64                 | ❌ not built                                   |

Sources: `v3/Cargo.toml:142–162`, `.github/workflows/release-v3.yml:92–108`,
`.github/workflows/ci-v3.yml:128–145`.

### Runtime targets (what Sindri provisions)

- **Supported distros:** Ubuntu, Fedora, openSUSE only (`Distro` enum,
  `extension_types.rs`). Detection via `SINDRI_DISTRO` or `/etc/os-release`;
  unrecognized distros fall back to Ubuntu and fail at `apt`.
- **Alpine, Arch, NixOS, RHEL/CentOS/Rocky, Amazon Linux, Debian proper:** not first-
  class. Some extensions work by accident; most don't.
- **macOS as a runtime target:** no Homebrew backend, no macOS-native install method.
  `mise` and `npm` happen to work; `apt` does not; `script` works only if the script
  author wrote portable bash.
- **Windows as a runtime target:** no winget, Chocolatey, or scoop backend. Scripts
  are `.sh` with POSIX assumptions. 38 of ~60 extensions are script-based → broken
  on Windows unless the user is in WSL2.

### Architecture handling

- **mise:** arch-aware, cross-platform. Works cleanly.
- **apt/dnf/zypper:** the PM handles arch; Linux-only.
- **binary** (the `InstallMethod::Binary` path): today's extensions mostly don't use
  this; when they do, arch selection is ad-hoc string matching inside the extension
  author's `asset` pattern. No central arch-normalization helper.
- **script:** whatever the script does. Some scripts do `uname -m`, most don't.
- **multi-arch Docker images:** linux/amd64 + linux/arm64 only
  (`docs/MULTI_ARCH_SUPPORT.md`). Apple Silicon hits linux/arm64 via Docker Desktop's
  virtualization; acceptable.

### Implicit assumptions baked into v3

1. **Bash is available.** `install.sh`, `uninstall.sh`, and `SINDRI_PKG_MANAGER_LIB`
   (`/docker/lib/pkg-manager.sh`) assume POSIX shell.
2. **A Linux distro package manager exists.** Hybrid and apt methods have no mac/win
   fallback.
3. **`/etc/os-release` is present.** macOS and Windows fail this probe silently.
4. **PATH layout is Unix-like.** `executor.rs:84–141` reconstructs a POSIX PATH with
   `~/.mise/shims`, `~/go/bin`, `~/.cargo/bin`, etc. No Windows equivalents (`%PATH%`,
   `%USERPROFILE%\...`).
5. **Docker is implicitly the compatibility layer.** "Windows user? Use WSL2. macOS
   user? Use Docker Desktop." This was tenable for v3's container-provisioner framing;
   it is not good enough for a v4 that claims to be a general developer-environment tool.

## 2. What v4 must cover

### 2.1 Platform target matrix (the contract)

v4 should commit to this matrix at launch:

| Platform                    | CLI binary       | Native provisioning                                     | Notes                                  |
| --------------------------- | ---------------- | ------------------------------------------------------- | -------------------------------------- |
| Linux x86_64                | ✅               | ✅ (apt/dnf/zypper + mise + binary + script + npm)      | Primary                                |
| Linux aarch64               | ✅               | ✅ (same)                                               | Primary — CI must test                 |
| macOS Apple Silicon (arm64) | ✅               | ✅ (brew + mise + binary + script + npm)                | Primary                                |
| Windows x86_64              | ✅ **must test** | ✅ (winget + scoop + mise + binary + npm + pwsh script) | Tier 1 — real parity, not WSL fallback |
| Windows ARM64               | ✅               | ✅ (same)                                               | Tier 2 — track Windows on ARM adoption |

"Primary" = full CI (build + test + smoke-install a reference component set).
"Tier 2" = built and smoke-tested but not blocking releases.

### 2.2 Backend coverage matrix

Today v4 proposes these backends: `mise`, `apt`, `binary`, `npm`, `script`. The gap:

| Backend      | Linux                    | macOS     | Windows                      | Gap for v4                                                                   |
| ------------ | ------------------------ | --------- | ---------------------------- | ---------------------------------------------------------------------------- |
| `mise`       | ✅                       | ✅        | ✅ (via mise's Windows port) | None — mise is the workhorse                                                 |
| `apt`        | ✅                       | ❌        | ❌                           | Linux-scoped; v4 must declare this and fail gracefully off-Linux             |
| `dnf`        | ✅ (Fedora/RHEL)         | ❌        | ❌                           | Break out from v3 `apt`-as-catch-all into explicit backend                   |
| `zypper`     | ✅ (openSUSE)            | ❌        | ❌                           | Same                                                                         |
| `pacman`     | ✅ (Arch)                | ❌        | ❌                           | **NEW in v4** — Arch coverage                                                |
| `apk`        | ✅ (Alpine)              | ❌        | ❌                           | **NEW in v4** — Alpine coverage (also used by apko)                          |
| `brew`       | ⚠️ (Linuxbrew, optional) | ✅        | ❌                           | **NEW in v4** — the macOS native story                                       |
| `winget`     | ❌                       | ❌        | ✅                           | **NEW in v4** — primary Windows PM                                           |
| `scoop`      | ❌                       | ❌        | ✅                           | **NEW in v4** — dev-tool oriented Windows PM; complements winget             |
| `choco`      | ❌                       | ❌        | ✅                           | **Optional v4 add** — older ecosystem; lower priority                        |
| `binary`     | ✅                       | ✅        | ✅                           | Needs central arch/os-normalized asset selection (see §2.3)                  |
| `npm`        | ✅                       | ✅        | ✅                           | No change                                                                    |
| `pipx`       | ✅                       | ✅        | ✅                           | **NEW in v4** — first-class Python-CLI backend, currently hidden inside mise |
| `cargo`      | ✅                       | ✅        | ✅                           | **NEW in v4** — for rust-published CLIs                                      |
| `go-install` | ✅                       | ✅        | ✅                           | **NEW in v4** — for go-published CLIs                                        |
| `script`     | ✅ (bash)                | ✅ (bash) | ⚠️ (pwsh + bash variants)    | **Biggest UX gap** — see §2.4                                                |

**Recommendation:** split the backend dispatch into an explicit trait
(`Backend::install`, `Backend::supports(platform)`) and have components declare their
eligible backends per platform. Component manifests should be able to say
"on macOS use `brew:foo`, on Linux use `apt:foo`, on Windows use `winget:foo`" without
the user writing three entries. See §4 for the manifest ergonomics.

### 2.3 Architecture selection

The biggest pain in v3 is that **arch selection is inconsistent**. Each extension
author writes their own `uname -m` logic, there's no canonical mapping from uname
strings to backend-native names, and asset patterns in `install.binary.downloads`
don't have a standard templating vocabulary.

v4 should ship a **platform matrix helper** accessible from every backend and every
component:

```yaml
# In a v4 component definition:
install:
  binary:
    source: github-release
    repo: cli/cli
    version: "{{ version }}"
    assets:
      linux-x86_64: "gh_{{version}}_linux_amd64.tar.gz"
      linux-aarch64: "gh_{{version}}_linux_arm64.tar.gz"
      macos-aarch64: "gh_{{version}}_macOS_arm64.zip"
      windows-x86_64: "gh_{{version}}_windows_amd64.zip"
      windows-aarch64: "gh_{{version}}_windows_arm64.zip"
```

Sindri resolves `${os}-${arch}` once centrally from a normalized table:

| OS detection | Canonical OS | Canonical arch (from uname -m / GetNativeSystemInfo) |
| ------------ | ------------ | ---------------------------------------------------- |
| Linux        | `linux`      | `x86_64` / `aarch64`                                 |
| Darwin       | `macos`      | `aarch64` only                                       |
| Windows      | `windows`    | `x86_64` / `aarch64`                                 |

Authors don't re-derive this. If a component doesn't declare assets for the current
platform, resolution fails with a clear "this component doesn't support macos-aarch64"
message. No more "installed but broken" surprises.

### 2.4 Scripts — the deepest UX gap

38 of v3's ~60 extensions are script-based. Bash-on-Windows-via-WSL is the only
current option, which breaks the "native Windows" promise.

Three options for v4, in order of preference:

1. **Dual-variant scripts per component.** Authors ship `install.sh` (bash) and
   `install.ps1` (PowerShell) and optionally `install.{distro}.sh`. The backend picks
   by host. Already half-done in v3 per-distro script overrides; extend to OS-level.
2. **Canonical scripting language.** Require scripts to be written in a cross-
   platform interpreter Sindri ships or depends on: e.g., a bundled `rhai` / `starlark`
   / `nushell` / `deno` interpreter. Reduces author freedom but kills OS variance.
3. **Make "script" a last resort.** If v4's typed backends cover winget/scoop/brew/
   apt/dnf/zypper/pacman/apk + binary + mise + npm + pipx + cargo + go-install, the
   universe of things requiring a bespoke script shrinks dramatically. Most
   v3 scripts exist because no typed backend fit; v4 closes that gap.

**Recommendation:** combine (1) and (3). Authors may ship `install.sh` + `install.ps1`;
registry CI validates both exist when a script component declares support for that OS.
Push authors toward typed backends by making the script backend verbose in the
discovery UI ("⚠ custom script").

### 2.5 Path and shell integration

Today's `ensure_path_includes_required_dirs` (`executor.rs:84–141`) is Unix-only and
hardcodes shim/bin directory conventions. v4 needs a per-OS implementation:

| OS                   | Profile files touched                         | Env-var mechanism                                          |
| -------------------- | --------------------------------------------- | ---------------------------------------------------------- |
| Linux                | `~/.bashrc`, `~/.zshrc`, `~/.profile`         | POSIX `export`                                             |
| macOS                | `~/.zshrc` (default shell), `~/.bash_profile` | POSIX `export`                                             |
| Windows (PowerShell) | `$PROFILE`                                    | `$env:PATH += …` / `[Environment]::SetEnvironmentVariable` |
| Windows (cmd, opt)   | n/a                                           | `setx` (machine scope via registry)                        |

`configure.environment[].scope` values need reviewing: `bashrc`/`profile`/`session`
is Unix-framed. Suggest: `scope: shell-rc | login | session | user-env-var`, with the
backend translating to the right mechanism per OS.

### 2.6 Container-provisioning framing

v3 leans heavily on "provision a Linux container, run extensions inside it." That's
still a valid mode — but v4 should make the **host-provisioning** story equally
first-class:

- `sindri install` on a macOS laptop installs onto the laptop (brew + mise).
- `sindri install --target container:ubuntu-24.04` installs into a throwaway container.
- `sindri install --target ssh:user@host` installs onto a remote Linux machine.

The installation pipeline is the same; only the executor backend changes (local shell
vs docker-exec vs SSH). This framing is how Chezmoi, mise, and devbox all stayed
relevant — decouple "what to install" from "where to run the installer."

## 3. CI must prove it

Today CI tests only on `ubuntu-latest`. For v4 that has to become:

| Job                 | Runner                                                                                      | What runs                                                                                                    |
| ------------------- | ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Build matrix        | `ubuntu-latest`, `ubuntu-24.04-arm`, `macos-14` (arm64), `windows-latest`, `windows-11-arm` | `cargo build --release` per target                                                                           |
| Unit tests          | Same five                                                                                   | `cargo test`                                                                                                 |
| Smoke-install       | Same five                                                                                   | `sindri install` a reference `sindri.yaml` (mise:nodejs, binary:gh, one native PM per OS) and assert success |
| Clippy/fmt          | `ubuntu-latest` only                                                                        | Style gate                                                                                                   |
| Registry validation | `ubuntu-latest`                                                                             | Resolve-and-checksum every component in `sindri/core`                                                        |

This matrix is affordable on GitHub Actions (ARM Linux and ARM Windows runners went
GA in 2025). Without the smoke-install job, v4's cross-platform claim is aspirational.

## 4. Component authoring ergonomics

One component, many platforms — authors shouldn't write three components called
`gh-linux`, `gh-macos`, `gh-windows`. The component manifest should let a single
component declare per-platform install blocks:

```yaml
apiVersion: sindri.dev/v4
kind: Component
metadata:
  name: gh
  category: ai-dev

platforms:
  - linux-x86_64
  - linux-aarch64
  - macos-aarch64
  - windows-x86_64
  - windows-aarch64

install:
  # Default install: binary download, arch-selected at resolve time.
  default:
    binary:
      source: github-release
      repo: cli/cli
      assets: { ...see §2.3... }

  # Optional per-platform override: prefer native PM when available.
  overrides:
    macos-aarch64: { brew: { package: gh } }
    linux-x86_64: { apt: { packages: [gh], repositories: [...] } }
    linux-aarch64: { apt: { packages: [gh], repositories: [...] } }
    windows-x86_64: { winget: { package: GitHub.cli } }
    windows-aarch64: { winget: { package: GitHub.cli } }
```

Users don't see this complexity: they write `binary:gh@2.62.0` (or `gh@2.62.0` if
`binary` is the default) in their `sindri.yaml`, and resolution picks the right block.
Registry CI enforces that every declared `platforms:` entry has either a default or
an override that works on it.

## 5. Gap list — what v4 must deliver

Prioritized. Not-doing is stated explicitly.

### Must (v4.0 blockers)

1. **Build & test matrix** covers Linux x86_64/aarch64, macOS Apple Silicon, Windows x86_64. CI runs `cargo test` + a smoke-install on every combo.
2. **Native package-manager backends** land for macOS (`brew`) and Windows (`winget` + `scoop`). Sindri/core ships components exercising each.
3. **Per-OS install-script variants** (`install.sh` + `install.ps1`) with registry-CI validation that declared platforms have a working path.
4. **Central platform-matrix resolver** for `binary` asset selection — eliminates ad-hoc `uname -m` logic.
5. **PATH and shell-rc abstraction** for Windows PowerShell.
6. **Platform matrix declared in every component** — `platforms:` list is mandatory; resolution fails clearly when absent.
7. **Clear docs for unsupported runtimes** — Windows in cmd.exe-only, Linux on 32-bit ARM, BSD: not supported, stated upfront.

### Should (v4.0 if possible, v4.1 otherwise)

8. **Additional Linux distros:** `pacman` (Arch), `apk` (Alpine) backends. Detection expanded; fallback to `script` or failure, no silent Ubuntu default.
9. **`pipx`, `cargo`, `go-install` as first-class backends.**
10. **Execution-target abstraction** — local / docker / ssh / WSL — decoupled from backend.
11. **Homebrew tap model** for private registries on macOS (users install via `brew install sindri` from a private tap).
12. **ARM64 Windows** (`aarch64-pc-windows-msvc`) in the release matrix.

### Won't (explicitly deferred)

13. **FreeBSD/OpenBSD/Illumos** — not blocking v4; track community demand.
14. **32-bit x86 / armv7** — not supported.
    14b. **macOS Intel (x86_64)** — out of scope for v4; Apple-deprecated host, v3 or the Docker image covers remaining users.
15. **Chocolatey** backend — winget + scoop cover the use cases; revisit if users ask.
16. **Native package-manager _publishing_** (Sindri itself published to brew/winget/apt by automation) — ergonomic win but separate project from this research.

## 6. Open questions for this domain

Added to `05-open-questions.md` as §20–§24:

20. How opinionated should v4 be about Windows shell — PowerShell 7+ only, or both pwsh and Windows PowerShell 5.1?
21. Do we ship our own `brew` tap for Sindri itself in v4.0, or point users at direct downloads?
22. When a component supports multiple backends on a platform (e.g., macOS has both `brew:gh` and `binary:gh` variants), does the user pick explicitly, does Sindri auto-pick, or does the component declare a preference?
23. WSL handling on Windows: detect and warn, detect and use, or ignore?
24. Container-execution backend: docker only, or abstract over docker/podman/nerdctl/finch?
