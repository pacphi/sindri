use crate::error::BackendError;
use crate::traits::{binary_available, run_command, InstallBackend};
use sindri_core::component::Backend;
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::Platform;

/// `go install` backend — universal Go toolchain (ADR-009).
///
/// `go install` requires an explicit version suffix (`@latest` or a semver
/// tag); the resolved component carries the version, so this backend always
/// renders `module@version`.
///
/// Note: `go install` has no first-class uninstall command. [`Self::remove`]
/// performs a best-effort cleanup by deleting the binary from `$GOBIN` (or
/// `$GOPATH/bin`, falling back to `~/go/bin`).
pub struct GoInstallBackend;

impl GoInstallBackend {
    /// Resolve the directory `go install` writes binaries into, in the order
    /// the Go toolchain itself does: `$GOBIN`, then `$GOPATH/bin`, then
    /// `~/go/bin`.
    fn install_dir() -> std::path::PathBuf {
        if let Ok(gobin) = std::env::var("GOBIN") {
            if !gobin.is_empty() {
                return std::path::PathBuf::from(gobin);
            }
        }
        if let Ok(gopath) = std::env::var("GOPATH") {
            if !gopath.is_empty() {
                return std::path::PathBuf::from(gopath).join("bin");
            }
        }
        dirs_next::home_dir()
            .unwrap_or_default()
            .join("go")
            .join("bin")
    }

    /// `go install` produces a binary named after the last path segment of
    /// the module (e.g. `github.com/foo/bar/cmd/baz` → `baz`).
    fn binary_name(module: &str) -> &str {
        module.rsplit('/').next().unwrap_or(module)
    }
}

impl InstallBackend for GoInstallBackend {
    fn name(&self) -> Backend {
        Backend::GoInstall
    }

    fn supports(&self, _platform: &Platform) -> bool {
        binary_available("go")
    }

    fn install(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        let module_at_ver = format!("{}@{}", comp.id.name, comp.version);
        tracing::info!("go: installing {}", module_at_ver);
        run_command("go", &["install", &module_at_ver])?;
        Ok(())
    }

    fn remove(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
        // `go install` has no first-class uninstall; delete the binary path.
        let bin_name = Self::binary_name(&comp.id.name);
        let bin_path = Self::install_dir().join(bin_name);
        if bin_path.exists() {
            tracing::info!("go: removing binary {}", bin_path.display());
            std::fs::remove_file(&bin_path)?;
            Ok(())
        } else {
            Err(BackendError::RemoveFailed {
                component: comp.id.to_address(),
                detail: format!(
                    "go-install has no native uninstall and no binary was found at {}; \
                     remove it manually or set GOBIN to point at the install dir",
                    bin_path.display()
                ),
            })
        }
    }

    fn is_installed(&self, comp: &ResolvedComponent) -> bool {
        // Best-effort: check that the binary exists in the resolved install dir.
        let bin_name = Self::binary_name(&comp.id.name);
        Self::install_dir().join(bin_name).is_file()
    }
}

#[cfg(test)]
mod tests {
    use sindri_core::component::ComponentManifest;

    #[test]
    fn deserializes_go_install_config() {
        let yaml = r#"
metadata:
  name: golangci-lint
  version: 1.61.0
  description: Go linter aggregator
  license: GPL-3.0
  tags: []
platforms:
  - { os: linux, arch: x86_64 }
install:
  go-install:
    module: github.com/golangci/golangci-lint/cmd/golangci-lint
    version: v1.61.0
"#;
        let manifest: ComponentManifest =
            serde_yaml::from_str(yaml).expect("parse go-install config");
        let go = manifest
            .install
            .go_install
            .expect("go-install config present");
        assert_eq!(
            go.module,
            "github.com/golangci/golangci-lint/cmd/golangci-lint"
        );
        assert_eq!(go.version, "v1.61.0");
    }

    #[test]
    fn go_install_accepts_at_latest() {
        let yaml = r#"
metadata:
  name: gopls
  version: latest
  description: Go language server
  license: BSD-3-Clause
  tags: []
platforms:
  - { os: linux, arch: x86_64 }
install:
  go-install:
    module: golang.org/x/tools/gopls
    version: latest
"#;
        let manifest: ComponentManifest =
            serde_yaml::from_str(yaml).expect("parse go-install config");
        let go = manifest.install.go_install.expect("go-install config");
        assert_eq!(go.module, "golang.org/x/tools/gopls");
        assert_eq!(go.version, "latest");
    }

    #[test]
    fn binary_name_uses_last_path_segment() {
        assert_eq!(
            super::GoInstallBackend::binary_name(
                "github.com/golangci/golangci-lint/cmd/golangci-lint"
            ),
            "golangci-lint"
        );
        assert_eq!(
            super::GoInstallBackend::binary_name("golang.org/x/tools/gopls"),
            "gopls"
        );
        assert_eq!(super::GoInstallBackend::binary_name("plain"), "plain");
    }
}
