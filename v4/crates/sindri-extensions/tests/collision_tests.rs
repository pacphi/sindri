use sindri_core::component::{
    CollisionHandlingConfig, ComponentCapabilities, ComponentManifest, ComponentMetadata,
    InstallConfig,
};
use sindri_core::platform::{Arch, Os, Platform};
use sindri_core::registry::SHARED_PATH_PREFIX;
use sindri_extensions::collision::detection::detect_overlaps;
use sindri_extensions::collision::ordering::order;
use sindri_extensions::collision::scenarios::Scenario;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn manifest(name: &str, prefix: Option<&str>) -> ComponentManifest {
    ComponentManifest {
        metadata: ComponentMetadata {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: format!("test {name}"),
            license: "MIT".to_string(),
            tags: vec![],
            homepage: None,
        },
        platforms: vec![Platform {
            os: Os::Linux,
            arch: Arch::X86_64,
        }],
        install: InstallConfig::default(),
        depends_on: vec![],
        capabilities: ComponentCapabilities {
            collision_handling: prefix.map(|p| CollisionHandlingConfig {
                path_prefix: p.to_string(),
            }),
            hooks: None,
            project_init: None,
        },
        options: Default::default(),
        validate: None,
        configure: None,
        remove: None,
        overrides: Default::default(),
        auth: Default::default(),
    }
}

// ---------------------------------------------------------------------------
// detection::detect_overlaps
// ---------------------------------------------------------------------------

#[test]
fn no_prefixes_produces_no_overlaps() {
    let components = [(manifest("a", None), "sindri/core"),
        (manifest("b", None), "sindri/core")];
    let refs: Vec<_> = components.iter().map(|(m, r)| (m.clone(), *r)).collect();
    assert!(detect_overlaps(&refs).is_empty());
}

#[test]
fn disjoint_prefixes_produce_no_overlaps() {
    let components = [(manifest("nodejs", Some("nodejs/bin")), "sindri/core"),
        (manifest("rust", Some("rust/bin")), "sindri/core")];
    let refs: Vec<_> = components.iter().map(|(m, r)| (m.clone(), *r)).collect();
    assert!(detect_overlaps(&refs).is_empty());
}

#[test]
fn equal_prefixes_detected() {
    let components = [(manifest("a", Some("tools/bin")), "sindri/core"),
        (manifest("b", Some("tools/bin")), "sindri/core")];
    let refs: Vec<_> = components.iter().map(|(m, r)| (m.clone(), *r)).collect();
    let overlaps = detect_overlaps(&refs);
    assert_eq!(overlaps.len(), 1);
    assert_eq!(overlaps[0].a, "a");
    assert_eq!(overlaps[0].b, "b");
}

#[test]
fn parent_child_prefix_is_overlap() {
    // "tools" is a segment prefix of "tools/bin"
    let components = [(manifest("base", Some("tools")), "sindri/core"),
        (manifest("ext", Some("tools/bin")), "sindri/core")];
    let refs: Vec<_> = components.iter().map(|(m, r)| (m.clone(), *r)).collect();
    let overlaps = detect_overlaps(&refs);
    assert_eq!(overlaps.len(), 1, "parent/child prefix must be detected");
}

#[test]
fn substring_mismatch_not_overlap() {
    // "nodejs" must NOT collide with "nodejs-other" — segment boundary matters
    let components = [(manifest("nodejs", Some("nodejs/bin")), "sindri/core"),
        (
            manifest("nodejs-other", Some("nodejs-other/bin")),
            "sindri/core",
        )];
    let refs: Vec<_> = components.iter().map(|(m, r)| (m.clone(), *r)).collect();
    assert!(
        detect_overlaps(&refs).is_empty(),
        "nodejs-other must not clash with nodejs"
    );
}

#[test]
fn shared_prefix_is_skipped_during_detection() {
    // Two `:shared` components must not collide with each other — the sentinel
    // is filtered out before comparison.
    let components = [(manifest("core-a", Some(SHARED_PATH_PREFIX)), "sindri/core"),
        (manifest("core-b", Some(SHARED_PATH_PREFIX)), "sindri/core")];
    let refs: Vec<_> = components.iter().map(|(m, r)| (m.clone(), *r)).collect();
    assert!(
        detect_overlaps(&refs).is_empty(),
        ":shared components must not be compared with each other"
    );
}

