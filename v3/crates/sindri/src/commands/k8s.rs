//! Kubernetes cluster management commands
//!
//! This module provides CLI commands for managing local Kubernetes
//! clusters using kind or k3d.

use crate::cli::{
    K8sCommands, K8sConfigArgs, K8sCreateArgs, K8sDestroyArgs, K8sInstallArgs, K8sListArgs,
    K8sStatusArgs,
};
use anyhow::{anyhow, Result};
use dialoguer::Confirm;
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use sindri_clusters::{
    create_cluster_provider, get_available_providers, has_cluster_provider, installer,
    ClusterConfig, ClusterInfo, ClusterProvider, ClusterProviderType, ClusterState, ClusterStatus,
    K3dConfig, K3dProvider, K3dRegistryConfig, KindProvider,
};
use std::time::Duration;
use tabled::{settings::Style, Table, Tabled};
use tracing::info;

/// Run a k8s subcommand
pub async fn run(cmd: K8sCommands) -> Result<()> {
    match cmd {
        K8sCommands::Create(args) => create(args).await,
        K8sCommands::Destroy(args) => destroy(args).await,
        K8sCommands::List(args) => list(args).await,
        K8sCommands::Status(args) => status(args).await,
        K8sCommands::Config(args) => config(args).await,
        K8sCommands::Install(args) => install(args).await,
    }
}

/// Create a local Kubernetes cluster
async fn create(args: K8sCreateArgs) -> Result<()> {
    let provider_type: ClusterProviderType = args.provider.parse()?;
    let provider = create_cluster_provider(provider_type)?;

    // Check if provider is installed
    if !provider.check_installed() {
        eprintln!(
            "{} {} is not installed",
            "Error:".red().bold(),
            provider.name()
        );
        eprintln!();
        eprintln!(
            "Install with: {} k8s install {}",
            "sindri".cyan(),
            provider.name()
        );
        return Err(anyhow!("{} is not installed", provider.name()));
    }

    // Check Docker
    if !provider.check_docker() {
        return Err(anyhow!(
            "Docker is not running. Please start Docker and try again."
        ));
    }

    // Build cluster config
    let mut config = ClusterConfig::new(&args.name)
        .with_version(&args.version)
        .with_nodes(args.nodes);

    // Add k3d-specific config if needed
    if provider_type == ClusterProviderType::K3d && args.registry {
        config = config.with_k3d_config(K3dConfig {
            image: None,
            registry: K3dRegistryConfig {
                enabled: true,
                name: "k3d-registry".to_string(),
                port: args.registry_port,
            },
        });
    }

    // Show progress
    let pb = create_spinner(&format!(
        "Creating {} cluster '{}'...",
        provider.name(),
        args.name
    ));

    // Create cluster
    let cluster_info = provider.create(&config).await?;

    pb.finish_and_clear();

    if args.json {
        println!("{}", serde_json::to_string_pretty(&cluster_info)?);
    } else {
        print_cluster_created(&cluster_info);
    }

    Ok(())
}

/// Destroy a local Kubernetes cluster
async fn destroy(args: K8sDestroyArgs) -> Result<()> {
    // Try to auto-detect which provider has this cluster
    let provider = detect_cluster_provider(&args.name).await?;

    // Check if cluster exists
    if !provider.exists(&args.name).await {
        eprintln!(
            "{} Cluster '{}' does not exist",
            "Warning:".yellow().bold(),
            args.name
        );
        return Ok(());
    }

    // Confirmation
    if !args.force {
        eprintln!(
            "{} This will destroy cluster: {}",
            "Warning:".yellow().bold(),
            args.name.cyan()
        );

        let confirm = Confirm::new()
            .with_prompt("Are you sure?")
            .default(false)
            .interact()?;

        if !confirm {
            eprintln!("Cancelled");
            return Ok(());
        }
    }

    let pb = create_spinner(&format!("Destroying cluster '{}'...", args.name));

    provider.destroy(&args.name, true).await?;

    pb.finish_and_clear();

    eprintln!(
        "{} Cluster '{}' destroyed",
        "Success:".green().bold(),
        args.name
    );

    Ok(())
}

