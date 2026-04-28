use sindri_core::component::Backend;
use sindri_core::platform::{Os, Platform};
use sindri_core::registry::ComponentEntry;
use std::str::FromStr;

/// Built-in backend preference chains per OS (ADR-009, ADR-010, Sprint 3)
///
/// macOS: brew > mise > pipx/npm/cargo/go-install > binary > script
/// Linux: mise > apt/dnf/zypper/pacman/apk > binary > script
/// Windows: winget > scoop > mise > binary > script (ps1)
pub fn default_preference(os: &Os) -> Vec<Backend> {
    match os {
        Os::Macos => vec![
            Backend::Brew,
            Backend::Mise,
            Backend::Pipx,
            Backend::Npm,
            Backend::Cargo,
            Backend::GoInstall,
            Backend::Binary,
            Backend::Script,
        ],
        Os::Linux => vec![
            Backend::Mise,
            Backend::Apt,
            Backend::Dnf,
            Backend::Zypper,
            Backend::Pacman,
            Backend::Apk,
            Backend::Pipx,
            Backend::Npm,
            Backend::Cargo,
            Backend::GoInstall,
            Backend::Binary,
            Backend::Script,
        ],
        Os::Windows => vec![
            Backend::Winget,
            Backend::Scoop,
            Backend::Mise,
            Backend::Pipx,
            Backend::Npm,
            Backend::Cargo,
            Backend::GoInstall,
            Backend::Binary,
            Backend::Script,
        ],
    }
}

/// Choose the best backend for a component on the given platform.
/// Returns the backend from the preference chain that is declared in the component's install config.
/// Falls back to the platform default chain if the component has no explicit backend annotation.
pub fn choose_backend(
    entry: &ComponentEntry,
    platform: &Platform,
    user_prefs: Option<&[Backend]>,
) -> Backend {
    let chain: Vec<Backend> = if let Some(prefs) = user_prefs {
        prefs.to_vec()
    } else {
        default_preference(&platform.os)
    };

    // If the entry has an explicit backend (e.g. "mise:nodejs"), honour it
    if let Ok(backend) = entry.backend.parse::<BackendStr>() {
        return backend.into_backend();
    }

    // Otherwise pick first in preference chain (Sprint 4+ adds capability checks)
    chain.into_iter().next().unwrap_or(Backend::Script)
}

/// Explain the backend preference chain for a component
pub fn explain_choice(entry: &ComponentEntry, platform: &Platform) -> String {
    let chain = default_preference(&platform.os);
    let chosen = choose_backend(entry, platform, None);
    let lines = [
        format!("Component: {}:{}", entry.backend, entry.name),
        format!("Platform:  {}", platform.triple()),
        format!(
            "Preference chain: {}",
            chain
                .iter()
                .map(|b| b.as_str())
                .collect::<Vec<_>>()
                .join(" > ")
        ),
        format!("Chosen: {}", chosen.as_str()),
    ];
    lines.join("\n")
}

/// Target-kind-aware backend preference chain (Wave 5F — D18).
///
/// Different deployment targets have different "native" backends. A `local`
/// target uses the host package managers (`brew`/`apt`/`mise`); a `docker` or
/// `kubernetes` target prefers container-image / binary backends because the
/// host's brew/apt isn't reachable from inside the container; an `ssh` target
/// behaves like a remote `local` (we'll still use brew/apt on the remote box,
/// the `mise` chain works there too).
///
/// The chain returned here is *additive on top of the platform default*: if
/// the target kind doesn't override, callers fall back to
/// [`default_preference`]. Returning `None` means "use the platform default."
pub fn target_kind_preference(target_kind: &str, platform: &Platform) -> Option<Vec<Backend>> {
    match target_kind {
        // Local target: identical to platform default.
        "local" => None,

        // Container-style targets — the user-installed package managers on
        // the host are not reachable inside the container, so we prefer
        // tarball / static-binary backends. `mise` is preserved because it
        // installs into `$XDG_DATA_HOME` and works inside containers when
        // available.
        "docker" | "kubernetes" | "k8s" | "fly" | "e2b" | "runpod" | "northflank" => Some(vec![
            Backend::Mise,
            Backend::Binary,
            Backend::Cargo,
            Backend::GoInstall,
            Backend::Npm,
            Backend::Pipx,
            Backend::Script,
        ]),

        // Remote shell targets: same as local default — the remote host has
        // its own package managers. The CLI is responsible for setting
        // `platform` to the remote's platform when the target probes its
        // profile; if it can't, we fall through to the host's defaults.
        "ssh"
        | "wsl"
        | "devpod-aws"
        | "devpod-gcp"
        | "devpod-azure"
        | "devpod-digitalocean"
        | "devpod-k8s"
        | "devpod-ssh"
        | "devpod-docker" => None,

        // Unknown kind (likely a plugin target): be conservative and let the
        // platform default apply. This keeps plugin targets working before
        // they declare their own preference chain.
        _ => {
            tracing::debug!(
                "target kind '{}' has no built-in backend preference; \
                 falling back to platform default for {}",
                target_kind,
                platform.triple()
            );
            None
        }
    }
}

