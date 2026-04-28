//! Per-target-kind classification rules.
//!
//! Each target kind exposes a [`TargetSchema`] describing which fields
//! on a resource are *immutable*. Changing an immutable field forces a
//! destroy+recreate; changing anything else is an in-place update.
//!
//! The lists below come from the provider APIs themselves:
//!
//! | Kind         | Immutable fields                                                                 |
//! |--------------|-----------------------------------------------------------------------------------|
//! | `local`      | (none — only `Noop` ever makes sense; mutations are rejected by the engine)      |
//! | `docker`     | `image`, `name`, `network`, `dind`                                               |
//! | `ssh`        | `host`, `user`, `port`, `jumpHost`                                               |
//! | `e2b`        | `template.id`, `sandbox.timeout`                                                  |
//! | `fly`        | `app`, `organization`, `regions`, `machine.size`                                  |
//! | `kubernetes` | `namespace`, `pod.image`, `storage.storageClass`                                  |
//! | `runpod`     | `gpuTypeId`, `cloudType`, `region`, `imageName`                                   |
//! | `northflank` | `project`, `service`, `image.repository`, `image.tag`                             |
//! | `wsl`        | `distribution`, `name`                                                            |
//! | `devpod-*`   | `provider`, `region`, `instanceType`                                              |
//! | *plugin*     | (none by default — plugins opt in to immutability via their own classifier)      |
//!
//! These match the destroy-and-recreate semantics enforced by the
//! upstream APIs (you can't move a Fly Machine across regions; you
//! can't change a RunPod's GPU type without re-provisioning, etc.).

use std::collections::BTreeSet;

/// Describes mutability rules for a target kind.
///
/// Implementations declare an *immutable-field* set; everything else is
/// considered in-place mutable. A kind that should reject all mutations
/// (e.g. `local`) returns `accepts_mutations() == false`.
pub trait TargetSchema: Send + Sync {
    /// Target kind string (matches `Target::kind()`).
    fn kind(&self) -> &'static str;

    /// Sorted list of dotted field paths that, when changed, force a
    /// destroy+recreate. Returning `&[]` means *all* fields are
    /// in-place mutable.
    fn immutable_fields(&self) -> &'static [&'static str];

    /// Whether the kind accepts mutations at all. `local` returns
    /// `false`; the engine then classifies any difference as Noop and
    /// emits a warning.
    fn accepts_mutations(&self) -> bool {
        true
    }
}

/// Lookup helper used by the CLI — returns the built-in schema for
/// the kind, falling back to [`PluginSchema`] for unknown kinds.
pub fn schema_for_kind(kind: &str) -> Box<dyn TargetSchema> {
    match kind {
        "local" => Box::new(LocalSchema),
        "docker" => Box::new(DockerSchema),
        "ssh" => Box::new(SshSchema),
        "e2b" => Box::new(E2bSchema),
        "fly" => Box::new(FlySchema),
        "kubernetes" | "k8s" => Box::new(K8sSchema),
        "runpod" => Box::new(RunPodSchema),
        "northflank" => Box::new(NorthflankSchema),
        "wsl" => Box::new(WslSchema),
        "devpod-aws"
        | "devpod-gcp"
        | "devpod-azure"
        | "devpod-digitalocean"
        | "devpod-k8s"
        | "devpod-ssh"
        | "devpod-docker" => Box::new(DevPodSchema),
        _ => Box::new(PluginSchema),
    }
}

// ─── Built-in schemas ────────────────────────────────────────────────

pub struct LocalSchema;
impl TargetSchema for LocalSchema {
    fn kind(&self) -> &'static str {
        "local"
    }
    fn immutable_fields(&self) -> &'static [&'static str] {
        &[]
    }
    fn accepts_mutations(&self) -> bool {
        false
    }
}

pub struct DockerSchema;
impl TargetSchema for DockerSchema {
    fn kind(&self) -> &'static str {
        "docker"
    }
    fn immutable_fields(&self) -> &'static [&'static str] {
        &["image", "name", "network", "dind"]
    }
}

pub struct SshSchema;
impl TargetSchema for SshSchema {
    fn kind(&self) -> &'static str {
        "ssh"
    }
    fn immutable_fields(&self) -> &'static [&'static str] {
        &["host", "user", "port", "jumpHost"]
    }
}

