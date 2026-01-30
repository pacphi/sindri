//! Status command

use anyhow::Result;
use camino::Utf8Path;
use sindri_core::config::SindriConfig;
use sindri_providers::create_provider;

use crate::cli::StatusArgs;
use crate::output;

pub async fn run(args: StatusArgs, config_path: Option<&Utf8Path>) -> Result<()> {
    // Load config
    let config = SindriConfig::load(config_path)?;

    // Create provider
    let provider = create_provider(config.provider())?;

    // Check prerequisites
    let prereqs = provider.check_prerequisites()?;
    if !prereqs.satisfied {
        output::error("Missing prerequisites:");
        for p in &prereqs.missing {
            output::kv(&p.name, &p.description);
        }
        output::info("");
        output::info(&format!(
            "Run 'sindri doctor --provider {}' for detailed installation instructions",
            config.provider()
        ));
        return Err(anyhow::anyhow!("Prerequisites not satisfied"));
    }

    loop {
        // Get status
        let status = provider.status(&config).await?;

        if args.json {
            println!("{}", serde_json::to_string_pretty(&status)?);
        } else {
            output::header(&format!("Status: {}", config.name()));
            output::kv("Provider", &status.provider);
            output::kv("State", &status.state.to_string());

            if let Some(id) = &status.instance_id {
                output::kv("Instance ID", id);
            }

            // Always show image field, display "none" if not configured
            let image_display = status.image.as_deref().unwrap_or("none");
            output::kv("Image", image_display);

            if !status.addresses.is_empty() {
                println!("\nAddresses:");
                for addr in &status.addresses {
                    let port_str = addr.port.map(|p| format!(":{}", p)).unwrap_or_default();
                    output::kv(
                        &format!("{:?}", addr.r#type),
                        &format!("{}{}", addr.value, port_str),
                    );
                }
            }

            if let Some(res) = &status.resources {
                println!("\nResources:");
                if let Some(cpu) = res.cpu_percent {
                    output::kv("CPU", &format!("{:.1}%", cpu));
                }
                if let (Some(used), Some(limit)) = (res.memory_bytes, res.memory_limit) {
                    output::kv(
                        "Memory",
                        &format!("{} / {}", format_bytes(used), format_bytes(limit)),
                    );
                }
            }
        }

        // Check if watching
        if let Some(interval) = args.watch {
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            if !args.json {
                // Clear screen for refresh
                print!("\x1B[2J\x1B[1;1H");
            }
        } else {
            break;
        }
    }

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    }
}