/// Choose the best backend for a component, taking the target's *kind* into
/// account (Wave 5F — D18).
///
/// Wraps [`choose_backend`] but consults [`target_kind_preference`] first.
/// User-supplied preferences (`user_prefs`) still win — both over the target
/// chain and the platform chain — to preserve the override hook.
pub fn choose_backend_for_target(
    entry: &ComponentEntry,
    platform: &Platform,
    target_kind: Option<&str>,
    user_prefs: Option<&[Backend]>,
) -> Backend {
    if user_prefs.is_some() {
        return choose_backend(entry, platform, user_prefs);
    }
    let target_chain = target_kind.and_then(|k| target_kind_preference(k, platform));
    choose_backend(entry, platform, target_chain.as_deref())
}

// Helper: parse the backend string from a registry entry
struct BackendStr(String);

impl BackendStr {
    fn into_backend(self) -> Backend {
        sindri_core::component::Backend::from_str(&self.0).unwrap_or(Backend::Script)
    }
}

impl std::str::FromStr for BackendStr {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if sindri_core::component::Backend::from_str(s).is_ok() {
            Ok(BackendStr(s.to_string()))
        } else {
            Err(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::platform::{Arch, Os, Platform};
    use sindri_core::registry::{ComponentEntry, ComponentKind};

    fn linux_platform() -> Platform {
        Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
        }
    }

    fn macos_platform() -> Platform {
        Platform {
            os: Os::Macos,
            arch: Arch::Aarch64,
        }
    }

    fn entry_no_explicit_backend(name: &str) -> ComponentEntry {
        ComponentEntry {
            name: name.into(),
            // Empty backend means "no explicit annotation"; the chooser will
            // fall through to the preference chain.
            backend: "".into(),
            latest: "1.0.0".into(),
            versions: vec!["1.0.0".into()],
            description: "test".into(),
            kind: ComponentKind::Component,
            oci_ref: format!("ghcr.io/sindri-dev/registry-core/{}:1.0.0", name),
            license: "MIT".into(),
            depends_on: vec![],
        }
    }

    #[test]
    fn target_kind_local_falls_back_to_platform_default() {
        // Wave 5F — D18: a `local` target uses the platform default chain.
        let pf = macos_platform();
        assert!(target_kind_preference("local", &pf).is_none());
        let chosen = choose_backend_for_target(
            &entry_no_explicit_backend("nodejs"),
            &pf,
            Some("local"),
            None,
        );
        // macOS default chain heads with brew.
        assert_eq!(chosen, Backend::Brew);
    }

    #[test]
    fn target_kind_kubernetes_prefers_container_friendly_backends() {
        // Wave 5F — D18: a `k8s` target should NOT pick brew (host-only),
        // even on macOS. Mise is the first container-friendly backend.
        let pf = macos_platform();
        let chain = target_kind_preference("k8s", &pf).expect("k8s overrides default");
        assert!(!chain.contains(&Backend::Brew));
        assert!(!chain.contains(&Backend::Apt));
        assert_eq!(chain.first(), Some(&Backend::Mise));
        let chosen = choose_backend_for_target(
            &entry_no_explicit_backend("kubectl"),
            &pf,
            Some("k8s"),
            None,
        );
        assert_eq!(chosen, Backend::Mise);
    }

    #[test]
    fn target_kind_docker_omits_host_package_managers() {
        // Wave 5F — D18: docker shares the same chain as k8s.
        let pf = linux_platform();
        let chain = target_kind_preference("docker", &pf).expect("docker overrides default");
        for forbidden in [
            Backend::Brew,
            Backend::Apt,
            Backend::Dnf,
            Backend::Pacman,
            Backend::Apk,
            Backend::Zypper,
        ] {
            assert!(
                !chain.contains(&forbidden),
                "docker chain leaked host package manager: {:?}",
                forbidden
            );
        }
    }

    #[test]
    fn target_kind_ssh_inherits_platform_default() {
        // SSH installs onto a remote host that has its own package
        // managers — preserve the platform default.
        let pf = linux_platform();
        assert!(target_kind_preference("ssh", &pf).is_none());
        assert!(target_kind_preference("wsl", &pf).is_none());
    }

    #[test]
    fn target_kind_unknown_falls_back_silently() {
        // Plugin / unknown kinds should not error — they fall back to the
        // platform default. (Plugins can override later via per-target prefs.)
        let pf = linux_platform();
        assert!(target_kind_preference("modal", &pf).is_none());
        assert!(target_kind_preference("lambda-labs", &pf).is_none());
    }

    #[test]
    fn user_prefs_win_over_target_kind() {
        // Even with a docker target, an explicit user preference list wins.
        let pf = macos_platform();
        let prefs = [Backend::Brew, Backend::Script];
        let chosen = choose_backend_for_target(
            &entry_no_explicit_backend("nodejs"),
            &pf,
            Some("docker"),
            Some(&prefs),
        );
        assert_eq!(chosen, Backend::Brew);
    }

    #[test]
    fn explicit_entry_backend_still_honoured_under_target_kind() {
        // If the registry entry pins `backend: "mise"`, that wins regardless
        // of the target chain.
        let pf = macos_platform();
        let mut e = entry_no_explicit_backend("nodejs");
        e.backend = "mise".into();
        let chosen = choose_backend_for_target(&e, &pf, Some("k8s"), None);
        assert_eq!(chosen, Backend::Mise);
    }
}
