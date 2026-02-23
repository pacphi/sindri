# @sindri/cli

The Sindri CLI distributed as an npm package.

## Installation

```bash
npm install -g @sindri/cli
# or
pnpm add -g @sindri/cli
```

## Usage

```bash
sindri --help
sindri extension list --all --json
sindri profile list --json
sindri version --json
```

## How it works

This package uses the **optionalDependencies + platform packages** pattern (same as
[esbuild](https://esbuild.github.io), [Biome](https://biomejs.dev), and [SWC](https://swc.rs)).

On install, npm/pnpm selects only the platform-specific binary package that matches your OS and
CPU. The wrapper `lib/index.js` resolves the correct binary path and delegates all CLI arguments
to it.

### Supported platforms

| Platform | Package |
|----------|---------|
| macOS Apple Silicon | `@sindri/cli-darwin-arm64` |
| macOS Intel | `@sindri/cli-darwin-x64` |
| Linux x64 | `@sindri/cli-linux-x64` |
| Linux arm64 | `@sindri/cli-linux-arm64` |
| Windows x64 | `@sindri/cli-win32-x64` |

## Programmatic usage

```javascript
const { getInstalledBinaryPath } = require("@sindri/cli");
const bin = getInstalledBinaryPath();
// bin = "/path/to/node_modules/@sindri/cli-linux-x64/sindri"
```

## Environment variables

| Variable | Description |
|----------|-------------|
| `SINDRI_BIN_PATH` | Override the binary path entirely |
| `SINDRI_SKIP_DOWNLOAD` | Set to `1` to skip postinstall download fallback |

## Version alignment

The npm package version is always identical to the Rust binary version (`Cargo.toml`
is the single source of truth). Releases are automated via
[cargo-dist](https://opensource.axo.dev/cargo-dist/) on `git tag v3.x.y`.
