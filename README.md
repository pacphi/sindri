# Sindri

[![License](https://img.shields.io/github/license/pacphi/sindri)](LICENSE)
[![CI v2](https://github.com/pacphi/sindri/actions/workflows/ci-v2.yml/badge.svg?branch=v2)](https://github.com/pacphi/sindri/actions/workflows/ci-v2.yml)
[![CI v3](https://github.com/pacphi/sindri/actions/workflows/ci-v3.yml/badge.svg?branch=v3)](https://github.com/pacphi/sindri/actions/workflows/ci-v3.yml)
[![CI v4](https://github.com/pacphi/sindri/actions/workflows/ci-v4.yml/badge.svg?branch=v4)](https://github.com/pacphi/sindri/actions/workflows/ci-v4.yml)
[![FAQ](https://img.shields.io/badge/FAQ-on%20fly.dev-blue)](https://sindri-faq.fly.dev)
[![GHCR](https://img.shields.io/badge/GHCR-container%20registry-blue)](https://github.com/pacphi/sindri/pkgs/container/sindri)

A declarative, provider-agnostic cloud development environment system. Deploy consistent
development environments to Fly.io, local Docker, or via DevPod to Kubernetes, AWS, GCP,
Azure, and other cloud providers using YAML-defined extensions.

```text
   ███████╗██╗███╗   ██╗██████╗ ██████╗ ██╗
   ██╔════╝██║████╗  ██║██╔══██╗██╔══██╗██║
   ███████╗██║██╔██╗ ██║██║  ██║██████╔╝██║
   ╚════██║██║██║╚██╗██║██║  ██║██╔══██╗██║
   ███████║██║██║ ╚████║██████╔╝██║  ██║██║
   ╚══════╝╚═╝╚═╝  ╚═══╝╚═════╝ ╚═╝  ╚═╝╚═╝

   🔨 Forging Development Environments
   📦 https://github.com/pacphi/sindri
```

> ## You are on `main`
>
> **`main` carries no product source.** It hosts the umbrella documentation, project-wide
> governance (`SECURITY.md`, `CONTRIBUTING.md`, `CODEOWNERS`), and the centralized
> `.github/` that routes CI/CD workflows to the active maintenance branches.
>
> Pick a version below to see source, build, deploy, and version-specific docs.

## Pick your version

| Branch                                               | Stack                        | Status                                    | What's there                                                         |
| ---------------------------------------------------- | ---------------------------- | ----------------------------------------- | -------------------------------------------------------------------- |
| **[`v1`](https://github.com/pacphi/sindri/tree/v1)** | legacy bash                  | **End-of-life** — security backports only | Historical changelog                                                 |
| **[`v2`](https://github.com/pacphi/sindri/tree/v2)** | Bash + Docker                | **Maintenance**                           | CLI, Docker compose, deploy scripts, extensions                      |
| **[`v3`](https://github.com/pacphi/sindri/tree/v3)** | Rust workspace + npm wrapper | **Active**                                | Cargo workspace, `@pacphi/sindri-cli` packages, full provider matrix |
| **[`v4`](https://github.com/pacphi/sindri/tree/v4)** | Rust, redesigned             | **Pre-release**                           | New architecture: registry-core, renovate-plugin, tools              |

Not sure which to use? Read [docs/COMPARISON_GUIDE.md](docs/COMPARISON_GUIDE.md).

## About the name

**Sindri** (Old Norse: "spark") was a legendary dwarf blacksmith in Norse mythology,
renowned for forging three of the most powerful artifacts: Mjölnir (Thor's hammer),
Draupnir (Odin's ring), and Gullinbursti (Freyr's golden boar).

Like its mythological namesake, Sindri forges powerful development environments from
raw materials — transforming cloud infrastructure, YAML configuration, and Docker into
consistent, reproducible developer workspaces.

## Repository layout

```
main         ← you are here. Umbrella docs + centralized .github/.
├─ v1        ← end-of-life (read-only)
├─ v2        ← Bash/Docker maintenance
├─ v3        ← Rust active development
└─ v4        ← Rust pre-release
```

All CI, release, and provider workflows live under `.github/workflows/` on **`main`** and
trigger automatically on push or pull request to a `v*` branch. There are no workflows on
the `v*` branches themselves. See [`.github/WORKFLOW_ARCHITECTURE.md`](.github/WORKFLOW_ARCHITECTURE.md).

## Contributing

- New work targets a `v*` branch — never `main`. Open your PR against `v3` (or `v4` if
  it's a v4-only feature). See [CONTRIBUTING.md](CONTRIBUTING.md).
- Security disclosures: [SECURITY.md](SECURITY.md).
- Release process: [docs/RELEASE.md](docs/RELEASE.md).
- Changelog management: [docs/CHANGELOG_MANAGEMENT.md](docs/CHANGELOG_MANAGEMENT.md).

## Related projects

Sindri is part of a three-project ecosystem:

| Repository                                     | Description                                                                                     |
| ---------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| **sindri** (this repo)                         | CLI tool and extension ecosystem — provisions and configures instances                          |
| [mimir](https://github.com/pacphi/mimir)       | Fleet management control plane — orchestrates, observes, and administers instances at scale     |
| [draupnir](https://github.com/pacphi/draupnir) | Lightweight per-instance agent — bridges each instance to the mimir control plane via WebSocket |

## License

MIT License — see [LICENSE](LICENSE).

![Sindri at his forge](sindri.png)
