#![allow(dead_code)]

//! Target trait and built-in implementations for sindri v4.
//!
//! Targets replace v3's "providers" (ADR-017). A `Target` is anywhere a
//! component can be installed: the local machine, a Docker container, a
//! remote SSH host, an E2B sandbox, a Fly Machine, a Kubernetes pod, a
//! RunPod GPU pod, a Northflank service, a DevPod workspace, or a WSL
//! distribution. New target kinds can be added without forking sindri
//! via the subprocess-JSON plugin protocol (ADR-019, see [`plugin`]).

pub mod auth;
pub mod cloud;
pub mod convergence;
pub mod docker;
pub mod error;
pub mod local;
pub mod plugin;
pub mod ssh;
pub mod traits;

pub use auth::AuthValue;
pub use cloud::{
    DevPodKind, DevPodTarget, E2bTarget, FlyTarget, KubernetesTarget, NorthflankTarget,
    RunPodTarget, WslTarget,
};
pub use docker::DockerTarget;
pub use error::TargetError;
pub use local::LocalTarget;
pub use plugin::{Handshake, PluginRequest, PluginResponse, PluginTarget, WirePrereqCheck};
pub use ssh::SshTarget;
pub use traits::{PrereqCheck, Target};

use std::path::PathBuf;

/// Resolve `~/.sindri/plugins/<kind>/sindri-target-<kind>` for the host
/// user. Returns `None` if `$HOME` cannot be determined.
pub fn plugin_binary_path(kind: &str) -> Option<PathBuf> {
    let home = dirs_next::home_dir()?;
    Some(
        home.join(".sindri")
            .join("plugins")
            .join(kind)
            .join(format!("sindri-target-{}", kind)),
    )
}

/// Returns true for any built-in target kind. Anything else is dispatched
/// to a plugin (if installed) by [`load_plugin_target`].
pub fn is_builtin_kind(kind: &str) -> bool {
    matches!(
        kind,
        "local"
            | "docker"
            | "ssh"
            | "e2b"
            | "fly"
            | "kubernetes"
            | "k8s"
            | "runpod"
            | "northflank"
            | "wsl"
            | "devpod-aws"
            | "devpod-gcp"
            | "devpod-azure"
            | "devpod-digitalocean"
            | "devpod-k8s"
            | "devpod-ssh"
            | "devpod-docker"
    )
}

/// Try to load a plugin target for `kind` from the user's plugin
/// directory. Returns:
///
/// * `Ok(Some(plugin))` if `~/.sindri/plugins/<kind>/sindri-target-<kind>`
///   exists and is executable.
/// * `Ok(None)` if the kind is a builtin (the caller should construct
///   the matching builtin instead).
/// * `Err(TargetError::Unavailable)` if the kind is unknown and no plugin
///   is installed — the error message tells the user how to install one.
pub fn load_plugin_target(
    name: &str,
    kind: &str,
    config: serde_json::Value,
) -> Result<Option<PluginTarget>, TargetError> {
    if is_builtin_kind(kind) {
        return Ok(None);
    }
    let path = plugin_binary_path(kind).ok_or_else(|| TargetError::Unavailable {
        name: name.to_string(),
        reason: "could not resolve $HOME to look up plugin binary".into(),
    })?;
    if !path.is_file() {
        return Err(TargetError::Unavailable {
            name: name.to_string(),
            reason: format!(
                "no target plugin installed for kind '{}'; try `sindri target plugin install <oci-ref>`",
                kind
            ),
        });
    }
    Ok(Some(PluginTarget::new(name, kind, path, config)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_kinds_recognised() {
        for k in [
            "local",
            "docker",
            "ssh",
            "e2b",
            "fly",
            "kubernetes",
            "runpod",
            "northflank",
            "wsl",
            "devpod-aws",
            "devpod-gcp",
            "devpod-azure",
            "devpod-digitalocean",
            "devpod-k8s",
            "devpod-ssh",
            "devpod-docker",
        ] {
            assert!(is_builtin_kind(k), "expected builtin: {}", k);
        }
        assert!(!is_builtin_kind("modal"));
        assert!(!is_builtin_kind("lambda-labs"));
    }

    #[test]
    fn load_plugin_returns_unavailable_for_unknown_uninstalled() {
        // Use a kind that is virtually guaranteed not to be installed.
        let res = load_plugin_target("x", "this-kind-does-not-exist-zzz", serde_json::json!({}));
        match res {
            Err(TargetError::Unavailable { reason, .. }) => {
                assert!(reason.contains("plugin install"));
            }
            other => panic!("expected Unavailable, got {:?}", other),
        }
    }

    #[test]
    fn load_plugin_returns_none_for_builtin() {
        let res = load_plugin_target("x", "local", serde_json::json!({})).unwrap();
        assert!(res.is_none());
    }
}
