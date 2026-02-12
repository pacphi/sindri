//! Compatibility matrix checking

use anyhow::{anyhow, Result};
use semver::{Version, VersionReq};
use sindri_core::config::HierarchicalConfigLoader;
use sindri_core::types::{CompatibilityMatrix, ExtensionState, GitHubConfig, RuntimeConfig};
use sindri_extensions::StatusLedger;
use std::collections::HashMap;
use std::path::PathBuf;

/// Compatibility check result
#[derive(Debug, Clone)]
pub struct CompatResult {
    /// Whether the upgrade is compatible
    pub compatible: bool,

    /// Incompatible extensions
    pub incompatible_extensions: Vec<IncompatibleExtension>,

    /// Warning messages
    pub warnings: Vec<String>,

    /// Breaking changes
    pub breaking_changes: Vec<String>,
}

/// Incompatible extension info
#[derive(Debug, Clone)]
pub struct IncompatibleExtension {
    /// Extension name
    pub name: String,

    /// Current version
    pub current_version: String,

    /// Required version range
    pub required_range: String,

    /// Reason for incompatibility
    pub reason: String,
}

/// Compatibility checker for CLI upgrades
pub struct CompatibilityChecker {
    /// Compatibility matrix
    matrix: Option<CompatibilityMatrix>,

    /// Path to status ledger file
    ledger_path: PathBuf,

    /// GitHub configuration
    github_config: GitHubConfig,

    /// Runtime configuration (for user agent, etc.)
    runtime_config: RuntimeConfig,
}

impl CompatibilityChecker {
    /// Create a new checker
    pub fn new() -> Self {
        let home = directories::BaseDirs::new()
            .map(|base| base.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        let ledger_path = home.join(".sindri").join("status_ledger.jsonl");

        // Load configuration
        let config_loader =
            HierarchicalConfigLoader::new().expect("Failed to create config loader");
        let runtime_config = config_loader
            .load_runtime_config()
            .expect("Failed to load runtime config");

        Self {
            matrix: None,
            ledger_path,
            github_config: runtime_config.github.clone(),
            runtime_config,
        }
    }

    /// Create a new checker with custom ledger path
    pub fn with_ledger_path(ledger_path: PathBuf) -> Self {
        // Load configuration
        let config_loader =
            HierarchicalConfigLoader::new().expect("Failed to create config loader");
        let runtime_config = config_loader
            .load_runtime_config()
            .expect("Failed to load runtime config");

        Self {
            matrix: None,
            ledger_path,
            github_config: runtime_config.github.clone(),
            runtime_config,
        }
    }

    /// Load compatibility matrix from URL
    pub async fn load_matrix(&mut self, url: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let response = client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to fetch compatibility matrix"));
        }

        let content = response.text().await?;
        self.matrix = Some(serde_yaml_ng::from_str(&content)?);

        Ok(())
    }

    /// Load compatibility matrix from string
    pub fn load_matrix_from_str(&mut self, content: &str) -> Result<()> {
        self.matrix = Some(serde_yaml_ng::from_str(content)?);
        Ok(())
    }

