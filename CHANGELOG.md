# Sindri Changelog

`main` carries no product source. Each maintenance branch keeps its own
changelog, located at the branch root.

## Per-version changelogs

| Branch                                                        | Changelog             | Status      |
| ------------------------------------------------------------- | --------------------- | ----------- |
| [`v1`](https://github.com/pacphi/sindri/blob/v1/CHANGELOG.md) | v1 (legacy bash)      | End-of-life |
| [`v2`](https://github.com/pacphi/sindri/blob/v2/CHANGELOG.md) | v2 (Bash + Docker)    | Maintenance |
| [`v3`](https://github.com/pacphi/sindri/blob/v3/CHANGELOG.md) | v3 (Rust workspace)   | Active      |
| [`v4`](https://github.com/pacphi/sindri/blob/v4/CHANGELOG.md) | v4 (Rust, redesigned) | Pre-release |

## Per-version release notes

Long-form release notes (breaking changes, migration steps) live alongside each
branch's changelog:

| Branch | Release notes                                                                                                                                                                  |
| ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `v2`   | [`v2/RELEASE_NOTES.md`](https://github.com/pacphi/sindri/blob/v2/RELEASE_NOTES.md)                                                                                             |
| `v3`   | [`v3/RELEASE_NOTES.md`](https://github.com/pacphi/sindri/blob/v3/RELEASE_NOTES.md), [`v3/RELEASE_NOTES.3.1.md`](https://github.com/pacphi/sindri/blob/v3/RELEASE_NOTES.3.1.md) |

## Format

All changelogs follow [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and
adhere to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Sections used: `Added`, `Fixed`, `Changed`, `Documentation`, `Dependencies`,
`Performance`, `Tests`, `Maintenance`.

## Reorganization (April 2026)

This repository was reorganized into dedicated `v1`/`v2`/`v3`/`v4` maintenance
branches. See [`docs/REPO_REORG_PLAN.md`](docs/REPO_REORG_PLAN.md) and the
`pre-reorg-2026-04-25` tag for the prior layout.

## Quick links

- [Latest release](https://github.com/pacphi/sindri/releases/latest)
- [All releases](https://github.com/pacphi/sindri/releases)
- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [Comparison guide](docs/COMPARISON_GUIDE.md)
