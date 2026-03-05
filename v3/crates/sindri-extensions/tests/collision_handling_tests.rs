//! Integration tests for collision handling during project-init

mod common;

use common::builders::ExtensionBuilder;
use sindri_core::types::{
    AuthProvider, CollisionHandlingConfig, CollisionScenario, ConflictActionType,
    ConflictResourceType, ConflictRule, DetectionMethod, OnConflictAction, ProjectInitCommand,
    ScenarioAction, StateMarkerType, VersionDetection, VersionMarker,
};
use sindri_extensions::collision::{CollisionResolver, InteractivityMode, ProjectInitEntry};
use sindri_extensions::InitOutcome;
use tempfile::TempDir;

/// Build a ProjectInitEntry from an ExtensionBuilder with given priority
fn entry_from_builder(builder: ExtensionBuilder, priority: u32) -> ProjectInitEntry {
    let ext = builder.build();
    ProjectInitEntry {
        extension_name: ext.metadata.name.clone(),
        extension: ext,
        priority,
    }
}

/// Always-true auth checker for tests
fn auth_always_ok(_ext: &sindri_core::types::Extension, _auth: &AuthProvider) -> bool {
    true
}

/// Always-false auth checker for tests
fn auth_always_fail(_ext: &sindri_core::types::Extension, _auth: &AuthProvider) -> bool {
    false
}

fn make_ruflo_collision_config() -> CollisionHandlingConfig {
    CollisionHandlingConfig {
        enabled: true,
        conflict_rules: vec![
            ConflictRule {
                path: "CLAUDE.md".to_string(),
                r#type: ConflictResourceType::File,
                on_conflict: OnConflictAction {
                    action: ConflictActionType::Append,
                    separator: Some("\n\n---\n\n".to_string()),
                    backup_suffix: ".backup".to_string(),
                    backup: false,
                    prompt_options: vec![],
                },
            },
            ConflictRule {
                path: ".claude".to_string(),
                r#type: ConflictResourceType::Directory,
                on_conflict: OnConflictAction {
                    action: ConflictActionType::Merge,
                    separator: None,
                    backup_suffix: ".backup".to_string(),
                    backup: false,
                    prompt_options: vec![],
                },
            },
        ],
        version_markers: vec![VersionMarker {
            path: ".agentic-qe".to_string(),
            r#type: StateMarkerType::Directory,
            version: "agentic-qe".to_string(),
            detection: VersionDetection {
                method: DetectionMethod::DirectoryExists,
                patterns: vec![],
                match_any: false,
                exclude_if: vec![],
            },
        }],
        scenarios: vec![CollisionScenario {
            name: "agentic-qe-coexist".to_string(),
            detected_version: "agentic-qe".to_string(),
            installing_version: "1.0.0".to_string(),
            action: ScenarioAction::Proceed,
            message: "Co-tenant detected".to_string(),
            options: vec![],
        }],
    }
}

fn make_skip_collision_config() -> CollisionHandlingConfig {
    CollisionHandlingConfig {
        enabled: true,
        conflict_rules: vec![],
        version_markers: vec![VersionMarker {
            path: ".already-done".to_string(),
            r#type: StateMarkerType::Directory,
            version: "installed".to_string(),
            detection: VersionDetection {
                method: DetectionMethod::DirectoryExists,
                patterns: vec![],
                match_any: false,
                exclude_if: vec![],
            },
        }],
        scenarios: vec![CollisionScenario {
            name: "already-initialized".to_string(),
            detected_version: "installed".to_string(),
            installing_version: "1.0.0".to_string(),
            action: ScenarioAction::Skip,
            message: "Already initialized".to_string(),
            options: vec![],
        }],
    }
}

