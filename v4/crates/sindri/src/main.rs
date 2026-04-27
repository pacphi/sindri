#![allow(dead_code)]

use clap::{Parser, Subcommand};

mod commands;
mod validate;

use commands::policy::PolicyCmd;
use commands::registry::RegistryCmd;
use commands::target::TargetCmd;

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
        #[arg(long)]
        fix: bool,
        #[arg(long)]
        components: bool,
    },
    /// Target management (ADR-017, ADR-023)
    Target {
        #[command(subcommand)]
        cmd: TargetSubcmds,
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
    },
}

#[derive(Subcommand)]
enum RegistrySubcmds {
    /// Fetch and cache the registry index
    Refresh { name: String, url: String },
    /// Validate a component.yaml or directory
    Lint {
        path: String,
        #[arg(long)]
        json: bool,
    },
    /// Store a registry signer key (ADR-014)
    Trust {
        name: String,
        #[arg(long)]
        signer: String,
    },
    /// Verify a registry's cosign signature against trusted keys (ADR-014).
    ///
    /// Wave 3A.1 placeholder: trust-key loading is in place, but live
    /// signature verification (cosign signature manifest fetch +
    /// simple-signing payload verify) is deferred to Wave 3A.2. This
    /// subcommand currently exits non-zero with an explanatory message so
    /// it cannot silently pass in CI.
    Verify { name: String },
    /// Download assets and write sha256 checksums
    FetchChecksums { path: String },
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

fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
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
                RegistrySubcmds::Refresh { name, url } => RegistryCmd::Refresh { name, url },
                RegistrySubcmds::Lint { path, json } => RegistryCmd::Lint { path, json },
                RegistrySubcmds::Trust { name, signer } => RegistryCmd::Trust { name, signer },
                RegistrySubcmds::Verify { name } => RegistryCmd::Verify { name },
                RegistrySubcmds::FetchChecksums { path } => RegistryCmd::FetchChecksums { path },
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
        }) => commands::resolve::run(commands::resolve::ResolveArgs {
            manifest,
            offline,
            refresh,
            strict,
            explain,
            target,
            json: false,
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
            components,
        }) => commands::doctor::run(commands::doctor::DoctorArgs {
            target,
            fix,
            components,
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
            };
            commands::target::run(tc)
        }
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
        }) => commands::apply::run(commands::apply::ApplyArgs {
            yes,
            dry_run,
            target,
            no_bom,
        }),
        None => {
            use clap::CommandFactory;
            Cli::command().print_help().ok();
            0
        }
    };
    std::process::exit(code);
}
