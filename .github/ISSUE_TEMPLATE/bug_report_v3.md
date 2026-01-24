---
name: "Bug Report (V3 - Rust CLI)"
about: Report a bug in Sindri V3 (Rust CLI implementation)
title: "[V3 BUG]: "
labels: ["bug", "v3", "triage"]
assignees: ""
---

## Version Information

- **Sindri Version**: <!-- Run: sindri version --json -->
- **Rust Version**: <!-- If building from source: rustc --version -->
- **OS**: <!-- e.g., macOS 14.2, Ubuntu 22.04, Windows 11 -->

## Provider

- [ ] Docker (local)
- [ ] Fly.io
- [ ] DevPod
- [ ] E2B
- [ ] Kubernetes (kind/k3d)

## Command

<!-- Which sindri command triggered the bug? -->

```bash
sindri <command> [args]
```

## Bug Description

<!-- A clear and concise description of the bug -->

## Steps to Reproduce

1.
2.
3.

## Expected Behavior

<!-- What you expected to happen -->

## Actual Behavior

<!-- What actually happened -->

## Configuration

<details>
<summary>sindri.yaml (sanitized)</summary>

```yaml
# Paste your sindri.yaml here (remove any secrets/tokens)
```

</details>

## Logs

<details>
<summary>Error output</summary>

```
# Run with RUST_BACKTRACE=1 for more details
# Paste relevant error output here
```

</details>

## Environment

<details>
<summary>sindri doctor output</summary>

```
# Run: sindri doctor --all
```

</details>

## Additional Context

<!-- Any other context about the problem -->
