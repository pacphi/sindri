//! Extension installation executor
//!
//! This module orchestrates extension installation by interpreting YAML declarations
//! and executing the appropriate installation method (mise, apt, binary, npm, script, hybrid).

use crate::configure::ConfigureProcessor;
use crate::validation::ValidationConfig;
use anyhow::{anyhow, Context, Result};
use regex::Regex;
use sindri_core::types::{
    AptInstallConfig, Distro, Extension, HookConfig, InstallMethod, ServiceConfig,
};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::process::Command;
use tracing::{debug, info, warn};

static DETECTED_DISTRO: OnceLock<Distro> = OnceLock::new();

/// Detect the running Linux distribution.
/// Checks SINDRI_DISTRO env var first, then /etc/os-release.
fn detect_distro() -> &'static Distro {
    DETECTED_DISTRO.get_or_init(|| {
        // 1. Check env override
        if let Ok(val) = std::env::var("SINDRI_DISTRO") {
            match val.as_str() {
                "ubuntu" => return Distro::Ubuntu,
                "fedora" => return Distro::Fedora,
                "opensuse" => return Distro::Opensuse,
                _ => {} // fall through to file detection
            }
        }

        // 2. Parse /etc/os-release
        if let Ok(contents) = std::fs::read_to_string("/etc/os-release") {
            for line in contents.lines() {
                if let Some(id) = line.strip_prefix("ID=") {
                    let id = id.trim_matches('"');
                    return match id {
                        "ubuntu" => Distro::Ubuntu,
                        "fedora" => Distro::Fedora,
                        "opensuse-leap" | "opensuse-tumbleweed" | "opensuse" => Distro::Opensuse,
                        _ => Distro::Ubuntu, // fallback
                    };
                }
            }
        }

        Distro::Ubuntu // default fallback
    })
}

