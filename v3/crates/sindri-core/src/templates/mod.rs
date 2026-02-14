//! Template rendering for config generation and extension docs
//!
//! Uses Tera templates to generate provider-specific sindri.yaml files
//! and extension documentation from extension.yaml definitions.

mod context;

pub use context::{ConfigInitContext, ProfileInfo};

use anyhow::Result;
use serde::Serialize;
use tera::{Context, Tera};
use tracing::debug;

use crate::types::{Extension, InstallMethod, UpgradeStrategy};

/// Output format for extension documentation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocOutputFormat {
    /// Plain markdown suitable for writing to files
    Markdown,
    /// Terminal-friendly colorized output (default)
    #[default]
    Terminal,
}

/// Template registry for config file generation
pub struct ConfigTemplateRegistry {
    tera: Tera,
}

impl ConfigTemplateRegistry {
    /// Create a new template registry with embedded templates
    pub fn new() -> Result<Self> {
        let mut tera = Tera::default();

        // Register the sindri.yaml template
        tera.add_raw_template("sindri.yaml", include_str!("sindri.yaml.tera"))?;

        Ok(Self { tera })
    }

    /// Render the sindri.yaml template with the given context
    pub fn render_config(&self, context: &ConfigInitContext) -> Result<String> {
        debug!(
            "Rendering sindri.yaml template for provider: {}",
            context.provider
        );
        let tera_context = context.to_tera_context()?;
        let rendered = self.tera.render("sindri.yaml", &tera_context)?;
        Ok(rendered)
    }
}

// =============================================================================
// Extension Documentation Rendering
// =============================================================================

/// BOM tool for template context
#[derive(Debug, Serialize)]
struct BomToolContext {
    name: String,
    version: String,
    r#type: String,
    license: String,
    description: String,
}

/// Environment variable for template context
#[derive(Debug, Serialize)]
struct EnvVarContext {
    key: String,
    value: String,
    scope: String,
}

/// Template config for template context
#[derive(Debug, Serialize)]
struct TemplateContext {
    source: String,
    destination: String,
    mode: String,
}

/// Validation command for template context
#[derive(Debug, Serialize)]
struct ValidationCmdContext {
    name: String,
    version_flag: String,
    expected_pattern: Option<String>,
}

/// Usage section for template context
#[derive(Debug, Serialize)]
struct UsageSectionContext {
    section: String,
    examples: Vec<UsageExampleContext>,
}

/// Usage example for template context
#[derive(Debug, Serialize)]
struct UsageExampleContext {
    description: Option<String>,
    code: String,
    language: String,
}

/// Related extension for template context
#[derive(Debug, Serialize)]
struct RelatedContext {
    name: String,
    description: String,
}

/// Capability summary for template context
#[derive(Debug, Serialize)]
struct CapabilitySummary {
    name: String,
    description: String,
}

