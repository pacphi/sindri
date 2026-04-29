//! Integration tests for the convergence engine (Wave 5E, audit D2).
//!
//! Drives a fake Applier through a full plan that contains a Create,
//! an InPlaceUpdate, a Destroy, and a DestroyAndRecreate, and asserts
//! on:
//!
//! * the rendered plan summary,
//! * the lockfile written after apply,
//! * the destructive prompt firing exactly once when there are
//!   destructive entries (and not at all when the plan is purely
//!   additive),
//! * `--auto-approve` skipping the prompt.

use serde_json::{json, Value};
use sindri_targets::convergence::{
    apply_plan, build_plan, render_plan, write_lock_atomic, AlwaysNoConfirm, AlwaysYesConfirm,
    Applier, DockerSchema, InfraDocument, InfraLock, RenderOptions, ResourceState, ScriptedConfirm,
};
use sindri_targets::error::TargetError;
use std::collections::BTreeMap;
use tempfile::tempdir;

/// Fake provider Applier that records every call. Stamps each created
/// resource with a deterministic `id` so we can verify lock-file
/// contents.
#[derive(Default)]
struct RecordingApplier {
    pub created: Vec<String>,
    pub destroyed: Vec<String>,
    pub updated: Vec<String>,
    pub next_id: u32,
}

impl Applier for RecordingApplier {
    fn create(&mut self, name: &str, desired: &Value) -> Result<ResourceState, TargetError> {
        self.created.push(name.to_string());
        self.next_id += 1;
        let mut state = desired.clone();
        if let Some(map) = state.as_object_mut() {
            map.insert("id".into(), json!(format!("res-{}-{}", name, self.next_id)));
        }
        Ok(state)
    }

    fn destroy(&mut self, name: &str, _recorded: &ResourceState) -> Result<(), TargetError> {
        self.destroyed.push(name.to_string());
        Ok(())
    }

    fn update_in_place(
        &mut self,
        name: &str,
        recorded: &ResourceState,
        desired: &Value,
    ) -> Result<ResourceState, TargetError> {
        self.updated.push(name.to_string());
        // Carry over the recorded `id` field, then overlay desired
        // mutable fields.
        let mut new_state = desired.clone();
        if let (Some(new_map), Some(rec_map)) = (new_state.as_object_mut(), recorded.as_object()) {
            if let Some(id) = rec_map.get("id") {
                new_map.insert("id".into(), id.clone());
            }
        }
        Ok(new_state)
    }
}

fn make_desired() -> InfraDocument {
    let mut resources = BTreeMap::new();
    // Existing — image unchanged, env changed → InPlaceUpdate
    resources.insert(
        "web".to_string(),
        json!({"image": "ghcr.io/example/web:1", "env": {"LOG": "debug"}}),
    );
    // New — Create
    resources.insert(
        "worker".to_string(),
        json!({"image": "ghcr.io/example/worker:1"}),
    );
    // Existing — image changed (immutable) → DestroyAndRecreate
    resources.insert("db".to_string(), json!({"image": "postgres:16"}));
    // (note: `cache` is in recorded but not desired → Destroy)
    InfraDocument {
        kind: "docker".to_string(),
        resources,
    }
}

fn make_recorded() -> InfraDocument {
    let mut resources = BTreeMap::new();
    resources.insert(
        "web".to_string(),
        json!({"image": "ghcr.io/example/web:1", "env": {"LOG": "info"}, "id": "res-web-old"}),
    );
    resources.insert(
        "cache".to_string(),
        json!({"image": "redis:7", "id": "res-cache-old"}),
    );
    resources.insert(
        "db".to_string(),
        json!({"image": "postgres:15", "id": "res-db-old"}),
    );
    InfraDocument {
        kind: "docker".to_string(),
        resources,
    }
}

#[test]
fn full_plan_renders_all_action_classes() {
    let desired = make_desired();
    let recorded = make_recorded();
    let plan = build_plan("edge", "docker", &desired, &recorded, &DockerSchema);

    let counts = plan.counts();
    assert_eq!(counts.create, 1, "one Create (worker)");
    assert_eq!(counts.in_place, 1, "one InPlaceUpdate (web env)");
    assert_eq!(counts.destroy, 1, "one Destroy (cache)");
    assert_eq!(counts.recreate, 1, "one DestroyAndRecreate (db image)");

    let rendered = render_plan(&plan, RenderOptions { color: false });
    // Summary line must look like Terraform's.
    assert!(
        rendered.contains("Plan: 1 to add, 1 to change, 1 to destroy, 1 to recreate, 0 unchanged"),
        "got: {}",
        rendered
    );
    // Glyphs for each entry kind.
    assert!(rendered.contains("+ worker"));
    assert!(rendered.contains("~ web"));
    assert!(rendered.contains("- cache"));
    assert!(rendered.contains("± db"));
    // Immutable change is annotated.
    assert!(
        rendered.contains("image (immutable — forces recreate)"),
        "got: {}",
        rendered
    );
}