fn make_stop_collision_config() -> CollisionHandlingConfig {
    CollisionHandlingConfig {
        enabled: true,
        conflict_rules: vec![],
        version_markers: vec![VersionMarker {
            path: ".legacy".to_string(),
            r#type: StateMarkerType::Directory,
            version: "legacy".to_string(),
            detection: VersionDetection {
                method: DetectionMethod::DirectoryExists,
                patterns: vec![],
                match_any: false,
                exclude_if: vec![],
            },
        }],
        scenarios: vec![CollisionScenario {
            name: "legacy-detected".to_string(),
            detected_version: "legacy".to_string(),
            installing_version: "1.0.0".to_string(),
            action: ScenarioAction::Stop,
            message: "Legacy version detected, manual migration required".to_string(),
            options: vec![],
        }],
    }
}

#[test]
fn test_full_pipeline_ruflo_then_agentic_qe() {
    let tmp = TempDir::new().unwrap();
    // Create workspace files that trigger conflict rules
    std::fs::write(tmp.path().join("CLAUDE.md"), "# Project").unwrap();
    std::fs::create_dir(tmp.path().join(".claude")).unwrap();
    std::fs::create_dir(tmp.path().join(".agentic-qe")).unwrap();

    let ruflo = ExtensionBuilder::new()
        .with_name("ruflo")
        .with_project_init(
            20,
            vec![ProjectInitCommand {
                command: "echo ruflo-init".to_string(),
                description: "Init ruflo".to_string(),
                requires_auth: AuthProvider::None,
                conditional: false,
            }],
        )
        .with_collision_handling(make_ruflo_collision_config());
    let ruflo_entry = entry_from_builder(ruflo, 20);

    let aqe = ExtensionBuilder::new()
        .with_name("agentic-qe")
        .with_project_init(
            50,
            vec![ProjectInitCommand {
                command: "echo aqe-init".to_string(),
                description: "Init AQE".to_string(),
                requires_auth: AuthProvider::None,
                conditional: false,
            }],
        );
    let aqe_entry = entry_from_builder(aqe, 50);

    let resolver =
        CollisionResolver::new(tmp.path().to_path_buf(), InteractivityMode::NonInteractive);
    let results = resolver
        .resolve_and_execute(vec![aqe_entry, ruflo_entry], &auth_always_ok)
        .unwrap();

    // Ruflo should be first (priority 20)
    assert_eq!(results[0].extension_name, "ruflo");
    assert_eq!(results[1].extension_name, "agentic-qe");

    // Ruflo should have executed (agentic-qe-coexist -> proceed)
    match &results[0].outcome {
        InitOutcome::Executed {
            commands_run,
            conflicts_resolved,
        } => {
            assert_eq!(*commands_run, 1);
            assert!(!conflicts_resolved.is_empty());
        }
        other => panic!("Expected Executed, got {:?}", other),
    }
}

#[test]
fn test_full_pipeline_skip_already_initialized() {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir(tmp.path().join(".already-done")).unwrap();

    let ext = ExtensionBuilder::new()
        .with_name("skipper")
        .with_project_init(
            50,
            vec![ProjectInitCommand {
                command: "echo should-not-run".to_string(),
                description: "Should be skipped".to_string(),
                requires_auth: AuthProvider::None,
                conditional: false,
            }],
        )
        .with_collision_handling(make_skip_collision_config());
    let entry = entry_from_builder(ext, 50);

    let resolver =
        CollisionResolver::new(tmp.path().to_path_buf(), InteractivityMode::NonInteractive);
    let results = resolver
        .resolve_and_execute(vec![entry], &auth_always_ok)
        .unwrap();
    assert!(matches!(&results[0].outcome, InitOutcome::Skipped { .. }));
}

