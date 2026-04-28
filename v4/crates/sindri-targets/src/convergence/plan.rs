//! Plan classifier — produces a Terraform-style [`Plan`] from desired
//! and recorded infra documents.

use super::lock::InfraDocument;
use super::schema::{immutable_set, TargetSchema};
use serde_json::Value;
use std::collections::BTreeSet;

/// Classification of a single resource diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Recorded == desired. Nothing to do.
    Noop,
    /// Resource only in desired — must be created.
    Create,
    /// Resource only in recorded — must be destroyed.
    Destroy,
    /// Resource exists on both sides; only mutable fields changed.
    InPlaceUpdate { changed_fields: Vec<String> },
    /// Resource exists on both sides; at least one immutable field
    /// changed — must be destroyed and recreated.
    DestroyAndRecreate { immutable_changes: Vec<String> },
}

impl Action {
    /// True when applying this action removes infrastructure.
    pub fn is_destructive(&self) -> bool {
        matches!(self, Action::Destroy | Action::DestroyAndRecreate { .. })
    }

    /// Single-character glyph used in [`render_plan`](super::render::render_plan).
    pub fn glyph(&self) -> &'static str {
        match self {
            Action::Noop => " ",
            Action::Create => "+",
            Action::Destroy => "-",
            Action::InPlaceUpdate { .. } => "~",
            Action::DestroyAndRecreate { .. } => "±",
        }
    }
}

/// One row in a [`Plan`].
#[derive(Debug, Clone)]
pub struct PlanEntry {
    pub name: String,
    pub action: Action,
    pub desired: Option<Value>,
    pub recorded: Option<Value>,
}

/// Aggregate plan covering every resource referenced in either
/// document.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub target_name: String,
    pub kind: String,
    pub entries: Vec<PlanEntry>,
}

