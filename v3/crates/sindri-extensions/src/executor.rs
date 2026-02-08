//! Extension installation executor
//!
//! This module orchestrates extension installation by interpreting YAML declarations
//! and executing the appropriate installation method (mise, apt, binary, npm, script, hybrid).

use crate::configure::ConfigureProcessor;
use crate::validation::ValidationConfig;
use anyhow::{anyhow, Context, Result};
use regex::Regex;
use sindri_core::types::{AptInstallConfig, Extension, HookConfig, InstallMethod};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Extension executor for running install/remove/upgrade operations
pub struct ExtensionExecutor {
    /// Extension base directory (v3/extensions in repo, ~/.sindri/extensions when deployed)
    /// OR the direct path to a specific extension directory (for versioned downloads)
    extension_dir: PathBuf,

    /// Default timeout in seconds for installation operations
    default_timeout: u64,

    /// Workspace directory for project operations
    workspace_dir: PathBuf,

    /// Home directory
    home_dir: PathBuf,
}

impl ExtensionExecutor {
    /// Create a new executor
    pub fn new(
        extension_dir: impl Into<PathBuf>,
        workspace_dir: impl Into<PathBuf>,
        home_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            extension_dir: extension_dir.into(),
            default_timeout: 300,
            workspace_dir: workspace_dir.into(),
            home_dir: home_dir.into(),
        }
    }

    /// Set default timeout
    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Resolve the actual extension directory for an extension
    ///
    /// Handles three cases:
    /// 1. Direct path: extension_dir itself contains extension.yaml (already resolved)
    /// 2. Flat structure: extension_dir/{name}/extension.yaml (bundled mode)
    /// 3. Versioned structure: extension_dir/{name}/{version}/extension.yaml (downloaded mode)
    fn resolve_extension_dir(&self, name: &str) -> PathBuf {
        // Case 1: extension_dir itself contains the extension (already resolved)
        if self.extension_dir.join("extension.yaml").exists() {
            debug!(
                "Extension {} found at direct path: {:?}",
                name, self.extension_dir
            );
            return self.extension_dir.clone();
        }

        // Case 2: Flat structure (bundled) - extension_dir/{name}/
        let flat_path = self.extension_dir.join(name);
        if flat_path.join("extension.yaml").exists() {
            debug!("Extension {} found at flat path: {:?}", name, flat_path);
            return flat_path;
        }

        // Case 3: Versioned structure (downloaded) - extension_dir/{name}/{version}/
        // Find the latest version directory
        if flat_path.exists() {
            if let Ok(entries) = std::fs::read_dir(&flat_path) {
                let versions: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .filter(|e| e.path().join("extension.yaml").exists())
                    .collect();

                // Return newest version (last in directory order)
                if let Some(version_entry) = versions.into_iter().last() {
                    let version_path = version_entry.path();
                    debug!(
                        "Extension {} found at versioned path: {:?}",
                        name, version_path
                    );
                    return version_path;
                }
            }
        }

        // Fallback: use flat path even if it doesn't exist (will error later)
        debug!(
            "Extension {} not found, using fallback path: {:?}",
            name, flat_path
        );
        flat_path
    }

    /// Install an extension
    pub async fn install(&self, extension: &Extension) -> Result<()> {
        let name = &extension.metadata.name;
        info!("Installing extension: {}", name);

        // Get timeout from extension requirements or use default
        let timeout = extension
            .requirements
            .as_ref()
            .map(|r| r.install_timeout as u64)
            .unwrap_or(self.default_timeout);

        // Execute pre-install hooks if configured
        if let Some(capabilities) = &extension.capabilities {
            if let Some(hooks) = &capabilities.hooks {
                if let Some(pre_install) = &hooks.pre_install {
                    self.execute_hook(name, pre_install, "pre-install").await?;
                }
            }
        }

        // Execute installation based on method
        let result = match extension.install.method {
            InstallMethod::Mise => self.install_mise(extension).await,
            InstallMethod::Apt => self.install_apt(extension).await,
            InstallMethod::Binary => self.install_binary(extension).await,
            InstallMethod::Npm | InstallMethod::NpmGlobal => self.install_npm(extension).await,
            InstallMethod::Script => self.install_script(extension, timeout).await,
            InstallMethod::Hybrid => self.install_hybrid(extension, timeout).await,
        };

        // Execute post-install hooks if installation succeeded
        if result.is_ok() {
            if let Some(capabilities) = &extension.capabilities {
                if let Some(hooks) = &capabilities.hooks {
                    if let Some(post_install) = &hooks.post_install {
                        self.execute_hook(name, post_install, "post-install")
                            .await?;
                    }
                }
            }

            // Execute configure phase
            if let Some(configure) = &extension.configure {
                self.execute_configure(name, configure).await?;
            }
        }

        result
    }

    /// Install via mise
    async fn install_mise(&self, extension: &Extension) -> Result<()> {
        let name = &extension.metadata.name;
        let mise_config = extension
            .install
            .mise
            .as_ref()
            .ok_or_else(|| anyhow!("mise configuration is missing"))?;

        info!(
            "Installing {} via mise (this may take 1-5 minutes)...",
            name
        );

        // Step 1: Verify mise is available
        debug!("[1/4] Verifying mise availability...");
        self.verify_command_exists("mise").await?;

        // Step 2: Load and verify mise configuration
        debug!("[2/4] Loading mise configuration...");
        let config_file = mise_config.config_file.as_deref().unwrap_or("mise.toml");
        let ext_dir = self.resolve_extension_dir(name);
        let config_path = ext_dir.join(config_file);

        if !config_path.exists() {
            return Err(anyhow!("mise config not found: {:?}", config_path));
        }

        // Ensure mise config directory exists
        let mise_conf_dir = self.home_dir.join(".config/mise/conf.d");
        tokio::fs::create_dir_all(&mise_conf_dir)
            .await
            .context("Failed to create mise conf.d directory")?;

        // Copy config to conf.d BEFORE installing
        let dest_config = mise_conf_dir.join(format!("{}.toml", name));
        tokio::fs::copy(&config_path, &dest_config)
            .await
            .context("Failed to copy mise config to conf.d")?;

        // Trust the config directory (required by mise 2024+)
        // Trust the entire conf.d directory to cover all config files
        let _ = Command::new("mise")
            .arg("trust")
            .arg(&mise_conf_dir)
            .output()
            .await;

        debug!("Configuration saved and trusted in {:?}", mise_conf_dir);

        // Step 3: Install tools
        debug!("[3/5] Installing tools with mise (timeout: 5min, 3 retries if needed)...");

        // Change to workspace directory for installation
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(&self.workspace_dir)?;

        // Ensure mise shims are in PATH
        let mise_shims = self.home_dir.join(".local/share/mise/shims");
        if mise_shims.exists() {
            let path_var = std::env::var("PATH").unwrap_or_default();
            if !path_var.contains(&mise_shims.to_string_lossy().to_string()) {
                let new_path = format!("{}:{}", mise_shims.display(), path_var);
                std::env::set_var("PATH", new_path);
            }
        }

        // Run mise install with timeout and retry logic
        let mut retry_count = 0;
        let max_retries = 3;
        let mut install_successful = false;

        while retry_count < max_retries && !install_successful {
            let result = self
                .run_mise_install(&config_path, Duration::from_secs(300))
                .await;

            match result {
                Ok(_) => {
                    install_successful = true;
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count < max_retries {
                        warn!(
                            "mise install attempt {} failed: {}, retrying...",
                            retry_count, e
                        );
                        tokio::time::sleep(Duration::from_secs(2u64.pow(retry_count))).await;
                    } else {
                        std::env::set_current_dir(original_dir)?;
                        return Err(anyhow!(
                            "mise install failed after {} attempts: {}",
                            max_retries,
                            e
                        ));
                    }
                }
            }
        }

        // Restore original directory
        std::env::set_current_dir(original_dir)?;

        // Step 4: Reshim to update shims
        debug!("[4/5] Running mise reshim to update shims...");
        if mise_config.reshim_after_install {
            let _ = Command::new("mise").arg("reshim").output().await;
        }

        info!("{} installation via mise completed successfully", name);
        Ok(())
    }

    /// Run mise install with configuration and timeout
    async fn run_mise_install(&self, config_path: &Path, timeout: Duration) -> Result<()> {
        let mut cmd = Command::new("mise");
        cmd.arg("install");
        cmd.env("MISE_CONFIG_FILE", config_path);

        // Pass GITHUB_TOKEN as MISE_GITHUB_TOKEN if available
        if let Ok(github_token) = std::env::var("GITHUB_TOKEN") {
            cmd.env("MISE_GITHUB_TOKEN", github_token);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .context("Failed to spawn mise install command")?;

        // Stream stdout - only log complete lines (ending with \n)
        // Discard carriage return progress indicators
        if let Some(mut stdout) = child.stdout.take() {
            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut buffer = vec![0u8; 1024];
                let mut line_buffer = String::new();

                while let Ok(n) = stdout.read(&mut buffer).await {
                    if n == 0 {
                        break;
                    }

                    let chunk = String::from_utf8_lossy(&buffer[..n]);
                    for ch in chunk.chars() {
                        match ch {
                            '\n' => {
                                let line = line_buffer.trim();
                                if !line.is_empty() {
                                    let clean_line = console::strip_ansi_codes(line);
                                    debug!("mise: {}", clean_line);
                                }
                                line_buffer.clear();
                            }
                            '\r' => line_buffer.clear(),
                            _ => line_buffer.push(ch),
                        }
                    }
                }

                let line = line_buffer.trim();
                if !line.is_empty() {
                    debug!("mise: {}", line);
                }
            });
        }

        // Stream stderr - same logic
        if let Some(mut stderr) = child.stderr.take() {
            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut buffer = vec![0u8; 1024];
                let mut line_buffer = String::new();

                while let Ok(n) = stderr.read(&mut buffer).await {
                    if n == 0 {
                        break;
                    }

                    let chunk = String::from_utf8_lossy(&buffer[..n]);
                    for ch in chunk.chars() {
                        match ch {
                            '\n' => {
                                let line = line_buffer.trim();
                                if !line.is_empty() {
                                    let clean_line = console::strip_ansi_codes(line);
                                    debug!("mise: {}", clean_line);
                                }
                                line_buffer.clear();
                            }
                            '\r' => line_buffer.clear(),
                            _ => line_buffer.push(ch),
                        }
                    }
                }

                let line = line_buffer.trim();
                if !line.is_empty() {
                    debug!("mise: {}", line);
                }
            });
        }

        // Wait with timeout
        let result = tokio::time::timeout(timeout, child.wait()).await;

        match result {
            Ok(Ok(status)) => {
                if status.success() {
                    Ok(())
                } else {
                    Err(anyhow!("mise install failed with exit code: {}", status))
                }
            }
            Ok(Err(e)) => Err(anyhow!("Failed to wait for mise install: {}", e)),
            Err(_) => Err(anyhow!("mise install timed out after {:?}", timeout)),
        }
    }

    /// Install via apt
    async fn install_apt(&self, extension: &Extension) -> Result<()> {
        let name = &extension.metadata.name;
        let apt_config = extension
            .install
            .apt
            .as_ref()
            .ok_or_else(|| anyhow!("apt configuration is missing"))?;

        info!("Installing {} via apt...", name);

        // Determine if we need sudo
        let use_sudo = self.needs_sudo().await;

        // Ensure keyrings directory exists
        self.ensure_directory_with_sudo("/etc/apt/keyrings", use_sudo)
            .await?;

        // Add repositories using modern GPG keyring method
        self.add_apt_repositories(name, apt_config, use_sudo)
            .await?;

        // Update package list if configured
        if apt_config.update_first {
            self.run_apt_command(&["update", "-qq"], use_sudo).await?;
        }

        // Install packages
        if !apt_config.packages.is_empty() {
            let mut args = vec!["install", "-y", "-qq"];
            for pkg in &apt_config.packages {
                args.push(pkg.as_str());
            }
            self.run_apt_command(&args, use_sudo).await?;
        }

        info!("{} installation via apt completed successfully", name);
        Ok(())
    }

    /// Add APT repositories with GPG keys
    async fn add_apt_repositories(
        &self,
        ext_name: &str,
        config: &AptInstallConfig,
        use_sudo: bool,
    ) -> Result<()> {
        if config.repositories.is_empty() {
            return Ok(());
        }

        // Sanitize extension name to prevent path traversal
        let safe_name = ext_name.replace(['/', '.'], "");
        if safe_name != ext_name {
            return Err(anyhow!(
                "Invalid extension name contains path separators: {}",
                ext_name
            ));
        }

        for repo in &config.repositories {
            let keyring_file = format!("/etc/apt/keyrings/{}.gpg", safe_name);
            let sources_file = format!("/etc/apt/sources.list.d/{}.list", safe_name);

            // Download and install GPG key
            info!("Adding GPG key for {}...", ext_name);
            let key_data = self.download_file(&repo.gpg_key).await?;

            // Try to dearmor the key
            let dearmored_key = self.dearmor_gpg_key(&key_data).await.unwrap_or(key_data);

            // Write keyring file
            self.write_file_with_sudo(&keyring_file, &dearmored_key, use_sudo)
                .await?;
            self.chmod_with_sudo(&keyring_file, "644", use_sudo).await?;

            // Add signed-by to sources if not present
            let sources = if repo.sources.contains("signed-by=") {
                repo.sources.clone()
            } else if repo.sources.contains("deb [") {
                repo.sources
                    .replace("] ", &format!(" signed-by={}] ", keyring_file))
            } else {
                repo.sources
                    .replace("deb ", &format!("deb [signed-by={}] ", keyring_file))
            };

            // Write sources file
            self.write_file_with_sudo(&sources_file, sources.as_bytes(), use_sudo)
                .await?;
        }

        Ok(())
    }

    /// Install via binary download
    async fn install_binary(&self, extension: &Extension) -> Result<()> {
        let name = &extension.metadata.name;
        let binary_config = extension
            .install
            .binary
            .as_ref()
            .ok_or_else(|| anyhow!("binary configuration is missing"))?;

        info!("Installing {} via binary download...", name);

        if binary_config.downloads.is_empty() {
            return Err(anyhow!("No binary downloads specified"));
        }

        let bin_dir = self.workspace_dir.join("bin");
        tokio::fs::create_dir_all(&bin_dir)
            .await
            .context("Failed to create bin directory")?;

        for download in &binary_config.downloads {
            info!("Downloading {}...", download.name);

            let url = &download.source.url;
            let data = self.download_file(url).await?;

            let destination = download
                .destination
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| bin_dir.clone());

            tokio::fs::create_dir_all(&destination)
                .await
                .context("Failed to create destination directory")?;

            if download.extract {
                // Extract archive
                self.extract_tarball(&data, &destination).await?;
            } else {
                // Write binary directly
                let binary_path = destination.join(&download.name);
                tokio::fs::write(&binary_path, &data)
                    .await
                    .context("Failed to write binary")?;

                // Make executable
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(0o755);
                    tokio::fs::set_permissions(&binary_path, perms)
                        .await
                        .context("Failed to set executable permissions")?;
                }
            }

            info!("Downloaded and installed {}", download.name);
        }

        info!(
            "{} installation via binary download completed successfully",
            name
        );
        Ok(())
    }

    /// Install via npm
    async fn install_npm(&self, extension: &Extension) -> Result<()> {
        let name = &extension.metadata.name;
        let npm_config = extension
            .install
            .npm
            .as_ref()
            .ok_or_else(|| anyhow!("npm configuration is missing"))?;

        info!("Installing {} via npm...", name);

        // Verify npm is available
        self.verify_command_exists("npm").await?;

        // Install package globally
        info!("Installing npm package globally: {}", npm_config.package);

        let output = Command::new("npm")
            .arg("install")
            .arg("-g")
            .arg(&npm_config.package)
            .output()
            .await
            .context("Failed to run npm install")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("npm install failed: {}", stderr));
        }

        info!("{} installation via npm completed successfully", name);
        Ok(())
    }

    /// Install via script
    async fn install_script(&self, extension: &Extension, _timeout: u64) -> Result<()> {
        let name = &extension.metadata.name;
        let script_config = extension
            .install
            .script
            .as_ref()
            .ok_or_else(|| anyhow!("script configuration is missing"))?;

        info!("Installing {} via script...", name);

        // Debug: log the executor's extension_dir before resolution
        debug!(
            "install_script for {}: executor.extension_dir={:?}",
            name, self.extension_dir
        );

        let ext_dir = self.resolve_extension_dir(name);

        // Debug: log the resolved extension directory
        debug!(
            "install_script for {}: resolved ext_dir={:?}",
            name, ext_dir
        );

        let script_path = ext_dir.join(&script_config.path);

        // Debug: log the final script path
        debug!("install_script for {}: script_path={:?}", name, script_path);

        // Validate script path for directory traversal
        self.validate_script_path(&script_path, &ext_dir)?;

        if !script_path.exists() {
            // Include more context in error message for debugging
            return Err(anyhow!(
                "Install script not found: {:?} (executor.extension_dir={:?}, resolved ext_dir={:?})",
                script_path,
                self.extension_dir,
                ext_dir
            ));
        }

        // Get timeout from script config or parameter
        let script_timeout = Duration::from_secs(script_config.timeout as u64);

        // Execute script with timeout
        debug!("Running install script: {:?}", script_path);

        // CRITICAL: Pass absolute path to bash so BASH_SOURCE contains full path
        // This allows scripts to use dirname resolution to find common.sh
        // If we pass relative path, BASH_SOURCE is just filename and dirname fails
        let mut cmd = Command::new("bash");
        let resolved_script = script_path.canonicalize().unwrap_or_else(|e| {
            warn!(
                "Failed to canonicalize {:?}: {}, using original path",
                script_path, e
            );
            script_path.clone()
        });
        debug!(
            "Executing script: bash {} (cwd: {:?})",
            resolved_script.display(),
            ext_dir
        );
        cmd.arg(&resolved_script);
        cmd.args(&script_config.args);
        cmd.current_dir(&ext_dir);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().context("Failed to spawn install script")?;

        // Stream stdout - only log complete lines (ending with \n)
        // Carriage returns (\r) are used for progress indicators that overwrite
        // the current line in a terminal - we discard these to avoid log spam
        if let Some(mut stdout) = child.stdout.take() {
            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut buffer = vec![0u8; 1024];
                let mut line_buffer = String::new();

                while let Ok(n) = stdout.read(&mut buffer).await {
                    if n == 0 {
                        break;
                    }

                    let chunk = String::from_utf8_lossy(&buffer[..n]);

                    for ch in chunk.chars() {
                        match ch {
                            '\n' => {
                                // Complete line - log it with ANSI codes stripped
                                let line = line_buffer.trim();
                                if !line.is_empty() {
                                    let clean_line = console::strip_ansi_codes(line);
                                    info!("script: {}", clean_line);
                                }
                                line_buffer.clear();
                            }
                            '\r' => {
                                // Carriage return - discard current line (progress indicator)
                                line_buffer.clear();
                            }
                            _ => {
                                line_buffer.push(ch);
                            }
                        }
                    }
                }

                // Flush any remaining output with ANSI codes stripped
                let line = line_buffer.trim();
                if !line.is_empty() {
                    let clean_line = console::strip_ansi_codes(line);
                    info!("script: {}", clean_line);
                }
            });
        }

        // Stream stderr - same logic, but many tools use stderr for normal output
        // so we use debug level, not warn
        if let Some(mut stderr) = child.stderr.take() {
            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut buffer = vec![0u8; 1024];
                let mut line_buffer = String::new();

                while let Ok(n) = stderr.read(&mut buffer).await {
                    if n == 0 {
                        break;
                    }

                    let chunk = String::from_utf8_lossy(&buffer[..n]);

                    for ch in chunk.chars() {
                        match ch {
                            '\n' => {
                                // Complete line - log it with ANSI codes stripped
                                let line = line_buffer.trim();
                                if !line.is_empty() {
                                    let clean_line = console::strip_ansi_codes(line);
                                    debug!("script: {}", clean_line);
                                }
                                line_buffer.clear();
                            }
                            '\r' => {
                                // Carriage return - discard current line (progress indicator)
                                line_buffer.clear();
                            }
                            _ => {
                                line_buffer.push(ch);
                            }
                        }
                    }
                }

                // Flush any remaining output with ANSI codes stripped
                let line = line_buffer.trim();
                if !line.is_empty() {
                    let clean_line = console::strip_ansi_codes(line);
                    debug!("script: {}", clean_line);
                }
            });
        }

        // Wait with timeout
        let result = tokio::time::timeout(script_timeout, child.wait()).await;

        match result {
            Ok(Ok(status)) => {
                if status.success() {
                    info!("{} installation via script completed successfully", name);
                    Ok(())
                } else {
                    Err(anyhow!(
                        "Script installation failed for {} (exit code: {})",
                        name,
                        status
                    ))
                }
            }
            Ok(Err(e)) => Err(anyhow!("Failed to wait for script: {}", e)),
            Err(_) => Err(anyhow!(
                "Script installation timed out after {:?} for {}",
                script_timeout,
                name
            )),
        }
    }

    /// Install via hybrid method (combination of methods)
    async fn install_hybrid(&self, extension: &Extension, timeout: u64) -> Result<()> {
        let name = &extension.metadata.name;
        info!("Installing {} via hybrid method...", name);

        let mut has_steps = false;

        // Execute in order: apt -> mise -> npm -> binary -> script
        // This order ensures:
        // - apt installs system dependencies first
        // - mise installs runtime/language tools
        // - npm/binary install additional tools
        // - script runs last for post-processing that may depend on above

        // Execute apt installation if specified
        if extension.install.apt.is_some() {
            has_steps = true;
            self.install_apt(extension).await?;
        }

        // Execute mise installation if specified
        if extension.install.mise.is_some() {
            has_steps = true;
            self.install_mise(extension).await?;
        }

        // Execute npm installation if specified
        if extension.install.npm.is_some() {
            has_steps = true;
            self.install_npm(extension).await?;
        }

        // Execute binary installation if specified
        if extension.install.binary.is_some() {
            has_steps = true;
            self.install_binary(extension).await?;
        }

        // Execute script installation if specified (runs last)
        if extension.install.script.is_some() {
            has_steps = true;
            self.install_script(extension, timeout).await?;
        }

        if !has_steps {
            return Err(anyhow!("No installation steps specified for hybrid method"));
        }

        info!(
            "{} installation via hybrid method completed successfully",
            name
        );
        Ok(())
    }

    /// Validate an installed extension
    ///
    /// Sets up comprehensive PATH including:
    /// - mise shims (for node, python, etc.)
    /// - npm global packages directory
    /// - Go binaries
    /// - Cargo binaries
    /// - User local binaries
    /// - Additional paths from SINDRI_VALIDATION_EXTRA_PATHS
    pub async fn validate_extension(&self, extension: &Extension) -> Result<bool> {
        let name = &extension.metadata.name;
        info!("Validating extension: {}", name);

        // Get validation timeout from extension or use default
        // Default is 30s to accommodate slower environments (e.g., Fly.io with network-attached volumes)
        // where tools may take longer due to I/O latency
        let validation_timeout = extension
            .requirements
            .as_ref()
            .map(|r| r.validation_timeout as u64)
            .unwrap_or_else(|| {
                std::env::var("SINDRI_VALIDATION_TIMEOUT")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30)
            });

        debug!("Validation timeout: {}s", validation_timeout);

        // Build comprehensive PATH for validation
        // This ensures tools installed via various methods are discoverable
        let validation_config = ValidationConfig::new(&self.home_dir, &self.workspace_dir);
        let validation_path = validation_config.build_validation_path();

        debug!(
            "Validation PATH includes: {:?}",
            validation_config.get_all_paths()
        );

        for cmd in &extension.validate.commands {
            let args: Vec<&str> = cmd.version_flag.split_whitespace().collect();

            debug!("Validating command: {} {}", cmd.name, cmd.version_flag);

            // Execute validation command with timeout
            let timeout_duration = Duration::from_secs(validation_timeout);
            let cmd_future = Command::new(&cmd.name)
                .args(&args)
                .env("PATH", &validation_path)
                .output();

            let output_result = tokio::time::timeout(timeout_duration, cmd_future).await;

            let output = match output_result {
                Ok(Ok(output)) => output,
                Ok(Err(e)) => {
                    return Err(anyhow!(
                        "Failed to run validation command {}: {}",
                        cmd.name,
                        e
                    ));
                }
                Err(_) => {
                    warn!(
                        "Validation timed out: {} took longer than {}s",
                        cmd.name, validation_timeout
                    );
                    return Ok(false);
                }
            };

            if !output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!(
                    "Validation failed: {} exited with status {:?}, stdout='{}', stderr='{}'",
                    cmd.name,
                    output.status.code(),
                    stdout.trim(),
                    stderr.trim()
                );
                return Ok(false);
            }

            if let Some(pattern) = &cmd.expected_pattern {
                // Check both stdout and stderr (some tools like java output to stderr)
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined_output = format!("{}{}", stdout, stderr);

                let re =
                    Regex::new(pattern).context(format!("Invalid regex pattern: {}", pattern))?;

                if !re.is_match(&combined_output) {
                    warn!(
                        "Version pattern mismatch for {}: expected {}, got stdout='{}' stderr='{}'",
                        cmd.name,
                        pattern,
                        stdout.trim(),
                        stderr.trim()
                    );
                    return Ok(false);
                }
            }
        }

        info!("Extension {} validation passed", name);
        Ok(true)
    }

    /// Execute a lifecycle hook
    async fn execute_hook(&self, ext_name: &str, hook: &HookConfig, phase: &str) -> Result<()> {
        if let Some(desc) = &hook.description {
            info!("Executing {} hook for {}: {}", phase, ext_name, desc);
        } else {
            info!("Executing {} hook for {}", phase, ext_name);
        }

        let output = Command::new("bash")
            .arg("-c")
            .arg(&hook.command)
            .current_dir(&self.workspace_dir)
            .output()
            .await
            .context(format!("Failed to execute {} hook", phase))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("{} hook failed for {}: {}", phase, ext_name, stderr);
            // Don't fail the installation if hooks fail, just warn
        }

        Ok(())
    }

    /// Execute configure phase (templates and environment variables)
    async fn execute_configure(
        &self,
        ext_name: &str,
        configure: &sindri_core::types::ConfigureConfig,
    ) -> Result<()> {
        info!("Executing configure phase for {}", ext_name);

        let ext_dir = self.resolve_extension_dir(ext_name);
        let processor =
            ConfigureProcessor::new(ext_dir, self.workspace_dir.clone(), self.home_dir.clone());

        let result = processor
            .execute(ext_name, configure)
            .await
            .context(format!("Configure phase failed for {}", ext_name))?;

        info!(
            "Configure completed for {}: {} templates, {} env vars",
            ext_name, result.templates_processed, result.environment_vars_set
        );

        if !result.backups_created.is_empty() {
            debug!(
                "Created {} backup(s) during configure",
                result.backups_created.len()
            );
        }

        Ok(())
    }

    // Helper methods

    /// Verify a command exists
    async fn verify_command_exists(&self, command: &str) -> Result<()> {
        let output = Command::new("which")
            .arg(command)
            .output()
            .await
            .context(format!("Failed to check if {} exists", command))?;

        if !output.status.success() {
            return Err(anyhow!("{} is not available", command));
        }

        Ok(())
    }

    /// Check if sudo is needed (returns true if not running as root)
    async fn needs_sudo(&self) -> bool {
        let output = Command::new("whoami").output().await;

        match output {
            Ok(output) => {
                let user = String::from_utf8_lossy(&output.stdout);
                user.trim() != "root"
            }
            Err(_) => true, // Assume sudo needed if can't determine
        }
    }

    /// Ensure directory exists with sudo if needed
    async fn ensure_directory_with_sudo(&self, path: &str, use_sudo: bool) -> Result<()> {
        let mut cmd = if use_sudo {
            let mut c = Command::new("sudo");
            c.arg("mkdir");
            c
        } else {
            Command::new("mkdir")
        };

        cmd.arg("-p").arg(path);

        let output = cmd
            .output()
            .await
            .context(format!("Failed to create directory: {}", path))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to create directory {}: {}", path, stderr));
        }

        Ok(())
    }

    /// Write file with sudo if needed
    async fn write_file_with_sudo(&self, path: &str, data: &[u8], use_sudo: bool) -> Result<()> {
        // Create temporary file
        let temp_path = format!("{}.tmp", path);

        // Write to temp file
        tokio::fs::write(&temp_path, data)
            .await
            .context("Failed to write temporary file")?;

        // Move with sudo if needed
        let mut cmd = if use_sudo {
            let mut c = Command::new("sudo");
            c.arg("mv");
            c
        } else {
            Command::new("mv")
        };

        cmd.arg(&temp_path).arg(path);

        let output = cmd
            .output()
            .await
            .context(format!("Failed to move file to: {}", path))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Clean up temp file
            let _ = tokio::fs::remove_file(&temp_path).await;
            return Err(anyhow!("Failed to write file {}: {}", path, stderr));
        }

        Ok(())
    }

    /// Change file permissions with sudo if needed
    async fn chmod_with_sudo(&self, path: &str, mode: &str, use_sudo: bool) -> Result<()> {
        let mut cmd = if use_sudo {
            let mut c = Command::new("sudo");
            c.arg("chmod");
            c
        } else {
            Command::new("chmod")
        };

        cmd.arg(mode).arg(path);

        let output = cmd
            .output()
            .await
            .context(format!("Failed to chmod: {}", path))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to chmod {}: {}", path, stderr));
        }

        Ok(())
    }

    /// Run apt command with sudo if needed
    async fn run_apt_command(&self, args: &[&str], use_sudo: bool) -> Result<()> {
        let mut cmd = if use_sudo {
            let mut c = Command::new("sudo");
            c.arg("/usr/bin/env");
            c.arg("DEBIAN_FRONTEND=noninteractive");
            c.arg("/usr/bin/apt-get");
            c
        } else {
            let mut c = Command::new("/usr/bin/env");
            c.arg("DEBIAN_FRONTEND=noninteractive");
            c.arg("/usr/bin/apt-get");
            c
        };

        cmd.args(args);

        let output = cmd
            .output()
            .await
            .context(format!("Failed to run apt-get {:?}", args))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("apt-get {:?} failed: {}", args, stderr));
        }

        Ok(())
    }

    /// Download file from URL
    async fn download_file(&self, url: &str) -> Result<Vec<u8>> {
        let response = reqwest::get(url)
            .await
            .context(format!("Failed to download from: {}", url))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Download failed with status: {}",
                response.status()
            ));
        }

        let data = response
            .bytes()
            .await
            .context("Failed to read response bytes")?;

        Ok(data.to_vec())
    }

    /// Dearmor GPG key (convert ASCII-armored to binary)
    async fn dearmor_gpg_key(&self, key_data: &[u8]) -> Result<Vec<u8>> {
        let mut cmd = Command::new("gpg");
        cmd.arg("--dearmor");
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());

        let mut child = cmd.spawn().context("Failed to spawn gpg")?;

        // Write key data to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin
                .write_all(key_data)
                .await
                .context("Failed to write to gpg stdin")?;
        }

        let output = child
            .wait_with_output()
            .await
            .context("Failed to wait for gpg")?;

        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(anyhow!("gpg --dearmor failed"))
        }
    }

    /// Extract tarball to destination
    async fn extract_tarball(&self, data: &[u8], destination: &Path) -> Result<()> {
        use flate2::read::GzDecoder;
        use std::io::Cursor;
        use tar::Archive;

        // Clone data to move into blocking task
        let data_vec = data.to_vec();
        let dest = destination.to_path_buf();

        tokio::task::spawn_blocking(move || {
            let cursor = Cursor::new(data_vec);
            let decoder = GzDecoder::new(cursor);
            let mut archive = Archive::new(decoder);

            archive.unpack(&dest).context("Failed to extract tarball")
        })
        .await
        .context("Extraction task failed")??;

        Ok(())
    }

    /// Validate script path to prevent directory traversal
    fn validate_script_path(&self, script_path: &Path, ext_dir: &Path) -> Result<()> {
        // Check for .. as a path component (not just substring)
        for component in script_path.components() {
            if component == std::path::Component::ParentDir {
                return Err(anyhow!(
                    "Invalid script path: directory traversal (..) detected"
                ));
            }
        }

        // Canonicalize both paths and ensure script is within extension directory
        // This is the primary security check
        match (script_path.canonicalize(), ext_dir.canonicalize()) {
            (Ok(canonical_script), Ok(canonical_ext)) => {
                // Both paths exist - verify script is within extension directory
                if !canonical_script.starts_with(&canonical_ext) {
                    return Err(anyhow!(
                        "Script path escapes extension directory (security violation)"
                    ));
                }
            }
            (Err(_), _) | (_, Err(_)) => {
                // If canonicalize fails, paths may not exist yet
                // Fall back to string-based validation
                // Check if script path is within ext_dir (works for both relative and absolute)
                if script_path.is_absolute() && ext_dir.is_absolute() {
                    // Both absolute - check prefix
                    if !script_path.starts_with(ext_dir) {
                        return Err(anyhow!("Script path must be within extension directory"));
                    }
                } else if script_path.is_relative() && ext_dir.is_relative() {
                    // Both relative - check prefix
                    if !script_path.starts_with(ext_dir) {
                        return Err(anyhow!("Script path must be within extension directory"));
                    }
                } else {
                    // Mixed absolute/relative - can't reliably validate without canonicalization
                    return Err(anyhow!(
                        "Cannot validate mixed absolute/relative paths without existing files"
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_validate_script_path() {
        // Create temporary directories for testing
        let temp_dir = TempDir::new().unwrap();
        let ext_dir = temp_dir.path().join("test-extension");
        let scripts_dir = ext_dir.join("scripts");

        fs::create_dir_all(&scripts_dir).unwrap();

        // Create a test script file
        let script_file = scripts_dir.join("install.sh");
        fs::write(&script_file, "#!/bin/bash\necho test").unwrap();

        let executor = ExtensionExecutor::new(
            temp_dir.path().to_str().unwrap(),
            "/tmp/workspace",
            "/tmp/home",
        );

        // Test 1: Valid path - script inside extension directory
        assert!(
            executor
                .validate_script_path(&script_file, &ext_dir)
                .is_ok(),
            "Valid script path should pass validation"
        );

        // Test 2: Invalid - contains .. in path string
        let invalid_path = PathBuf::from(&ext_dir).join("../../../etc/passwd");
        assert!(
            executor
                .validate_script_path(&invalid_path, &ext_dir)
                .is_err(),
            "Path with .. should fail validation"
        );

        // Test 3: Invalid - absolute path to system file
        let absolute_path = PathBuf::from("/etc/passwd");
        assert!(
            executor
                .validate_script_path(&absolute_path, &ext_dir)
                .is_err(),
            "Absolute path should fail validation"
        );

        // Test 4: Invalid - path outside extension directory (create actual directory)
        let outside_dir = temp_dir.path().join("outside");
        fs::create_dir_all(&outside_dir).unwrap();
        let outside_file = outside_dir.join("malicious.sh");
        fs::write(&outside_file, "#!/bin/bash\nrm -rf /").unwrap();

        assert!(
            executor
                .validate_script_path(&outside_file, &ext_dir)
                .is_err(),
            "Path outside extension directory should fail validation"
        );
    }

    #[test]
    fn test_validate_script_path_string_checks() {
        let temp_dir = TempDir::new().unwrap();
        let ext_dir = temp_dir.path().join("test-extension");
        fs::create_dir(&ext_dir).unwrap();

        let executor = ExtensionExecutor::new(
            temp_dir.path().to_str().unwrap(),
            "/tmp/workspace",
            "/tmp/home",
        );

        // These should fail even without file existence due to string validation

        // Contains ".."
        let path_with_dotdot = ext_dir.join("scripts/../../../etc/passwd");
        assert!(
            executor
                .validate_script_path(&path_with_dotdot, &ext_dir)
                .is_err(),
            "Path containing .. should fail"
        );

        // Absolute path
        let absolute = PathBuf::from("/etc/passwd");
        assert!(
            executor.validate_script_path(&absolute, &ext_dir).is_err(),
            "Absolute path should fail"
        );
    }
}