#[test]
fn test_full_pipeline_stop_blocks_init() {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir(tmp.path().join(".legacy")).unwrap();

    let ext = ExtensionBuilder::new()
        .with_name("stopper")
        .with_project_init(
            50,
            vec![ProjectInitCommand {
                command: "echo should-not-run".to_string(),
                description: "Should be stopped".to_string(),
                requires_auth: AuthProvider::None,
                conditional: false,
            }],
        )
        .with_collision_handling(make_stop_collision_config());
    let entry = entry_from_builder(ext, 50);

    let resolver =
        CollisionResolver::new(tmp.path().to_path_buf(), InteractivityMode::NonInteractive);
    let results = resolver
        .resolve_and_execute(vec![entry], &auth_always_ok)
        .unwrap();
    assert!(matches!(&results[0].outcome, InitOutcome::Stopped { .. }));
}

#[test]
fn test_full_pipeline_proceed_cotenant() {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir(tmp.path().join(".agentic-qe")).unwrap();

    let ext = ExtensionBuilder::new()
        .with_name("ruflo")
        .with_project_init(
            20,
            vec![ProjectInitCommand {
                command: "echo proceed".to_string(),
                description: "Proceed with cotenant".to_string(),
                requires_auth: AuthProvider::None,
                conditional: false,
            }],
        )
        .with_collision_handling(make_ruflo_collision_config());
    let entry = entry_from_builder(ext, 20);

    let resolver =
        CollisionResolver::new(tmp.path().to_path_buf(), InteractivityMode::NonInteractive);
    let results = resolver
        .resolve_and_execute(vec![entry], &auth_always_ok)
        .unwrap();
    assert!(matches!(&results[0].outcome, InitOutcome::Executed { .. }));
}

#[test]
fn test_full_pipeline_no_collision_handling() {
    let tmp = TempDir::new().unwrap();

    let ext = ExtensionBuilder::new()
        .with_name("simple")
        .with_project_init(
            100,
            vec![ProjectInitCommand {
                command: "echo simple-init".to_string(),
                description: "Simple init".to_string(),
                requires_auth: AuthProvider::None,
                conditional: false,
            }],
        );
    let entry = entry_from_builder(ext, 100);

    let resolver =
        CollisionResolver::new(tmp.path().to_path_buf(), InteractivityMode::NonInteractive);
    let results = resolver
        .resolve_and_execute(vec![entry], &auth_always_ok)
        .unwrap();
    assert!(matches!(
        &results[0].outcome,
        InitOutcome::Executed {
            commands_run: 1,
            ..
        }
    ));
}

#[test]
fn test_full_pipeline_collision_disabled() {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir(tmp.path().join(".already-done")).unwrap();

    let mut config = make_skip_collision_config();
    config.enabled = false; // Disable collision handling

    let ext = ExtensionBuilder::new()
        .with_name("disabled")
        .with_project_init(
            50,
            vec![ProjectInitCommand {
                command: "echo runs-anyway".to_string(),
                description: "Should run despite marker".to_string(),
                requires_auth: AuthProvider::None,
                conditional: false,
            }],
        )
        .with_collision_handling(config);
    let entry = entry_from_builder(ext, 50);

    let resolver =
        CollisionResolver::new(tmp.path().to_path_buf(), InteractivityMode::NonInteractive);
    let results = resolver
        .resolve_and_execute(vec![entry], &auth_always_ok)
        .unwrap();
    // Should execute because collision handling is disabled
    assert!(matches!(
        &results[0].outcome,
        InitOutcome::Executed {
            commands_run: 1,
            ..
        }
    ));
}

#[test]
fn test_ordering_deterministic() {
    let entries: Vec<ProjectInitEntry> = (0..5)
        .map(|i| {
            let name = format!("ext-{}", i);
            let ext = ExtensionBuilder::new()
                .with_name(&name)
                .with_project_init(50, vec![])
                .build();
            ProjectInitEntry {
                extension_name: ext.metadata.name.clone(),
                extension: ext,
                priority: 50,
            }
        })
        .collect();

    // Run ordering 100 times and verify same result
    for _ in 0..100 {
        let sorted = sindri_extensions::collision::ordering::priority_sort(entries.clone());
        let names: Vec<&str> = sorted.iter().map(|e| e.extension_name.as_str()).collect();
        assert_eq!(names, vec!["ext-0", "ext-1", "ext-2", "ext-3", "ext-4"]);
    }
}

