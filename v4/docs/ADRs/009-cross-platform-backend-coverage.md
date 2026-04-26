# ADR-009: Full Cross-Platform Backend Coverage

**Status:** Accepted
**Date:** 2026-04-24
**Deciders:** sindri-dev team

## Context

v3 is essentially a Linux-container provisioning tool with implicit assumptions:

- Bash is available everywhere.
- A distro package manager (apt/dnf/zypper) exists.
- `/etc/os-release` is present.
- PATH is POSIX-structured.

macOS and Windows are supported only accidentally (mise and npm happen to work; apt does
not; 38/60 extensions are script-based and broken on native Windows).

v4 commits to "excellent UX everywhere" on the stated target matrix.

## Decision

### Committed platform matrix

| Platform                    | CLI binary | Native provisioning               |
| --------------------------- | ---------- | --------------------------------- |
| Linux x86_64                | ✅         | ✅ Full CI (build + test + smoke) |
| Linux aarch64               | ✅         | ✅ Full CI                        |
| macOS Apple Silicon (arm64) | ✅         | ✅ Full CI                        |
| Windows x86_64              | ✅         | ✅ Tier 1 — real parity           |
| Windows ARM64               | ✅         | ✅ Tier 2 — built + smoke         |

macOS Intel (x86_64) is explicitly out of scope (Apple-deprecated host). Users on
Intel Mac should use the Docker image or remain on v3.

### New backends required for v4.0

The following backends are **new to v4** (not present in v3):

| Backend      | Platform        | Rationale                                                                                        |
| ------------ | --------------- | ------------------------------------------------------------------------------------------------ |
| `brew`       | macOS (primary) | The native macOS story; no brew = no macOS parity                                                |
| `winget`     | Windows         | Primary Windows PM; necessary for Tier 1 parity                                                  |
| `scoop`      | Windows         | Dev-tool-focused; complements winget                                                             |
| `pacman`     | Linux (Arch)    | Arch is a major distro; v3 treated as Ubuntu-fallback                                            |
| `apk`        | Linux (Alpine)  | Alpine is dominant in CI/container contexts                                                      |
| `pipx`       | All             | First-class Python CLI tools (hidden inside mise in v3)                                          |
| `cargo`      | All             | Rust-published CLIs (e.g., `ripgrep`, `fd`, `bat`)                                               |
| `go-install` | All             | Go-published CLIs (e.g., `golangci-lint`)                                                        |
| `sdkman`     | Linux + macOS   | JVM ecosystem (Java, Kotlin, Scala, Gradle, Maven, Groovy) — `sdk install <candidate> <version>` |

Existing backends (`mise`, `apt`, `dnf`, `zypper`, `binary`, `npm`, `script`) are
retained; `apt`/`dnf`/`zypper` are made explicit siblings rather than a single `Linux PM`
arm.

### SDKMAN backend — JVM ecosystem decomposition

`sdkman` is the canonical backend for the JVM ecosystem. SDKMAN is Unix-only (Linux and
macOS) — Windows users of JVM tools should use the `scoop` or `winget` overrides in the
per-component `install.overrides` block (deferred to component authoring).

The v3 `jvm` bundle extension is replaced in v4 by:

1. **`script:sdkman`** — installs SDKMAN itself via its install script (prerequisite)
2. **Seven atomic `sdkman:` components** — each installs one candidate:
   `sdkman:java`, `sdkman:maven`, `sdkman:gradle`, `sdkman:kotlin`, `sdkman:scala`,
   `sdkman:groovy`, `sdkman:springboot`
3. **`collection:jvm`** — meta-component that depends on `script:sdkman` + all seven atoms,
   preserving the "install everything JVM" UX of the v3 bundle

Component manifest shape:

```yaml
install:
  sdkman:
    candidate: java # SDKMAN candidate name (sdk install <candidate> <version>)
    version: "21.0.5-tem" # Exact SDKMAN version identifier
depends_on:
  - "script:sdkman" # SDKMAN must be bootstrapped first
```

Install is delegated to `bash -c 'source "$SDKMAN_DIR/bin/sdkman-init.sh" && sdk install ...'`
because `sdk` is a shell function, not a standalone binary.

### Central platform-matrix resolver for binary assets

Component authors use a structured `assets:` map keyed by `{os}-{arch}`:

```yaml
install:
  default:
    binary:
      source: github-release
      repo: cli/cli
      assets:
        linux-x86_64: "gh_{{version}}_linux_amd64.tar.gz"
        linux-aarch64: "gh_{{version}}_linux_arm64.tar.gz"
        macos-aarch64: "gh_{{version}}_macOS_arm64.zip"
        windows-x86_64: "gh_{{version}}_windows_amd64.zip"
        windows-aarch64: "gh_{{version}}_windows_arm64.zip"
      checksums:
        linux-x86_64: sha256:aaaa...
        ...
```

Sindri resolves `{os}-{arch}` once centrally using a canonical table derived from
`uname -m` / `GetNativeSystemInfo`. Ad-hoc `uname -m` logic inside extensions is gone.
If a component doesn't declare assets for the current platform, resolution fails with
a clear message. Open question from `07-cross-platform.md` §2.3 resolved.

### Per-OS install-block overrides in components

A single component can express per-platform install preferences without creating three
separate components:

```yaml
install:
  default:
    binary: { ... }
  overrides:
    macos-aarch64: { brew: { package: gh } }
    linux-x86_64: { apt: { packages: [gh] } }
    windows-x86_64: { winget: { package: GitHub.cli } }
```

`platforms:` list is **mandatory** on every component. Resolution fails explicitly when
a component is used on an undeclared platform. Open question Q7 from `07-cross-platform.md`
resolved: "platform matrix declared in every component."

### Script backends — dual-variant

Script components may ship `install.sh` (bash) and `install.ps1` (PowerShell). Registry
CI validates that both exist when a `script:` component declares Windows support.
`sindri ls` marks script-backend components with `⚠ custom script` to push authors
toward typed backends.

### PATH and shell-rc abstraction

`configure.environment[].scope` is extended from Unix-only (`bashrc`/`profile`/`session`)
to cross-platform: `shell-rc | login | session | user-env-var`. The backend translates to
the right mechanism per OS (`export` on POSIX; `$env:PATH += …` / `[Environment]::Set…` on
Windows PowerShell 7+). Open question Q24 resolved: PowerShell 7+ only.

### CI proof

Four GHA runner types (ubuntu-latest, ubuntu-24.04-arm, macos-14, windows-latest) run:

- `cargo build --release`
- `cargo test`
- Smoke-install of a reference `sindri.yaml` (one component per new backend)

## What's deferred to v4.1

- Additional new PM backends: `choco` (Windows), `Homebrew tap publishing`.
- Execution-target abstraction (local/docker/ssh/WSL) — covered by ADR-017.
- FreeBSD/OpenBSD — not supported; track demand.

## References

- Research: `07-cross-platform.md`, `05-open-questions.md` Q20–Q24