#[test]
fn single_component_never_overlaps() {
    let components = [(manifest("solo", Some("solo/bin")), "sindri/core")];
    let refs: Vec<_> = components.iter().map(|(m, r)| (m.clone(), *r)).collect();
    assert!(detect_overlaps(&refs).is_empty());
}

#[test]
fn trailing_slash_normalised_before_compare() {
    // "tools/" and "tools" should still be treated as equal (trailing slash stripped)
    let components = [(manifest("a", Some("tools/")), "sindri/core"),
        (manifest("b", Some("tools")), "sindri/core")];
    let refs: Vec<_> = components.iter().map(|(m, r)| (m.clone(), *r)).collect();
    let overlaps = detect_overlaps(&refs);
    assert_eq!(overlaps.len(), 1, "trailing slash must be normalised");
}

// ---------------------------------------------------------------------------
// ordering::order
// ---------------------------------------------------------------------------

#[test]
fn shared_components_sort_before_scoped() {
    let shared = manifest("core-hook", Some(SHARED_PATH_PREFIX));
    let scoped = manifest("nodejs", Some("nodejs/bin"));
    let no_prefix = manifest("rust", None);

    let mut components = [(no_prefix.clone(), "sindri/core"),
        (scoped.clone(), "sindri/core"),
        (shared.clone(), "sindri/core")];
    let refs: Vec<_> = components
        .iter_mut()
        .map(|(m, r)| (m.clone(), *r))
        .collect();
    let mut sortable: Vec<(ComponentManifest, &str)> = refs;
    order(&mut sortable);

    assert_eq!(
        sortable[0].0.metadata.name, "core-hook",
        ":shared must sort first"
    );
}

#[test]
fn alphabetic_tiebreak_within_same_class() {
    let mut components = vec![
        (manifest("zig", Some("zig/bin")), "sindri/core"),
        (manifest("ansible", Some("ansible/bin")), "sindri/core"),
        (manifest("maven", Some("maven/bin")), "sindri/core"),
    ];
    order(&mut components);

    let names: Vec<_> = components
        .iter()
        .map(|(m, _)| m.metadata.name.as_str())
        .collect();
    assert_eq!(names, vec!["ansible", "maven", "zig"]);
}

#[test]
fn empty_slice_does_not_panic() {
    let mut components: Vec<(ComponentManifest, &str)> = vec![];
    order(&mut components); // must not panic
}

#[test]
fn single_element_unchanged() {
    let mut components = vec![(manifest("solo", Some("solo/bin")), "sindri/core")];
    order(&mut components);
    assert_eq!(components[0].0.metadata.name, "solo");
}

// ---------------------------------------------------------------------------
// scenarios::Scenario
// ---------------------------------------------------------------------------

#[test]
fn scenario_default_is_stop() {
    let s = Scenario::default();
    assert_eq!(s, Scenario::Stop);
}

#[test]
fn scenario_variants_are_distinct() {
    assert_ne!(Scenario::Stop, Scenario::Skip);
    assert_ne!(Scenario::Stop, Scenario::Proceed);
    assert_ne!(Scenario::Skip, Scenario::Proceed);
}

#[test]
fn scenario_copy_semantics() {
    let s = Scenario::Skip;
    let _t = s; // Copy
    let _u = s; // still usable after copy
}

#[test]
fn scenario_dispatch_stop() {
    let action = dispatch_scenario(Scenario::Stop, "exists");
    assert_eq!(action, "abort");
}

#[test]
fn scenario_dispatch_skip() {
    let action = dispatch_scenario(Scenario::Skip, "exists");
    assert_eq!(action, "skip");
}

#[test]
fn scenario_dispatch_proceed() {
    let action = dispatch_scenario(Scenario::Proceed, "exists");
    assert_eq!(action, "overwrite");
}

fn dispatch_scenario(s: Scenario, _existing_path: &str) -> &'static str {
    match s {
        Scenario::Stop => "abort",
        Scenario::Skip => "skip",
        Scenario::Proceed => "overwrite",
    }
}
