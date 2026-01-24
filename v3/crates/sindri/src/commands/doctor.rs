//! Doctor command - check system for required tools and dependencies
//!
//! This command provides a comprehensive diagnostic of the user's system,
//! checking for all tools that Sindri depends on based on the user's
//! intended usage (provider, commands, etc.).

use anyhow::Result;
use owo_colors::OwoColorize;
use sindri_doctor::{Doctor, DoctorOptions, ExtensionChecker, OutputFormat, ToolInstaller};

use crate::cli::DoctorArgs;
use crate::output;

/// Run the doctor diagnostic command
pub async fn run(args: DoctorArgs) -> Result<()> {
    // Create doctor instance
    let doctor = Doctor::new();

    // Build options from CLI args
    let options = DoctorOptions {
        provider: args.provider.clone(),
        command: args.command.clone(),
        all: args.all,
        check_auth: args.check_auth,
        verbose: args.verbose_output,
        ci: args.ci,
    };

    // Determine output format
    let format = match args.format.to_lowercase().as_str() {
        "json" => OutputFormat::Json,
        "yaml" | "yml" => OutputFormat::Yaml,
        _ => OutputFormat::Human,
    };

    // Run the diagnostic
    let result = doctor.run(&options).await?;

    // Format and display output
    if args.verbose_output {
        println!("{}", result.format_verbose(format));
    } else {
        println!("{}", result.format(format));
    }

    // Check extension tools if requested
    if args.check_extensions || args.extension.is_some() {
        println!("\n{}", "Extension Tool Checks".bold());
        println!("{}", "─".repeat(40));

        let ext_checker = ExtensionChecker::new();

        let ext_result = if let Some(ext_name) = &args.extension {
            ext_checker.check_extension(ext_name).await?
        } else {
            ext_checker.check_all().await?
        };

        if ext_result.tool_statuses.is_empty() {
            println!("No extension tools to check.");
        } else {
            println!(
                "Checked {} tool(s) from {} extension(s)\n",
                ext_result.tool_statuses.len(),
                ext_result.extensions_checked.len()
            );

            for status in &ext_result.tool_statuses {
                let icon = if status.available {
                    "✓".green().to_string()
                } else {
                    "✗".red().to_string()
                };

                let version_info = status
                    .version
                    .as_ref()
                    .map(|v| format!(" ({})", v.trim()))
                    .unwrap_or_default();

                println!(
                    "  {} {} [{}]{}",
                    icon,
                    status.tool.tool,
                    status.tool.extension.dimmed(),
                    version_info.dimmed()
                );
            }

            println!();
            if ext_result.all_available() {
                println!("{}", "All extension tools available.".green());
            } else {
                println!(
                    "{}",
                    format!("{} extension tool(s) missing.", ext_result.missing_count).yellow()
                );
            }
        }
    }

    // Handle --fix flag
    if args.fix {
        // Check if there are missing tools to fix
        let missing_tools: Vec<_> = result
            .tools
            .iter()
            .filter(|t| matches!(t.state, sindri_doctor::ToolState::Missing))
            .collect();

        if missing_tools.is_empty() {
            output::success("No missing tools to install.");
        } else {
            output::info(&format!(
                "\nAttempting to install {} missing tool(s)...\n",
                missing_tools.len()
            ));

            // Create installer
            let installer = ToolInstaller::new(args.dry_run, !args.yes);

            // Install missing tools
            let install_results = installer.install_all(&result.tools).await;

            // Show summary
            let summary = ToolInstaller::summarize_results(&install_results);
            summary.display();

            // Re-run diagnostic if installations occurred
            if summary.succeeded > 0 && !args.dry_run {
                println!("\nRe-checking after installations...\n");
                let recheck_result = doctor.run(&options).await?;
                println!("{}", recheck_result.format(format));
            }
        }
    }

    // In CI mode, exit with appropriate code
    if args.ci {
        let exit_code = result.exit_code();
        if exit_code != 0 {
            std::process::exit(exit_code);
        }
    }

    Ok(())
}
