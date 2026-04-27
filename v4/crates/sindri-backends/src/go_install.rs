use crate::error::BackendError;
use crate::traits::{binary_available, target_exec, InstallBackend, InstallContext};
use async_trait::async_trait;
use sindri_core::component::Backend;
use sindri_core::platform::Platform;

/// `go install` backend — universal Go toolchain (ADR-009).
///
/// `go install` requires an explicit version suffix (`@latest` or a semver
/// tag); the resolved component (or the manifest, when present) carries
/// the version, so this backend always renders `module@version`.
///
/// Note: `go install` has no first-class uninstall command. [`Self::remove`]
/// performs a best-effort cleanup by deleting the binary from `$GOBIN` (or
/// `$GOPATH/bin`, falling back to `~/go/bin`) **on the local host only**.
/// Removing on a remote target is currently unsupported and will be
/// addressed when remote-target install support lands (Wave 3+).
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
        sindri_core::paths::home_dir()
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

#[async_trait]
impl InstallBackend for GoInstallBackend {
    fn name(&self) -> Backend {
        Backend::GoInstall
    }

    fn supports(&self, _platform: &Platform) -> bool {
        binary_available("go")
    }

    async fn install(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        let comp = ctx.component;
        let module_at_ver = if let Some(manifest) = ctx.manifest {
            if let Some(cfg) = manifest.install.go_install.as_ref() {
                format!("{}@{}", cfg.module, cfg.version)
            } else {
                tracing::debug!(
                    "go: manifest present but no go-install block for {}; \
                     using minimal command",
                    comp.id.to_address()
                );
                format!("{}@{}", comp.id.name, comp.version)
            }
        } else {
            tracing::debug!(
                "go: manifest not yet plumbed; using minimal command for {}",
                comp.id.to_address()
            );
            format!("{}@{}", comp.id.name, comp.version)
        };
        tracing::info!("go: installing {}", module_at_ver);
        target_exec(ctx.target, "go", &["install", &module_at_ver]).await?;
        Ok(())
    }

    async fn remove(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
        // reason: go-install remove inspects the local filesystem; remote
        // targets are out of scope until Wave 3+ remote-target work lands.
        if ctx.target.kind() != "local" {
            return Err(BackendError::RemoveFailed {
                component: ctx.component.id.to_address(),
                detail: "go-install remove is only supported on local targets; \
                         remote remove will land with Wave 3+ remote target work"
                    .into(),
            });
        }
        let comp = ctx.component;
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

    async fn is_installed(&self, ctx: &InstallContext<'_>) -> bool {
        // Best-effort, local-only: check binary in resolved install dir.
        // For remote targets we cannot easily inspect the filesystem; return
        // false (conservative) so the caller will issue the install command.
        if ctx.target.kind() != "local" {
            return false;
        }
        let bin_name = Self::binary_name(&ctx.component.id.name);
        Self::install_dir().join(bin_name).is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::test_support::MockTarget;
    use sindri_core::component::{ComponentId, ComponentManifest};
    use sindri_core::lockfile::ResolvedComponent;
    use sindri_core::version::Version;
    use std::collections::HashMap;

    fn comp(name: &str, ver: &str) -> ResolvedComponent {
        ResolvedComponent {
            id: ComponentId {
                backend: Backend::GoInstall,
                name: name.into(),
                qualifier: None,
            },
            version: Version::new(ver),
            backend: Backend::GoInstall,
            oci_digest: None,
            checksums: HashMap::new(),
            depends_on: vec![],
            manifest: None,
            manifest_digest: None,
        }
    }

    #[tokio::test]
    async fn install_with_manifest_uses_module_path() {
        let yaml = r#"
metadata:
  name: golangci-lint
  version: 1.61.0
  description: x
  license: GPL-3.0
  tags: []
platforms:
  - { os: linux, arch: x86_64 }
install:
  go-install:
    module: github.com/golangci/golangci-lint/cmd/golangci-lint
    version: v1.61.0
"#;
        let m: ComponentManifest = serde_yaml::from_str(yaml).unwrap();
        let mock = MockTarget::new();
        let c = comp("golangci-lint", "1.61.0");
        let ctx = InstallContext::new(&c, Some(&m), &mock);
        GoInstallBackend.install(&ctx).await.unwrap();
        assert_eq!(
            mock.last_call().as_deref(),
            Some("go install github.com/golangci/golangci-lint/cmd/golangci-lint@v1.61.0")
        );
    }

    #[tokio::test]
    async fn install_without_manifest_falls_back_to_name() {
        let mock = MockTarget::new();
        let c = comp("gopls", "latest");
        let ctx = InstallContext::new(&c, None, &mock);
        GoInstallBackend.install(&ctx).await.unwrap();
        assert_eq!(mock.last_call().as_deref(), Some("go install gopls@latest"));
    }

    #[test]
    fn binary_name_uses_last_path_segment() {
        assert_eq!(
            GoInstallBackend::binary_name("github.com/golangci/golangci-lint/cmd/golangci-lint"),
            "golangci-lint"
        );
        assert_eq!(
            GoInstallBackend::binary_name("golang.org/x/tools/gopls"),
            "gopls"
        );
        assert_eq!(GoInstallBackend::binary_name("plain"), "plain");
    }

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
}