impl Plan {
    /// Number of entries by action class.
    pub fn counts(&self) -> PlanCounts {
        let mut c = PlanCounts::default();
        for e in &self.entries {
            match e.action {
                Action::Noop => c.noop += 1,
                Action::Create => c.create += 1,
                Action::Destroy => c.destroy += 1,
                Action::InPlaceUpdate { .. } => c.in_place += 1,
                Action::DestroyAndRecreate { .. } => c.recreate += 1,
            }
        }
        c
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PlanCounts {
    pub noop: usize,
    pub create: usize,
    pub destroy: usize,
    pub in_place: usize,
    pub recreate: usize,
}

/// Classify a single resource — exposed for unit tests.
pub fn classify_resource(
    desired: Option<&Value>,
    recorded: Option<&Value>,
    schema: &dyn TargetSchema,
) -> Action {
    match (desired, recorded) {
        (None, None) => Action::Noop,
        (Some(_), None) => Action::Create,
        (None, Some(_)) => Action::Destroy,
        (Some(d), Some(r)) => {
            if d == r {
                return Action::Noop;
            }
            if !schema.accepts_mutations() {
                // Local target etc. — never mutate. Treat as Noop with
                // a render-time warning.
                return Action::Noop;
            }
            let changed = changed_paths(d, r, "");
            if changed.is_empty() {
                return Action::Noop;
            }
            let immutable = immutable_set(schema);
            let immutable_changes: Vec<String> = changed
                .iter()
                .filter(|p| path_matches_any(p, &immutable))
                .cloned()
                .collect();
            if !immutable_changes.is_empty() {
                Action::DestroyAndRecreate { immutable_changes }
            } else {
                Action::InPlaceUpdate {
                    changed_fields: changed,
                }
            }
        }
    }
}

/// Build a plan covering every resource named in either document.
pub fn build_plan(
    target_name: &str,
    kind: &str,
    desired: &InfraDocument,
    recorded: &InfraDocument,
    schema: &dyn TargetSchema,
) -> Plan {
    let mut names: BTreeSet<&String> = BTreeSet::new();
    names.extend(desired.resources.keys());
    names.extend(recorded.resources.keys());

    let entries: Vec<PlanEntry> = names
        .into_iter()
        .map(|name| {
            let d = desired.resources.get(name);
            let r = recorded.resources.get(name);
            let action = classify_resource(d, r, schema);
            PlanEntry {
                name: name.clone(),
                action,
                desired: d.cloned(),
                recorded: r.cloned(),
            }
        })
        .collect();

    Plan {
        target_name: target_name.to_string(),
        kind: kind.to_string(),
        entries,
    }
}

/// Walk two JSON values and collect the dotted paths of every leaf
/// that differs. Object keys are joined with `.`; array elements use
/// `[i]` indexing.
fn changed_paths(a: &Value, b: &Value, prefix: &str) -> Vec<String> {
    if a == b {
        return Vec::new();
    }
    match (a, b) {
        (Value::Object(am), Value::Object(bm)) => {
            let mut keys: BTreeSet<&String> = BTreeSet::new();
            keys.extend(am.keys());
            keys.extend(bm.keys());
            let mut out = Vec::new();
            for k in keys {
                let next = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                let av = am.get(k).unwrap_or(&Value::Null);
                let bv = bm.get(k).unwrap_or(&Value::Null);
                out.extend(changed_paths(av, bv, &next));
            }
            out
        }
        (Value::Array(av), Value::Array(bv)) => {
            let max = av.len().max(bv.len());
            let mut out = Vec::new();
            for i in 0..max {
                let next = format!("{}[{}]", prefix, i);
                let a_i = av.get(i).unwrap_or(&Value::Null);
                let b_i = bv.get(i).unwrap_or(&Value::Null);
                out.extend(changed_paths(a_i, b_i, &next));
            }
            out
        }
        _ => {
            if prefix.is_empty() {
                vec!["<root>".to_string()]
            } else {
                vec![prefix.to_string()]
            }
        }
    }
}

/// `path` matches `pattern` if it equals the pattern OR begins with
/// `pattern.` (so `regions[0]` matches `regions`, and
/// `pod.image.tag` matches `pod.image`).
fn path_matches_any(path: &str, patterns: &BTreeSet<String>) -> bool {
    for pat in patterns {
        if path == pat {
            return true;
        }
        if let Some(rest) = path.strip_prefix(pat) {
            if rest.starts_with('.') || rest.starts_with('[') {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convergence::schema::{
        DockerSchema, FlySchema, LocalSchema, NorthflankSchema, RunPodSchema,
    };
    use serde_json::json;

    #[test]
    fn classify_create_when_only_desired() {
        let d = json!({"image": "ubuntu:24.04"});
        let action = classify_resource(Some(&d), None, &DockerSchema);
        assert!(matches!(action, Action::Create));
    }

    #[test]
    fn classify_destroy_when_only_recorded() {
        let r = json!({"image": "ubuntu:24.04"});
        let action = classify_resource(None, Some(&r), &DockerSchema);
        assert!(matches!(action, Action::Destroy));
    }

    #[test]
    fn classify_noop_when_equal() {
        let v = json!({"image": "ubuntu:24.04"});
        let action = classify_resource(Some(&v), Some(&v), &DockerSchema);
        assert!(matches!(action, Action::Noop));
    }

    #[test]
    fn classify_in_place_when_only_mutable_changes() {
        let d = json!({"image": "ubuntu:24.04", "env": {"LOG": "debug"}});
        let r = json!({"image": "ubuntu:24.04", "env": {"LOG": "info"}});
        let action = classify_resource(Some(&d), Some(&r), &DockerSchema);
        match action {
            Action::InPlaceUpdate { changed_fields } => {
                assert_eq!(changed_fields, vec!["env.LOG".to_string()]);
            }
            other => panic!("expected InPlaceUpdate, got {:?}", other),
        }
    }

    #[test]
    fn classify_destroy_and_recreate_when_immutable_changes() {
        let d = json!({"image": "ubuntu:24.04"});
        let r = json!({"image": "ubuntu:22.04"});
        let action = classify_resource(Some(&d), Some(&r), &DockerSchema);
        match action {
            Action::DestroyAndRecreate { immutable_changes } => {
                assert_eq!(immutable_changes, vec!["image".to_string()]);
            }
            other => panic!("expected DestroyAndRecreate, got {:?}", other),
        }
    }

    #[test]
    fn runpod_region_change_is_destroy_and_recreate() {
        let d = json!({"gpuTypeId": "A100", "region": "US-WEST"});
        let r = json!({"gpuTypeId": "A100", "region": "EU-CENTRAL"});
        let action = classify_resource(Some(&d), Some(&r), &RunPodSchema);
        assert!(matches!(action, Action::DestroyAndRecreate { .. }));
    }

    #[test]
    fn runpod_gpu_type_change_is_destroy_and_recreate() {
        let d = json!({"gpuTypeId": "A100"});
        let r = json!({"gpuTypeId": "RTX4090"});
        let action = classify_resource(Some(&d), Some(&r), &RunPodSchema);
        match action {
            Action::DestroyAndRecreate { immutable_changes } => {
                assert!(immutable_changes.iter().any(|p| p == "gpuTypeId"));
            }
            _ => panic!("expected DestroyAndRecreate"),
        }
    }

    #[test]
    fn northflank_image_tag_change_is_destroy_and_recreate() {
        let d = json!({"project": "p", "service": "s", "image": {"repository": "ghcr.io/x/y", "tag": "v2"}});
        let r = json!({"project": "p", "service": "s", "image": {"repository": "ghcr.io/x/y", "tag": "v1"}});
        let action = classify_resource(Some(&d), Some(&r), &NorthflankSchema);
        assert!(matches!(action, Action::DestroyAndRecreate { .. }));
    }

    #[test]
    fn fly_regions_array_change_is_destroy_and_recreate() {
        let d = json!({"app": "a", "regions": ["sjc", "ord"]});
        let r = json!({"app": "a", "regions": ["sjc"]});
        let action = classify_resource(Some(&d), Some(&r), &FlySchema);
        assert!(matches!(action, Action::DestroyAndRecreate { .. }));
    }

    #[test]
    fn local_target_never_mutates() {
        let d = json!({"root": "/tmp/sindri"});
        let r = json!({"root": "/var/sindri"});
        let action = classify_resource(Some(&d), Some(&r), &LocalSchema);
        assert!(matches!(action, Action::Noop));
    }

    #[test]
    fn glyphs_match_terraform_style() {
        assert_eq!(Action::Create.glyph(), "+");
        assert_eq!(Action::Destroy.glyph(), "-");
        assert_eq!(
            Action::InPlaceUpdate {
                changed_fields: vec![]
            }
            .glyph(),
            "~"
        );
        assert_eq!(
            Action::DestroyAndRecreate {
                immutable_changes: vec![]
            }
            .glyph(),
            "±"
        );
    }

    #[test]
    fn path_matching_handles_nested_immutable_prefix() {
        let mut s = BTreeSet::new();
        s.insert("pod.image".to_string());
        assert!(path_matches_any("pod.image", &s));
        assert!(path_matches_any("pod.image.tag", &s));
        assert!(!path_matches_any("pod.imageX", &s));
    }

    #[test]
    fn path_matching_handles_array_index() {
        let mut s = BTreeSet::new();
        s.insert("regions".to_string());
        assert!(path_matches_any("regions[0]", &s));
        assert!(path_matches_any("regions[3]", &s));
    }
}
