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
    // RunPod Provider
    // ==========================================================================
    ToolDefinition {
        id: "runpodctl",
        name: "RunPod CLI",
        description: "Command-line interface for RunPod platform",
        command: "runpodctl",
        version_flag: "version",
        min_version: Some("1.14.0"),
        categories: &[ToolCategory::ProviderRunpod],
        auth_check: Some(AuthCheck {
            command: "runpodctl",
            args: &["get", "pod"],
            success_indicator: AuthSuccessIndicator::ExitCode(0),
        }),
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: None,
                command: "curl -L https://github.com/runpod/runpodctl/releases/latest/download/runpodctl-darwin-amd64 -o runpodctl && chmod +x runpodctl && sudo mv runpodctl /usr/local/bin/",
                notes: Some("Then set API key: runpodctl config --apiKey=YOUR_API_KEY"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl -L https://github.com/runpod/runpodctl/releases/latest/download/runpodctl-linux-amd64 -o runpodctl && chmod +x runpodctl && sudo mv runpodctl /usr/local/bin/",
                notes: Some("Then set API key: runpodctl config --apiKey=YOUR_API_KEY"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: None,
                command: "Download runpodctl-windows-amd64.exe from https://github.com/runpod/runpodctl/releases",
                notes: Some("Then set API key: runpodctl config --apiKey=YOUR_API_KEY"),
            },
        ],
        docs_url: "https://github.com/runpod/runpodctl",
        optional: false,
    },
    // ==========================================================================
    // Northflank Provider
    // ==========================================================================
    ToolDefinition {
        id: "northflank",
        name: "Northflank CLI",
        description: "Command-line interface for Northflank platform",
        command: "northflank",
        version_flag: "--version",
        min_version: Some("0.10.0"),
        categories: &[ToolCategory::ProviderNorthflank],
        auth_check: Some(AuthCheck {
            command: "northflank",
            args: &["list", "projects"],
            success_indicator: AuthSuccessIndicator::ExitCode(0),
        }),
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Npm),
                command: "npm install -g @northflank/cli",
                notes: Some("Then authenticate: northflank login"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: Some(PackageManager::Npm),
                command: "npm install -g @northflank/cli",
                notes: Some("Then authenticate: northflank login"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Npm),
                command: "npm install -g @northflank/cli",
                notes: Some("Then authenticate: northflank login"),
            },
        ],
        docs_url: "https://northflank.com/docs/v1/api/cli",
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
    ToolDefinition {
        id: "cosign",
        name: "Cosign",
        description: "Container image signing and verification tool (Sigstore)",
        command: "cosign",
        version_flag: "version",
        min_version: Some("2.0.0"),
        categories: &[ToolCategory::Optional],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install cosign",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: Some(PackageManager::Apt),
                command: "sudo apt-get install cosign",
                notes: Some("Or download from: https://github.com/sigstore/cosign/releases"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Fedora),
                package_manager: Some(PackageManager::Dnf),
                command: "sudo dnf install cosign",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Arch),
                package_manager: Some(PackageManager::Pacman),
                command: "sudo pacman -S cosign",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Alpine),
                package_manager: Some(PackageManager::Apk),
                command: "apk add cosign",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl -sSfL https://github.com/sigstore/cosign/releases/latest/download/cosign-linux-amd64 -o /usr/local/bin/cosign && chmod +x /usr/local/bin/cosign",
                notes: Some("Or use: go install github.com/sigstore/cosign/v2/cmd/cosign@latest"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Scoop),
                command: "scoop install cosign",
                notes: Some("Or download from: https://github.com/sigstore/cosign/releases"),
            },
        ],
        docs_url: "https://docs.sigstore.dev/cosign/installation/",
        optional: true,
    },
    // ==========================================================================
    // Local Kubernetes Clusters
    // ==========================================================================
    ToolDefinition {
        id: "kind",
        name: "kind",
        description: "Kubernetes IN Docker - local clusters for testing",
        command: "kind",
        version_flag: "version",
        min_version: Some("0.17.0"),
        categories: &[ToolCategory::KubernetesClusters],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install kind",
                notes: Some("Requires Docker to be running"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl -Lo ./kind https://kind.sigs.k8s.io/dl/v0.20.0/kind-linux-amd64 && chmod +x ./kind && sudo mv ./kind /usr/local/bin/kind",
                notes: Some("Requires Docker to be running"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Chocolatey),
                command: "choco install kind",
                notes: Some("Requires Docker Desktop"),
            },
        ],
        docs_url: "https://kind.sigs.k8s.io/docs/user/quick-start/",
        optional: true,
    },
    ToolDefinition {
        id: "k3d",
        name: "k3d",
        description: "K3s in Docker - lightweight Kubernetes clusters",
        command: "k3d",
        version_flag: "version",
        min_version: Some("5.0.0"),
        categories: &[ToolCategory::KubernetesClusters],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install k3d",
                notes: Some("Requires Docker to be running"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash",
                notes: Some("Requires Docker to be running"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Chocolatey),
                command: "choco install k3d",
                notes: Some("Requires Docker Desktop"),
            },
        ],
        docs_url: "https://k3d.io/",
        optional: true,
    },
    // ==========================================================================
    // Packer / VM Image Building
    // ==========================================================================
    ToolDefinition {
        id: "packer",
        name: "HashiCorp Packer",
        description: "VM image builder for AWS, Azure, GCP, and more",
        command: "packer",
        version_flag: "version",
        min_version: Some("1.8.0"),
        categories: &[ToolCategory::ProviderPacker],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install packer",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: Some(PackageManager::Apt),
                command: "wget -O- https://apt.releases.hashicorp.com/gpg | sudo gpg --dearmor -o /usr/share/keyrings/hashicorp-archive-keyring.gpg && echo \"deb [signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] https://apt.releases.hashicorp.com $(lsb_release -cs) main\" | sudo tee /etc/apt/sources.list.d/hashicorp.list && sudo apt-get update && sudo apt-get install packer",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Fedora),
                package_manager: Some(PackageManager::Dnf),
                command: "sudo dnf install -y dnf-plugins-core && sudo dnf config-manager --add-repo https://rpm.releases.hashicorp.com/fedora/hashicorp.repo && sudo dnf -y install packer",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Chocolatey),
                command: "choco install packer",
                notes: None,
            },
        ],
        docs_url: "https://developer.hashicorp.com/packer/docs",
        optional: true,
    },
    // ==========================================================================
    // Infrastructure-as-Code Tools
    // ==========================================================================
    ToolDefinition {
        id: "terraform",
        name: "HashiCorp Terraform",
        description: "Infrastructure-as-code provisioning tool",
        command: "terraform",
        version_flag: "version",
        min_version: Some("1.0.0"),
        categories: &[ToolCategory::Infrastructure],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install terraform",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: Some(PackageManager::Apt),
                command: "wget -O- https://apt.releases.hashicorp.com/gpg | sudo gpg --dearmor -o /usr/share/keyrings/hashicorp-archive-keyring.gpg && echo \"deb [signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] https://apt.releases.hashicorp.com $(lsb_release -cs) main\" | sudo tee /etc/apt/sources.list.d/hashicorp.list && sudo apt-get update && sudo apt-get install terraform",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Fedora),
                package_manager: Some(PackageManager::Dnf),
                command: "sudo dnf install -y dnf-plugins-core && sudo dnf config-manager --add-repo https://rpm.releases.hashicorp.com/fedora/hashicorp.repo && sudo dnf -y install terraform",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Chocolatey),
                command: "choco install terraform",
                notes: None,
            },
        ],
        docs_url: "https://developer.hashicorp.com/terraform/docs",
        optional: true,
    },
    ToolDefinition {
        id: "ansible",
        name: "Ansible",
        description: "Configuration management and automation tool",
        command: "ansible",
        version_flag: "--version",
        min_version: Some("2.10.0"),
        categories: &[ToolCategory::Infrastructure],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install ansible",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: Some(PackageManager::Apt),
                command: "sudo apt-get install ansible",
                notes: Some("Or use pip: pip install ansible"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Fedora),
                package_manager: Some(PackageManager::Dnf),
                command: "sudo dnf install ansible",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "pip install ansible",
                notes: Some("Requires Python and pip"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: None,
                command: "pip install ansible",
                notes: Some("Run in WSL or use pip in Windows"),
            },
        ],
        docs_url: "https://docs.ansible.com/",
        optional: true,
    },
    ToolDefinition {
        id: "pulumi",
        name: "Pulumi",
        description: "Infrastructure-as-code using programming languages",
        command: "pulumi",
        version_flag: "version",
        min_version: Some("3.0.0"),
        categories: &[ToolCategory::Infrastructure],
        auth_check: Some(AuthCheck {
            command: "pulumi",
            args: &["whoami"],
            success_indicator: AuthSuccessIndicator::ExitCode(0),
        }),
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install pulumi",
                notes: Some("Then run: pulumi login"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl -fsSL https://get.pulumi.com | sh",
                notes: Some("Then run: pulumi login"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Chocolatey),
                command: "choco install pulumi",
                notes: Some("Then run: pulumi login"),
            },
        ],
        docs_url: "https://www.pulumi.com/docs/",
        optional: true,
    },
    // ==========================================================================
    // Cloud Provider CLIs
    // ==========================================================================
    ToolDefinition {
        id: "aws",
        name: "AWS CLI",
        description: "Amazon Web Services command-line interface",
        command: "aws",
        version_flag: "--version",
        min_version: Some("2.0.0"),
        categories: &[ToolCategory::CloudCLI],
        auth_check: Some(AuthCheck {
            command: "aws",
            args: &["sts", "get-caller-identity"],
            success_indicator: AuthSuccessIndicator::ExitCode(0),
        }),
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install awscli",
                notes: Some("Then run: aws configure"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl \"https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip\" -o \"awscliv2.zip\" && unzip awscliv2.zip && sudo ./aws/install",
                notes: Some("Then run: aws configure"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Winget),
                command: "winget install Amazon.AWSCLI",
                notes: Some("Then run: aws configure"),
            },
        ],
        docs_url: "https://docs.aws.amazon.com/cli/",
        optional: true,
    },
    ToolDefinition {
        id: "az",
        name: "Azure CLI",
        description: "Microsoft Azure command-line interface",
        command: "az",
        version_flag: "version",
        min_version: Some("2.40.0"),
        categories: &[ToolCategory::CloudCLI],
        auth_check: Some(AuthCheck {
            command: "az",
            args: &["account", "show"],
            success_indicator: AuthSuccessIndicator::ExitCode(0),
        }),
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install azure-cli",
                notes: Some("Then run: az login"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: None,
                command: "curl -sL https://aka.ms/InstallAzureCLIDeb | sudo bash",
                notes: Some("Then run: az login"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Fedora),
                package_manager: Some(PackageManager::Dnf),
                command: "sudo rpm --import https://packages.microsoft.com/keys/microsoft.asc && sudo dnf install -y https://packages.microsoft.com/config/rhel/9.0/packages-microsoft-prod.rpm && sudo dnf install azure-cli",
                notes: Some("Then run: az login"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Winget),
                command: "winget install Microsoft.AzureCLI",
                notes: Some("Then run: az login"),
            },
        ],
        docs_url: "https://docs.microsoft.com/en-us/cli/azure/",
        optional: true,
    },
    ToolDefinition {
        id: "gcloud",
        name: "Google Cloud CLI",
        description: "Google Cloud Platform command-line interface",
        command: "gcloud",
        version_flag: "version",
        min_version: Some("400.0.0"),
        categories: &[ToolCategory::CloudCLI],
        auth_check: Some(AuthCheck {
            command: "gcloud",
            args: &["auth", "list"],
            success_indicator: AuthSuccessIndicator::ExitCode(0),
        }),
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install --cask google-cloud-sdk",
                notes: Some("Then run: gcloud init"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: Some(PackageManager::Apt),
                command: "echo \"deb [signed-by=/usr/share/keyrings/cloud.google.gpg] https://packages.cloud.google.com/apt cloud-sdk main\" | sudo tee -a /etc/apt/sources.list.d/google-cloud-sdk.list && curl https://packages.cloud.google.com/apt/doc/apt-key.gpg | sudo apt-key --keyring /usr/share/keyrings/cloud.google.gpg add - && sudo apt-get update && sudo apt-get install google-cloud-cli",
                notes: Some("Then run: gcloud init"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl https://sdk.cloud.google.com | bash",
                notes: Some("Then run: gcloud init"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Winget),
                command: "winget install Google.CloudSDK",
                notes: Some("Then run: gcloud init"),
            },
        ],
        docs_url: "https://cloud.google.com/sdk/docs/",
        optional: true,
    },
    ToolDefinition {
        id: "oci",
        name: "Oracle Cloud CLI",
        description: "Oracle Cloud Infrastructure command-line interface",
        command: "oci",
        version_flag: "--version",
        min_version: Some("3.0.0"),
        categories: &[ToolCategory::CloudCLI],
        auth_check: Some(AuthCheck {
            command: "oci",
            args: &["iam", "region", "list"],
            success_indicator: AuthSuccessIndicator::ExitCode(0),
        }),
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install oci-cli",
                notes: Some("Then run: oci setup config"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "bash -c \"$(curl -L https://raw.githubusercontent.com/oracle/oci-cli/master/scripts/install/install.sh)\"",
                notes: Some("Then run: oci setup config"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: None,
                command: "powershell -NoProfile -ExecutionPolicy Bypass -Command \"iex ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/oracle/oci-cli/master/scripts/install/install.ps1'))\"",
                notes: Some("Then run: oci setup config"),
            },
        ],
        docs_url: "https://docs.oracle.com/en-us/iaas/Content/API/Concepts/cliconcepts.htm",
        optional: true,
    },
    ToolDefinition {
        id: "aliyun",
        name: "Alibaba Cloud CLI",
        description: "Alibaba Cloud command-line interface",
        command: "aliyun",
        version_flag: "version",
        min_version: Some("3.0.0"),
        categories: &[ToolCategory::CloudCLI],
        auth_check: Some(AuthCheck {
            command: "aliyun",
            args: &["configure", "list"],
            success_indicator: AuthSuccessIndicator::ExitCode(0),
        }),
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install aliyun-cli",
                notes: Some("Then run: aliyun configure"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl -sSL https://aliyuncli.alicdn.com/aliyun-cli-linux-latest-amd64.tgz | tar xz && sudo mv aliyun /usr/local/bin/",
                notes: Some("Then run: aliyun configure"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: None,
                command: "Download from https://aliyuncli.alicdn.com/aliyun-cli-windows-latest-amd64.zip and add to PATH",
                notes: Some("Then run: aliyun configure"),
            },
        ],
        docs_url: "https://www.alibabacloud.com/help/en/cli/",
        optional: true,
    },
    // ==========================================================================
    // Additional Extension Backends
    // ==========================================================================
    ToolDefinition {
        id: "pnpm",
        name: "pnpm",
        description: "Fast, disk space efficient Node.js package manager",
        command: "pnpm",
        version_flag: "--version",
        min_version: Some("8.0.0"),
        categories: &[ToolCategory::ExtensionBackend],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install pnpm",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl -fsSL https://get.pnpm.io/install.sh | sh -",
                notes: Some("Or use: npm install -g pnpm"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Winget),
                command: "winget install pnpm.pnpm",
                notes: Some("Or use: npm install -g pnpm"),
            },
        ],
        docs_url: "https://pnpm.io/",
        optional: true,
    },
    // ==========================================================================
    // Kubernetes Utilities
    // ==========================================================================
    ToolDefinition {
        id: "helm",
        name: "Helm",
        description: "Kubernetes package manager",
        command: "helm",
        version_flag: "version --short",
        min_version: Some("3.0.0"),
        categories: &[ToolCategory::Optional, ToolCategory::ProviderKubernetes],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install helm",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Chocolatey),
                command: "choco install kubernetes-helm",
                notes: None,
            },
        ],
        docs_url: "https://helm.sh/docs/",
        optional: true,
    },
    ToolDefinition {
        id: "kustomize",
        name: "Kustomize",
        description: "Kubernetes resource customization tool",
        command: "kustomize",
        version_flag: "version",
        min_version: Some("4.0.0"),
        categories: &[ToolCategory::Optional, ToolCategory::ProviderKubernetes],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install kustomize",
                notes: Some("Also available via: kubectl kustomize"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl -s \"https://raw.githubusercontent.com/kubernetes-sigs/kustomize/master/hack/install_kustomize.sh\" | bash && sudo mv kustomize /usr/local/bin/",
                notes: Some("Also available via: kubectl kustomize"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Chocolatey),
                command: "choco install kustomize",
                notes: None,
            },
        ],
        docs_url: "https://kustomize.io/",
        optional: true,
    },
    ToolDefinition {
        id: "kubectx",
        name: "kubectx",
        description: "Switch between Kubernetes contexts quickly",
        command: "kubectx",
        version_flag: "--version",
        min_version: None,
        categories: &[ToolCategory::Optional],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install kubectx",
                notes: Some("Also installs kubens"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: Some(PackageManager::Apt),
                command: "sudo apt-get install kubectx",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "sudo git clone https://github.com/ahmetb/kubectx /opt/kubectx && sudo ln -s /opt/kubectx/kubectx /usr/local/bin/kubectx && sudo ln -s /opt/kubectx/kubens /usr/local/bin/kubens",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Chocolatey),
                command: "choco install kubectx",
                notes: None,
            },
        ],
        docs_url: "https://github.com/ahmetb/kubectx",
        optional: true,
    },
    ToolDefinition {
        id: "kubens",
        name: "kubens",
        description: "Switch between Kubernetes namespaces quickly",
        command: "kubens",
        version_flag: "--version",
        min_version: None,
        categories: &[ToolCategory::Optional],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install kubectx",
                notes: Some("kubens is included with kubectx"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Debian),
                package_manager: Some(PackageManager::Apt),
                command: "sudo apt-get install kubectx",
                notes: Some("kubens is included with kubectx"),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "sudo git clone https://github.com/ahmetb/kubectx /opt/kubectx && sudo ln -s /opt/kubectx/kubens /usr/local/bin/kubens",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Chocolatey),
                command: "choco install kubens",
                notes: None,
            },
        ],
        docs_url: "https://github.com/ahmetb/kubectx",
        optional: true,
    },
    ToolDefinition {
        id: "k9s",
        name: "k9s",
        description: "Terminal UI for Kubernetes clusters",
        command: "k9s",
        version_flag: "version --short",
        min_version: Some("0.25.0"),
        categories: &[ToolCategory::Optional],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install k9s",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl -sS https://webi.sh/k9s | sh",
                notes: Some("Or download from: https://github.com/derailed/k9s/releases"),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Chocolatey),
                command: "choco install k9s",
                notes: None,
            },
        ],
        docs_url: "https://k9scli.io/",
        optional: true,
    },
    // ==========================================================================
    // YAML/JSON Utilities
    // ==========================================================================
    ToolDefinition {
        id: "yq",
        name: "yq",
        description: "YAML/JSON processor and query tool",
        command: "yq",
        version_flag: "--version",
        min_version: Some("4.0.0"),
        categories: &[ToolCategory::Optional],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install yq",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl -sSfL https://github.com/mikefarah/yq/releases/latest/download/yq_linux_amd64 -o /usr/local/bin/yq && chmod +x /usr/local/bin/yq",
                notes: None,
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Chocolatey),
                command: "choco install yq",
                notes: None,
            },
        ],
        docs_url: "https://mikefarah.gitbook.io/yq/",
        optional: true,
    },
    // ==========================================================================
    // Shell Enhancement Tools
    // ==========================================================================
    ToolDefinition {
        id: "starship",
        name: "Starship",
        description: "Cross-shell customizable prompt",
        command: "starship",
        version_flag: "--version",
        min_version: Some("1.0.0"),
        categories: &[ToolCategory::Optional],
        auth_check: None,
        install_instructions: &[
            InstallInstruction {
                platform: Platform::MacOS,
                package_manager: Some(PackageManager::Homebrew),
                command: "brew install starship",
                notes: Some("Add to shell: eval \"$(starship init bash)\""),
            },
            InstallInstruction {
                platform: Platform::Linux(LinuxDistro::Unknown),
                package_manager: None,
                command: "curl -sS https://starship.rs/install.sh | sh",
                notes: Some("Add to shell: eval \"$(starship init bash)\""),
            },
            InstallInstruction {
                platform: Platform::Windows,
                package_manager: Some(PackageManager::Winget),
                command: "winget install Starship.Starship",
                notes: Some("Add to PowerShell profile: Invoke-Expression (&starship init powershell)"),
            },
        ],
        docs_url: "https://starship.rs/",
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
            "runpod" => Some(ToolCategory::ProviderRunpod),
            "northflank" => Some(ToolCategory::ProviderNorthflank),
            "packer" | "vm" => Some(ToolCategory::ProviderPacker),
            "kind" | "k3d" | "local-k8s" => Some(ToolCategory::KubernetesClusters),
            "terraform" | "ansible" | "pulumi" | "iac" => Some(ToolCategory::Infrastructure),
            "aws" | "azure" | "gcp" | "cloud" => Some(ToolCategory::CloudCLI),
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
                tools.extend(Self::by_category(ToolCategory::ProviderRunpod));
                tools.extend(Self::by_category(ToolCategory::ProviderNorthflank));
                tools
            }
            "cluster" | "clusters" => Self::by_category(ToolCategory::KubernetesClusters),
            "image" | "packer" | "vm" => Self::by_category(ToolCategory::ProviderPacker),
            "infra" | "infrastructure" => Self::by_category(ToolCategory::Infrastructure),
            "cloud" => Self::by_category(ToolCategory::CloudCLI),
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

    #[test]
    fn test_get_cosign() {
        let cosign = ToolRegistry::get("cosign");
        assert!(cosign.is_some());
        let cosign = cosign.unwrap();
        assert_eq!(cosign.name, "Cosign");
        assert_eq!(cosign.command, "cosign");
        assert!(cosign.optional);
        assert!(cosign.categories.contains(&ToolCategory::Optional));
    }

    #[test]
    fn test_get_kind() {
        let kind = ToolRegistry::get("kind");
        assert!(kind.is_some());
        let kind = kind.unwrap();
        assert_eq!(kind.name, "kind");
        assert!(kind.categories.contains(&ToolCategory::KubernetesClusters));
    }

    #[test]
    fn test_get_k3d() {
        let k3d = ToolRegistry::get("k3d");
        assert!(k3d.is_some());
        let k3d = k3d.unwrap();
        assert_eq!(k3d.name, "k3d");
        assert!(k3d.categories.contains(&ToolCategory::KubernetesClusters));
    }

    #[test]
    fn test_get_packer() {
        let packer = ToolRegistry::get("packer");
        assert!(packer.is_some());
        let packer = packer.unwrap();
        assert_eq!(packer.name, "HashiCorp Packer");
        assert!(packer.categories.contains(&ToolCategory::ProviderPacker));
    }

    #[test]
    fn test_get_terraform() {
        let terraform = ToolRegistry::get("terraform");
        assert!(terraform.is_some());
        let terraform = terraform.unwrap();
        assert_eq!(terraform.name, "HashiCorp Terraform");
        assert!(terraform.categories.contains(&ToolCategory::Infrastructure));
    }

    #[test]
    fn test_get_aws_cli() {
        let aws = ToolRegistry::get("aws");
        assert!(aws.is_some());
        let aws = aws.unwrap();
        assert_eq!(aws.name, "AWS CLI");
        assert!(aws.categories.contains(&ToolCategory::CloudCLI));
        assert!(aws.auth_check.is_some());
    }

    #[test]
    fn test_by_category_kubernetes_clusters() {
        let cluster_tools = ToolRegistry::by_category(ToolCategory::KubernetesClusters);
        assert!(!cluster_tools.is_empty());
        assert!(cluster_tools.iter().any(|t| t.id == "kind"));
        assert!(cluster_tools.iter().any(|t| t.id == "k3d"));
    }

    #[test]
    fn test_by_category_infrastructure() {
        let infra_tools = ToolRegistry::by_category(ToolCategory::Infrastructure);
        assert!(!infra_tools.is_empty());
        assert!(infra_tools.iter().any(|t| t.id == "terraform"));
        assert!(infra_tools.iter().any(|t| t.id == "ansible"));
        assert!(infra_tools.iter().any(|t| t.id == "pulumi"));
    }

    #[test]
    fn test_by_category_cloud_cli() {
        let cloud_tools = ToolRegistry::by_category(ToolCategory::CloudCLI);
        assert!(!cloud_tools.is_empty());
        assert!(cloud_tools.iter().any(|t| t.id == "aws"));
        assert!(cloud_tools.iter().any(|t| t.id == "az"));
        assert!(cloud_tools.iter().any(|t| t.id == "gcloud"));
        assert!(cloud_tools.iter().any(|t| t.id == "oci"));
        assert!(cloud_tools.iter().any(|t| t.id == "aliyun"));
    }

    #[test]
    fn test_get_oci_cli() {
        let oci = ToolRegistry::get("oci");
        assert!(oci.is_some());
        let oci = oci.unwrap();
        assert_eq!(oci.name, "Oracle Cloud CLI");
        assert!(oci.categories.contains(&ToolCategory::CloudCLI));
        assert!(oci.auth_check.is_some());
        assert!(oci.optional);
    }

    #[test]
    fn test_get_aliyun_cli() {
        let aliyun = ToolRegistry::get("aliyun");
        assert!(aliyun.is_some());
        let aliyun = aliyun.unwrap();
        assert_eq!(aliyun.name, "Alibaba Cloud CLI");
        assert!(aliyun.categories.contains(&ToolCategory::CloudCLI));
        assert!(aliyun.auth_check.is_some());
        assert!(aliyun.optional);
    }

    #[test]
    fn test_by_provider_packer() {
        let packer_tools = ToolRegistry::by_provider("packer");
        assert!(!packer_tools.is_empty());
        assert!(packer_tools.iter().any(|t| t.id == "packer"));
    }

    #[test]
    fn test_by_provider_cloud() {
        let cloud_tools = ToolRegistry::by_provider("cloud");
        assert!(!cloud_tools.is_empty());
    }

    #[test]
    fn test_by_command_cluster() {
        let cluster_tools = ToolRegistry::by_command("cluster");
        assert!(!cluster_tools.is_empty());
        assert!(cluster_tools.iter().any(|t| t.id == "kind"));
        assert!(cluster_tools.iter().any(|t| t.id == "k3d"));
    }

    #[test]
    fn test_by_command_infra() {
        let infra_tools = ToolRegistry::by_command("infra");
        assert!(!infra_tools.is_empty());
        assert!(infra_tools.iter().any(|t| t.id == "terraform"));
    }

    #[test]
    fn test_get_runpodctl() {
        let runpod = ToolRegistry::get("runpodctl");
        assert!(runpod.is_some());
        let runpod = runpod.unwrap();
        assert_eq!(runpod.name, "RunPod CLI");
        assert_eq!(runpod.command, "runpodctl");
        assert!(runpod.categories.contains(&ToolCategory::ProviderRunpod));
        assert!(runpod.auth_check.is_some());
        assert!(!runpod.optional);
        assert_eq!(runpod.min_version, Some("1.14.0"));
    }

    #[test]
    fn test_get_northflank() {
        let nf = ToolRegistry::get("northflank");
        assert!(nf.is_some());
        let nf = nf.unwrap();
        assert_eq!(nf.name, "Northflank CLI");
        assert_eq!(nf.command, "northflank");
        assert!(nf.categories.contains(&ToolCategory::ProviderNorthflank));
        assert!(nf.auth_check.is_some());
        assert!(!nf.optional);
        assert_eq!(nf.min_version, Some("0.10.0"));
    }

    #[test]
    fn test_by_provider_runpod() {
        let runpod_tools = ToolRegistry::by_provider("runpod");
        assert!(!runpod_tools.is_empty());
        assert!(runpod_tools.iter().any(|t| t.id == "runpodctl"));
    }

    #[test]
    fn test_by_provider_northflank() {
        let nf_tools = ToolRegistry::by_provider("northflank");
        assert!(!nf_tools.is_empty());
        assert!(nf_tools.iter().any(|t| t.id == "northflank"));
    }

    #[test]
    fn test_by_category_runpod() {
        let runpod_tools = ToolRegistry::by_category(ToolCategory::ProviderRunpod);
        assert_eq!(runpod_tools.len(), 1);
        assert_eq!(runpod_tools[0].id, "runpodctl");
    }

    #[test]
    fn test_by_category_northflank() {
        let nf_tools = ToolRegistry::by_category(ToolCategory::ProviderNorthflank);
        assert_eq!(nf_tools.len(), 1);
        assert_eq!(nf_tools[0].id, "northflank");
    }

    #[test]
    fn test_deploy_command_includes_runpod_and_northflank() {
        let deploy_tools = ToolRegistry::by_command("deploy");
        assert!(deploy_tools.iter().any(|t| t.id == "runpodctl"));
        assert!(deploy_tools.iter().any(|t| t.id == "northflank"));
    }
}
