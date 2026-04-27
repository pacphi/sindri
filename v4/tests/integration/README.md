# sindri v4 — integration test harness

End-to-end scenarios that drive the `sindri` CLI binary through `assert_cmd`.
Each test isolates state in a tempdir and points `$HOME` at it so the
registry cache (`~/.sindri/cache/registries/...`) stays sandboxed.

## How to run

```bash
cd v4
cargo test -p integration-tests
```

To run a single scenario:

```bash
cd v4
cargo test -p integration-tests --test init_validate_resolve
```

To include the `#[ignore]`-d scenarios (currently only the admission gate
test, pending Wave 3A.2 manifest fetch):

```bash
cd v4
cargo test -p integration-tests -- --ignored
```

## Layout

```
fixtures/
├── registries/
│   ├── prototype/         # well-formed local registry (mise:nodejs, binary:gh, binary:shellcheck)
│   └── bad-no-license/    # deliberately broken — drives `registry lint`
├── manifests/             # sample sindri.yaml inputs
└── policies/              # sample sindri.policy.yaml
tests/
├── helpers.rs                                       # shared utilities (sindri_cmd, temp_workdir, …)
├── init_validate_resolve.rs                         # init → validate → resolve --offline
├── apply_local_idempotent.rs                        # apply --dry-run twice → identical plan
├── admission_gate_denies_unsupported_platform.rs    # ignored, see FIXME(wave-4a-followup)
├── registry_lint_finds_missing_license.rs           # lint flags missing metadata.license
└── lockfile_bom_emission.rs                         # bom --format spdx emits valid SPDX 2.3 JSON
```

## Test hooks

* `SINDRI_TEST_PLATFORM_OVERRIDE=<os>-<arch>` — short-circuits
  `sindri_core::platform::Platform::current()` so admission gates can be
  exercised without cross-compiling. Recognised values: `linux-x86_64`,
  `linux-aarch64`, `macos-x86_64`, `macos-aarch64`, `windows-x86_64`,
  `windows-aarch64`.

## Skip-on-CI gating

Scenarios that touch tempdirs in ways that have proven flaky on Windows
runners are guarded with `#[cfg_attr(windows, ignore)]` and tagged
`FIXME(wave-4a-followup):`.