#[test]
fn auto_approve_applies_full_plan_and_writes_lockfile() {
    let dir = tempdir().unwrap();
    let lock_path = dir.path().join("sindri.edge.infra.lock");

    let desired = make_desired();
    let recorded = make_recorded();
    let plan = build_plan("edge", "docker", &desired, &recorded, &DockerSchema);

    let mut lock = InfraLock::new("edge", "docker");
    lock.resources = recorded.resources.clone();

    let mut applier = RecordingApplier::default();
    let mut confirm = AlwaysYesConfirm;
    let (new_lock, outcome) =
        apply_plan(&plan, &lock, &mut applier, &mut confirm, true).expect("apply ok");

    // Every non-noop entry was applied.
    assert_eq!(outcome.applied, 4);
    assert!(!outcome.destructive_aborted);

    // Applier saw exactly the right calls.
    assert_eq!(applier.created.len(), 2, "worker + db (recreate)");
    assert!(applier.created.contains(&"worker".to_string()));
    assert!(applier.created.contains(&"db".to_string()));
    assert_eq!(applier.destroyed.len(), 2, "cache + db (recreate)");
    assert!(applier.destroyed.contains(&"cache".to_string()));
    assert!(applier.destroyed.contains(&"db".to_string()));
    assert_eq!(applier.updated, vec!["web".to_string()]);

    // Persist + read back.
    write_lock_atomic(&lock_path, &new_lock).unwrap();
    let read_back = InfraLock::read(&lock_path, "edge", "docker").unwrap();

    assert!(read_back.resources.contains_key("web"));
    assert!(read_back.resources.contains_key("worker"));
    assert!(read_back.resources.contains_key("db"));
    assert!(
        !read_back.resources.contains_key("cache"),
        "destroyed resource should not be in the lock"
    );

    // db got a fresh id because it was recreated.
    let db_id = read_back
        .resources
        .get("db")
        .unwrap()
        .get("id")
        .unwrap()
        .as_str()
        .unwrap();
    assert!(db_id.starts_with("res-db-"));
    assert_ne!(db_id, "res-db-old");
}

#[test]
fn destructive_prompt_fires_when_not_auto_approved() {
    let desired = make_desired();
    let recorded = make_recorded();
    let plan = build_plan("edge", "docker", &desired, &recorded, &DockerSchema);

    let mut lock = InfraLock::new("edge", "docker");
    lock.resources = recorded.resources.clone();

    let mut applier = RecordingApplier::default();
    let mut confirm = ScriptedConfirm::new(vec![true]);

    let (new_lock, outcome) =
        apply_plan(&plan, &lock, &mut applier, &mut confirm, false).expect("apply ok");

    // Prompt fired exactly once for the whole plan.
    assert_eq!(confirm.prompts_seen.len(), 1);
    assert!(
        confirm.prompts_seen[0].contains("destructive"),
        "got: {:?}",
        confirm.prompts_seen
    );
    assert!(!outcome.destructive_aborted);
    assert!(new_lock.resources.contains_key("worker"));
}

#[test]
fn declined_prompt_aborts_without_destroying() {
    let desired = make_desired();
    let recorded = make_recorded();
    let plan = build_plan("edge", "docker", &desired, &recorded, &DockerSchema);

    let mut lock = InfraLock::new("edge", "docker");
    lock.resources = recorded.resources.clone();

    let mut applier = RecordingApplier::default();
    let mut confirm = AlwaysNoConfirm;

    let (new_lock, outcome) =
        apply_plan(&plan, &lock, &mut applier, &mut confirm, false).expect("apply ok");

    assert!(outcome.destructive_aborted);
    assert!(applier.created.is_empty());
    assert!(applier.destroyed.is_empty());
    assert!(applier.updated.is_empty());
    // Lock unchanged when aborted.
    assert_eq!(new_lock.resources.len(), recorded.resources.len());
}

#[test]
fn additive_only_plan_skips_prompt() {
    // Empty recorded → only Creates → no destruction → no prompt.
    let desired = make_desired();
    let recorded = InfraDocument {
        kind: "docker".to_string(),
        resources: BTreeMap::new(),
    };
    let plan = build_plan("edge", "docker", &desired, &recorded, &DockerSchema);

    let lock = InfraLock::new("edge", "docker");
    let mut applier = RecordingApplier::default();
    let mut confirm = ScriptedConfirm::new(vec![]);

    let (_new_lock, outcome) =
        apply_plan(&plan, &lock_clone(&lock), &mut applier, &mut confirm, false).expect("apply ok");

    assert!(
        confirm.prompts_seen.is_empty(),
        "no prompt for additive-only plan"
    );
    assert_eq!(outcome.applied, 3);
    assert_eq!(applier.created.len(), 3);
}

fn lock_clone(l: &InfraLock) -> InfraLock {
    InfraLock {
        target_name: l.target_name.clone(),
        kind: l.kind.clone(),
        resources: l.resources.clone(),
    }
}

#[test]
fn schema_for_kind_runpod_classifies_region_change_as_destroy_recreate() {
    // Reach into the public schema_for_kind to make sure the CLI's
    // dynamic dispatch path matches the static unit-test path.
    let schema = sindri_targets::convergence::schema_for_kind("runpod");
    assert_eq!(schema.kind(), "runpod");

    let mut desired = BTreeMap::new();
    desired.insert(
        "pod".to_string(),
        json!({"gpuTypeId": "A100", "region": "EU-CENTRAL"}),
    );
    let mut recorded = BTreeMap::new();
    recorded.insert(
        "pod".to_string(),
        json!({"gpuTypeId": "A100", "region": "US-WEST"}),
    );
    let d = InfraDocument {
        kind: "runpod".to_string(),
        resources: desired,
    };
    let r = InfraDocument {
        kind: "runpod".to_string(),
        resources: recorded,
    };
    let plan = build_plan("gpu", "runpod", &d, &r, schema.as_ref());
    let counts = plan.counts();
    assert_eq!(counts.recreate, 1);
}
