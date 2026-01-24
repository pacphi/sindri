//! Tool registry
//!
//! Static registry of all known tools that Sindri may depend on.
//! Each tool includes detection commands, version requirements,
//! and platform-specific installation instructions.

use crate::platform::{LinuxDistro, PackageManager, Platform};
use crate::tool::{
    AuthCheck, AuthSuccessIndicator, InstallInstruction, ToolCategory, ToolDefinition,
};

/// Static registry of all known tools
pub static TOOL_REGISTRY: &[ToolDefinition] = &[
    // ==========================================================================
    // Core Tools
    // ==========================================================================
    ToolDefinition {
        id: "git",
        name: "Git",
        description: "Distributed version control system",
        command: "git",
        version_flag: "--version",
        min_version: Some("2.0.0"),
        categories: &[ToolCategory::Core],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install git",
                notes: Some("Or install Xcode Command Line Tools: xcode-select --install"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: Some(PackageManager::Apt),
                command: "sudo apt-get install git",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Fedora),
                package_manager: Some(PackageManager::Dnf),
                command: "sudo dnf install git",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Arch),
                package_manager: Some(PackageManager::Pacman),
                command: "sudo pacman -S git",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Alpine),
                package_manager: Some(PackageManager::Apk),
                command: "apk add git",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Winget),
                command: "winget install Git.Git",
                notes: Some("Or download from https://git-scm.com/download/win"),
            },
        ],
        docs_url: "https://git-scm.com/doc",
        optional: false,
    },
    // ==========================================================================
    // Docker Provider
    // ==========================================================================
    ToolDefinition {
        id: "docker",
        name: "Docker",
        description: "Container runtime for building and running applications",
        command: "docker",
        version_flag: "--version",
        min_version: Some("20.10.0"),
        categories: &[ToolCategory::ProviderDocker],
        auth_check: Some(AuthCheck {
            command: "docker",
            args: &["info"],
            success_indicator: AuthSuccessIndicator::ExitCode(0),
        }),
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install --cask docker",
                notes: Some("Or download Docker Desktop from https://docker.com/products/docker-desktop"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: None,
                command: "curl -fsSL https://get.docker.com | sh",
                notes: Some("Add user to docker group: sudo usermod -aG docker $USER"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Fedora),
                package_manager: Some(PackageManager::Dnf),
                command: "sudo dnf install docker-ce docker-ce-cli containerd.io",
                notes: Some("Enable: sudo systemctl enable --now docker"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: None,
                command: "Download Docker Desktop from https://docker.com/products/docker-desktop",
                notes: Some("Requires WSL2 or Hyper-V"),
            },
        ],
        docs_url: "https://docs.docker.com/get-docker/",
        optional: false,
    },
    ToolDefinition {
        id: "docker-compose",
        name: "Docker Compose v2",
        description: "Multi-container orchestration tool",
        command: "docker",
        version_flag: "compose version",
        min_version: Some("2.0.0"),
        categories: &[ToolCategory::ProviderDocker],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: None,
                command: "Included with Docker Desktop",
                notes: Some("Docker Compose v2 is built into Docker Desktop"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: Some(PackageManager::Apt),
                command: "sudo apt-get install docker-compose-plugin",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: None,
                command: "Included with Docker Desktop",
                notes: None,
            },
        ],
        docs_url: "https://docs.docker.com/compose/",
        optional: false,
    },
    // ==========================================================================
    // Fly.io Provider
    // ==========================================================================
    ToolDefinition {
        id: "flyctl",
        name: "Fly CLI",
        description: "Command-line interface for Fly.io platform",
        command: "flyctl",
        version_flag: "version",
        min_version: Some("0.1.0"),
        categories: &[ToolCategory::ProviderFly],
        auth_check: Some(AuthCheck {
            command: "flyctl",
            args: &["auth", "whoami"],
            success_indicator: AuthSuccessIndicator::ExitCode(0),
        }),
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install flyctl",
                notes: Some("Then run: flyctl auth login"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl -L https://fly.io/install.sh | sh",
                notes: Some("Then run: flyctl auth login"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Scoop),
                command: "scoop install flyctl",
                notes: Some("Or: iwr https://fly.io/install.ps1 -useb | iex"),
            },
        ],
        docs_url: "https://fly.io/docs/hands-on/install-flyctl/",
        optional: false,
    },
    // ==========================================================================
    // DevPod Provider
    // ==========================================================================
    ToolDefinition {
        id: "devpod",
        name: "DevPod",
        description: "Open source dev-environments-as-code",
        command: "devpod",
        version_flag: "version",
        min_version: Some("0.4.0"),
        categories: &[ToolCategory::ProviderDevpod],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install devpod",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl -L -o devpod \"https://github.com/loft-sh/devpod/releases/latest/download/devpod-linux-amd64\" && chmod +x devpod && sudo mv devpod /usr/local/bin/",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Winget),
                command: "winget install loft-sh.devpod",
                notes: None,
            },
        ],
        docs_url: "https://devpod.sh/docs/getting-started/install",
        optional: false,
    },
    // ==========================================================================
    // E2B Provider
    // ==========================================================================
    ToolDefinition {
        id: "e2b",
        name: "E2B CLI",
        description: "Command-line interface for E2B cloud sandboxes",
        command: "e2b",
        version_flag: "--version",
        min_version: None,
        categories: &[ToolCategory::ProviderE2B],
        auth_check: Some(AuthCheck {
            command: "e2b",
            args: &["auth", "status"],
            success_indicator: AuthSuccessIndicator::ExitCode(0),
        }),
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: None,
                command: "npm install -g @e2b/cli",
                notes: Some("Requires Node.js. Then run: e2b auth login"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "npm install -g @e2b/cli",
                notes: Some("Requires Node.js. Then run: e2b auth login"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: None,
                command: "npm install -g @e2b/cli",
                notes: Some("Requires Node.js. Then run: e2b auth login"),
            },
        ],
        docs_url: "https://e2b.dev/docs",
        optional: false,
    },
    // ==========================================================================
    // Kubernetes Provider
    // ==========================================================================
    ToolDefinition {
        id: "kubectl",
        name: "kubectl",
        description: "Kubernetes command-line tool",
        command: "kubectl",
        version_flag: "version --client",
        min_version: Some("1.20.0"),
        categories: &[ToolCategory::ProviderKubernetes],
        auth_check: Some(AuthCheck {
            command: "kubectl",
            args: &["cluster-info"],
            success_indicator: AuthSuccessIndicator::ExitCode(0),
        }),
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install kubectl",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: Some(PackageManager::Apt),
                command: "sudo apt-get install -y apt-transport-https ca-certificates curl && curl -fsSL https://pkgs.k8s.io/core:/stable:/v1.28/deb/Release.key | sudo gpg --dearmor -o /etc/apt/keyrings/kubernetes-apt-keyring.gpg && echo 'deb [signed-by=/etc/apt/keyrings/kubernetes-apt-keyring.gpg] https://pkgs.k8s.io/core:/stable:/v1.28/deb/ /' | sudo tee /etc/apt/sources.list.d/kubernetes.list && sudo apt-get update && sudo apt-get install -y kubectl",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Fedora),
                package_manager: Some(PackageManager::Dnf),
                command: "sudo dnf install kubernetes-client",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Winget),
                command: "winget install Kubernetes.kubectl",
                notes: None,
            },
        ],
        docs_url: "https://kubernetes.io/docs/tasks/tools/",
        optional: false,
    },
    // ==========================================================================
    // Extension Installation Backends
    // ==========================================================================
    ToolDefinition {
        id: "mise",
        name: "mise",
        description: "Polyglot tool version manager (formerly rtx)",
        command: "mise",
        version_flag: "--version",
        min_version: Some("2024.0.0"),
        categories: &[ToolCategory::ExtensionBackend],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install mise",
                notes: Some("Then add to shell: eval \"$(mise activate bash)\""),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl https://mise.run | sh",
                notes: Some("Then add to shell: eval \"$(mise activate bash)\""),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Scoop),
                command: "scoop install mise",
                notes: None,
            },
        ],
        docs_url: "https://mise.jdx.dev/getting-started.html",
        optional: false,
    },
    ToolDefinition {
        id: "npm",
        name: "npm",
        description: "Node.js package manager",
        command: "npm",
        version_flag: "--version",
        min_version: Some("8.0.0"),
        categories: &[ToolCategory::ExtensionBackend],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install node",
                notes: Some("npm is included with Node.js"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: Some(PackageManager::Apt),
                command: "sudo apt-get install nodejs npm",
                notes: Some("Or use nvm for version management"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Fedora),
                package_manager: Some(PackageManager::Dnf),
                command: "sudo dnf install nodejs npm",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Winget),
                command: "winget install OpenJS.NodeJS",
                notes: Some("npm is included with Node.js"),
            },
        ],
        docs_url: "https://nodejs.org/",
        optional: false,
    },
    ToolDefinition {
        id: "apt-get",
        name: "APT",
        description: "Advanced Package Tool for Debian/Ubuntu",
        command: "apt-get",
        version_flag: "--version",
        min_version: None,
        categories: &[ToolCategory::ExtensionBackend],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: None,
                command: "Pre-installed on Debian/Ubuntu",
                notes: None,
            },
        ],
        docs_url: "https://wiki.debian.org/Apt",
        optional: true, // Only available on Debian-based systems
    },
    ToolDefinition {
        id: "curl",
        name: "curl",
        description: "Command-line tool for transferring data",
        command: "curl",
        version_flag: "--version",
        min_version: None,
        categories: &[ToolCategory::ExtensionBackend],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: None,
                command: "Pre-installed on macOS",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: Some(PackageManager::Apt),
                command: "sudo apt-get install curl",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Winget),
                command: "winget install cURL.cURL",
                notes: None,
            },
        ],
        docs_url: "https://curl.se/",
        optional: false,
    },
    // ==========================================================================
    // Secret Management Tools
    // ==========================================================================
    ToolDefinition {
        id: "vault",
        name: "HashiCorp Vault",
        description: "Secrets management and encryption",
        command: "vault",
        version_flag: "--version",
        min_version: Some("1.10.0"),
        categories: &[ToolCategory::Secrets],
        auth_check: Some(AuthCheck {
            command: "vault",
            args: &["status"],
            success_indicator: AuthSuccessIndicator::ExitCode(0),
        }),
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install vault",
                notes: Some("Configure VAULT_ADDR and run: vault login"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: Some(PackageManager::Apt),
                command: "wget -O- https://apt.releases.hashicorp.com/gpg | sudo gpg --dearmor -o /usr/share/keyrings/hashicorp-archive-keyring.gpg && echo \"deb [signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] https://apt.releases.hashicorp.com $(lsb_release -cs) main\" | sudo tee /etc/apt/sources.list.d/hashicorp.list && sudo apt-get update && sudo apt-get install vault",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Chocolatey),
                command: "choco install vault",
                notes: None,
            },
        ],
        docs_url: "https://developer.hashicorp.com/vault/docs",
        optional: true, // Only needed if using Vault for secrets
    },
    // ==========================================================================
    // Optional Enhancement Tools
    // ==========================================================================
    ToolDefinition {
        id: "gh",
        name: "GitHub CLI",
        description: "GitHub's official command-line tool",
        command: "gh",
        version_flag: "--version",
        min_version: Some("2.0.0"),
        categories: &[ToolCategory::Optional],
        auth_check: Some(AuthCheck {
            command: "gh",
            args: &["auth", "status"],
            success_indicator: AuthSuccessIndicator::ExitCode(0),
        }),
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install gh",
                notes: Some("Then run: gh auth login"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: Some(PackageManager::Apt),
                command: "sudo apt-get install gh",
                notes: Some("Or see: https://github.com/cli/cli/blob/trunk/docs/install_linux.md"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Fedora),
                package_manager: Some(PackageManager::Dnf),
                command: "sudo dnf install gh",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Winget),
                command: "winget install GitHub.cli",
                notes: Some("Then run: gh auth login"),
            },
        ],
        docs_url: "https://cli.github.com/",
        optional: true,
    },
];

