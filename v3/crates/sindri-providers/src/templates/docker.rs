//! Docker-specific template utilities

use super::context::TemplateContext;
use super::TemplateRegistry;
use anyhow::Result;
use std::path::Path;

/// Docker template generator
pub struct DockerTemplates {
    registry: TemplateRegistry,
}

impl DockerTemplates {
    /// Create a new Docker template generator
    pub fn new() -> Result<Self> {
        Ok(Self {
            registry: TemplateRegistry::new()?,
        })
    }

    /// Generate docker-compose.yml
    pub fn generate_compose(&self, context: &TemplateContext) -> Result<String> {
        self.registry.render("docker-compose.yml", context)
    }

    /// Generate docker-compose.yml and write to file
    pub fn generate_compose_file(
        &self,
        context: &TemplateContext,
        output_dir: &Path,
    ) -> Result<()> {
        let compose_path = output_dir.join("docker-compose.yml");
        self.registry
            .render_to_file("docker-compose.yml", context, &compose_path)
    }

    /// Determine the DinD mode based on configuration and host capabilities
    pub fn detect_dind_mode(
        requested_mode: &str,
        has_sysbox: bool,
        privileged_allowed: bool,
    ) -> String {
        match requested_mode {
            "sysbox" => {
                if has_sysbox {
                    "sysbox".to_string()
                } else {
                    // Sysbox requested but not available
                    "none".to_string()
                }
            }
            "privileged" => "privileged".to_string(),
            "socket" => "socket".to_string(),
            "auto" => {
                if has_sysbox {
                    "sysbox".to_string()
                } else if privileged_allowed {
                    "privileged".to_string()
                } else {
                    "none".to_string()
                }
            }
            _ => "none".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_dind_mode_auto_with_sysbox() {
        let mode = DockerTemplates::detect_dind_mode("auto", true, false);
        assert_eq!(mode, "sysbox");
    }

    #[test]
    fn test_detect_dind_mode_auto_with_privileged() {
        let mode = DockerTemplates::detect_dind_mode("auto", false, true);
        assert_eq!(mode, "privileged");
    }

    #[test]
    fn test_detect_dind_mode_explicit_sysbox() {
        let mode = DockerTemplates::detect_dind_mode("sysbox", true, false);
        assert_eq!(mode, "sysbox");
    }

    #[test]
    fn test_detect_dind_mode_sysbox_not_available() {
        let mode = DockerTemplates::detect_dind_mode("sysbox", false, false);
        assert_eq!(mode, "none");
    }
}