#[test]
fn test_auth_check_honored() {
    let tmp = TempDir::new().unwrap();

    let ext = ExtensionBuilder::new()
        .with_name("auth-ext")
        .with_project_init(
            50,
            vec![
                ProjectInitCommand {
                    command: "echo unconditional".to_string(),
                    description: "Always runs".to_string(),
                    requires_auth: AuthProvider::None,
                    conditional: false,
                },
                ProjectInitCommand {
                    command: "echo conditional".to_string(),
                    description: "Needs auth".to_string(),
                    requires_auth: AuthProvider::Anthropic,
                    conditional: true,
                },
            ],
        );
    let entry = entry_from_builder(ext, 50);

    let resolver =
        CollisionResolver::new(tmp.path().to_path_buf(), InteractivityMode::NonInteractive);
    let results = resolver
        .resolve_and_execute(vec![entry], &auth_always_fail)
        .unwrap();
    match &results[0].outcome {
        InitOutcome::Executed { commands_run, .. } => {
            // Only the unconditional command should run
            assert_eq!(*commands_run, 1);
        }
        other => panic!("Expected Executed, got {:?}", other),
    }
}

#[test]
fn test_mixed_extensions() {
    let tmp = TempDir::new().unwrap();

    // One with collision handling, one without
    let ext1 = ExtensionBuilder::new()
        .with_name("with-collision")
        .with_project_init(
            30,
            vec![ProjectInitCommand {
                command: "echo with".to_string(),
                description: "With collision".to_string(),
                requires_auth: AuthProvider::None,
                conditional: false,
            }],
        )
        .with_collision_handling(CollisionHandlingConfig {
            enabled: true,
            conflict_rules: vec![],
            version_markers: vec![],
            scenarios: vec![],
        });
    let entry1 = entry_from_builder(ext1, 30);

    let ext2 = ExtensionBuilder::new()
        .with_name("without-collision")
        .with_project_init(
            60,
            vec![ProjectInitCommand {
                command: "echo without".to_string(),
                description: "Without collision".to_string(),
                requires_auth: AuthProvider::None,
                conditional: false,
            }],
        );
    let entry2 = entry_from_builder(ext2, 60);

    let resolver =
        CollisionResolver::new(tmp.path().to_path_buf(), InteractivityMode::NonInteractive);
    let results = resolver
        .resolve_and_execute(vec![entry2, entry1], &auth_always_ok)
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].extension_name, "with-collision");
    assert_eq!(results[1].extension_name, "without-collision");
    assert!(matches!(&results[0].outcome, InitOutcome::Executed { .. }));
    assert!(matches!(&results[1].outcome, InitOutcome::Executed { .. }));
}

#[test]
fn test_single_extension_no_conflicts() {
    let tmp = TempDir::new().unwrap();

    let ext = ExtensionBuilder::new()
        .with_name("lonely")
        .with_project_init(
            100,
            vec![ProjectInitCommand {
                command: "echo alone".to_string(),
                description: "Solo run".to_string(),
                requires_auth: AuthProvider::None,
                conditional: false,
            }],
        );
    let entry = entry_from_builder(ext, 100);

    let resolver =
        CollisionResolver::new(tmp.path().to_path_buf(), InteractivityMode::NonInteractive);
    let results = resolver
        .resolve_and_execute(vec![entry], &auth_always_ok)
        .unwrap();
    match &results[0].outcome {
        InitOutcome::Executed {
            commands_run,
            conflicts_resolved,
        } => {
            assert_eq!(*commands_run, 1);
            assert!(conflicts_resolved.is_empty());
        }
        other => panic!("Expected Executed, got {:?}", other),
    }
}
