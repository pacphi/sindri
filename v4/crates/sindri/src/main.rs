#![allow(dead_code)]

use clap::{Parser, Subcommand};

mod commands;
mod validate;

use commands::policy::PolicyCmd;
use commands::registry::RegistryCmd;
use commands::target::{PluginSub, TargetCmd};

#[derive(Parser)]
#[command(
    name = "sindri",
    version = env!("CARGO_PKG_VERSION"),
    about = "Sindri v4 — environment bootstrapping CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate sindri.yaml against the JSON schema
    Validate {
        #[arg(default_value = "sindri.yaml")]
        path: String,
        #[arg(long)]
        online: bool,
        #[arg(long)]
        json: bool,
    },
    /// List available components from configured registries
    Ls {
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        installed: bool,
        #[arg(long)]
        outdated: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        refresh: bool,
    },
    /// Registry management
    Registry {
        #[command(subcommand)]
        cmd: RegistrySubcmds,
    },
    /// Resolve sindri.yaml into sindri.lock (Sprint 3)
    ///
    /// Equivalent to the `lock` verb in the source-modes plan; Sindri v4
    /// keeps a single `resolve` command that writes the per-target
    /// lockfile. The `--strict-oci` flag (DDD-08, ADR-028 — Phase 2)
    /// fails the resolve when any component's source is not OCI / local-
    /// OCI; it overrides `registry.policy.strict_oci` in `sindri.yaml`
    /// when both are set.
    Resolve {
        #[arg(short, long, default_value = "sindri.yaml")]
        manifest: String,
        #[arg(long)]
        offline: bool,
        #[arg(long)]
        refresh: bool,
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        explain: Option<String>,
        #[arg(long, default_value = "local")]
        target: String,
        /// Reject any lockfile that contains a non-production-grade
        /// source (DDD-08, ADR-028 — Phase 2). Overrides
        /// `registry.policy.strict_oci` in `sindri.yaml` when both are
        /// set. CI templates flip this on.
        #[arg(long = "strict-oci")]
        strict_oci: bool,
    },
    /// Policy management (ADR-008)
    Policy {
        #[command(subcommand)]
        cmd: PolicySubcmds,
    },
    /// Initialize a new sindri.yaml in the current directory (ADR-011)
    Init {
        #[arg(long)]
        template: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        policy: Option<String>,
        #[arg(long)]
        non_interactive: bool,
        #[arg(long)]
        force: bool,
    },
    /// Add a component to sindri.yaml
    Add {
        address: String,
        #[arg(short, long, default_value = "sindri.yaml")]
        manifest: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        apply: bool,
    },
    /// Remove a component from sindri.yaml
    Remove {
        address: String,
        #[arg(short, long, default_value = "sindri.yaml")]
        manifest: String,
    },
    /// Pin a component to an exact version
    Pin {
        address: String,
        version: String,
        #[arg(short, long, default_value = "sindri.yaml")]
        manifest: String,
    },
    /// Unpin a component (track latest)
    Unpin {
        address: String,
        #[arg(short, long, default_value = "sindri.yaml")]
        manifest: String,
    },
    /// Upgrade one or all components to their latest available version
    Upgrade {
        component: Option<String>,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        check: bool,
        #[arg(short, long, default_value = "sindri.yaml")]
        manifest: String,
    },
    /// Show the apply plan from sindri.lock (without applying)
    Plan {
        #[arg(long, default_value = "local")]
        target: String,
        #[arg(long)]
        json: bool,
    },
    /// Show divergence between sindri.lock and installed state
    Diff {
        #[arg(long, default_value = "local")]
        target: String,
        #[arg(long)]
        json: bool,
    },
    /// Search for components by name, tag, or description
    Search {
        query: String,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show detailed info about a component
    Show {
        address: String,
        #[arg(long)]
        versions: bool,
        #[arg(long)]
        json: bool,
    },
    /// Show dependency graph for a component or collection
    Graph {
        address: String,
        #[arg(long, default_value = "text")]
        format: String,
        #[arg(long)]
        reverse: bool,
    },
    /// Explain why a component is in the dependency graph
    Explain {
        component: String,
        #[arg(long, name = "in")]
        in_collection: Option<String>,
    },
    /// Generate an SBOM from the resolved lockfile (ADR-007)
    Bom {
        #[arg(long, default_value = "spdx")]
        format: String,
        #[arg(long, default_value = "local")]
        target: String,
        #[arg(long, short)]
        output: Option<String>,
    },
    /// Show installed-state log (StatusLedger)
    Log {
        #[arg(long)]
        last: Option<usize>,
        #[arg(long)]
        json: bool,
    },
    /// Diagnose configuration issues and backend availability
    Doctor {
        #[arg(long)]
        target: Option<String>,
        /// Apply remediations for fixable failures.
        #[arg(long, conflicts_with = "dry_run")]
        fix: bool,
        /// Print would-be remediations without writing.
        #[arg(long, conflicts_with = "fix")]
        dry_run: bool,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
        #[arg(long)]
        components: bool,
        /// Phase 5 (ADR-027 §Phase 5): focused doctor view that runs
        /// Gate 5 against the current manifest+target set with no apply
        /// side effects.
        #[arg(long)]
        auth: bool,
        /// Manifest path (default: `sindri.yaml`).
        #[arg(long, default_value = "sindri.yaml")]
        manifest: String,
    },
    /// Validate / inspect / store secret references (Sprint 12)
    Secrets {
        #[command(subcommand)]
        cmd: SecretsSubcmds,
    },
    /// Create a tarball of the user's sindri state
    Backup {
        /// Output path (file or directory). Defaults to cwd with a
        /// timestamped filename.
        #[arg(long, short)]
        output: Option<std::path::PathBuf>,
        /// Include `~/.sindri/cache/registries/` (large; off by default).
        #[arg(long)]
        include_cache: bool,
        /// Compression algorithm: `gzip` (default, `.tar.gz`) or `zstd` (`.tar.zst`).
        /// Restore auto-detects by magic bytes regardless of this flag.
        #[arg(long, default_value = "gzip")]
        compression: String,
    },
    /// Restore a `sindri backup` archive
    Restore {
        /// Path to the archive.
        archive: std::path::PathBuf,
        /// Print the file list without writing.
        #[arg(long)]
        dry_run: bool,
        /// Overwrite existing destination files.
        #[arg(long)]
        force: bool,
    },
    /// Target management (ADR-017, ADR-023)
    Target {
        #[command(subcommand)]
        cmd: TargetSubcmds,
    },
    /// Inspect and manage auth bindings (Phase 5, ADR-027)
    Auth {
        #[command(subcommand)]
        cmd: AuthSubcmds,
    },
    /// Apply sindri.lock to the target
    Apply {
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = "local")]
        target: String,
        /// Skip SBOM auto-emit on success (ADR-007).
        #[arg(long)]
        no_bom: bool,
        /// Resume from the last failing component instead of restarting the
        /// whole apply (Wave 5H, D19).  Components already in `completed`
        /// state are skipped; `failed` / `pending` components are retried.
        #[arg(long)]
        resume: bool,
        /// Wipe the apply-state file for the current BOM so the next apply
        /// starts from scratch (Wave 5H, D19).  Can be combined with
        /// `--resume` to clear-then-resume (effectively a full re-apply).
        #[arg(long)]
        clear_state: bool,
        /// Bypass auth-aware credential redemption (Phase 2A, ADR-027).
        /// Use only as an emergency override. Every component whose
        /// redemption was skipped emits an `AuthSkippedByUser` ledger
        /// event so the bypass is auditable. Required-binding presence
        /// is still enforced by Gate 5 unless that gate is also relaxed.
        #[arg(long)]
        skip_auth: bool,
    },
    /// Open `$EDITOR` on a sindri config with save-time validation (ADR-011)
    Edit {
        /// `policy` to edit `sindri.policy.yaml`. Omit to edit `sindri.yaml`.
        target: Option<String>,
        /// Print the local JSON-schema path and exit.
        #[arg(long)]
        schema: bool,
        /// Skip the interactive re-open prompt on validation failure.
        #[arg(long)]
        no_prompt: bool,
    },
    /// Roll one component back to its previous pinned version (ADR-011)
    Rollback {
        component: String,
        #[arg(long, default_value = "sindri.lock")]
        lockfile: String,
        #[arg(long)]
        reason: Option<String>,
    },
    /// Self-update the `sindri` CLI binary (ADR-011)
    SelfUpgrade {
        /// Detect the install method and print what would run, but do not execute.
        #[arg(long)]
        dry_run: bool,
    },
    /// Emit a shell-completion script (ADR-011).
    ///
    /// Suggested install paths:
    ///   bash       — ~/.local/share/bash-completion/completions/sindri
    ///   zsh        — a directory in $fpath, e.g. ~/.zsh/completions/_sindri
    ///   fish       — ~/.config/fish/completions/sindri.fish
    ///   powershell — source the output from your $PROFILE
    Completions {
        /// One of: bash | zsh | fish | powershell
        shell: String,
    },
    /// Write a per-OS backend preference into sindri.yaml (ADR-011)
    Prefer {
        /// One of: linux | macos | windows
        os: String,
        /// Comma-separated backend list, e.g. `brew,mise,binary,script`
        order: String,
        #[arg(short, long, default_value = "sindri.yaml")]
        manifest: String,
    },
    /// StatusLedger maintenance (ADR-007)
    Ledger {
        #[command(subcommand)]
        cmd: LedgerSubcmds,
    },
}