/// Captured output from an extension installation
#[derive(Debug, Clone, Default)]
pub struct InstallOutput {
    /// Lines captured from stdout
    pub stdout_lines: Vec<String>,
    /// Lines captured from stderr
    pub stderr_lines: Vec<String>,
    /// Install method used (e.g., "mise", "script", "apt")
    pub install_method: String,
    /// Exit status description (e.g., "success", "exit code 1", "timed out")
    pub exit_status: String,
}

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
    /// Directories that must be in PATH for child commands.
    ///
    /// `/usr/local/bin` is where mise, gh, cosign, and starship are installed.
    /// Some distros (notably openSUSE) do not include it by default when
    /// commands are run via `sudo -u … --preserve-env` or non-login shells.
    const REQUIRED_PATH_DIRS: &[&str] = &["/usr/local/bin"];

    /// Create a new executor
    ///
    /// Ensures system tool directories (e.g. `/usr/local/bin`) are present in
    /// `PATH` so that child processes can find mise, gh, and other binaries
    /// regardless of distro-specific PATH defaults.
    pub fn new(
        extension_dir: impl Into<PathBuf>,
        workspace_dir: impl Into<PathBuf>,
        home_dir: impl Into<PathBuf>,
    ) -> Self {
        // Ensure required directories are in PATH for child commands
        Self::ensure_path_includes_required_dirs();

        Self {
            extension_dir: extension_dir.into(),
            default_timeout: 300,
            workspace_dir: workspace_dir.into(),
            home_dir: home_dir.into(),
        }
    }

    /// Ensure required system directories are present in the process PATH.
    ///
    /// On Ubuntu `/usr/local/bin` is set via `/etc/environment` (PAM) and is
    /// always present. On Fedora it is added by `pathmunge` in `/etc/profile`.
    /// On openSUSE it is added by `/etc/profile` (aaa_base) for login shells,
    /// but is **absent** in non-login contexts such as `sudo -u … bash -c …`.
    ///
    /// Since the executor spawns child commands that need tools in these dirs
    /// (mise, gh, cosign, starship), we prepend any missing dirs once at
    /// construction time rather than patching every `Command` call site.
    fn ensure_path_includes_required_dirs() {
        let current_path = std::env::var("PATH").unwrap_or_default();
        let mut missing: Vec<&str> = Vec::new();

        for dir in Self::REQUIRED_PATH_DIRS {
            if !current_path.split(':').any(|p| p == *dir) {
                missing.push(dir);
            }
        }

        if !missing.is_empty() {
            let new_path = format!("{}:{}", missing.join(":"), current_path);
            std::env::set_var("PATH", &new_path);
            tracing::debug!(
                "Prepended {} to PATH for distro compatibility",
                missing.join(", ")
            );
        }
    }

    /// Set default timeout
    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.default_timeout = timeout;
        self
    }

    /// Get the standard SINDRI_LOG_DIR path for an extension
    fn sindri_log_dir(&self, ext_name: &str) -> PathBuf {
        self.home_dir.join(".sindri").join("logs").join(ext_name)
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
    ///
    /// Returns `(InstallOutput, Result<()>)` — the output is always populated
    /// (even on failure) so callers can write log files regardless of outcome.
    pub async fn install(&self, extension: &Extension, force: bool) -> (InstallOutput, Result<()>) {
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
                    if let Err(e) = self.execute_hook(name, pre_install, "pre-install").await {
                        let output = InstallOutput {
                            install_method: format!("{:?}", extension.install.method),
                            exit_status: format!("pre-install hook failed: {}", e),
                            ..Default::default()
                        };
                        return (output, Err(e));
                    }
                }
            }
        }

        // Execute installation based on method
        let (mut output, result) = match extension.install.method {
            InstallMethod::Mise => self.install_mise(extension, force).await,
            InstallMethod::Apt | InstallMethod::Dnf | InstallMethod::Zypper => {
                self.install_pkg_manager(extension).await
            }
            InstallMethod::Binary => self.install_binary(extension).await,
            InstallMethod::Npm | InstallMethod::NpmGlobal => self.install_npm(extension).await,
            InstallMethod::Script => self.install_script(extension, timeout).await,
            InstallMethod::Hybrid => self.install_hybrid(extension, timeout, force).await,
        };

        // Execute post-install hooks if installation succeeded
        if result.is_ok() {
            if let Some(capabilities) = &extension.capabilities {
                if let Some(hooks) = &capabilities.hooks {
                    if let Some(post_install) = &hooks.post_install {
                        if let Err(e) = self.execute_hook(name, post_install, "post-install").await
                        {
                            output.exit_status = format!("post-install hook failed: {}", e);
                            return (output, Err(e));
                        }
                    }
                }
            }

            // Execute configure phase
            if let Some(configure) = &extension.configure {
                if let Err(e) = self.execute_configure(name, configure).await {
                    output.exit_status = format!("configure failed: {}", e);
                    return (output, Err(e));
                }
            }

            // Generate service script if extension declares a service: block
            if let Some(service) = &extension.service {
                if service.enabled {
                    if let Err(e) = self.generate_service_script(name, service).await {
                        warn!("Failed to generate service script for {}: {}", name, e);
                    } else {
                        info!("Generated service script for {}", name);
                    }
                }
            }
        }

        (output, result)
    }

    /// Install via mise
    async fn install_mise(
        &self,
        extension: &Extension,
        force: bool,
    ) -> (InstallOutput, Result<()>) {
        let name = &extension.metadata.name;
        let mise_config = match extension.install.mise.as_ref() {
            Some(c) => c,
            None => {
                return (
                    InstallOutput {
                        install_method: "mise".to_string(),
                        exit_status: "missing config".to_string(),
                        ..Default::default()
                    },
                    Err(anyhow!("mise configuration is missing")),
                )
            }
        };

        info!(
            "Installing {} via mise (this may take 1-5 minutes)...",
            name
        );

        // Step 1: Verify mise is available
        debug!("[1/4] Verifying mise availability...");
        if let Err(e) = self.verify_command_exists("mise").await {
            return (
                InstallOutput {
                    install_method: "mise".to_string(),
                    exit_status: "mise not found".to_string(),
                    ..Default::default()
                },
                Err(e),
            );
        }

        // Step 2: Load and verify mise configuration
        debug!("[2/4] Loading mise configuration...");
        let config_file = mise_config.config_file.as_deref().unwrap_or("mise.toml");
        let ext_dir = self.resolve_extension_dir(name);
        let config_path = ext_dir.join(config_file);

        if !config_path.exists() {
            return (
                InstallOutput {
                    install_method: "mise".to_string(),
                    exit_status: "config not found".to_string(),
                    ..Default::default()
                },
                Err(anyhow!("mise config not found: {:?}", config_path)),
            );
        }

        // Ensure mise config directory exists
        let mise_conf_dir = self.home_dir.join(".config/mise/conf.d");
        if let Err(e) = tokio::fs::create_dir_all(&mise_conf_dir)
            .await
            .context("Failed to create mise conf.d directory")
        {
            return (
                InstallOutput {
                    install_method: "mise".to_string(),
                    exit_status: "setup failed".to_string(),
                    ..Default::default()
                },
                Err(e),
            );
        }

        // Copy config to conf.d BEFORE installing
        let dest_config = mise_conf_dir.join(format!("{}.toml", name));
        if let Err(e) = tokio::fs::copy(&config_path, &dest_config)
            .await
            .context("Failed to copy mise config to conf.d")
        {
            return (
                InstallOutput {
                    install_method: "mise".to_string(),
                    exit_status: "config copy failed".to_string(),
                    ..Default::default()
                },
                Err(e),
            );
        }

        // Trust the config directory (required by mise 2024+)
        let _ = Command::new("mise")
            .arg("trust")
            .arg(&mise_conf_dir)
            .output()
            .await;

        debug!("Configuration saved and trusted in {:?}", mise_conf_dir);

        // Step 3: Install tools
        debug!("[3/5] Installing tools with mise (timeout: 5min, 3 retries if needed)...");

        // Build PATH with mise shims prepended
        let mise_path = {
            let mise_shims = self.home_dir.join(".local/share/mise/shims");
            let path_var = std::env::var("PATH").unwrap_or_default();
            if mise_shims.exists() && !path_var.contains(&mise_shims.to_string_lossy().to_string())
            {
                Some(format!("{}:{}", mise_shims.display(), path_var))
            } else {
                None
            }
        };

        // Run mise install with timeout and retry logic
        let mut retry_count = 0;
        let max_retries = 3;
        let mut install_successful = false;
        let mut last_output = InstallOutput {
            install_method: "mise".to_string(),
            ..Default::default()
        };

        while retry_count < max_retries && !install_successful {
            let result = self
                .run_mise_install(
                    &config_path,
                    Duration::from_secs(300),
                    mise_path.as_deref(),
                    force,
                )
                .await;

            match result {
                Ok(output) => {
                    last_output = output;
                    install_successful = true;
                }
                Err((output, e)) => {
                    last_output = output;
                    retry_count += 1;
                    if retry_count < max_retries {
                        warn!(
                            "mise install attempt {} failed: {}, retrying...",
                            retry_count, e
                        );
                        tokio::time::sleep(Duration::from_secs(2u64.pow(retry_count))).await;
                    } else {
                        last_output.exit_status = format!("failed after {} attempts", max_retries);
                        return (
                            last_output,
                            Err(anyhow!(
                                "mise install failed after {} attempts: {}",
                                max_retries,
                                e
                            )),
                        );
                    }
                }
            }
        }

        // Step 4: Reshim to update shims
        debug!("[4/5] Running mise reshim to update shims...");
        if mise_config.reshim_after_install {
            let _ = Command::new("mise").arg("reshim").output().await;
        }

        last_output.exit_status = "success".to_string();
        info!("{} installation via mise completed successfully", name);
        (last_output, Ok(()))
    }

    /// Run mise install with configuration and timeout
    ///
    /// Uses `Command::current_dir` and `Command::env` to set the working directory
    /// and PATH for the child process, avoiding process-global mutations that are
    /// unsound in async contexts.
    async fn run_mise_install(
        &self,
        config_path: &Path,
        timeout: Duration,
        mise_path: Option<&str>,
        force: bool,
    ) -> std::result::Result<InstallOutput, (InstallOutput, anyhow::Error)> {
        let mut cmd = Command::new("mise");
        cmd.arg("install");
        if force {
            cmd.arg("--force");
            // mise install --force requires explicit TOOL@VERSION arguments.
            // Parse them from the mise config file (TOML [tools] section).
            if let Ok(content) = std::fs::read_to_string(config_path) {
                if let Ok(toml) = content.parse::<toml::Table>() {
                    if let Some(tools) = toml.get("tools").and_then(|t| t.as_table()) {
                        for (tool, version) in tools {
                            let ver = match version {
                                toml::Value::String(s) => s.clone(),
                                _ => continue,
                            };
                            cmd.arg(format!("{}@{}", tool, ver));
                        }
                    }
                }
            }
        }
        cmd.current_dir(&self.workspace_dir);
        if let Some(path) = mise_path {
            cmd.env("PATH", path);
        }
        cmd.env("MISE_CONFIG_FILE", config_path);

        // Pass GITHUB_TOKEN as MISE_GITHUB_TOKEN if available
        if let Ok(github_token) = std::env::var("GITHUB_TOKEN") {
            cmd.env("MISE_GITHUB_TOKEN", github_token);
        }

        // Pass NPM_TOKEN if available (authenticates with npm registry, avoids rate limits)
        if let Ok(npm_token) = std::env::var("NPM_TOKEN") {
            cmd.env("NPM_TOKEN", &npm_token);
            cmd.env("NPM_CONFIG_TOKEN", &npm_token);
        }

        // Ensure npm packages with native addons (e.g., sharp) install correctly.
        cmd.env("npm_config_foreground_scripts", "true");
        cmd.env("SHARP_IGNORE_GLOBAL_LIBVIPS", "1");

        // Set platform and arch so npm downloads correct prebuilt native binaries.
        // Without this, packages like sharp fail on ARM (aarch64) containers because
        // the install script can't determine the target architecture in mise's environment.
        let npm_arch = match std::env::consts::ARCH {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            "arm" => "arm",
            other => other,
        };
        let npm_platform = match std::env::consts::OS {
            "macos" => "darwin",
            other => other,
        };
        cmd.env("npm_config_arch", npm_arch);
        cmd.env("npm_config_platform", npm_platform);

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Shared buffers to capture output for error reporting
        let stdout_lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let stderr_lines = Arc::new(Mutex::new(Vec::<String>::new()));

        let mut child = match cmd.spawn().context("Failed to spawn mise install command") {
            Ok(c) => c,
            Err(e) => {
                return Err((
                    InstallOutput {
                        install_method: "mise".to_string(),
                        exit_status: "spawn failed".to_string(),
                        ..Default::default()
                    },
                    e,
                ))
            }
        };

        // Stream stdout - only log complete lines (ending with \n)
        // Discard carriage return progress indicators
        let stdout_handle = if let Some(mut stdout) = child.stdout.take() {
            let captured = Arc::clone(&stdout_lines);
            Some(tokio::spawn(async move {
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
                                    let clean_line = console::strip_ansi_codes(line).to_string();
                                    debug!("mise: {}", clean_line);
                                    if let Ok(mut lines) = captured.lock() {
                                        lines.push(clean_line);
                                    }
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
                    let clean_line = console::strip_ansi_codes(line).to_string();
                    debug!("mise: {}", clean_line);
                    if let Ok(mut lines) = captured.lock() {
                        lines.push(clean_line);
                    }
                }
            }))
        } else {
            None
        };

        // Stream stderr - same logic
        let stderr_handle = if let Some(mut stderr) = child.stderr.take() {
            let captured = Arc::clone(&stderr_lines);
            Some(tokio::spawn(async move {
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
                                    let clean_line = console::strip_ansi_codes(line).to_string();
                                    debug!("mise: {}", clean_line);
                                    if let Ok(mut lines) = captured.lock() {
                                        lines.push(clean_line);
                                    }
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
                    let clean_line = console::strip_ansi_codes(line).to_string();
                    debug!("mise: {}", clean_line);
                    if let Ok(mut lines) = captured.lock() {
                        lines.push(clean_line);
                    }
                }
            }))
        } else {
            None
        };

        // Wait with timeout
        let result = tokio::time::timeout(timeout, child.wait()).await;

        // Wait for output streaming tasks to finish so buffers are complete
        if let Some(handle) = stdout_handle {
            let _ = handle.await;
        }
        if let Some(handle) = stderr_handle {
            let _ = handle.await;
        }

        let make_output = |status_str: String| -> InstallOutput {
            InstallOutput {
                stdout_lines: stdout_lines.lock().map(|l| l.clone()).unwrap_or_default(),
                stderr_lines: stderr_lines.lock().map(|l| l.clone()).unwrap_or_default(),
                install_method: "mise".to_string(),
                exit_status: status_str,
            }
        };

        match result {
            Ok(Ok(status)) => {
                if status.success() {
                    Ok(make_output("success".to_string()))
                } else {
                    let output = make_output(format!("exit code: {}", status));
                    let output_tail = collect_output_tail_from_vecs(
                        &output.stdout_lines,
                        &output.stderr_lines,
                        20,
                    );
                    let err = if output_tail.is_empty() {
                        anyhow!("mise install failed with exit code: {}", status)
                    } else {
                        anyhow!(
                            "mise install failed with exit code: {}\n--- mise output ---\n{}\n---",
                            status,
                            output_tail
                        )
                    };
                    Err((output, err))
                }
            }
            Ok(Err(e)) => {
                let output = make_output("process error".to_string());
                Err((output, anyhow!("Failed to wait for mise install: {}", e)))
            }
            Err(_) => {
                let output = make_output("timed out".to_string());
                Err((
                    output,
                    anyhow!("mise install timed out after {:?}", timeout),
                ))
            }
        }
    }

    /// Dispatch package manager install based on runtime distro and extension config.
    ///
    /// For `method: apt` on Fedora, uses `dnf` config if present.
    /// For `method: apt` on openSUSE, uses `zypper` config if present.
    /// For `method: dnf`, always uses `dnf` config.
    /// For `method: zypper`, always uses `zypper` config.
    async fn install_pkg_manager(&self, extension: &Extension) -> (InstallOutput, Result<()>) {
        let distro = detect_distro();
        let name = &extension.metadata.name;

        match (distro, &extension.install.method) {
            // Ubuntu: always use apt
            (Distro::Ubuntu, InstallMethod::Apt) => self.install_apt(extension).await,
            // Fedora: use dnf config
            (Distro::Fedora, InstallMethod::Apt) | (_, InstallMethod::Dnf) => {
                if extension.install.dnf.is_some() {
                    self.install_dnf(extension).await
                } else {
                    (
                        InstallOutput {
                            install_method: "dnf".to_string(),
                            exit_status: "missing config".to_string(),
                            ..Default::default()
                        },
                        Err(anyhow!(
                            "Extension '{}' has no 'dnf' config for Fedora",
                            name
                        )),
                    )
                }
            }
            // openSUSE: use zypper config
            (Distro::Opensuse, InstallMethod::Apt) | (_, InstallMethod::Zypper) => {
                if extension.install.zypper.is_some() {
                    self.install_zypper(extension).await
                } else {
                    (
                        InstallOutput {
                            install_method: "zypper".to_string(),
                            exit_status: "missing config".to_string(),
                            ..Default::default()
                        },
                        Err(anyhow!(
                            "Extension '{}' has no 'zypper' config for openSUSE",
                            name
                        )),
                    )
                }
            }
            _ => (
                InstallOutput {
                    install_method: format!("{:?}", extension.install.method),
                    exit_status: "unsupported distro".to_string(),
                    ..Default::default()
                },
                Err(anyhow!(
                    "Extension '{}' method {:?} not supported on {:?}",
                    name,
                    extension.install.method,
                    distro
                )),
            ),
        }
    }

    /// Install via apt
    async fn install_apt(&self, extension: &Extension) -> (InstallOutput, Result<()>) {
        let name = &extension.metadata.name;
        let apt_config = match extension.install.apt.as_ref() {
            Some(c) => c,
            None => {
                return (
                    InstallOutput {
                        install_method: "apt".to_string(),
                        exit_status: "missing config".to_string(),
                        ..Default::default()
                    },
                    Err(anyhow!("apt configuration is missing")),
                )
            }
        };

        info!("Installing {} via apt...", name);

        let mut output = InstallOutput {
            install_method: "apt".to_string(),
            ..Default::default()
        };

        // Determine if we need sudo
        let use_sudo = self.needs_sudo().await;

        // Ensure keyrings directory exists
        if let Err(e) = self
            .ensure_directory_with_sudo("/etc/apt/keyrings", use_sudo)
            .await
        {
            output.exit_status = "setup failed".to_string();
            return (output, Err(e));
        }

        // Add repositories using modern GPG keyring method
        if let Err(e) = self.add_apt_repositories(name, apt_config, use_sudo).await {
            output.exit_status = "repo setup failed".to_string();
            return (output, Err(e));
        }

        // Update package list if configured
        if apt_config.update_first {
            if let Err(e) = self.run_apt_command(&["update", "-qq"], use_sudo).await {
                output.exit_status = "apt update failed".to_string();
                return (output, Err(e));
            }
        }

        // Install packages
        if !apt_config.packages.is_empty() {
            let mut args = vec!["install", "-y", "-qq"];
            for pkg in &apt_config.packages {
                args.push(pkg.as_str());
            }
            if let Err(e) = self.run_apt_command(&args, use_sudo).await {
                output.exit_status = "apt install failed".to_string();
                return (output, Err(e));
            }
        }

        output.exit_status = "success".to_string();
        info!("{} installation via apt completed successfully", name);
        (output, Ok(()))
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

            // Ensure correct architecture in apt sources
            let sources = Self::fix_apt_sources_arch(&sources);

            // Write sources file
            self.write_file_with_sudo(&sources_file, sources.as_bytes(), use_sudo)
                .await?;
        }

        Ok(())
    }

    /// Install via DNF (Fedora)
    async fn install_dnf(&self, extension: &Extension) -> (InstallOutput, Result<()>) {
        let name = &extension.metadata.name;
        let dnf_config = match extension.install.dnf.as_ref() {
            Some(c) => c,
            None => {
                return (
                    InstallOutput {
                        install_method: "dnf".to_string(),
                        exit_status: "missing config".to_string(),
                        ..Default::default()
                    },
                    Err(anyhow!("dnf configuration is missing")),
                )
            }
        };

        info!("Installing {} via dnf...", name);

        let mut output = InstallOutput {
            install_method: "dnf".to_string(),
            ..Default::default()
        };

        let use_sudo = self.needs_sudo().await;

        // Add repositories if configured
        for repo in &dnf_config.repositories {
            let mut args = vec!["config-manager".to_string(), "addrepo".to_string()];
            args.push(format!("--set=baseurl={}", repo.base_url));
            if let Some(ref name) = repo.name {
                args.push(format!("--id={}", name));
            }
            if let Some(ref gpg_key) = repo.gpg_key {
                args.push("--set=gpgcheck=1".to_string());
                args.push(format!("--set=gpgkey={}", gpg_key));
            }
            let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            if let Err(e) = self.run_dnf_command(&str_args, use_sudo).await {
                output.exit_status = "repo setup failed".to_string();
                return (output, Err(e));
            }
        }

        // Install groups if configured
        for group in &dnf_config.groups {
            let args = vec!["group", "install", "-y", group.as_str()];
            if let Err(e) = self.run_dnf_command(&args, use_sudo).await {
                output.exit_status = "group install failed".to_string();
                return (output, Err(e));
            }
        }

        // Update cache if configured
        if dnf_config.update_first {
            if let Err(e) = self
                .run_dnf_command(&["makecache", "--refresh"], use_sudo)
                .await
            {
                output.exit_status = "dnf makecache failed".to_string();
                return (output, Err(e));
            }
        }

        // Install packages
        if !dnf_config.packages.is_empty() {
            let mut args = vec!["install", "-y", "--setopt=install_weak_deps=False"];
            for pkg in &dnf_config.packages {
                args.push(pkg.as_str());
            }
            if let Err(e) = self.run_dnf_command(&args, use_sudo).await {
                output.exit_status = "dnf install failed".to_string();
                return (output, Err(e));
            }
        }

        output.exit_status = "success".to_string();
        info!("{} installation via dnf completed successfully", name);
        (output, Ok(()))
    }

    /// Run a dnf command with optional sudo
    async fn run_dnf_command(&self, args: &[&str], use_sudo: bool) -> Result<()> {
        let mut cmd = Command::new(if use_sudo { "sudo" } else { "dnf" });
        if use_sudo {
            cmd.arg("dnf");
        }
        cmd.args(args);

        debug!("Running: dnf {}", args.join(" "));

        let output = cmd.output().await.context("Failed to run dnf command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("dnf {} failed: {}", args[0], stderr));
        }

        Ok(())
    }

    /// Install via Zypper (openSUSE)
    async fn install_zypper(&self, extension: &Extension) -> (InstallOutput, Result<()>) {
        let name = &extension.metadata.name;
        let zypper_config = match extension.install.zypper.as_ref() {
            Some(c) => c,
            None => {
                return (
                    InstallOutput {
                        install_method: "zypper".to_string(),
                        exit_status: "missing config".to_string(),
                        ..Default::default()
                    },
                    Err(anyhow!("zypper configuration is missing")),
                )
            }
        };

        info!("Installing {} via zypper...", name);

        let mut output = InstallOutput {
            install_method: "zypper".to_string(),
            ..Default::default()
        };

        let use_sudo = self.needs_sudo().await;

        // Add repositories if configured
        for repo in &zypper_config.repositories {
            let alias = repo.name.as_deref().unwrap_or(&repo.base_url);
            let mut args = vec!["addrepo"];
            args.push(repo.base_url.as_str());
            args.push(alias);
            if let Err(e) = self.run_zypper_command(&args, use_sudo).await {
                // Ignore "already exists" errors
                let err_msg = format!("{}", e);
                if !err_msg.contains("already exists") {
                    output.exit_status = "repo setup failed".to_string();
                    return (output, Err(e));
                }
            }
        }

        // Refresh if configured
        if zypper_config.update_first {
            if let Err(e) = self
                .run_zypper_command(&["--gpg-auto-import-keys", "refresh"], use_sudo)
                .await
            {
                output.exit_status = "zypper refresh failed".to_string();
                return (output, Err(e));
            }
        }

        // Install patterns if configured
        for pattern in &zypper_config.patterns {
            let args = vec!["install", "-t", "pattern", pattern.as_str()];
            if let Err(e) = self.run_zypper_command(&args, use_sudo).await {
                output.exit_status = "pattern install failed".to_string();
                return (output, Err(e));
            }
        }

        // Install packages
        if !zypper_config.packages.is_empty() {
            let mut args = vec!["install", "--no-recommends"];
            for pkg in &zypper_config.packages {
                args.push(pkg.as_str());
            }
            if let Err(e) = self.run_zypper_command(&args, use_sudo).await {
                output.exit_status = "zypper install failed".to_string();
                return (output, Err(e));
            }
        }

        output.exit_status = "success".to_string();
        info!("{} installation via zypper completed successfully", name);
        (output, Ok(()))
    }

    /// Run a zypper command with optional sudo
    async fn run_zypper_command(&self, args: &[&str], use_sudo: bool) -> Result<()> {
        let mut cmd = Command::new(if use_sudo { "sudo" } else { "zypper" });
        if use_sudo {
            cmd.arg("zypper");
        }
        cmd.arg("--non-interactive");
        cmd.args(args);

        debug!("Running: zypper --non-interactive {}", args.join(" "));

        let output = cmd.output().await.context("Failed to run zypper command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("zypper {} failed: {}", args[0], stderr));
        }

        Ok(())
    }

    /// Install via binary download
    async fn install_binary(&self, extension: &Extension) -> (InstallOutput, Result<()>) {
        let name = &extension.metadata.name;
        let binary_config = match extension.install.binary.as_ref() {
            Some(c) => c,
            None => {
                return (
                    InstallOutput {
                        install_method: "binary".to_string(),
                        exit_status: "missing config".to_string(),
                        ..Default::default()
                    },
                    Err(anyhow!("binary configuration is missing")),
                )
            }
        };

        info!("Installing {} via binary download...", name);

        let mut output = InstallOutput {
            install_method: "binary".to_string(),
            ..Default::default()
        };

        if binary_config.downloads.is_empty() {
            output.exit_status = "no downloads".to_string();
            return (output, Err(anyhow!("No binary downloads specified")));
        }

        let bin_dir = self.workspace_dir.join("bin");
        if let Err(e) = tokio::fs::create_dir_all(&bin_dir)
            .await
            .context("Failed to create bin directory")
        {
            output.exit_status = "setup failed".to_string();
            return (output, Err(e));
        }

        for download in &binary_config.downloads {
            info!("Downloading {}...", download.name);
            output
                .stdout_lines
                .push(format!("Downloading {}", download.name));

            let url = &download.source.url;
            let data = match self.download_file(url).await {
                Ok(d) => d,
                Err(e) => {
                    output.exit_status = "download failed".to_string();
                    return (output, Err(e));
                }
            };

            let destination = download
                .destination
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| bin_dir.clone());

            if let Err(e) = tokio::fs::create_dir_all(&destination)
                .await
                .context("Failed to create destination directory")
            {
                output.exit_status = "setup failed".to_string();
                return (output, Err(e));
            }

            if download.extract {
                if let Err(e) = self.extract_tarball(&data, &destination).await {
                    output.exit_status = "extraction failed".to_string();
                    return (output, Err(e));
                }
            } else {
                let binary_path = destination.join(&download.name);
                if let Err(e) = tokio::fs::write(&binary_path, &data)
                    .await
                    .context("Failed to write binary")
                {
                    output.exit_status = "write failed".to_string();
                    return (output, Err(e));
                }

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = std::fs::Permissions::from_mode(0o755);
                    if let Err(e) = tokio::fs::set_permissions(&binary_path, perms)
                        .await
                        .context("Failed to set executable permissions")
                    {
                        output.exit_status = "chmod failed".to_string();
                        return (output, Err(e));
                    }
                }
            }

            output
                .stdout_lines
                .push(format!("Installed {}", download.name));
        }

        output.exit_status = "success".to_string();
        info!(
            "{} installation via binary download completed successfully",
            name
        );
        (output, Ok(()))
    }

    /// Install via npm
    async fn install_npm(&self, extension: &Extension) -> (InstallOutput, Result<()>) {
        let name = &extension.metadata.name;
        let npm_config = match extension.install.npm.as_ref() {
            Some(c) => c,
            None => {
                return (
                    InstallOutput {
                        install_method: "npm".to_string(),
                        exit_status: "missing config".to_string(),
                        ..Default::default()
                    },
                    Err(anyhow!("npm configuration is missing")),
                )
            }
        };

        info!("Installing {} via npm...", name);

        if let Err(e) = self.verify_command_exists("npm").await {
            return (
                InstallOutput {
                    install_method: "npm".to_string(),
                    exit_status: "npm not found".to_string(),
                    ..Default::default()
                },
                Err(e),
            );
        }

        info!("Installing npm package globally: {}", npm_config.package);

        let cmd_output = match Command::new("npm")
            .arg("install")
            .arg("-g")
            .arg(&npm_config.package)
            .output()
            .await
            .context("Failed to run npm install")
        {
            Ok(o) => o,
            Err(e) => {
                return (
                    InstallOutput {
                        install_method: "npm".to_string(),
                        exit_status: "spawn failed".to_string(),
                        ..Default::default()
                    },
                    Err(e),
                )
            }
        };

        let stdout_str = String::from_utf8_lossy(&cmd_output.stdout);
        let stderr_str = String::from_utf8_lossy(&cmd_output.stderr);
        let mut install_output = InstallOutput {
            stdout_lines: stdout_str.lines().map(|l| l.to_string()).collect(),
            stderr_lines: stderr_str.lines().map(|l| l.to_string()).collect(),
            install_method: "npm".to_string(),
            exit_status: if cmd_output.status.success() {
                "success".to_string()
            } else {
                format!("exit code: {}", cmd_output.status)
            },
        };

        if !cmd_output.status.success() {
            return (
                install_output,
                Err(anyhow!("npm install failed: {}", stderr_str)),
            );
        }

        install_output.exit_status = "success".to_string();
        info!("{} installation via npm completed successfully", name);
        (install_output, Ok(()))
    }

    /// Find common.sh by searching multiple locations in priority order.
    ///
    /// The executor's `extension_dir` may point to a specific extension directory
    /// (e.g., `/opt/sindri/extensions/docker` for bundled, or
    /// `~/.sindri/extensions/jira-mcp/1.0.0` for versioned downloads),
    /// so we walk up ancestors to find common.sh at any level.
    ///
    /// Search order:
    /// 1. Walk up from extension_dir checking each ancestor (up to 4 levels)
    /// 2. /docker/config/sindri/common.sh (Docker image fallback)
    fn find_common_sh(&self) -> Option<PathBuf> {
        // 1. Walk up from extension_dir checking each directory
        // Handles: ext_dir itself, parent (extensions root), grandparent (/opt/sindri), etc.
        let mut dir = Some(self.extension_dir.as_path());
        for _ in 0..5 {
            if let Some(d) = dir {
                let candidate = d.join("common.sh");
                if candidate.exists() {
                    debug!("Found common.sh at {:?}", candidate);
                    return Some(candidate);
                }
                dir = d.parent();
            } else {
                break;
            }
        }

        // 2. Docker image fallback
        let docker_path = PathBuf::from("/docker/config/sindri/common.sh");
        if docker_path.exists() {
            debug!("Found common.sh at {:?}", docker_path);
            return Some(docker_path);
        }

        debug!("common.sh not found in any search path");
        None
    }

    /// Create a bash Command with BASH_ENV set to common.sh if found.
    ///
    /// When bash runs a non-interactive script, it automatically sources
    /// the file specified by BASH_ENV before executing the script body.
    /// This eliminates the need for boilerplate sourcing in every extension script.
    fn create_bash_command(&self) -> Command {
        let mut cmd = Command::new("bash");
        if let Some(common_sh) = self.find_common_sh() {
            debug!("Setting BASH_ENV={:?}", common_sh);
            cmd.env("BASH_ENV", common_sh);
        }
        cmd
    }

    /// Install via script
    async fn install_script(
        &self,
        extension: &Extension,
        _timeout: u64,
    ) -> (InstallOutput, Result<()>) {
        let name = &extension.metadata.name;
        let script_config = match extension.install.script.as_ref() {
            Some(c) => c,
            None => {
                return (
                    InstallOutput {
                        install_method: "script".to_string(),
                        exit_status: "missing config".to_string(),
                        ..Default::default()
                    },
                    Err(anyhow!("script configuration is missing")),
                )
            }
        };

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
        if let Err(e) = self.validate_script_path(&script_path, &ext_dir) {
            return (
                InstallOutput {
                    install_method: "script".to_string(),
                    exit_status: "path validation failed".to_string(),
                    ..Default::default()
                },
                Err(e),
            );
        }

        if !script_path.exists() {
            return (
                InstallOutput {
                    install_method: "script".to_string(),
                    exit_status: "script not found".to_string(),
                    ..Default::default()
                },
                Err(anyhow!(
                    "Install script not found: {:?} (executor.extension_dir={:?}, resolved ext_dir={:?})",
                    script_path,
                    self.extension_dir,
                    ext_dir
                )),
            );
        }

        // Get timeout from script config or parameter
        let script_timeout = Duration::from_secs(script_config.timeout as u64);

        // Execute script with timeout
        debug!("Running install script: {:?}", script_path);

        // CRITICAL: Pass absolute path to bash so BASH_SOURCE contains full path
        // BASH_ENV is set by create_bash_command() to auto-source common.sh
        let mut cmd = self.create_bash_command();
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
        // Inject SINDRI_LOG_DIR so scripts log to ~/.sindri/logs/<name>/
        let log_dir = self.sindri_log_dir(name);
        let _ = std::fs::create_dir_all(&log_dir);
        cmd.env("SINDRI_LOG_DIR", &log_dir);
        // Inject distro info for scripts to branch on
        cmd.env("SINDRI_DISTRO", detect_distro().as_str());
        cmd.env("SINDRI_PKG_MANAGER_LIB", "/docker/lib/pkg-manager.sh");
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Shared buffers to capture output for error reporting
        let stdout_lines = Arc::new(Mutex::new(Vec::<String>::new()));
        let stderr_lines = Arc::new(Mutex::new(Vec::<String>::new()));

        let mut child = match cmd.spawn().context("Failed to spawn install script") {
            Ok(c) => c,
            Err(e) => {
                return (
                    InstallOutput {
                        install_method: "script".to_string(),
                        exit_status: "spawn failed".to_string(),
                        ..Default::default()
                    },
                    Err(e),
                )
            }
        };

        // Stream stdout - only log complete lines (ending with \n)
        // Carriage returns (\r) are used for progress indicators that overwrite
        // the current line in a terminal - we discard these to avoid log spam
        let stdout_handle = if let Some(mut stdout) = child.stdout.take() {
            let captured = Arc::clone(&stdout_lines);
            Some(tokio::spawn(async move {
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
                                    let clean_line = console::strip_ansi_codes(line).to_string();
                                    info!("script: {}", clean_line);
                                    if let Ok(mut lines) = captured.lock() {
                                        lines.push(clean_line);
                                    }
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
                    let clean_line = console::strip_ansi_codes(line).to_string();
                    info!("script: {}", clean_line);
                    if let Ok(mut lines) = captured.lock() {
                        lines.push(clean_line);
                    }
                }
            }))
        } else {
            None
        };

        // Stream stderr - same logic, but many tools use stderr for normal output
        // so we use debug level, not warn
        let stderr_handle = if let Some(mut stderr) = child.stderr.take() {
            let captured = Arc::clone(&stderr_lines);
            Some(tokio::spawn(async move {
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
                                    let clean_line = console::strip_ansi_codes(line).to_string();
                                    debug!("script: {}", clean_line);
                                    if let Ok(mut lines) = captured.lock() {
                                        lines.push(clean_line);
                                    }
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
                    let clean_line = console::strip_ansi_codes(line).to_string();
                    debug!("script: {}", clean_line);
                    if let Ok(mut lines) = captured.lock() {
                        lines.push(clean_line);
                    }
                }
            }))
        } else {
            None
        };

        // Wait with timeout
        let result = tokio::time::timeout(script_timeout, child.wait()).await;

        // Wait for output streaming tasks to finish so buffers are complete
        if let Some(handle) = stdout_handle {
            let _ = handle.await;
        }
        if let Some(handle) = stderr_handle {
            let _ = handle.await;
        }

        let make_output = |status_str: String| -> InstallOutput {
            InstallOutput {
                stdout_lines: stdout_lines.lock().map(|l| l.clone()).unwrap_or_default(),
                stderr_lines: stderr_lines.lock().map(|l| l.clone()).unwrap_or_default(),
                install_method: "script".to_string(),
                exit_status: status_str,
            }
        };

        match result {
            Ok(Ok(status)) => {
                if status.success() {
                    info!("{} installation via script completed successfully", name);
                    (make_output("success".to_string()), Ok(()))
                } else {
                    let output = make_output(format!("exit code: {}", status));
                    let output_tail = collect_output_tail_from_vecs(
                        &output.stdout_lines,
                        &output.stderr_lines,
                        20,
                    );
                    let err = if output_tail.is_empty() {
                        anyhow!(
                            "Script installation failed for {} (exit code: {})",
                            name,
                            status
                        )
                    } else {
                        anyhow!(
                            "Script installation failed for {} (exit code: {})\n--- script output ---\n{}\n---",
                            name,
                            status,
                            output_tail
                        )
                    };
                    (output, Err(err))
                }
            }
            Ok(Err(e)) => (
                make_output("process error".to_string()),
                Err(anyhow!("Failed to wait for script: {}", e)),
            ),
            Err(_) => (
                make_output("timed out".to_string()),
                Err(anyhow!(
                    "Script installation timed out after {:?} for {}",
                    script_timeout,
                    name
                )),
            ),
        }
    }

    /// Install via hybrid method (combination of methods)
    async fn install_hybrid(
        &self,
        extension: &Extension,
        timeout: u64,
        force: bool,
    ) -> (InstallOutput, Result<()>) {
        let name = &extension.metadata.name;
        info!("Installing {} via hybrid method...", name);

        let mut has_steps = false;
        let mut combined_output = InstallOutput {
            install_method: "hybrid".to_string(),
            ..Default::default()
        };

        macro_rules! run_step {
            ($step:expr) => {{
                has_steps = true;
                let (step_output, step_result) = $step;
                let method = step_output.install_method.clone();
                combined_output
                    .stdout_lines
                    .extend(step_output.stdout_lines);
                combined_output
                    .stderr_lines
                    .extend(step_output.stderr_lines);
                if let Err(e) = step_result {
                    combined_output.exit_status = format!("{} failed", method);
                    return (combined_output, Err(e));
                }
            }};
        }

        // Dispatch package install based on distro
        let distro = detect_distro();
        match distro {
            Distro::Ubuntu => {
                if extension.install.apt.is_some() {
                    run_step!(self.install_apt(extension).await);
                }
            }
            Distro::Fedora => {
                if extension.install.dnf.is_some() {
                    run_step!(self.install_dnf(extension).await);
                } else if extension.install.apt.is_some() {
                    // Fallback to apt if no dnf config (will fail gracefully on non-Ubuntu)
                    run_step!(self.install_apt(extension).await);
                }
            }
            Distro::Opensuse => {
                if extension.install.zypper.is_some() {
                    run_step!(self.install_zypper(extension).await);
                } else if extension.install.apt.is_some() {
                    run_step!(self.install_apt(extension).await);
                }
            }
        }
        if extension.install.mise.is_some() {
            run_step!(self.install_mise(extension, force).await);
        }
        if extension.install.npm.is_some() {
            run_step!(self.install_npm(extension).await);
        }
        if extension.install.binary.is_some() {
            run_step!(self.install_binary(extension).await);
        }
        if extension.install.script.is_some() {
            run_step!(self.install_script(extension, timeout).await);
        }

        if !has_steps {
            combined_output.exit_status = "no steps".to_string();
            return (
                combined_output,
                Err(anyhow!("No installation steps specified for hybrid method")),
            );
        }

        combined_output.exit_status = "success".to_string();
        info!(
            "{} installation via hybrid method completed successfully",
            name
        );
        (combined_output, Ok(()))
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
        // This ensures tools installed via various methods are discoverable.
        // Include any PATH directories declared in the extension's configure.environment
        // so that binaries installed to custom locations (e.g., ~/.openfang/bin) are
        // discoverable without hardcoding them in DEFAULT_VALIDATION_PATHS.
        let extra_paths = Self::extract_configure_paths(extension, &self.home_dir);
        let validation_config = ValidationConfig::new(&self.home_dir, &self.workspace_dir)
            .with_extra_paths(extra_paths);
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

    /// Extract PATH directories from an extension's `configure.environment` entries.
    ///
    /// Extensions declare PATH additions like `$HOME/.openfang/bin:$PATH` in their
    /// configure block. This method parses those values and returns resolved directory
    /// paths that can be injected into the validation PATH, so binaries installed to
    /// custom locations are discoverable during post-install validation.
    fn extract_configure_paths(extension: &Extension, home_dir: &Path) -> Vec<String> {
        let configure = match &extension.configure {
            Some(c) => c,
            None => return Vec::new(),
        };

        let home_str = home_dir.to_string_lossy();
        let mut paths = Vec::new();

        for env_var in &configure.environment {
            if env_var.key != "PATH" {
                continue;
            }

            // Split the value on ':' and extract directory components,
            // skipping $PATH itself and resolving $HOME
            for segment in env_var.value.split(':') {
                let trimmed = segment.trim();
                if trimmed.is_empty() || trimmed == "$PATH" {
                    continue;
                }

                let resolved = trimmed
                    .replace("$HOME", &home_str)
                    .replace("${HOME}", &home_str);

                debug!(
                    "Extracted configure PATH for {}: {}",
                    extension.metadata.name, resolved
                );
                paths.push(resolved);
            }
        }

        paths
    }

    /// Execute a lifecycle hook
    ///
    /// Hook commands run with current_dir set to the extension's own directory,
    /// so relative paths in hook commands (e.g., `bash scripts/install-plugins.sh`)
    /// resolve against the extension directory where those scripts live.
    async fn execute_hook(&self, ext_name: &str, hook: &HookConfig, phase: &str) -> Result<()> {
        if let Some(desc) = &hook.description {
            info!("Executing {} hook for {}: {}", phase, ext_name, desc);
        } else {
            info!("Executing {} hook for {}", phase, ext_name);
        }

        let ext_dir = self.resolve_extension_dir(ext_name);

        let log_dir = self.sindri_log_dir(ext_name);
        let _ = std::fs::create_dir_all(&log_dir);

        let output = self
            .create_bash_command()
            .arg("-c")
            .arg(&hook.command)
            .current_dir(&ext_dir)
            .env("SINDRI_LOG_DIR", &log_dir)
            .output()
            .await
            .context(format!("Failed to execute {} hook", phase))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let message = if !stdout.is_empty() {
                stdout.trim().to_string()
            } else {
                stderr.trim().to_string()
            };

            // Pre-install hooks are precondition checks — abort on failure
            // Post-install/project-init hooks are best-effort — warn only
            if phase.starts_with("pre-") {
                return Err(anyhow!(
                    "{} hook failed for {}: {}",
                    phase,
                    ext_name,
                    message
                ));
            } else {
                warn!("{} hook failed for {}: {}", phase, ext_name, message);
            }
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
        let processor = ConfigureProcessor::new(ext_dir, self.home_dir.clone());

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

    // Service management methods

    /// Generate an idempotent service start script for an extension
    ///
    /// Creates ~/.sindri/services/<name>.sh that checks the PID file,
    /// verifies required environment variables, and starts the daemon.
    async fn generate_service_script(&self, ext_name: &str, service: &ServiceConfig) -> Result<()> {
        let services_dir = self.home_dir.join(".sindri").join("services");
        std::fs::create_dir_all(&services_dir).context("Failed to create services directory")?;

        let script_path = services_dir.join(format!("{}.sh", ext_name));

        let pidfile = service.start.pidfile.as_deref().unwrap_or("");
        let logfile = service.start.logfile.as_deref().unwrap_or("");
        let args_str = if service.start.args.is_empty() {
            String::new()
        } else {
            format!(" {}", service.start.args.join(" "))
        };

        let readiness_block = if let Some(readiness) = &service.readiness {
            format!(
                r#"
# Readiness check
READY=false
for i in $(seq 1 {}); do
    if bash -c '{}' >/dev/null 2>&1; then
        READY=true
        break
    fi
    sleep 1
done
if [ "$READY" = "true" ]; then
    echo "{} is ready"
else
    echo "{} readiness check timed out after {}s"
fi"#,
                readiness.timeout,
                readiness.check.replace('\'', "'\\''"),
                ext_name,
                ext_name,
                readiness.timeout
            )
        } else {
            String::new()
        };

        // Build requires-env check block
        let env_check = if service.requires_env.is_empty() {
            String::new()
        } else {
            let checks: Vec<String> = service
                .requires_env
                .iter()
                .map(|var| {
                    format!(
                        r#"if [ -z "${{{var}:-}}" ]; then
    echo "{ext_name}: {var} not set — skipping service start"
    exit 0
fi"#,
                        var = var,
                        ext_name = ext_name
                    )
                })
                .collect();
            format!(
                "\n# Check required environment variables\n{}\n",
                checks.join("\n")
            )
        };

        let script = format!(
            r#"#!/usr/bin/env bash
set -euo pipefail
# Auto-generated service script for {ext_name}
# Do not edit — regenerated by sindri extension install

PIDFILE="{pidfile}"
LOGFILE="{logfile}"
{env_check}
# Check if already running
if [ -n "$PIDFILE" ] && [ -f "$PIDFILE" ]; then
    OLD_PID=$(cat "$PIDFILE")
    if kill -0 "$OLD_PID" 2>/dev/null; then
        echo "{ext_name} already running (PID $OLD_PID)"
        exit 0
    fi
    rm -f "$PIDFILE"
fi

# Ensure log directory exists
if [ -n "$LOGFILE" ]; then
    mkdir -p "$(dirname "$LOGFILE")"
fi

# Start the service
echo "Starting {ext_name}..."
if [ -n "$LOGFILE" ]; then
    nohup {command}{args} > "$LOGFILE" 2>&1 &
else
    nohup {command}{args} > /dev/null 2>&1 &
fi
SERVICE_PID=$!

if [ -n "$PIDFILE" ]; then
    mkdir -p "$(dirname "$PIDFILE")"
    echo "$SERVICE_PID" > "$PIDFILE"
fi

# Brief wait to check it didn't crash immediately
sleep 2
if kill -0 "$SERVICE_PID" 2>/dev/null; then
    echo "{ext_name} started (PID $SERVICE_PID)"
else
    echo "{ext_name} failed to start"
    [ -n "$PIDFILE" ] && rm -f "$PIDFILE"
    exit 1
fi
{readiness_block}
"#,
            ext_name = ext_name,
            pidfile = pidfile,
            logfile = logfile,
            command = service.start.command,
            args = args_str,
            env_check = env_check,
            readiness_block = readiness_block,
        );

        std::fs::write(&script_path, &script).context("Failed to write service script")?;

        // Make executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            std::fs::set_permissions(&script_path, perms)
                .context("Failed to set service script permissions")?;
        }

        info!("Service script written to {:?}", script_path);
        Ok(())
    }

    /// Stop and remove a service script for an extension
    pub async fn remove_service(&self, ext_name: &str) -> Result<()> {
        let services_dir = self.home_dir.join(".sindri").join("services");
        let script_path = services_dir.join(format!("{}.sh", ext_name));

        if !script_path.exists() {
            return Ok(());
        }

        // Try to stop the service via PID file
        let pid_file = self
            .home_dir
            .join(".sindri")
            .join(format!("{}.pid", ext_name));
        if pid_file.exists() {
            if let Ok(pid_str) = std::fs::read_to_string(&pid_file) {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    info!("Stopping service {} (PID {})", ext_name, pid);

                    // Send SIGTERM
                    let _ = Command::new("kill")
                        .arg("-TERM")
                        .arg(pid.to_string())
                        .output()
                        .await;

                    // Wait briefly for graceful shutdown
                    for _ in 0..10 {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        let check = Command::new("kill")
                            .arg("-0")
                            .arg(pid.to_string())
                            .output()
                            .await;
                        if let Ok(out) = check {
                            if !out.status.success() {
                                break;
                            }
                        }
                    }

                    // SIGKILL if still running
                    let check = Command::new("kill")
                        .arg("-0")
                        .arg(pid.to_string())
                        .output()
                        .await;
                    if let Ok(out) = check {
                        if out.status.success() {
                            let _ = Command::new("kill")
                                .arg("-KILL")
                                .arg(pid.to_string())
                                .output()
                                .await;
                        }
                    }

                    let _ = std::fs::remove_file(&pid_file);
                }
            }
        }

        // Remove the service script
        std::fs::remove_file(&script_path)
            .context(format!("Failed to remove service script for {}", ext_name))?;
        info!("Removed service script for {}", ext_name);

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
    ///
    /// Uses `tempfile::NamedTempFile` for secure temporary file creation,
    /// avoiding predictable filenames that could be exploited via symlink attacks.
    async fn write_file_with_sudo(&self, path: &str, data: &[u8], use_sudo: bool) -> Result<()> {
        use std::io::Write;

        // Create a secure temporary file with an unpredictable name.
        // NamedTempFile creates files with restricted permissions (0600) in the
        // system temp directory, preventing symlink and race-condition attacks.
        let data_owned = data.to_vec();
        let temp_path = tokio::task::spawn_blocking(move || -> Result<PathBuf> {
            let mut temp_file =
                tempfile::NamedTempFile::new().context("Failed to create secure temp file")?;
            temp_file
                .write_all(&data_owned)
                .context("Failed to write to temp file")?;
            // Persist to a concrete path so it survives the NamedTempFile drop.
            // into_temp_path() keeps the file on disk but releases the handle.
            let temp_path = temp_file.into_temp_path();
            let path = temp_path.to_path_buf();
            // Prevent auto-delete; we will move it with sudo/mv below.
            temp_path.keep().context("Failed to persist temp file")?;
            Ok(path)
        })
        .await
        .context("Temp file task failed")??;

        let temp_path_str = temp_path.to_string_lossy().to_string();

        // Move with sudo if needed (mv handles cross-filesystem moves automatically)
        let mut cmd = if use_sudo {
            let mut c = Command::new("sudo");
            c.arg("mv");
            c
        } else {
            Command::new("mv")
        };

        cmd.arg(&temp_path_str).arg(path);

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
    ///
    /// Iterates entries individually and validates each path to prevent
    /// Zip Slip attacks (absolute paths or `..` components that could write
    /// outside the destination directory).
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

            for entry_result in archive
                .entries()
                .context("Failed to read tarball entries")?
            {
                let mut entry = entry_result.context("Failed to read tarball entry")?;
                let entry_path = entry
                    .path()
                    .context("Failed to read entry path")?
                    .into_owned();

                // Reject absolute paths
                if entry_path.is_absolute() {
                    return Err(anyhow!(
                        "Tarball contains absolute path (Zip Slip): {}",
                        entry_path.display()
                    ));
                }

                // Reject paths containing parent directory components (..)
                for component in entry_path.components() {
                    if component == std::path::Component::ParentDir {
                        return Err(anyhow!(
                            "Tarball contains path traversal (Zip Slip): {}",
                            entry_path.display()
                        ));
                    }
                }

                // Safe to extract: unpack_in resolves relative to dest and
                // performs its own safety checks as an additional layer
                entry
                    .unpack_in(&dest)
                    .context(format!("Failed to extract entry: {}", entry_path.display()))?;
            }

            Ok(())
        })
        .await
        .context("Extraction task failed")??;

        Ok(())
    }

    /// Detect system architecture in Debian package naming convention
    fn detect_dpkg_arch() -> &'static str {
        match std::env::consts::ARCH {
            "x86_64" => "amd64",
            "aarch64" => "arm64",
            "arm" => "armhf",
            other => {
                warn!("Unknown architecture '{}', defaulting to amd64", other);
                "amd64"
            }
        }
    }

    /// Fix hardcoded architecture in apt sources line
    fn fix_apt_sources_arch(sources: &str) -> String {
        let system_arch = Self::detect_dpkg_arch();

        // Case 1: Replace existing hardcoded arch= value
        if let Some(start) = sources.find("arch=") {
            let after_arch = &sources[start + 5..];
            let end = after_arch.find([' ', ',', ']']).unwrap_or(after_arch.len());
            let old_arch = &after_arch[..end];
            if old_arch != system_arch {
                info!(
                    "Fixing apt source architecture: {} -> {}",
                    old_arch, system_arch
                );
            }
            return sources.replace(
                &format!("arch={}", old_arch),
                &format!("arch={}", system_arch),
            );
        }

        // Case 2: Has bracket section but no arch= — inject arch
        if sources.contains("deb [") {
            return sources.replace("deb [", &format!("deb [arch={} ", system_arch));
        }

        // Case 3: No bracket at all — add one with arch
        sources.replace("deb ", &format!("deb [arch={}] ", system_arch))
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

/// Collect the last N lines from already-extracted stdout/stderr vectors for error reporting
fn collect_output_tail_from_vecs(
    stdout_lines: &[String],
    stderr_lines: &[String],
    max_lines: usize,
) -> String {
    let mut all_lines: Vec<&str> = Vec::new();
    all_lines.extend(stdout_lines.iter().map(|s| s.as_str()));
    all_lines.extend(stderr_lines.iter().map(|s| s.as_str()));

    if all_lines.is_empty() {
        return String::new();
    }

    let start = all_lines.len().saturating_sub(max_lines);
    all_lines[start..].join("\n")
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

    #[test]
    fn test_detect_dpkg_arch() {
        let arch = ExtensionExecutor::detect_dpkg_arch();
        // Should return a valid Debian architecture name
        assert!(
            ["amd64", "arm64", "armhf"].contains(&arch),
            "detect_dpkg_arch should return a known Debian architecture, got: {}",
            arch
        );
    }

    #[test]
    fn test_fix_apt_sources_arch_replaces_hardcoded() {
        let sources = "deb [arch=amd64 signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu jammy stable";
        let result = ExtensionExecutor::fix_apt_sources_arch(sources);
        let system_arch = ExtensionExecutor::detect_dpkg_arch();
        assert!(
            result.contains(&format!("arch={}", system_arch)),
            "Should contain system arch '{}', got: {}",
            system_arch,
            result
        );
        // Should not contain the old arch if different
        if system_arch != "amd64" {
            assert!(
                !result.contains("arch=amd64"),
                "Should have replaced amd64, got: {}",
                result
            );
        }
    }

    #[test]
    fn test_fix_apt_sources_arch_injects_into_bracket() {
        let sources =
            "deb [signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu jammy stable";
        let result = ExtensionExecutor::fix_apt_sources_arch(sources);
        let system_arch = ExtensionExecutor::detect_dpkg_arch();
        assert!(
            result.contains(&format!("arch={}", system_arch)),
            "Should inject arch into bracket, got: {}",
            result
        );
        assert!(
            result.contains("signed-by="),
            "Should preserve signed-by, got: {}",
            result
        );
    }

    #[test]
    fn test_fix_apt_sources_arch_adds_bracket() {
        let sources = "deb https://download.docker.com/linux/ubuntu jammy stable";
        let result = ExtensionExecutor::fix_apt_sources_arch(sources);
        let system_arch = ExtensionExecutor::detect_dpkg_arch();
        assert!(
            result.contains(&format!("[arch={}]", system_arch)),
            "Should add bracket with arch, got: {}",
            result
        );
    }

    #[test]
    fn test_fix_apt_sources_arch_preserves_correct_arch() {
        let system_arch = ExtensionExecutor::detect_dpkg_arch();
        let sources = format!(
            "deb [arch={} signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu jammy stable",
            system_arch
        );
        let result = ExtensionExecutor::fix_apt_sources_arch(&sources);
        assert_eq!(
            result, sources,
            "Should not modify sources when arch already matches"
        );
    }

    #[tokio::test]
    async fn test_extract_tarball_safe_entries() {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use tar::Builder;

        let temp_dir = TempDir::new().unwrap();
        let dest_dir = temp_dir.path().join("output");
        fs::create_dir_all(&dest_dir).unwrap();

        // Build a tarball with a safe relative path
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        {
            let mut builder = Builder::new(&mut encoder);
            let data = b"hello world";
            let mut header = tar::Header::new_gnu();
            header.set_path("safe/file.txt").unwrap();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append(&header, &data[..]).unwrap();
            builder.finish().unwrap();
        }
        let tarball_data = encoder.finish().unwrap();

        let executor = ExtensionExecutor::new(
            temp_dir.path().to_str().unwrap(),
            "/tmp/workspace",
            "/tmp/home",
        );

        // Safe extraction should succeed
        let result = executor.extract_tarball(&tarball_data, &dest_dir).await;
        assert!(result.is_ok(), "Safe tarball extraction should succeed");
        assert!(
            dest_dir.join("safe/file.txt").exists(),
            "Extracted file should exist"
        );
    }

    /// Helper: build a gzipped tarball with a raw path (bypassing tar crate's safety checks)
    fn build_tarball_with_raw_path(path: &str, data: &[u8]) -> Vec<u8> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        {
            // Manually construct a tar entry to bypass the tar crate's set_path validation.
            // A tar header is 512 bytes; the name field is bytes 0..100.
            let mut header_bytes = [0u8; 512];
            let path_bytes = path.as_bytes();
            let copy_len = path_bytes.len().min(100);
            header_bytes[..copy_len].copy_from_slice(&path_bytes[..copy_len]);

            // typeflag (byte 156): '0' = regular file
            header_bytes[156] = b'0';

            // size field (bytes 124..136): octal ASCII representation
            let size_str = format!("{:011o}", data.len());
            header_bytes[124..135].copy_from_slice(size_str.as_bytes());

            // mode field (bytes 100..108): "0000644\0"
            header_bytes[100..107].copy_from_slice(b"0000644");

            // Compute checksum (bytes 148..156): sum of all header bytes
            // with the checksum field itself treated as spaces
            header_bytes[148..156].copy_from_slice(b"        ");
            let cksum: u32 = header_bytes.iter().map(|&b| b as u32).sum();
            let cksum_str = format!("{:06o}\0 ", cksum);
            header_bytes[148..156].copy_from_slice(&cksum_str.as_bytes()[..8]);

            encoder.write_all(&header_bytes).unwrap();

            // Write file data padded to 512-byte blocks
            encoder.write_all(data).unwrap();
            let padding = (512 - (data.len() % 512)) % 512;
            if padding > 0 {
                encoder.write_all(&vec![0u8; padding]).unwrap();
            }

            // Two zero blocks mark end of archive
            encoder.write_all(&[0u8; 1024]).unwrap();
        }
        encoder.finish().unwrap()
    }

    #[tokio::test]
    async fn test_extract_tarball_rejects_path_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let dest_dir = temp_dir.path().join("output");
        fs::create_dir_all(&dest_dir).unwrap();

        // Build a tarball with a path traversal entry (../../etc/malicious)
        let tarball_data = build_tarball_with_raw_path("../../etc/malicious", b"malicious content");

        let executor = ExtensionExecutor::new(
            temp_dir.path().to_str().unwrap(),
            "/tmp/workspace",
            "/tmp/home",
        );

        // Extraction should fail with Zip Slip error
        let result = executor.extract_tarball(&tarball_data, &dest_dir).await;
        assert!(result.is_err(), "Path traversal tarball should be rejected");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Zip Slip"),
            "Error should mention Zip Slip, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_extract_tarball_rejects_absolute_path() {
        let temp_dir = TempDir::new().unwrap();
        let dest_dir = temp_dir.path().join("output");
        fs::create_dir_all(&dest_dir).unwrap();

        // Build a tarball with an absolute path entry
        let tarball_data = build_tarball_with_raw_path("/etc/malicious", b"malicious content");

        let executor = ExtensionExecutor::new(
            temp_dir.path().to_str().unwrap(),
            "/tmp/workspace",
            "/tmp/home",
        );

        // Extraction should fail with Zip Slip error
        let result = executor.extract_tarball(&tarball_data, &dest_dir).await;
        assert!(result.is_err(), "Absolute path tarball should be rejected");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Zip Slip"),
            "Error should mention Zip Slip, got: {}",
            err_msg
        );
    }
}