/// List local Kubernetes clusters
async fn list(args: K8sListArgs) -> Result<()> {
    let mut all_clusters: Vec<ClusterInfo> = Vec::new();

    // Determine which providers to query
    let providers: Vec<Box<dyn ClusterProvider>> = match args.provider.as_deref() {
        Some("kind") => {
            if KindProvider::new().check_installed() {
                vec![Box::new(KindProvider::new())]
            } else {
                vec![]
            }
        }
        Some("k3d") => {
            if K3dProvider::new().check_installed() {
                vec![Box::new(K3dProvider::new())]
            } else {
                vec![]
            }
        }
        _ => get_available_providers(),
    };

    if providers.is_empty() {
        if args.json {
            println!("[]");
        } else {
            eprintln!("No cluster providers installed.");
            eprintln!(
                "Install with: {} k8s install kind  or  {} k8s install k3d",
                "sindri".cyan(),
                "sindri".cyan()
            );
        }
        return Ok(());
    }

    for provider in providers {
        match provider.list().await {
            Ok(clusters) => all_clusters.extend(clusters),
            Err(e) => {
                info!("Failed to list {} clusters: {}", provider.name(), e);
            }
        }
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&all_clusters)?);
    } else if all_clusters.is_empty() {
        eprintln!("No clusters found.");
        eprintln!();
        eprintln!("Create a cluster with:");
        eprintln!(
            "  {} k8s create --provider kind --name my-cluster",
            "sindri".cyan()
        );
        eprintln!(
            "  {} k8s create --provider k3d --name my-cluster --registry",
            "sindri".cyan()
        );
    } else {
        print_cluster_table(&all_clusters);
    }

    Ok(())
}

/// Show cluster status
async fn status(args: K8sStatusArgs) -> Result<()> {
    let provider = match args.provider {
        Some(ref p) => create_cluster_provider(p.parse()?)?,
        None => detect_cluster_provider(&args.name).await?,
    };

    let status = provider.status(&args.name).await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        print_cluster_status(&status);
    }

    Ok(())
}

/// Show kubeconfig for a cluster
async fn config(args: K8sConfigArgs) -> Result<()> {
    let provider = match args.provider {
        Some(ref p) => create_cluster_provider(p.parse()?)?,
        None => detect_cluster_provider(&args.name).await?,
    };

    let kubeconfig = provider.get_kubeconfig(&args.name).await?;
    print!("{}", kubeconfig);

    Ok(())
}

/// Install cluster management tools
async fn install(args: K8sInstallArgs) -> Result<()> {
    let tool = args.tool.to_lowercase();

    // Check if already installed
    let already_installed = match tool.as_str() {
        "kind" => KindProvider::new().check_installed(),
        "k3d" => K3dProvider::new().check_installed(),
        _ => return Err(anyhow!("Unknown tool: {}. Use 'kind' or 'k3d'", tool)),
    };

    if already_installed {
        let version = match tool.as_str() {
            "kind" => KindProvider::new().get_version(),
            "k3d" => K3dProvider::new().get_version(),
            _ => None,
        };
        eprintln!(
            "{} {} is already installed{}",
            "Info:".blue().bold(),
            tool,
            version.map(|v| format!(" ({})", v)).unwrap_or_default()
        );
        return Ok(());
    }

    // Confirmation
    if !args.yes {
        let confirm = Confirm::new()
            .with_prompt(format!("Install {}?", tool))
            .default(true)
            .interact()?;

        if !confirm {
            eprintln!("Cancelled");
            return Ok(());
        }
    }

    let pb = create_spinner(&format!("Installing {}...", tool));

    let result = match tool.as_str() {
        "kind" => installer::install_kind().await,
        "k3d" => installer::install_k3d().await,
        _ => unreachable!(),
    };

    pb.finish_and_clear();

    match result {
        Ok(()) => {
            eprintln!(
                "{} {} installed successfully",
                "Success:".green().bold(),
                tool
            );
        }
        Err(e) => {
            eprintln!(
                "{} Failed to install {}: {}",
                "Error:".red().bold(),
                tool,
                e
            );
            return Err(e);
        }
    }

    Ok(())
}

// Helper functions