pub struct E2bSchema;
impl TargetSchema for E2bSchema {
    fn kind(&self) -> &'static str {
        "e2b"
    }
    fn immutable_fields(&self) -> &'static [&'static str] {
        &["template.id", "sandbox.timeout"]
    }
}

pub struct FlySchema;
impl TargetSchema for FlySchema {
    fn kind(&self) -> &'static str {
        "fly"
    }
    fn immutable_fields(&self) -> &'static [&'static str] {
        &["app", "organization", "regions", "machine.size"]
    }
}

pub struct K8sSchema;
impl TargetSchema for K8sSchema {
    fn kind(&self) -> &'static str {
        "kubernetes"
    }
    fn immutable_fields(&self) -> &'static [&'static str] {
        &["namespace", "pod.image", "storage.storageClass"]
    }
}

pub struct RunPodSchema;
impl TargetSchema for RunPodSchema {
    fn kind(&self) -> &'static str {
        "runpod"
    }
    fn immutable_fields(&self) -> &'static [&'static str] {
        &["gpuTypeId", "cloudType", "region", "imageName"]
    }
}

pub struct NorthflankSchema;
impl TargetSchema for NorthflankSchema {
    fn kind(&self) -> &'static str {
        "northflank"
    }
    fn immutable_fields(&self) -> &'static [&'static str] {
        &["project", "service", "image.repository", "image.tag"]
    }
}

pub struct WslSchema;
impl TargetSchema for WslSchema {
    fn kind(&self) -> &'static str {
        "wsl"
    }
    fn immutable_fields(&self) -> &'static [&'static str] {
        &["distribution", "name"]
    }
}

pub struct DevPodSchema;
impl TargetSchema for DevPodSchema {
    fn kind(&self) -> &'static str {
        "devpod"
    }
    fn immutable_fields(&self) -> &'static [&'static str] {
        &["provider", "region", "instanceType"]
    }
}

/// Permissive default for plugin-supplied target kinds. All field
/// changes are classified as in-place updates; the plugin is free to
/// reject the update at apply time.
pub struct PluginSchema;
impl TargetSchema for PluginSchema {
    fn kind(&self) -> &'static str {
        "plugin"
    }
    fn immutable_fields(&self) -> &'static [&'static str] {
        &[]
    }
}

/// Convenience: collect the immutable-field set as a `BTreeSet` for
/// fast membership checks.
pub(crate) fn immutable_set(schema: &dyn TargetSchema) -> BTreeSet<String> {
    schema
        .immutable_fields()
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_rejects_mutations() {
        assert!(!LocalSchema.accepts_mutations());
        assert!(LocalSchema.immutable_fields().is_empty());
    }

    #[test]
    fn docker_immutable_fields_include_image() {
        let im = immutable_set(&DockerSchema);
        assert!(im.contains("image"));
        assert!(im.contains("network"));
    }

    #[test]
    fn runpod_immutable_fields_include_gpu_and_region() {
        let im = immutable_set(&RunPodSchema);
        assert!(im.contains("gpuTypeId"));
        assert!(im.contains("region"));
    }

    #[test]
    fn northflank_immutable_fields_include_project() {
        let im = immutable_set(&NorthflankSchema);
        assert!(im.contains("project"));
        assert!(im.contains("service"));
    }

    #[test]
    fn fly_immutable_fields_include_regions() {
        let im = immutable_set(&FlySchema);
        assert!(im.contains("regions"));
        assert!(im.contains("machine.size"));
    }

    #[test]
    fn k8s_alias_maps_to_kubernetes_schema() {
        let s = schema_for_kind("k8s");
        assert_eq!(s.kind(), "kubernetes");
    }

    #[test]
    fn unknown_kind_falls_back_to_plugin_schema() {
        let s = schema_for_kind("totally-novel-kind");
        assert_eq!(s.kind(), "plugin");
        assert!(s.immutable_fields().is_empty());
    }

    #[test]
    fn devpod_aws_uses_devpod_schema() {
        let s = schema_for_kind("devpod-aws");
        assert_eq!(s.kind(), "devpod");
    }
}