    /// Fetch compatibility matrix from GitHub releases
    pub async fn fetch_matrix_from_github(&mut self, version: &str) -> Result<()> {
        let url = format!(
            "https://github.com/{}/{}/releases/download/v{}/compatibility-matrix.yaml",
            self.github_config.repo_owner, self.github_config.repo_name, version
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("User-Agent", &self.runtime_config.network.user_agent)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch compatibility matrix from GitHub: HTTP {}",
                response.status()
            ));
        }

        let content = response.text().await?;
        self.load_matrix_from_str(&content)?;

        Ok(())
    }

    /// Load installed extensions from the status ledger
    pub fn load_installed_extensions(&self) -> Result<HashMap<String, String>> {
        let ledger = StatusLedger::new(self.ledger_path.clone());
        let status_map = ledger.get_all_latest_status()?;

        // Filter to only installed extensions and extract name -> version
        let extensions: HashMap<String, String> = status_map
            .into_iter()
            .filter(|(_, status)| status.current_state == ExtensionState::Installed)
            .filter_map(|(name, status)| status.version.map(|v| (name, v)))
            .collect();

        Ok(extensions)
    }

    /// Check compatibility for an upgrade
    pub fn check_compatibility(
        &self,
        target_version: &str,
        installed_extensions: &HashMap<String, String>,
    ) -> Result<CompatResult> {
        let matrix = self
            .matrix
            .as_ref()
            .ok_or_else(|| anyhow!("Compatibility matrix not loaded"))?;

        let mut incompatible = Vec::new();
        let warnings = Vec::new();
        let mut breaking_changes = Vec::new();

        // Find matching version entry (using semver matching)
        let version_entry = self.find_version_entry(matrix, target_version)?;

        // Check extension compatibility
        for (ext_name, ext_version) in installed_extensions {
            if let Some(required_range) = version_entry.compatible_extensions.get(ext_name) {
                let req = VersionReq::parse(required_range)
                    .map_err(|e| anyhow!("Invalid version range for {}: {}", ext_name, e))?;

                let current = Version::parse(ext_version).unwrap_or_else(|_| Version::new(0, 0, 0));

                if !req.matches(&current) {
                    incompatible.push(IncompatibleExtension {
                        name: ext_name.clone(),
                        current_version: ext_version.clone(),
                        required_range: required_range.clone(),
                        reason: format!(
                            "Version {} does not satisfy {}",
                            ext_version, required_range
                        ),
                    });
                }
            }
        }

        // Collect breaking changes
        breaking_changes.extend(version_entry.breaking_changes.iter().cloned());

        Ok(CompatResult {
            compatible: incompatible.is_empty(),
            incompatible_extensions: incompatible,
            warnings,
            breaking_changes,
        })
    }

    /// Find version entry in matrix (supports wildcards like "3.0.x")
    fn find_version_entry<'a>(
        &self,
        matrix: &'a CompatibilityMatrix,
        target_version: &str,
    ) -> Result<&'a sindri_core::types::CliVersionCompat> {
        let target = Version::parse(target_version)?;

        // Try exact match first
        if let Some(entry) = matrix.cli_versions.get(target_version) {
            return Ok(entry);
        }

        // Try wildcard matches (e.g., "3.0.x")
        for (pattern, entry) in &matrix.cli_versions {
            if pattern.ends_with(".x") {
                let prefix = &pattern[..pattern.len() - 2];
                let parts: Vec<&str> = prefix.split('.').collect();

                if parts.len() == 2 {
                    if let (Ok(major), Ok(minor)) =
                        (parts[0].parse::<u64>(), parts[1].parse::<u64>())
                    {
                        if target.major == major && target.minor == minor {
                            return Ok(entry);
                        }
                    }
                }
            }
        }

        Err(anyhow!(
            "No compatibility entry found for version {}",
            target_version
        ))
    }
}

impl Default for CompatibilityChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Display methods for compatibility results
impl CompatResult {
    /// Print compatibility warnings with colored output
    pub fn print_warnings(&self, force_enabled: bool) {
        use owo_colors::OwoColorize;

        if self.compatible {
            println!("{} All extensions are compatible!", "✓".green().bold());
            return;
        }

        println!(
            "\n{} Extension Compatibility Issues Detected\n",
            "⚠".yellow().bold()
        );

        if !self.incompatible_extensions.is_empty() {
            println!("{}", "Incompatible Extensions:".red().bold());
            println!();

            // Create a table header
            println!(
                "  {:<25} {:<15} {:<20}",
                "Extension".bold(),
                "Current".bold(),
                "Required".bold()
            );
            println!("  {}", "─".repeat(60).dimmed());

            for ext in &self.incompatible_extensions {
                println!(
                    "  {:<25} {:<15} {:<20}",
                    ext.name.yellow(),
                    ext.current_version.red(),
                    ext.required_range.green()
                );
                println!("    → {}", ext.reason.dimmed());
            }
            println!();
        }

        if !self.breaking_changes.is_empty() {
            println!("{}", "Breaking Changes:".yellow().bold());
            println!();
            for change in &self.breaking_changes {
                println!("  {} {}", "•".yellow(), change);
            }
            println!();
        }

        if !self.warnings.is_empty() {
            println!("{}", "Warnings:".blue().bold());
            println!();
            for warning in &self.warnings {
                println!("  {} {}", "•".blue(), warning);
            }
            println!();
        }

        if force_enabled {
            println!(
                "{} {} compatibility checks with --force flag",
                "⚠".yellow().bold(),
                "Bypassing".yellow().bold()
            );
            println!("  {} This may result in broken functionality!\n", "⚠".red());
        } else {
            println!(
                "{} Use {} to bypass these checks (not recommended)\n",
                "ℹ".blue(),
                "--force".cyan().bold()
            );
        }
    }

    /// Print a summary of compatibility check
    pub fn print_summary(&self) {
        use owo_colors::OwoColorize;

        if self.compatible {
            println!("{} Compatibility check passed", "✓".green().bold());
        } else {
            println!(
                "{} Found {} incompatible extension(s)",
                "✗".red().bold(),
                self.incompatible_extensions.len()
            );
        }
    }
}

/// Display method for incompatible extensions
impl IncompatibleExtension {
    /// Format as a table row
    pub fn format_row(&self) -> String {
        use owo_colors::OwoColorize;

        format!(
            "{:<25} {:<15} {:<20}",
            self.name.yellow(),
            self.current_version.red(),
            self.required_range.green()
        )
    }
}
