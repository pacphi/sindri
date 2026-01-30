//! Template context for rendering provider configurations

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sindri_core::config::SindriConfig;
use std::collections::HashMap;
use tera::Context;

/// Template context containing all data needed for rendering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateContext {
    // Basic info
    pub name: String,
    pub profile: String,
    pub image: String,

    // Resources
    pub memory: String,
    pub cpus: u32,
    pub volume_size: String,

    // GPU
    pub gpu_enabled: bool,
    pub gpu_type: String,
    pub gpu_count: u32,

    // Extensions
    pub custom_extensions: String,
    pub additional_extensions: String,
    pub skip_auto_install: bool,

    // DinD configuration
    pub dind: DindConfig,

    // Provider-specific
    pub runtime: Option<String>,
    pub privileged: bool,
    pub network_mode: String,
    pub extra_hosts: Vec<String>,
    pub ports: Vec<String>,

    // Secrets
    pub has_secrets: bool,
    pub secrets_file: String,

    // Environment variables
    pub env_vars: HashMap<String, String>,

    // CI mode
    pub ci_mode: bool,
}

/// Docker-in-Docker configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DindConfig {
    pub enabled: bool,
    pub mode: String, // sysbox, privileged, socket, none
    pub storage_size: String,
    pub storage_driver: String,
}

impl TemplateContext {
    /// Create a builder for TemplateContext
    pub fn builder() -> TemplateContextBuilder {
        TemplateContextBuilder::default()
    }