#[derive(Subcommand)]
enum RegistrySubcmds {
    /// Fetch and cache the registry index (live OCI pull + cosign verify, ADR-003 + ADR-014).
    Refresh {
        name: String,
        url: String,
        /// Bypass cosign signature verification with a loud warning.
        ///
        /// Forbidden when the active install policy requires signed
        /// registries (strict preset). Intended for development against
        /// unsigned local registries only.
        #[arg(long)]
        insecure: bool,
    },
    /// Validate a component.yaml or directory
    Lint {
        path: String,
        #[arg(long)]
        json: bool,
        /// Enable the auth-aware lint rule (ADR-026 Phase 3): warn on
        /// components in credentialed categories (cloud, ai-dev, mcp) that
        /// lack an `auth:` block. Warnings only — never fails the build.
        /// Opt out per-component with the comment
        /// `# sindri-lint: auth-not-required` at the top of component.yaml.
        #[arg(long)]
        auth: bool,
    },
    /// Store a registry signer key (ADR-014)
    Trust {
        name: String,
        #[arg(long)]
        signer: String,
    },
    /// Verify a registry's cosign signature against trusted keys (ADR-014).
    ///
    /// Resolves the cached OCI ref + digest for the registry and runs the
    /// full cosign verification flow against the trust set under
    /// `~/.sindri/trust/<name>/`. Run `sindri registry refresh` first to
    /// populate the cache.
    Verify {
        name: String,
        /// OCI reference for the registry artifact (e.g.
        /// `ghcr.io/sindri-dev/registry-core:1.0.0`). Required because the
        /// CLI does not yet maintain a registry-name → URL map.
        #[arg(long)]
        url: String,
    },
    /// Download assets and write sha256 checksums
    FetchChecksums { path: String },
    /// Run an embedded OCI registry over a components directory (Phase 3.2,
    /// ADR-028). Developer convenience for testing — single-process,
    /// read-only, no garbage collection. Press Ctrl-C to stop.
    Serve {
        /// Directory containing an OCI image layout (or a directory of
        /// them, one per repository).
        #[arg(long)]
        root: String,
        /// `host:port` to listen on. Default `127.0.0.1:5000`.
        #[arg(long, default_value = "127.0.0.1:5000")]
        addr: String,
        /// Optional cosign key file to declare in the served manifest's
        /// trust set. Phase 3.2 honors the flag for forward compatibility
        /// but does not actively re-sign — the served bytes are emitted
        /// verbatim from the underlying layout.
        #[arg(long = "sign-with")]
        sign_with: Option<String>,
    },
}

