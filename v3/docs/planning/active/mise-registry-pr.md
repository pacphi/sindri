# mise Registry PR Template

## PR Title

```
registry: add sindri (github:pacphi/sindri)
```

## PR Description

```markdown
Adds [Sindri](https://github.com/pacphi/sindri) - a multi-cloud development environment orchestrator written in Rust.

## Tool Information

- **Repository**: https://github.com/pacphi/sindri
- **Language**: Rust
- **Platforms**: Linux (x86_64, ARM64), macOS (x86_64, ARM64), Windows (x86_64)
- **Release Format**: GitHub Releases with tar.gz/zip archives
- **Checksums**: SHA256 checksums provided in `checksums.txt`

## Testing

Tested with:

- `mise use github:pacphi/sindri`
- `mise exec github:pacphi/sindri -- sindri --version`
```

## registry.toml Entry

Add the following to `registry.toml` in alphabetical order (under `[tools]` section):

```toml
[tools.sindri]
description = "Multi-cloud development environment orchestrator"
backends = ["github:pacphi/sindri"]
test = ["sindri --version", "Sindri"]
```

## Notes

### Asset Naming Convention

Sindri v3 releases use this naming pattern:

- Linux x86_64: `sindri-v{version}-linux-x86_64.tar.gz`
- Linux ARM64: `sindri-v{version}-linux-aarch64.tar.gz`
- macOS x86_64: `sindri-v{version}-macos-x86_64.tar.gz`
- macOS ARM64: `sindri-v{version}-macos-aarch64.tar.gz`
- Windows x86_64: `sindri-v{version}-windows-x86_64.zip`

mise's GitHub backend auto-detects the correct asset based on platform.

### Version Prefix

Sindri uses `v` prefix for tags (e.g., `v3.0.0`), which is the default for the GitHub backend.

### Test Command

The `sindri --version` command outputs: `Sindri 3.x.x (rustc 1.xx.x)`

The test pattern `Sindri` matches the output to verify installation.

## Submission Steps

1. Fork https://github.com/jdx/mise
2. Clone your fork locally
3. Edit `registry.toml`:
   ```bash
   # Find the right alphabetical location and add the entry
   vim registry.toml
   ```
4. Commit with conventional commit message:
   ```bash
   git commit -m "registry: add sindri (github:pacphi/sindri)"
   ```
5. Push and create PR:
   ```bash
   git push origin main
   gh pr create --title "registry: add sindri (github:pacphi/sindri)" --body "..."
   ```

## Post-Merge Testing

After the PR is merged:

```bash
# Update mise to get new registry
mise self-update

# Test installation
mise use -g sindri
sindri --version

# Test specific version
mise use -g sindri@3.0.0
```

## Future: aqua Backend

Once established, consider submitting to [aquaproj/aqua-registry](https://github.com/aquaproj/aqua-registry) for enhanced security (Cosign/SLSA verification), then update the mise entry to:

```toml
[tools.sindri]
description = "Multi-cloud development environment orchestrator"
backends = ["aqua:pacphi/sindri", "github:pacphi/sindri"]
test = ["sindri --version", "Sindri"]
```
