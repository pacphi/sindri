use crate::error::BackendError;
use crate::traits::{binary_available, target_exec, InstallBackend, InstallContext};
use async_trait::async_trait;
use sindri_core::component::Backend;
use sindri_core::platform::{Os, Platform};

/// Helper that builds `[base_args..., pkg]` and dispatches via `target_exec`.
async fn run_with_pkg(
    ctx: &InstallContext<'_>,
    program: &str,
    base_args: &[&str],
    pkg: &str,
) -> Result<(), BackendError> {
    let mut args: Vec<&str> = base_args.to_vec();
    args.push(pkg);
    target_exec(ctx.target, program, &args).await?;
    Ok(())
}

/// Build a `system_pm`-style backend. Generates the type, the async
/// [`InstallBackend`] impl, and routes through [`target_exec`] so every
/// dispatched command lands on `ctx.target`.
macro_rules! system_pm_backend {
    (
        $name:ident,
        $backend:ident,
        $os:expr,
        $cmd:literal,
        $install_args:expr,
        $remove_args:expr,
        $check_args:expr
    ) => {
        pub struct $name;

        #[async_trait]
        impl InstallBackend for $name {
            fn name(&self) -> Backend {
                Backend::$backend
            }

            fn supports(&self, platform: &Platform) -> bool {
                platform.os == $os && binary_available($cmd)
            }

            async fn install(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
                let pkg = ctx.component.id.name.clone();
                tracing::info!("{}: installing {}", $cmd, pkg);
                if ctx.manifest.is_none() {
                    tracing::debug!(
                        "{}: manifest not yet plumbed; using minimal command for {}",
                        $cmd,
                        ctx.component.id.to_address()
                    );
                }
                run_with_pkg(ctx, $cmd, &$install_args, &pkg).await
            }

            async fn remove(&self, ctx: &InstallContext<'_>) -> Result<(), BackendError> {
                let pkg = ctx.component.id.name.clone();
                tracing::info!("{}: removing {}", $cmd, pkg);
                run_with_pkg(ctx, $cmd, &$remove_args, &pkg).await
            }

            async fn is_installed(&self, ctx: &InstallContext<'_>) -> bool {
                let pkg = ctx.component.id.name.clone();
                let version = ctx.component.version.0.clone();
                let mut args: Vec<&str> = (&$check_args as &[&str]).to_vec();
                args.push(&pkg);
                target_exec(ctx.target, $cmd, &args)
                    .await
                    .map(|(out, _)| out.contains(&version))
                    .unwrap_or(false)
            }
        }
    };
}

system_pm_backend!(
    AptBackend,
    Apt,
    Os::Linux,
    "apt-get",
    ["install", "-y"],
    ["remove", "-y"],
    ["show"]
);

system_pm_backend!(
    DnfBackend,
    Dnf,
    Os::Linux,
    "dnf",
    ["install", "-y"],
    ["remove", "-y"],
    ["info"]
);

system_pm_backend!(
    ZypperBackend,
    Zypper,
    Os::Linux,
    "zypper",
    ["install", "-y"],
    ["remove", "-y"],
    ["info"]
);

system_pm_backend!(
    PacmanBackend,
    Pacman,
    Os::Linux,
    "pacman",
    ["-S", "--noconfirm"],
    ["-R", "--noconfirm"],
    ["-Qi"]
);

system_pm_backend!(
    ApkBackend,
    Apk,
    Os::Linux,
    "apk",
    ["add"],
    ["del"],
    ["info"]
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::test_support::MockTarget;
    use sindri_core::component::ComponentId;
    use sindri_core::lockfile::ResolvedComponent;
    use sindri_core::version::Version;
    use std::collections::HashMap;

    fn comp(backend: Backend, name: &str) -> ResolvedComponent {
        ResolvedComponent {
            id: ComponentId {
                backend: backend.clone(),
                name: name.into(),
                qualifier: None,
            },
            version: Version::new("1.0.0"),
            backend,
            oci_digest: None,
            checksums: HashMap::new(),
            depends_on: vec![],
            manifest: None,
            manifest_digest: None,
        }
    }

    #[tokio::test]
    async fn apt_install_dispatches_apt_get() {
        let mock = MockTarget::new();
        let c = comp(Backend::Apt, "ripgrep");
        let ctx = InstallContext::new(&c, None, &mock);
        AptBackend.install(&ctx).await.unwrap();
        assert_eq!(
            mock.last_call().as_deref(),
            Some("apt-get install -y ripgrep")
        );
    }

    #[tokio::test]
    async fn apt_remove_dispatches_apt_get_remove() {
        let mock = MockTarget::new();
        let c = comp(Backend::Apt, "ripgrep");
        let ctx = InstallContext::new(&c, None, &mock);
        AptBackend.remove(&ctx).await.unwrap();
        assert_eq!(
            mock.last_call().as_deref(),
            Some("apt-get remove -y ripgrep")
        );
    }

    #[tokio::test]
    async fn dnf_install_dispatches_dnf() {
        let mock = MockTarget::new();
        let c = comp(Backend::Dnf, "htop");
        let ctx = InstallContext::new(&c, None, &mock);
        DnfBackend.install(&ctx).await.unwrap();
        assert_eq!(mock.last_call().as_deref(), Some("dnf install -y htop"));
    }

    #[tokio::test]
    async fn pacman_install_uses_noconfirm_flag() {
        let mock = MockTarget::new();
        let c = comp(Backend::Pacman, "neovim");
        let ctx = InstallContext::new(&c, None, &mock);
        PacmanBackend.install(&ctx).await.unwrap();
        assert_eq!(
            mock.last_call().as_deref(),
            Some("pacman -S --noconfirm neovim")
        );
    }

    #[tokio::test]
    async fn apk_install_uses_add() {
        let mock = MockTarget::new();
        let c = comp(Backend::Apk, "git");
        let ctx = InstallContext::new(&c, None, &mock);
        ApkBackend.install(&ctx).await.unwrap();
        assert_eq!(mock.last_call().as_deref(), Some("apk add git"));
    }

    #[tokio::test]
    async fn zypper_install_dispatches_zypper() {
        let mock = MockTarget::new();
        let c = comp(Backend::Zypper, "vim");
        let ctx = InstallContext::new(&c, None, &mock);
        ZypperBackend.install(&ctx).await.unwrap();
        assert_eq!(mock.last_call().as_deref(), Some("zypper install -y vim"));
    }
}