#[derive(Subcommand)]
enum TargetSubcmds {
    /// Add a target to sindri.yaml
    Add { name: String, kind: String },
    /// List configured targets
    Ls,
    /// Show target health status
    Status { name: String },
    /// Provision the target resource
    Create { name: String },
    /// Destroy the target resource
    Destroy { name: String },
    /// Check target prerequisites
    Doctor { name: Option<String> },
    /// Open an interactive shell on the target
    Shell { name: String },
    /// Set the default target in sindri.yaml
    Use { name: String },
    /// Start a previously-created target resource
    Start { name: String },
    /// Stop a target resource without destroying it
    Stop { name: String },
    /// Inspect or manage per-target auth (ADR-020 wizard, ADR-027 §Phase 5
    /// inspect/bind). Three modes:
    ///   - bare:  `target auth <name>`             — print `provides:` list
    ///   - bind:  `target auth <name> --bind ID`   — write `provides:` entry from a rejected candidate
    ///   - wizard: `target auth <name> --value V`  — interactive credential setup (V = `oauth` for device flow)
    Auth {
        name: String,
        /// Wizard mode (Wave 6B): pre-supplied prefixed value
        /// (`env:VAR` | `file:PATH` | `cli:CMD` | `plain:VALUE` | `oauth`).
        #[arg(long)]
        value: Option<String>,
        /// Bind a previously-considered-but-rejected candidate by binding-id
        /// (or requirement-name) to this target's `provides:` list.
        #[arg(long)]
        bind: Option<String>,
        /// When the binding has multiple considered candidates, choose this
        /// one by `capability_id`.
        #[arg(long = "capability-id")]
        capability_id: Option<String>,
        /// Override the audience field of the new provides entry.
        #[arg(long)]
        audience: Option<String>,
        /// Override the priority of the new provides entry (default: 50).
        #[arg(long)]
        priority: Option<i32>,
        /// Manifest path (default: `sindri.yaml`).
        #[arg(long, default_value = "sindri.yaml")]
        manifest: String,
        /// JSON output (inspect/bind mode only).
        #[arg(long)]
        json: bool,
    },
    /// Reconcile `targets.<name>.infra` in sindri.yaml with the on-disk infra
    /// lock — Terraform-plan-style classifier with destructive-prompt gating
    /// (Wave 5E, audit D2).
    Update {
        name: String,
        /// Skip the interactive confirmation before destructive actions.
        #[arg(long)]
        auto_approve: bool,
        /// Disable colorized plan output.
        #[arg(long)]
        no_color: bool,
    },
    /// Plugin management (ADR-019)
    Plugin {
        #[command(subcommand)]
        cmd: TargetPluginSubcmds,
    },
}