/// Auto-detect which provider has a cluster with the given name
async fn detect_cluster_provider(name: &str) -> Result<Box<dyn ClusterProvider>> {
    // Try kind first
    let kind = KindProvider::new();
    if kind.check_installed() && kind.exists(name).await {
        return Ok(Box::new(kind));
    }

    // Try k3d
    let k3d = K3dProvider::new();
    if k3d.check_installed() && k3d.exists(name).await {
        return Ok(Box::new(k3d));
    }

    // Check if any provider is available
    if !has_cluster_provider() {
        return Err(anyhow!(
            "No cluster provider installed. Install kind or k3d first:\n\
             sindri k8s install kind\n\
             sindri k8s install k3d"
        ));
    }

    // Default to kind if no cluster found
    if kind.check_installed() {
        return Ok(Box::new(kind));
    }

    Ok(Box::new(k3d))
}

/// Create a spinner progress indicator
fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Print cluster creation success message
fn print_cluster_created(info: &ClusterInfo) {
    eprintln!();
    eprintln!(
        "{} Cluster '{}' created successfully",
        "Success:".green().bold(),
        info.name.cyan()
    );
    eprintln!();
    eprintln!("  Provider:  {}", info.provider);
    eprintln!("  Context:   {}", info.context.cyan());
    if let Some(version) = &info.version {
        eprintln!("  Version:   {}", version);
    }
    eprintln!("  Nodes:     {}", info.node_count);
    if let Some(registry) = &info.registry_url {
        eprintln!("  Registry:  {}", registry.cyan());
    }
    eprintln!();
    eprintln!("To use this cluster:");
    eprintln!("  kubectl --context {} get nodes", info.context.cyan());
    eprintln!();
    eprintln!("Or deploy with Sindri:");
    eprintln!("  {} deploy --provider kubernetes", "sindri".cyan());
}

/// Table row for cluster list
#[derive(Tabled)]
struct ClusterRow {
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "PROVIDER")]
    provider: String,
    #[tabled(rename = "CONTEXT")]
    context: String,
    #[tabled(rename = "NODES")]
    nodes: String,
}

/// Print cluster list as a table
fn print_cluster_table(clusters: &[ClusterInfo]) {
    let rows: Vec<ClusterRow> = clusters
        .iter()
        .map(|c| ClusterRow {
            name: c.name.clone(),
            provider: c.provider.clone(),
            context: c.context.clone(),
            nodes: c.node_count.to_string(),
        })
        .collect();

    let mut table = Table::new(rows);
    table.with(Style::sharp());
    println!("{}", table);
}

/// Print cluster status
fn print_cluster_status(status: &ClusterStatus) {
    eprintln!();
    eprintln!("Cluster: {}", status.name.cyan().bold());
    eprintln!();
    eprintln!("  Provider:  {}", status.provider);
    eprintln!("  Context:   {}", status.context);
    eprintln!("  Status:    {}", format_state(&status.state));
    eprintln!(
        "  Ready:     {}",
        if status.ready {
            "Yes".green().to_string()
        } else {
            "No".red().to_string()
        }
    );

    if !status.nodes.is_empty() {
        eprintln!();
        eprintln!("Nodes:");
        for node in &status.nodes {
            let status_color = if node.status == "Ready" {
                node.status.green().to_string()
            } else {
                node.status.red().to_string()
            };
            eprintln!("  - {} ({}) - {}", node.name, node.role, status_color);
        }
    }

    if !status.messages.is_empty() {
        eprintln!();
        for msg in &status.messages {
            eprintln!("  {}", msg.yellow());
        }
    }
    eprintln!();
}

/// Format cluster state with colors
fn format_state(state: &ClusterState) -> String {
    match state {
        ClusterState::Running => "Running".green().to_string(),
        ClusterState::Stopped => "Stopped".yellow().to_string(),
        ClusterState::Creating => "Creating".blue().to_string(),
        ClusterState::Deleting => "Deleting".yellow().to_string(),
        ClusterState::NotFound => "Not Found".red().to_string(),
        ClusterState::Error => "Error".red().bold().to_string(),
        ClusterState::Unknown => "Unknown".dimmed().to_string(),
    }
}
