use sindri_core::component::Backend;
use sindri_core::lockfile::ResolvedComponent;
use sindri_core::platform::{Os, Platform};
use crate::error::BackendError;
use crate::traits::{InstallBackend, binary_available, run_command};

macro_rules! system_pm_backend {
    ($name:ident, $backend:ident, $os:expr, $cmd:expr, $install_args:expr, $remove_args:expr, $check_cmd:expr) => {
        pub struct $name;

        impl InstallBackend for $name {
            fn name(&self) -> Backend {
                Backend::$backend
            }

            fn supports(&self, platform: &Platform) -> bool {
                platform.os == $os && binary_available($cmd)
            }

            fn install(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
                let pkg = &comp.id.name;
                tracing::info!("{}: installing {}", $cmd, pkg);
                let mut args = Vec::from($install_args);
                args.push(pkg.as_str());
                run_command($cmd, &args)?;
                Ok(())
            }

            fn remove(&self, comp: &ResolvedComponent) -> Result<(), BackendError> {
                let pkg = &comp.id.name;
                tracing::info!("{}: removing {}", $cmd, pkg);
                let mut args = Vec::from($remove_args);
                args.push(pkg.as_str());
                run_command($cmd, &args)?;
                Ok(())
            }

            fn is_installed(&self, comp: &ResolvedComponent) -> bool {
                let mut args = Vec::from($check_cmd);
                args.push(comp.id.name.as_str());
                run_command($cmd, &args)
                    .map(|(out, _)| out.contains(&comp.version.0))
                    .unwrap_or(false)
            }
        }
    };
}

system_pm_backend!(
    AptBackend, Apt, Os::Linux,
    "apt-get",
    &["install", "-y"],
    &["remove", "-y"],
    &["show"]
);

system_pm_backend!(
    DnfBackend, Dnf, Os::Linux,
    "dnf",
    &["install", "-y"],
    &["remove", "-y"],
    &["info"]
);

system_pm_backend!(
    ZypperBackend, Zypper, Os::Linux,
    "zypper",
    &["install", "-y"],
    &["remove", "-y"],
    &["info"]
);

system_pm_backend!(
    PacmanBackend, Pacman, Os::Linux,
    "pacman",
    &["-S", "--noconfirm"],
    &["-R", "--noconfirm"],
    &["-Qi"]
);

system_pm_backend!(
    ApkBackend, Apk, Os::Linux,
    "apk",
    &["add"],
    &["del"],
    &["info"]
);