/// Tool registry operations
pub struct ToolRegistry;

impl ToolRegistry {
    /// Get all tools in the registry
    pub fn all() -> &'static [ToolDefinition] {
        TOOL_REGISTRY
    }

    /// Get tools by category
    pub fn by_category(category: ToolCategory) -> Vec<&'static ToolDefinition> {
        TOOL_REGISTRY
            .iter()
            .filter(|t| t.categories.contains(&category))
            .collect()
    }

    /// Get tools required by a specific provider
    pub fn by_provider(provider: &str) -> Vec<&'static ToolDefinition> {
        let category = match provider.to_lowercase().as_str() {
            "docker" | "docker-compose" => Some(ToolCategory::ProviderDocker),
            "fly" | "fly.io" => Some(ToolCategory::ProviderFly),
            "devpod" => Some(ToolCategory::ProviderDevpod),
            "e2b" => Some(ToolCategory::ProviderE2B),
            "kubernetes" | "k8s" => Some(ToolCategory::ProviderKubernetes),
            _ => None,
        };

        match category {
            Some(cat) => Self::by_category(cat),
            None => vec![],
        }
    }

    /// Get tools required by a specific command
    pub fn by_command(command: &str) -> Vec<&'static ToolDefinition> {
        match command.to_lowercase().as_str() {
            "project" | "clone" | "new" => {
                vec![Self::get("git").unwrap(), Self::get("gh").unwrap()]
            }
            "extension" | "install" => Self::by_category(ToolCategory::ExtensionBackend),
            "secrets" => Self::by_category(ToolCategory::Secrets),
            "deploy" => {
                // Return all provider tools
                let mut tools = Vec::new();
                tools.extend(Self::by_category(ToolCategory::ProviderDocker));
                tools.extend(Self::by_category(ToolCategory::ProviderFly));
                tools.extend(Self::by_category(ToolCategory::ProviderDevpod));
                tools.extend(Self::by_category(ToolCategory::ProviderE2B));
                tools.extend(Self::by_category(ToolCategory::ProviderKubernetes));
                tools
            }
            _ => vec![],
        }
    }

    /// Get a specific tool by ID
    pub fn get(id: &str) -> Option<&'static ToolDefinition> {
        TOOL_REGISTRY.iter().find(|t| t.id == id)
    }

    /// Get total number of tools in registry
    pub fn count() -> usize {
        TOOL_REGISTRY.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_not_empty() {
        assert!(!TOOL_REGISTRY.is_empty());
    }

    #[test]
    fn test_registry_count() {
        assert!(ToolRegistry::count() > 0);
    }

    #[test]
    fn test_get_git() {
        let git = ToolRegistry::get("git");
        assert!(git.is_some());
        let git = git.unwrap();
        assert_eq!(git.name, "Git");
        assert_eq!(git.command, "git");
        assert!(!git.optional);
    }

    #[test]
    fn test_get_docker() {
        let docker = ToolRegistry::get("docker");
        assert!(docker.is_some());
        let docker = docker.unwrap();
        assert_eq!(docker.name, "Docker");
        assert!(docker.auth_check.is_some());
    }

    #[test]
    fn test_by_category_core() {
        let core_tools = ToolRegistry::by_category(ToolCategory::Core);
        assert!(!core_tools.is_empty());
        assert!(core_tools.iter().any(|t| t.id == "git"));
    }

    #[test]
    fn test_by_category_docker() {
        let docker_tools = ToolRegistry::by_category(ToolCategory::ProviderDocker);
        assert!(!docker_tools.is_empty());
        assert!(docker_tools.iter().any(|t| t.id == "docker"));
        assert!(docker_tools.iter().any(|t| t.id == "docker-compose"));
    }

    #[test]
    fn test_by_provider() {
        let docker_tools = ToolRegistry::by_provider("docker");
        assert!(!docker_tools.is_empty());

        let fly_tools = ToolRegistry::by_provider("fly");
        assert!(!fly_tools.is_empty());

        let unknown_tools = ToolRegistry::by_provider("unknown");
        assert!(unknown_tools.is_empty());
    }

    #[test]
    fn test_by_command() {
        let project_tools = ToolRegistry::by_command("project");
        assert!(!project_tools.is_empty());
        assert!(project_tools.iter().any(|t| t.id == "git"));

        let extension_tools = ToolRegistry::by_command("extension");
        assert!(!extension_tools.is_empty());
    }

    #[test]
    fn test_all_tools_have_install_instructions() {
        for tool in TOOL_REGISTRY {
            assert!(
                !tool.install_instructions.is_empty(),
                "Tool {} has no install instructions",
                tool.id
            );
        }
    }

    #[test]
    fn test_all_tools_have_docs_url() {
        for tool in TOOL_REGISTRY {
            assert!(
                !tool.docs_url.is_empty(),
                "Tool {} has no docs URL",
                tool.id
            );
        }
    }
}