#[derive(Subcommand)]
enum AuthSubcmds {
    /// Print a table of every requirement, its binding, and the
    /// considered candidates. Reads `sindri.lock` (ADR-027 §Phase 5).
    Show {
        /// Optional component address; if omitted, all components are shown.
        component: Option<String>,
        /// Target lockfile to read (default: `local`).
        #[arg(long, default_value = "local")]
        target: String,
        /// JSON output.
        #[arg(long)]
        json: bool,
        /// Manifest path (default: `sindri.yaml`).
        #[arg(long, default_value = "sindri.yaml")]
        manifest: String,
    },
    /// Re-run the resolver's binding pass and rewrite the lockfile's
    /// `auth_bindings` field. For OAuth bindings, the cached token is
    /// invalidated so the next apply re-acquires it (ADR-027 §Phase 5).
    Refresh {
        /// Optional component address; if omitted, all components are refreshed.
        component: Option<String>,
        /// Target lockfile to refresh (default: `local`).
        #[arg(long, default_value = "local")]
        target: String,
        /// JSON output.
        #[arg(long)]
        json: bool,
        /// Manifest path (default: `sindri.yaml`).
        #[arg(long, default_value = "sindri.yaml")]
        manifest: String,
    },
}

#[derive(Subcommand)]
enum TargetPluginSubcmds {
    /// List installed target plugins
    Ls,
    /// Install a target plugin from an OCI reference (EXPERIMENTAL — requires Wave 3A.2)
    Install {
        oci_ref: String,
        /// Override the kind under which to install (defaults to the trailing path component)
        #[arg(long)]
        kind: Option<String>,
    },
    /// Trust a cosign public key for a plugin kind
    Trust {
        kind: String,
        #[arg(long)]
        signer: String,
    },
    /// Uninstall a plugin
    Uninstall {
        kind: String,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum LedgerSubcmds {
    /// Print install/upgrade/remove/rollback counts.
    Stats {
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Export ledger entries (jsonl pass-through or csv with header).
    Export {
        #[arg(long, default_value = "jsonl")]
        format: String,
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Archive older entries to a gzip file, keeping only the most recent.
    Compact {
        #[arg(long, default_value_t = 1000)]
        keep_last: usize,
    },
}

#[derive(Subcommand)]
enum SecretsSubcmds {
    /// Resolve a configured secret and assert it succeeds (no value printed)
    Validate {
        id: String,
        #[arg(short, long, default_value = "sindri.yaml")]
        manifest: std::path::PathBuf,
    },
    /// List configured secrets (id + source kind only)
    List {
        #[arg(long)]
        json: bool,
        #[arg(short, long, default_value = "sindri.yaml")]
        manifest: std::path::PathBuf,
    },
    /// Test connectivity to the configured vault backend
    TestVault,
    /// Encode a file for embedding in sindri.yaml
    EncodeFile {
        path: std::path::PathBuf,
        #[arg(long, default_value = "base64")]
        algorithm: String,
        #[arg(long, short)]
        output: Option<std::path::PathBuf>,
    },
    /// S3 secrets backend (shells out to `aws s3`)
    S3 {
        #[command(subcommand)]
        cmd: SecretsS3Subcmds,
    },
}

#[derive(Subcommand)]
enum SecretsS3Subcmds {
    /// `aws s3 cp s3://<bucket>/<key> -`
    Get {
        key: String,
        #[arg(long)]
        bucket: String,
    },
    /// `aws s3 cp <file> s3://<bucket>/<key>`
    Put {
        key: String,
        file: std::path::PathBuf,
        #[arg(long)]
        bucket: String,
    },
    /// `aws s3 ls s3://<bucket>/<prefix>`
    List {
        #[arg(long)]
        bucket: String,
        #[arg(long)]
        prefix: Option<String>,
    },
}

#[derive(Subcommand)]
enum PolicySubcmds {
    /// Set the active policy preset (default | strict | offline)
    Use { preset: String },
    /// Show the effective merged policy with source annotations
    Show,
    /// Add a license to the allow list
    AllowLicense {
        spdx: String,
        #[arg(long)]
        reason: Option<String>,
    },
}

/// Generate shell completions for the requested shell, writing to stdout.
/// Phase 5 (ADR-027 §Phase 5): doesn't break existing completions because
/// it's an opt-in subcommand — users redirect output into their shell's
/// completion directory.
fn generate_completions(shell: &str) -> i32 {
    use clap::CommandFactory;
    use clap_complete::{generate, shells};
    let mut cmd = Cli::command();
    let bin_name = "sindri".to_string();
    let mut out = std::io::stdout();
    match shell.to_lowercase().as_str() {
        "bash" => generate(shells::Bash, &mut cmd, bin_name, &mut out),
        "zsh" => generate(shells::Zsh, &mut cmd, bin_name, &mut out),
        "fish" => generate(shells::Fish, &mut cmd, bin_name, &mut out),
        "powershell" | "pwsh" => generate(shells::PowerShell, &mut cmd, bin_name, &mut out),
        "elvish" => generate(shells::Elvish, &mut cmd, bin_name, &mut out),
        other => {
            eprintln!(
                "Unsupported shell '{}'. Supported: bash, zsh, fish, powershell, elvish.",
                other
            );
            return sindri_core::exit_codes::EXIT_ERROR;
        }
    }
    sindri_core::exit_codes::EXIT_SUCCESS
}

fn main() {
    // Windows MSVC defaults the main thread to a 1 MiB stack, which is too
    // small for clap's derive-generated parser given the size of `Cli` and
    // its nested subcommand enums in debug builds (STATUS_STACK_OVERFLOW =
    // 0xC00000FD). Run the real entry point on a worker thread with an
    // 8 MiB stack to match the Linux/macOS default.
    let code = std::thread::Builder::new()
        .name("sindri-main".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(run)
        .expect("spawn sindri main thread")
        .join()
        .expect("sindri main thread panicked");
    std::process::exit(code);
}

fn run() -> i32 {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Validate { path, json, .. }) => validate::run(&path, json),
        Some(Commands::Ls {
            registry,
            backend,
            installed,
            outdated,
            json,
            refresh,
        }) => commands::ls::run(commands::ls::LsArgs {
            registry,
            backend,
            installed,
            outdated,
            json,
            refresh,
        }),
        Some(Commands::Registry { cmd }) => {
            let registry_cmd = match cmd {
                RegistrySubcmds::Refresh {
                    name,
                    url,
                    insecure,
                } => RegistryCmd::Refresh {
                    name,
                    url,
                    insecure,
                },
                RegistrySubcmds::Lint { path, json, auth } => {
                    RegistryCmd::Lint { path, json, auth }
                }
                RegistrySubcmds::Trust { name, signer } => RegistryCmd::Trust { name, signer },
                RegistrySubcmds::Verify { name, url } => RegistryCmd::Verify { name, url },
                RegistrySubcmds::FetchChecksums { path } => RegistryCmd::FetchChecksums { path },
                RegistrySubcmds::Serve {
                    root,
                    addr,
                    sign_with,
                } => RegistryCmd::Serve {
                    addr,
                    root,
                    sign_with,
                },
            };
            commands::registry::run(registry_cmd)
        }
        Some(Commands::Resolve {
            manifest,
            offline,
            refresh,
            strict,
            explain,
            target,
            strict_oci,
        }) => commands::resolve::run(commands::resolve::ResolveArgs {
            manifest,
            offline,
            refresh,
            strict,
            explain,
            target,
            json: false,
            strict_oci,
        }),
        Some(Commands::Bom {
            format,
            target,
            output,
        }) => commands::bom::run(commands::bom::BomArgs {
            format,
            target,
            output,
        }),
        Some(Commands::Log { last, json }) => {
            commands::log::run_log(commands::log::LogArgs { last, json })
        }
        Some(Commands::Doctor {
            target,
            fix,
            dry_run,
            json,
            components,
            auth,
            manifest,
        }) => commands::doctor::run(commands::doctor::DoctorArgs {
            target,
            fix,
            dry_run,
            json,
            components,
            auth,
            manifest,
        }),
        Some(Commands::Secrets { cmd }) => {
            use commands::secrets::SecretsCmd;
            let mapped = match cmd {
                SecretsSubcmds::Validate { id, manifest } => SecretsCmd::Validate { id, manifest },
                SecretsSubcmds::List { json, manifest } => SecretsCmd::List { json, manifest },
                SecretsSubcmds::TestVault => SecretsCmd::TestVault,
                SecretsSubcmds::EncodeFile {
                    path,
                    algorithm,
                    output,
                } => SecretsCmd::EncodeFile {
                    path,
                    algorithm,
                    output,
                },
                SecretsSubcmds::S3 { cmd } => match cmd {
                    SecretsS3Subcmds::Get { key, bucket } => SecretsCmd::S3Get { bucket, key },
                    SecretsS3Subcmds::Put { key, file, bucket } => {
                        SecretsCmd::S3Put { bucket, key, file }
                    }
                    SecretsS3Subcmds::List { bucket, prefix } => {
                        SecretsCmd::S3List { bucket, prefix }
                    }
                },
            };
            commands::secrets::run(mapped)
        }
        Some(Commands::Backup {
            output,
            include_cache,
            compression,
        }) => {
            let algo = commands::backup::Compression2::parse(&compression).unwrap_or_else(|| {
                eprintln!(
                    "warning: unknown --compression `{}`; defaulting to gzip",
                    compression
                );
                commands::backup::Compression2::Gzip
            });
            commands::backup::run_backup(commands::backup::BackupArgs {
                output,
                include_cache,
                compression: algo,
            })
        }
        Some(Commands::Restore {
            archive,
            dry_run,
            force,
        }) => commands::backup::run_restore(commands::backup::RestoreArgs {
            archive,
            dry_run,
            force,
        }),
        Some(Commands::Target { cmd }) => {
            let tc = match cmd {
                TargetSubcmds::Add { name, kind } => TargetCmd::Add {
                    name,
                    kind,
                    opts: Vec::new(),
                },
                TargetSubcmds::Ls => TargetCmd::Ls,
                TargetSubcmds::Status { name } => TargetCmd::Status { name },
                TargetSubcmds::Create { name } => TargetCmd::Create { name },
                TargetSubcmds::Destroy { name } => TargetCmd::Destroy { name },
                TargetSubcmds::Doctor { name } => TargetCmd::Doctor { name },
                TargetSubcmds::Shell { name } => TargetCmd::Shell { name },
                TargetSubcmds::Use { name } => TargetCmd::Use { name },
                TargetSubcmds::Start { name } => TargetCmd::Start { name },
                TargetSubcmds::Stop { name } => TargetCmd::Stop { name },
                TargetSubcmds::Auth {
                    name,
                    value,
                    bind,
                    capability_id,
                    audience,
                    priority,
                    manifest,
                    json,
                } => TargetCmd::Auth(commands::target::AuthSubArgs {
                    name: name.clone(),
                    value,
                    bind,
                    manifest,
                    target: name,
                    capability_id,
                    audience,
                    priority,
                    json,
                }),
                TargetSubcmds::Update {
                    name,
                    auto_approve,
                    no_color,
                } => TargetCmd::Update {
                    name,
                    auto_approve,
                    no_color,
                },
                TargetSubcmds::Plugin { cmd } => {
                    let sub = match cmd {
                        TargetPluginSubcmds::Ls => PluginSub::Ls,
                        TargetPluginSubcmds::Install { oci_ref, kind } => {
                            PluginSub::Install { oci_ref, kind }
                        }
                        TargetPluginSubcmds::Trust { kind, signer } => {
                            PluginSub::Trust { kind, signer }
                        }
                        TargetPluginSubcmds::Uninstall { kind, yes } => {
                            PluginSub::Uninstall { kind, yes }
                        }
                    };
                    TargetCmd::Plugin { sub }
                }
            };
            commands::target::run(tc)
        }
        Some(Commands::Auth { cmd }) => match cmd {
            AuthSubcmds::Show {
                component,
                target,
                json,
                manifest,
            } => commands::auth::run_show(commands::auth::ShowArgs {
                component,
                target,
                json,
                manifest,
            }),
            AuthSubcmds::Refresh {
                component,
                target,
                json,
                manifest,
            } => commands::auth::run_refresh(commands::auth::RefreshArgs {
                component,
                target,
                json,
                manifest,
            }),
        },
        Some(Commands::Completions { shell }) => generate_completions(&shell),
        Some(Commands::Policy { cmd }) => {
            let policy_cmd = match cmd {
                PolicySubcmds::Use { preset } => PolicyCmd::Use { preset },
                PolicySubcmds::Show => PolicyCmd::Show,
                PolicySubcmds::AllowLicense { spdx, reason } => {
                    PolicyCmd::AllowLicense { spdx, reason }
                }
            };
            commands::policy::run(policy_cmd)
        }
        Some(Commands::Init {
            template,
            name,
            policy,
            non_interactive,
            force,
        }) => commands::init::run(commands::init::InitArgs {
            template,
            name,
            policy,
            non_interactive,
            force,
        }),
        Some(Commands::Add {
            address,
            manifest,
            dry_run,
            apply,
        }) => commands::add::run(commands::add::AddArgs {
            address,
            dry_run,
            apply,
            manifest,
        }),
        Some(Commands::Remove { address, manifest }) => {
            commands::remove::run(commands::remove::RemoveArgs { address, manifest })
        }
        Some(Commands::Pin {
            address,
            version,
            manifest,
        }) => commands::pin::run_pin(commands::pin::PinArgs {
            address,
            version,
            manifest,
        }),
        Some(Commands::Unpin { address, manifest }) => {
            commands::pin::run_unpin(commands::pin::UnpinArgs { address, manifest })
        }
        Some(Commands::Upgrade {
            component,
            all,
            check,
            manifest,
        }) => commands::upgrade::run(commands::upgrade::UpgradeArgs {
            component,
            all,
            check,
            manifest,
        }),
        Some(Commands::Plan { target, json }) => {
            commands::plan::run(commands::plan::PlanArgs { target, json })
        }
        Some(Commands::Diff { target, json }) => {
            commands::diff::run(commands::diff::DiffArgs { target, json })
        }
        Some(Commands::Search {
            query,
            registry,
            backend,
            json,
        }) => commands::search::run(commands::search::SearchArgs {
            query,
            registry,
            backend,
            json,
        }),
        Some(Commands::Show {
            address,
            versions,
            json,
        }) => commands::show::run(commands::show::ShowArgs {
            address,
            versions,
            json,
        }),
        Some(Commands::Graph {
            address,
            format,
            reverse,
        }) => commands::graph::run_graph(commands::graph::GraphArgs {
            address,
            format,
            reverse,
        }),
        Some(Commands::Explain {
            component,
            in_collection,
        }) => commands::graph::run_explain(commands::graph::ExplainArgs {
            component,
            in_collection,
        }),
        Some(Commands::Apply {
            yes,
            dry_run,
            target,
            no_bom,
            resume,
            clear_state,
            skip_auth,
        }) => commands::apply::run(commands::apply::ApplyArgs {
            yes,
            dry_run,
            target,
            no_bom,
            resume,
            clear_state,
            skip_auth,
        }),
        Some(Commands::Edit {
            target,
            schema,
            no_prompt,
        }) => commands::edit::run(commands::edit::EditArgs {
            target,
            schema,
            editor_override: None,
            non_interactive: no_prompt,
            path_override: None,
        }),
        Some(Commands::Rollback {
            component,
            lockfile,
            reason,
        }) => commands::rollback::run(commands::rollback::RollbackArgs {
            component,
            lockfile: Some(lockfile),
            history_root: None,
            reason,
        }),
        Some(Commands::SelfUpgrade { dry_run }) => {
            commands::self_upgrade::run(commands::self_upgrade::SelfUpgradeArgs {
                dry_run,
                binary_path_override: None,
            })
        }
        Some(Commands::Prefer {
            os,
            order,
            manifest,
        }) => commands::prefer::run(commands::prefer::PreferArgs {
            os,
            order,
            manifest,
        }),
        Some(Commands::Ledger { cmd }) => match cmd {
            LedgerSubcmds::Stats { since, json } => {
                commands::ledger::run_stats(commands::ledger::StatsArgs {
                    since,
                    json,
                    path_override: None,
                })
            }
            LedgerSubcmds::Export { format, output } => {
                commands::ledger::run_export(commands::ledger::ExportArgs {
                    format,
                    output,
                    path_override: None,
                })
            }
            LedgerSubcmds::Compact { keep_last } => {
                commands::ledger::run_compact(commands::ledger::CompactArgs {
                    keep_last,
                    path_override: None,
                    archive_dir_override: None,
                    timestamp_override: None,
                })
            }
        },
        None => {
            use clap::CommandFactory;
            Cli::command().print_help().ok();
            0
        }
    }
}
