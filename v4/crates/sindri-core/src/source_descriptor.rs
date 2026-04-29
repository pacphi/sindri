//! Lockfile-stable [`SourceDescriptor`] projection (DDD-08, ADR-028).
//!
//! Lives in `sindri-core` so the lockfile types can carry it without
//! depending on `sindri-registry`. The full source trait surface
//! (`Source`, `RegistrySource`, …) lives in `sindri-registry::source` and
//! re-exports this type unchanged.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Lockfile-stable projection of a registry source. Captures only what
/// `sindri apply` needs to refetch the same bytes (DDD-08 §"Lockfile
/// descriptor").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SourceDescriptor {
    /// Filesystem-path source. Non-reproducible across machines by design;
    /// `--strict-oci` rejects any lockfile that contains this descriptor.
    LocalPath {
        /// Absolute or repo-relative path the source was rooted at.
        path: PathBuf,
    },
    /// Git source. The `commit_sha` is the resolved sha at lock time, never
    /// the user-supplied ref.
    Git {
        /// Repository URL (HTTPS or SSH).
        url: String,
        /// Resolved commit sha.
        commit_sha: String,
        /// Optional sub-directory inside the repo.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        subdir: Option<PathBuf>,
    },
    /// Production OCI source. The `manifest_digest` lets apply-time fetch
    /// detect a republished tag as drift (DDD-08 §Invariants).
    Oci {
        /// Canonical OCI URL.
        url: String,
        /// Tag the lockfile was resolved against.
        tag: String,
        /// OCI manifest digest. Optional for legacy lockfiles that pre-date
        /// Phase 1.3 and have not been re-resolved.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        manifest_digest: Option<String>,
    },
    /// Local OCI image-layout source.
    LocalOci {
        /// Path to the OCI image layout directory.
        layout_path: PathBuf,
        /// Manifest digest of the artifact selected from the layout.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        manifest_digest: Option<String>,
    },
}

impl SourceDescriptor {
    /// Short kind name for display (`local-path`, `git`, `oci`, `local-oci`).
    pub fn kind(&self) -> &'static str {
        match self {
            SourceDescriptor::LocalPath { .. } => "local-path",
            SourceDescriptor::Git { .. } => "git",
            SourceDescriptor::Oci { .. } => "oci",
            SourceDescriptor::LocalOci { .. } => "local-oci",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_kind_strings() {
        assert_eq!(
            SourceDescriptor::LocalPath {
                path: PathBuf::from("/x")
            }
            .kind(),
            "local-path"
        );
        assert_eq!(
            SourceDescriptor::Oci {
                url: "oci://x/y".into(),
                tag: "1".into(),
                manifest_digest: None,
            }
            .kind(),
            "oci"
        );
    }

    #[test]
    fn descriptor_round_trips_through_json() {
        let d = SourceDescriptor::LocalPath {
            path: PathBuf::from("/tmp/registry"),
        };
        let json = serde_json::to_string(&d).unwrap();
        let parsed: SourceDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, d);
    }
}