/// Render extension documentation from an Extension struct
///
/// # Arguments
/// * `extension` - The extension to generate documentation for
/// * `format` - The output format (Terminal or Markdown)
pub fn render_extension_doc(extension: &Extension, format: DocOutputFormat) -> Result<String> {
    let mut tera = Tera::default();
    tera.add_raw_template("extension_doc.md", include_str!("extension_doc.md.tera"))?;

    let mut ctx = Context::new();

    // Basic metadata
    let title = extension
        .docs
        .as_ref()
        .and_then(|d| d.title.clone())
        .unwrap_or_else(|| title_case(&extension.metadata.name));

    let overview = extension
        .docs
        .as_ref()
        .and_then(|d| d.overview.clone())
        .unwrap_or_else(|| extension.metadata.description.clone());

    let last_updated = extension.docs.as_ref().and_then(|d| d.last_updated.clone());

    let features: Vec<String> = extension
        .docs
        .as_ref()
        .map(|d| d.features.clone())
        .unwrap_or_default();

    ctx.insert("title", &title);
    ctx.insert("name", &extension.metadata.name);
    ctx.insert("version", &extension.metadata.version);
    ctx.insert("category", &extension.metadata.category.to_string());
    ctx.insert("overview", &overview);
    ctx.insert("last_updated", &last_updated);
    ctx.insert("features", &features);

    // BOM tools
    let bom_tools: Vec<BomToolContext> = extension
        .bom
        .as_ref()
        .map(|bom| {
            bom.tools
                .iter()
                .map(|t| BomToolContext {
                    name: t.name.clone(),
                    version: t.version.clone().unwrap_or_else(|| "dynamic".to_string()),
                    r#type: t
                        .r#type
                        .as_ref()
                        .map(|ty| {
                            format!("{:?}", ty)
                                .to_lowercase()
                                .replace("cli_tool", "cli-tool")
                                .replace("packagemanager", "package-manager")
                        })
                        .unwrap_or_else(|| "tool".to_string()),
                    license: t.license.clone().unwrap_or_else(|| "-".to_string()),
                    description: t
                        .homepage
                        .as_ref()
                        .map(|h| format!("[Homepage]({})", h))
                        .unwrap_or_else(|| t.name.clone()),
                })
                .collect()
        })
        .unwrap_or_default();
    ctx.insert("bom_tools", &bom_tools);

    // Requirements
    let reqs = &extension.requirements;
    ctx.insert("disk_space", &reqs.as_ref().and_then(|r| r.disk_space));
    ctx.insert("memory", &reqs.as_ref().and_then(|r| r.memory));
    ctx.insert("install_time", &reqs.as_ref().and_then(|r| r.install_time));
    ctx.insert("dependencies", &extension.metadata.dependencies);

    let domains: Vec<String> = reqs.as_ref().map(|r| r.domains.clone()).unwrap_or_default();
    ctx.insert("domains", &domains);

    let secrets: Vec<String> = reqs.as_ref().map(|r| r.secrets.clone()).unwrap_or_default();
    ctx.insert("secrets", &secrets);

    // Environment variables
    let env_vars: Vec<EnvVarContext> = extension
        .configure
        .as_ref()
        .map(|c| {
            c.environment
                .iter()
                .map(|e| EnvVarContext {
                    key: e.key.clone(),
                    value: e.value.clone(),
                    scope: format!("{:?}", e.scope).to_lowercase(),
                })
                .collect()
        })
        .unwrap_or_default();
    ctx.insert("env_vars", &env_vars);

    // Templates
    let templates: Vec<TemplateContext> = extension
        .configure
        .as_ref()
        .map(|c| {
            c.templates
                .iter()
                .map(|t| TemplateContext {
                    source: t.source.clone(),
                    destination: t.destination.clone(),
                    mode: format!("{:?}", t.mode).to_lowercase(),
                })
                .collect()
        })
        .unwrap_or_default();
    ctx.insert("templates", &templates);

    // Install method
    let install_method = Some(format!("{:?}", extension.install.method).to_lowercase());
    let install_method_description = match extension.install.method {
        InstallMethod::Mise => {
            "Uses mise for tool management with automatic shim refresh.".to_string()
        }
        InstallMethod::Apt => "Uses apt package manager for installation.".to_string(),
        InstallMethod::Binary => "Downloads pre-built binaries.".to_string(),
        InstallMethod::Npm | InstallMethod::NpmGlobal => {
            "Uses npm for global package installation.".to_string()
        }
        InstallMethod::Script => "Uses a custom installation script.".to_string(),
        InstallMethod::Hybrid => {
            "Uses a hybrid installation approach combining multiple methods.".to_string()
        }
    };
    ctx.insert("install_method", &install_method);
    ctx.insert("install_method_description", &install_method_description);

    // Upgrade strategy
    let upgrade_strategy = extension.upgrade.as_ref().map(|u| match u.strategy {
        UpgradeStrategy::Automatic => "Automatic - uses built-in upgrade mechanism.".to_string(),
        UpgradeStrategy::Manual => "Manual - requires custom upgrade script.".to_string(),
        UpgradeStrategy::None => "None - no upgrade support. Remove and reinstall.".to_string(),
        UpgradeStrategy::Reinstall => {
            "Reinstall - removes and reinstalls the extension.".to_string()
        }
        UpgradeStrategy::InPlace => "In-place - updates existing installation.".to_string(),
    });
    ctx.insert("upgrade_strategy", &upgrade_strategy);

    // Validation commands
    let validation_commands: Vec<ValidationCmdContext> = extension
        .validate
        .commands
        .iter()
        .map(|c| ValidationCmdContext {
            name: c.name.clone(),
            version_flag: c.version_flag.clone(),
            expected_pattern: c.expected_pattern.clone(),
        })
        .collect();
    ctx.insert("validation_commands", &validation_commands);

    // Removal
    let remove_paths: Vec<String> = extension
        .remove
        .as_ref()
        .map(|r| r.paths.clone())
        .unwrap_or_default();
    ctx.insert("remove_paths", &remove_paths);

    let remove_mise = extension
        .remove
        .as_ref()
        .and_then(|r| r.mise.as_ref())
        .is_some();
    ctx.insert("remove_mise", &remove_mise);

    // Capabilities summary
    let mut capabilities_summary: Vec<CapabilitySummary> = Vec::new();
    if let Some(caps) = &extension.capabilities {
        if let Some(pi) = &caps.project_init {
            if pi.enabled {
                capabilities_summary.push(CapabilitySummary {
                    name: "Project Init".to_string(),
                    description: format!("{} initialization command(s)", pi.commands.len()),
                });
            }
        }
        if let Some(auth) = &caps.auth {
            capabilities_summary.push(CapabilitySummary {
                name: "Authentication".to_string(),
                description: format!("Provider: {:?}", auth.provider),
            });
        }
        if let Some(mcp) = &caps.mcp {
            if mcp.enabled {
                capabilities_summary.push(CapabilitySummary {
                    name: "MCP Server".to_string(),
                    description: format!("{} tool(s) available", mcp.tools.len()),
                });
            }
        }
        if let Some(ch) = &caps.collision_handling {
            if ch.enabled {
                capabilities_summary.push(CapabilitySummary {
                    name: "Collision Handling".to_string(),
                    description: format!("{} conflict rule(s)", ch.conflict_rules.len()),
                });
            }
        }
    }
    ctx.insert("capabilities_summary", &capabilities_summary);

    // Usage sections (from docs)
    let usage_sections: Vec<UsageSectionContext> = extension
        .docs
        .as_ref()
        .map(|d| {
            d.usage
                .iter()
                .map(|s| UsageSectionContext {
                    section: s.section.clone(),
                    examples: s
                        .examples
                        .iter()
                        .map(|e| UsageExampleContext {
                            description: e.description.clone(),
                            code: e.code.trim_end().to_string(),
                            language: e.language.clone(),
                        })
                        .collect(),
                })
                .collect()
        })
        .unwrap_or_default();
    ctx.insert("usage_sections", &usage_sections);

    // Related extensions (from docs)
    let related: Vec<RelatedContext> = extension
        .docs
        .as_ref()
        .map(|d| {
            d.related
                .iter()
                .map(|r| RelatedContext {
                    name: r.name.clone(),
                    description: r.description.clone(),
                })
                .collect()
        })
        .unwrap_or_default();
    ctx.insert("related", &related);

    // Notes
    let notes = extension.docs.as_ref().and_then(|d| d.notes.clone());
    ctx.insert("notes", &notes);

    let rendered = tera.render("extension_doc.md", &ctx)?;

    // Clean up excessive blank lines (collapse 3+ consecutive newlines to 2)
    let cleaned = collapse_blank_lines(&rendered);

    // Apply formatting based on output format
    match format {
        DocOutputFormat::Markdown => Ok(cleaned),
        DocOutputFormat::Terminal => {
            // Use termimad to render beautiful terminal output
            let mut skin = termimad::MadSkin::default();

            // Customize the skin for better readability
            skin.set_headers_fg(termimad::crossterm::style::Color::Cyan);
            skin.bold.set_fg(termimad::crossterm::style::Color::Yellow);
            skin.italic.set_fg(termimad::crossterm::style::Color::Green);
            skin.inline_code
                .set_fg(termimad::crossterm::style::Color::Magenta);

            // Remove background from code blocks for cleaner look
            skin.code_block
                .set_bg(termimad::crossterm::style::Color::Reset);

            // Render the markdown with terminal styling
            // Note: We convert to string to avoid lifetime issues
            let text = skin.text(&cleaned, None);
            Ok(format!("{}", text))
        }
    }
}