    /// Create a context from a SindriConfig
    pub fn from_config(config: &SindriConfig, detected_dind_mode: &str) -> Self {
        let file = config.inner();

        // Get profile
        let profile = file
            .extensions
            .profile
            .clone()
            .unwrap_or_else(|| "base".to_string());

        // Get active extensions
        let custom_extensions = file
            .extensions
            .active
            .as_ref()
            .map(|exts: &Vec<String>| exts.join(","))
            .unwrap_or_default();

        // Get additional extensions
        let additional_extensions = file
            .extensions
            .additional
            .as_ref()
            .map(|exts: &Vec<String>| exts.join(","))
            .unwrap_or_default();

        // Get memory
        let memory = file
            .deployment
            .resources
            .memory
            .clone()
            .unwrap_or_else(|| "4GB".to_string());

        // Get CPUs
        let cpus = file.deployment.resources.cpus.unwrap_or(2);

        // Get volume size
        let volume_size = file
            .deployment
            .volumes
            .workspace
            .as_ref()
            .map(|v| v.size.clone())
            .unwrap_or_else(|| "10GB".to_string());

        // Get GPU config
        let (gpu_enabled, gpu_type, gpu_count) = file
            .deployment
            .resources
            .gpu
            .as_ref()
            .map(|g| (g.enabled, format!("{:?}", g.r#type).to_lowercase(), g.count))
            .unwrap_or((false, "nvidia".to_string(), 0));

        // Get Docker-specific config
        let docker_config = file.providers.docker.as_ref();

        let runtime = match detected_dind_mode {
            "sysbox" => Some("sysbox-runc".to_string()),
            _ => None,
        };

        let privileged = docker_config.map(|c| c.privileged).unwrap_or(false)
            || detected_dind_mode == "privileged";

        let network_mode = docker_config
            .map(|c| format!("{:?}", c.network).to_lowercase())
            .unwrap_or_else(|| "bridge".to_string());

        let extra_hosts = docker_config
            .map(|c| c.extra_hosts.clone())
            .unwrap_or_default();

        let ports = docker_config.map(|c| c.ports.clone()).unwrap_or_default();

        // Get DinD config
        let dind = docker_config
            .and_then(|c| c.dind.as_ref())
            .map(|d| DindConfig {
                enabled: d.enabled,
                mode: detected_dind_mode.to_string(),
                storage_size: d.storage_size.clone(),
                storage_driver: format!("{:?}", d.storage_driver).to_lowercase(),
            })
            .unwrap_or_else(|| DindConfig {
                enabled: false,
                mode: "none".to_string(),
                storage_size: "20GB".to_string(),
                storage_driver: "auto".to_string(),
            });

        // Build environment variables map
        let mut env_vars = HashMap::new();

        // Set SINDRI_EXT_HOME based on build mode
        let ext_home = if file
            .deployment
            .build_from_source
            .as_ref()
            .map(|b| b.enabled)
            .unwrap_or(false)
        {
            // Development mode: bundled extensions at /opt/sindri/extensions
            // (built using Dockerfile.dev)
            "/opt/sindri/extensions".to_string()
        } else {
            // Production mode: runtime-installed extensions at ${HOME}/.sindri/extensions
            // (built using Dockerfile, respects ALT_HOME=/alt/home/developer volume mount)
            "${HOME}/.sindri/extensions".to_string()
        };

        env_vars.insert("SINDRI_EXT_HOME".to_string(), ext_home);

        // Keep SINDRI_SOURCE_REF for debugging purposes if building from source
        if let Some(git_ref) = file
            .deployment
            .build_from_source
            .as_ref()
            .and_then(|b| b.git_ref.as_ref())
        {
            env_vars.insert("SINDRI_SOURCE_REF".to_string(), git_ref.clone());
        }

        Self {
            name: file.name.clone(),
            profile,
            image: file.deployment.image.clone().unwrap_or_else(|| {
                // Default tag format: sindri:{version}-SOURCE
                // Actual tag determined at build time with git SHA
                format!("sindri:{}-SOURCE", env!("CARGO_PKG_VERSION"))
            }),
            memory,
            cpus,
            volume_size,
            gpu_enabled,
            gpu_type,
            gpu_count,
            custom_extensions,
            additional_extensions,
            skip_auto_install: !file.extensions.auto_install,
            dind,
            runtime,
            privileged,
            network_mode,
            extra_hosts,
            ports,
            has_secrets: !file.secrets.is_empty(),
            secrets_file: ".env.secrets".to_string(),
            env_vars,
            ci_mode: false,
        }
    }

    /// Convert to Tera context
    pub fn to_tera_context(&self) -> Result<Context> {
        let context = Context::from_serialize(self)?;
        Ok(context)
    }

    /// Set CI mode
    pub fn with_ci_mode(mut self, ci_mode: bool) -> Self {
        self.ci_mode = ci_mode;
        self
    }

    /// Add environment variables
    pub fn with_env_vars(mut self, vars: HashMap<String, String>) -> Self {
        self.env_vars.extend(vars);
        self
    }

    /// Set secrets file path
    pub fn with_secrets_file(mut self, path: &str) -> Self {
        self.secrets_file = path.to_string();
        self.has_secrets = true;
        self
    }
}

/// Builder for TemplateContext
#[derive(Debug, Default)]
pub struct TemplateContextBuilder {
    name: String,
    profile: String,
    image: String,
    memory: String,
    cpus: u32,
    volume_size: String,
    gpu_enabled: bool,
    gpu_type: String,
    gpu_count: u32,
    custom_extensions: String,
    additional_extensions: String,
    skip_auto_install: bool,
    dind: DindConfig,
    runtime: Option<String>,
    privileged: bool,
    network_mode: String,
    extra_hosts: Vec<String>,
    ports: Vec<String>,
    has_secrets: bool,
    secrets_file: String,
    env_vars: HashMap<String, String>,
    ci_mode: bool,
}

impl TemplateContextBuilder {
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn profile(mut self, profile: &str) -> Self {
        self.profile = profile.to_string();
        self
    }

    pub fn image(mut self, image: &str) -> Self {
        self.image = image.to_string();
        self
    }

    pub fn memory(mut self, memory: &str) -> Self {
        self.memory = memory.to_string();
        self
    }

    pub fn cpus(mut self, cpus: u32) -> Self {
        self.cpus = cpus;
        self
    }

    pub fn volume_size(mut self, size: &str) -> Self {
        self.volume_size = size.to_string();
        self
    }

    pub fn gpu(mut self, enabled: bool, gpu_type: &str, count: u32) -> Self {
        self.gpu_enabled = enabled;
        self.gpu_type = gpu_type.to_string();
        self.gpu_count = count;
        self
    }

    pub fn extensions(mut self, custom: &str, additional: &str) -> Self {
        self.custom_extensions = custom.to_string();
        self.additional_extensions = additional.to_string();
        self
    }

    pub fn skip_auto_install(mut self, skip: bool) -> Self {
        self.skip_auto_install = skip;
        self
    }

    pub fn dind(mut self, config: DindConfig) -> Self {
        self.dind = config;
        self
    }

    pub fn runtime(mut self, runtime: Option<String>) -> Self {
        self.runtime = runtime;
        self
    }

    pub fn privileged(mut self, privileged: bool) -> Self {
        self.privileged = privileged;
        self
    }

    pub fn network_mode(mut self, mode: &str) -> Self {
        self.network_mode = mode.to_string();
        self
    }

    pub fn extra_hosts(mut self, hosts: Vec<String>) -> Self {
        self.extra_hosts = hosts;
        self
    }

    pub fn ports(mut self, ports: Vec<String>) -> Self {
        self.ports = ports;
        self
    }

    pub fn secrets(mut self, has_secrets: bool, file: &str) -> Self {
        self.has_secrets = has_secrets;
        self.secrets_file = file.to_string();
        self
    }

    pub fn env_vars(mut self, vars: HashMap<String, String>) -> Self {
        self.env_vars = vars;
        self
    }

    pub fn ci_mode(mut self, ci_mode: bool) -> Self {
        self.ci_mode = ci_mode;
        self
    }

    pub fn build(self) -> TemplateContext {
        TemplateContext {
            name: if self.name.is_empty() {
                "sindri-dev".to_string()
            } else {
                self.name
            },
            profile: if self.profile.is_empty() {
                "base".to_string()
            } else {
                self.profile
            },
            image: if self.image.is_empty() {
                "sindri:latest".to_string()
            } else {
                self.image
            },
            memory: if self.memory.is_empty() {
                "4GB".to_string()
            } else {
                self.memory
            },
            cpus: if self.cpus == 0 { 2 } else { self.cpus },
            volume_size: if self.volume_size.is_empty() {
                "10GB".to_string()
            } else {
                self.volume_size
            },
            gpu_enabled: self.gpu_enabled,
            gpu_type: if self.gpu_type.is_empty() {
                "nvidia".to_string()
            } else {
                self.gpu_type
            },
            gpu_count: self.gpu_count,
            custom_extensions: self.custom_extensions,
            additional_extensions: self.additional_extensions,
            skip_auto_install: self.skip_auto_install,
            dind: self.dind,
            runtime: self.runtime,
            privileged: self.privileged,
            network_mode: if self.network_mode.is_empty() {
                "bridge".to_string()
            } else {
                self.network_mode
            },
            extra_hosts: self.extra_hosts,
            ports: self.ports,
            has_secrets: self.has_secrets,
            secrets_file: if self.secrets_file.is_empty() {
                ".env.secrets".to_string()
            } else {
                self.secrets_file
            },
            env_vars: self.env_vars,
            ci_mode: self.ci_mode,
        }
    }
}
