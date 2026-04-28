# @sindri-dev/renovate-config-sindri

Renovate manager plugin for [Sindri v4](https://github.com/pacphi/sindri) BOM manifests.

Enables automated version-bump PRs for components pinned in `sindri.yaml`, `mise.toml`,
and `sindri.lock`. Implements [ADR-015](../docs/ADRs/015-renovate-manager-plugin.md).

## Usage

Add this preset to your `renovate.json`:

```json
{
  "$schema": "https://docs.renovatebot.com/renovate-schema.json",
  "extends": ["@sindri-dev/renovate-config-sindri"]
}
```

Renovate will then:

1. Scan `sindri.yaml` for `address: backend:name@version` entries.
2. Scan `mise.toml` `[tools]` blocks for version pins.
3. Scan `sindri.lock` for pinned digests.
4. Open PRs bumping any outdated versions.
5. Run `sindri resolve` after each bump to regenerate `sindri.lock`.

## Datasource mapping

| Sindri backend | Example address | Renovate datasource | Versioning |
|---|---|---|---|
| `mise:nodejs` | `mise:nodejs@22.4.0` | `node` | `node` |
| `mise:python` | `mise:python@3.12.5` | `python-version` | `pep440` |
| `mise:rust` | `mise:rust@1.79.0` | `github-tags` (rust-lang/rust) | `semver` |
| `mise:go` | `mise:go@1.22.4` | `go-version` | `semver` |
| `mise:terraform` | `mise:terraform@1.9.3` | `github-releases` (hashicorp/terraform) | `hashicorp` |
| `mise:kubectl` | `mise:kubectl@1.31.3` | `github-releases` (kubernetes/kubernetes) | `semver` |
| `mise:<other>` | `mise:helm@3.15.2` | `mise` (fallback) | `semver` |
| `cargo:` | `cargo:ripgrep@14.1.0` | `crate` | `semver` |
| `npm:` | `npm:typescript@5.5.3` | `npm` | `npm` |
| `pipx:` | `pipx:black@24.4.2` | `pypi` | `pep440` |
| `go-install:` | `go-install:golang.org/x/tools/gopls@0.16.1` | `go` | `semver` |
| `binary:` | `binary:kubectl@1.31.3` (with `url:`) | `github-releases` | `semver` |
| `brew:` | `brew:jq@1.7.1` | `homebrew` | `semver` |

### Inline hints

For components whose datasource cannot be inferred automatically, add an inline hint comment
immediately before the `address:` line:

```yaml
components:
  # renovate: depName=kubernetes/kubernetes datasource=github-releases
  - address: "binary:kubectl@1.31.3"
    url: "https://github.com/kubernetes/kubectl/releases/download/v1.31.3/kubectl-linux-amd64"
```

Supported hint fields: `depName`, `datasource`, `versioning`.

## Post-upgrade tasks

After Renovate bumps a version, the plugin runs:

```sh
sindri resolve
```

This regenerates `sindri.lock` (digest + resolved timestamp), so the bumped commit is
always consistent. Requires `sindri` to be available in the CI environment.

## Local development

```sh
cd v4/renovate-plugin
npm install
npm test          # run vitest
npm run build     # validate all required files
```

## Publishing (deferred — not included in this PR)

This package is **publish-ready** but publication is deferred until the v4.0 CLI ships.

When ready:

```sh
# 1. Update version in package.json
# 2. Ensure you are authenticated with npm
npm whoami

# 3. Dry-run to verify the files list
npm pack --dry-run

# 4. Publish
npm publish --access public
```

Consumers can then reference the preset as `@sindri-dev/renovate-config-sindri` without
any version suffix — Renovate resolves the latest published version automatically.

## License

MIT