/// Convert hyphenated name to title case (e.g., "golang" -> "Golang", "mise-config" -> "Mise Config")
fn title_case(name: &str) -> String {
    name.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Collapse 3+ consecutive newlines into 2
fn collapse_blank_lines(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut consecutive_newlines = 0;

    for ch in input.chars() {
        if ch == '\n' {
            consecutive_newlines += 1;
            if consecutive_newlines <= 2 {
                result.push(ch);
            }
        } else {
            consecutive_newlines = 0;
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Provider;

    #[test]
    fn test_template_registry_creation() {
        let registry = ConfigTemplateRegistry::new().unwrap();
        assert!(registry.tera.get_template_names().count() > 0);
    }

    #[test]
    fn test_render_config_docker() {
        let registry = ConfigTemplateRegistry::new().unwrap();
        let context = ConfigInitContext::new("test-project", Provider::Docker, "minimal");

        let result = registry.render_config(&context);
        if let Err(ref e) = result {
            eprintln!("Template error: {}", e);
        }
        assert!(result.is_ok(), "Template error: {:?}", result.err());
        let content = result.unwrap();
        assert!(content.contains("name: test-project"));
        assert!(content.contains("provider: docker"));
        assert!(content.contains("profile: minimal"));
    }

    #[test]
    fn test_render_config_fly() {
        let registry = ConfigTemplateRegistry::new().unwrap();
        let context = ConfigInitContext::new("my-app", Provider::Fly, "fullstack");

        let result = registry.render_config(&context);
        let content = result.expect("render_config for Fly provider should succeed");
        assert!(content.contains("name: my-app"));
        assert!(content.contains("provider: fly"));
        assert!(content.contains("profile: fullstack"));
        // Fly-specific content
        assert!(content.contains("region:"));
    }

    #[test]
    fn test_render_config_e2b_no_gpu() {
        let registry = ConfigTemplateRegistry::new().unwrap();
        let context = ConfigInitContext::new("sandbox", Provider::E2b, "minimal");

        let result = registry.render_config(&context);
        let content = result.expect("render_config for E2B provider should succeed");
        // E2B doesn't support GPU, so GPU section should not appear
        assert!(!content.contains("gpu:") || content.contains("# Note: E2B"));
    }

    #[test]
    fn test_context_profiles_loaded() {
        let context = ConfigInitContext::new("test", Provider::Docker, "minimal");
        assert!(!context.profiles.is_empty());
        assert!(context.profiles.iter().any(|p| p.name == "minimal"));
        assert!(context.profiles.iter().any(|p| p.name == "fullstack"));
    }

    #[test]
    fn test_title_case() {
        assert_eq!(title_case("golang"), "Golang");
        assert_eq!(title_case("mise-config"), "Mise Config");
        assert_eq!(title_case("claude-flow-v2"), "Claude Flow V2");
    }

    #[test]
    fn test_collapse_blank_lines() {
        assert_eq!(collapse_blank_lines("a\n\n\n\nb"), "a\n\nb");
        assert_eq!(collapse_blank_lines("a\n\nb"), "a\n\nb");
        assert_eq!(collapse_blank_lines("a\nb"), "a\nb");
    }
}
