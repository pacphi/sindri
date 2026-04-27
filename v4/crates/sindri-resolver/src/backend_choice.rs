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
