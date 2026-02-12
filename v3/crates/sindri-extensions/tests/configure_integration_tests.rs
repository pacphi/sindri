// Integration tests for configure processing

use serial_test::serial;
use sindri_core::types::{
    ConfigureConfig, EnvCondition, EnvConditionLogical, EnvironmentConfig, EnvironmentScope,
    PlatformCondition, TemplateCondition, TemplateConfig, TemplateMode,
};
use sindri_extensions::ConfigureProcessor;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test extension directory with templates
async fn setup_test_extension(temp_dir: &TempDir, extension_name: &str) -> PathBuf {
    let ext_dir = temp_dir.path().join(extension_name);
    fs::create_dir_all(&ext_dir).unwrap();
    ext_dir
}

#[tokio::test]
async fn test_configure_processor_template_overwrite() {
    let temp = TempDir::new().unwrap();
    let ext_dir = setup_test_extension(&temp, "test-ext").await;
    let home_dir = temp.path().join("home");
    fs::create_dir_all(&home_dir).unwrap();

    // Create template in extension directory
    let template_content = "Test template content";
    fs::write(ext_dir.join("template.txt"), template_content).unwrap();

    // Configure to overwrite
    let config = ConfigureConfig {
        templates: vec![TemplateConfig {
            source: "template.txt".to_string(),
            destination: "~/output.txt".to_string(),
            mode: TemplateMode::Overwrite,
            condition: None,
        }],
        environment: vec![],
    };

    let processor = ConfigureProcessor::new(ext_dir.clone(), home_dir.clone());

    let result = processor.execute("test-ext", &config).await;
    let result = result.expect("ConfigureProcessor::execute should succeed");
    assert_eq!(result.templates_processed, 1);

    // Verify file was created
    let output_file = home_dir.join("output.txt");
    assert!(output_file.exists());

    let content = fs::read_to_string(&output_file).unwrap();
    assert_eq!(content, template_content);
}

#[tokio::test]
async fn test_configure_processor_template_append() {
    let temp = TempDir::new().unwrap();
    let ext_dir = setup_test_extension(&temp, "test-ext").await;
    let home_dir = temp.path().join("home");
    fs::create_dir_all(&home_dir).unwrap();

    // Create existing file
    let existing_content = "Existing content";
    let output_file = home_dir.join("output.txt");
    fs::write(&output_file, existing_content).unwrap();

    // Create template
    let template_content = "Appended content";
    fs::write(ext_dir.join("template.txt"), template_content).unwrap();

    // Configure to append
    let config = ConfigureConfig {
        templates: vec![TemplateConfig {
            source: "template.txt".to_string(),
            destination: "~/output.txt".to_string(),
            mode: TemplateMode::Append,
            condition: None,
        }],
        environment: vec![],
    };

    let processor = ConfigureProcessor::new(ext_dir.clone(), home_dir.clone());

    let result = processor.execute("test-ext", &config).await.unwrap();
    assert_eq!(result.templates_processed, 1);

    // Verify content was appended
    let final_content = fs::read_to_string(&output_file).unwrap();
    assert!(final_content.contains(existing_content));
    assert!(final_content.contains(template_content));
}

#[tokio::test]
async fn test_configure_processor_template_skip_if_exists() {
    let temp = TempDir::new().unwrap();
    let ext_dir = setup_test_extension(&temp, "test-ext").await;
    let home_dir = temp.path().join("home");
    fs::create_dir_all(&home_dir).unwrap();

    // Create existing file
    let existing_content = "Existing content";
    let output_file = home_dir.join("output.txt");
    fs::write(&output_file, existing_content).unwrap();

    // Create template (different content)
    let template_content = "Template content (should be skipped)";
    fs::write(ext_dir.join("template.txt"), template_content).unwrap();

    // Configure with skip-if-exists
    let config = ConfigureConfig {
        templates: vec![TemplateConfig {
            source: "template.txt".to_string(),
            destination: "~/output.txt".to_string(),
            mode: TemplateMode::SkipIfExists,
            condition: None,
        }],
        environment: vec![],
    };

    let processor = ConfigureProcessor::new(ext_dir.clone(), home_dir.clone());

    let result = processor.execute("test-ext", &config).await.unwrap();
    assert_eq!(result.templates_processed, 1);

    // Verify file was NOT modified
    let final_content = fs::read_to_string(&output_file).unwrap();
    assert_eq!(final_content, existing_content);
    assert!(!final_content.contains(template_content));
}

#[tokio::test]
async fn test_configure_processor_template_merge_yaml() {
    let temp = TempDir::new().unwrap();
    let ext_dir = setup_test_extension(&temp, "test-ext").await;
    let home_dir = temp.path().join("home");
    fs::create_dir_all(&home_dir).unwrap();

    // Create existing YAML file
    let existing_yaml = r#"
existing_key: existing_value
nested:
  from_existing: true
  priority: low
"#;
    let output_file = home_dir.join("config.yaml");
    fs::write(&output_file, existing_yaml).unwrap();

    // Create template YAML (will merge)
    let template_yaml = r#"
new_key: new_value
nested:
  from_template: true
  priority: high
"#;
    fs::write(ext_dir.join("template.yaml"), template_yaml).unwrap();

    // Configure with merge
    let config = ConfigureConfig {
        templates: vec![TemplateConfig {
            source: "template.yaml".to_string(),
            destination: "~/config.yaml".to_string(),
            mode: TemplateMode::Merge,
            condition: None,
        }],
        environment: vec![],
    };

    let processor = ConfigureProcessor::new(ext_dir.clone(), home_dir.clone());

    let result = processor.execute("test-ext", &config).await.unwrap();
    assert_eq!(result.templates_processed, 1);
    assert_eq!(result.backups_created.len(), 1);

    // Verify merge occurred
    let final_content = fs::read_to_string(&output_file).unwrap();
    let merged: serde_yaml_ng::Value = serde_yaml_ng::from_str(&final_content).unwrap();

    // Check that both existing and new keys are present
    assert!(merged.get("existing_key").is_some());
    assert!(merged.get("new_key").is_some());

    // Check that nested values were merged (template takes precedence)
    let nested = merged.get("nested").unwrap();
    assert!(nested.get("from_existing").is_some());
    assert!(nested.get("from_template").is_some());
    assert_eq!(nested.get("priority").unwrap().as_str().unwrap(), "high");
}

#[tokio::test]
async fn test_configure_processor_environment_session() {
    let temp = TempDir::new().unwrap();
    let ext_dir = setup_test_extension(&temp, "test-ext").await;
    let home_dir = temp.path().join("home");
    fs::create_dir_all(&home_dir).unwrap();

    let config = ConfigureConfig {
        templates: vec![],
        environment: vec![EnvironmentConfig {
            key: "TEST_SESSION_VAR".to_string(),
            value: "session_value".to_string(),
            scope: EnvironmentScope::Session,
        }],
    };

    let processor = ConfigureProcessor::new(ext_dir.clone(), home_dir.clone());

    let result = processor.execute("test-ext", &config).await.unwrap();
    assert_eq!(result.environment_vars_set, 1);

    // Verify session variable was set
    assert_eq!(std::env::var("TEST_SESSION_VAR").unwrap(), "session_value");
}

#[tokio::test]
async fn test_configure_processor_environment_bashrc() {
    let temp = TempDir::new().unwrap();
    let ext_dir = setup_test_extension(&temp, "test-ext").await;
    let home_dir = temp.path().join("home");
    fs::create_dir_all(&home_dir).unwrap();

    let config = ConfigureConfig {
        templates: vec![],
        environment: vec![EnvironmentConfig {
            key: "TEST_BASHRC_VAR".to_string(),
            value: "bashrc_value".to_string(),
            scope: EnvironmentScope::Bashrc,
        }],
    };

    let processor = ConfigureProcessor::new(ext_dir.clone(), home_dir.clone());

    let result = processor.execute("test-ext", &config).await.unwrap();
    assert_eq!(result.environment_vars_set, 1);

    // Verify bashrc was modified
    let bashrc_path = home_dir.join(".bashrc");
    assert!(bashrc_path.exists());

    let bashrc_content = fs::read_to_string(&bashrc_path).unwrap();
    assert!(bashrc_content.contains("export TEST_BASHRC_VAR=\"bashrc_value\""));
}

#[tokio::test]
async fn test_configure_processor_environment_profile() {
    let temp = TempDir::new().unwrap();
    let ext_dir = setup_test_extension(&temp, "test-ext").await;
    let home_dir = temp.path().join("home");
    fs::create_dir_all(&home_dir).unwrap();

    let config = ConfigureConfig {
        templates: vec![],
        environment: vec![EnvironmentConfig {
            key: "TEST_PROFILE_VAR".to_string(),
            value: "profile_value".to_string(),
            scope: EnvironmentScope::Profile,
        }],
    };

    let processor = ConfigureProcessor::new(ext_dir.clone(), home_dir.clone());

    let result = processor.execute("test-ext", &config).await.unwrap();
    assert_eq!(result.environment_vars_set, 1);

    // Verify profile was modified (should create .profile by default)
    let profile_path = home_dir.join(".profile");
    assert!(profile_path.exists());

    let profile_content = fs::read_to_string(&profile_path).unwrap();
    assert!(profile_content.contains("export TEST_PROFILE_VAR=\"profile_value\""));
}

#[tokio::test]
async fn test_configure_processor_full_workflow() {
    let temp = TempDir::new().unwrap();
    let ext_dir = setup_test_extension(&temp, "test-ext").await;
    let home_dir = temp.path().join("home");
    fs::create_dir_all(&home_dir).unwrap();

    // Create multiple templates
    fs::write(ext_dir.join("config.txt"), "Config content").unwrap();
    fs::write(ext_dir.join("readme.txt"), "Readme content").unwrap();

    let config = ConfigureConfig {
        templates: vec![
            TemplateConfig {
                source: "config.txt".to_string(),
                destination: "~/.myapp/config.txt".to_string(),
                mode: TemplateMode::Overwrite,
                condition: None,
            },
            TemplateConfig {
                source: "readme.txt".to_string(),
                destination: "~/.myapp/readme.txt".to_string(),
                mode: TemplateMode::SkipIfExists,
                condition: None,
            },
        ],
        environment: vec![
            EnvironmentConfig {
                key: "MYAPP_HOME".to_string(),
                value: "~/.myapp".to_string(),
                scope: EnvironmentScope::Bashrc,
            },
            EnvironmentConfig {
                key: "MYAPP_TEMP".to_string(),
                value: "temp_value".to_string(),
                scope: EnvironmentScope::Session,
            },
        ],
    };

    let processor = ConfigureProcessor::new(ext_dir.clone(), home_dir.clone());

    let result = processor.execute("test-ext", &config).await.unwrap();

    // Verify results
    assert_eq!(result.templates_processed, 2);
    assert_eq!(result.environment_vars_set, 2);

    // Verify templates were processed
    assert!(home_dir.join(".myapp/config.txt").exists());
    assert!(home_dir.join(".myapp/readme.txt").exists());

    // Verify environment variables
    let bashrc_content = fs::read_to_string(home_dir.join(".bashrc")).unwrap();
    assert!(bashrc_content.contains("MYAPP_HOME"));
    assert_eq!(std::env::var("MYAPP_TEMP").unwrap(), "temp_value");
}

#[tokio::test]
async fn test_configure_processor_idempotency() {
    let temp = TempDir::new().unwrap();
    let ext_dir = setup_test_extension(&temp, "test-ext").await;
    let home_dir = temp.path().join("home");
    fs::create_dir_all(&home_dir).unwrap();

    fs::write(ext_dir.join("template.txt"), "Content").unwrap();

    let config = ConfigureConfig {
        templates: vec![TemplateConfig {
            source: "template.txt".to_string(),
            destination: "~/output.txt".to_string(),
            mode: TemplateMode::Overwrite,
            condition: None,
        }],
        environment: vec![EnvironmentConfig {
            key: "IDEMPOTENT_VAR".to_string(),
            value: "value".to_string(),
            scope: EnvironmentScope::Bashrc,
        }],
    };

    let processor = ConfigureProcessor::new(ext_dir.clone(), home_dir.clone());

    // Run configure twice
    processor.execute("test-ext", &config).await.unwrap();
    let result2 = processor.execute("test-ext", &config).await.unwrap();

    // Should succeed both times
    assert_eq!(result2.templates_processed, 1);
    assert_eq!(result2.environment_vars_set, 1);

    // Bashrc should not have duplicates
    let bashrc_content = fs::read_to_string(home_dir.join(".bashrc")).unwrap();
    assert_eq!(
        bashrc_content.matches("export IDEMPOTENT_VAR=").count(),
        1,
        "Environment variable should only appear once"
    );
}

#[tokio::test]
#[serial]
async fn test_conditional_template_ci_mode() {
    let temp = TempDir::new().unwrap();
    let ext_dir = setup_test_extension(&temp, "test-ext").await;
    let home_dir = temp.path().join("home");
    fs::create_dir_all(&home_dir).unwrap();

    // Create two templates
    fs::write(ext_dir.join("local.txt"), "Local content").unwrap();
    fs::write(ext_dir.join("ci.txt"), "CI content").unwrap();

    // Set CI environment
    std::env::set_var("CI", "true");

    let config = ConfigureConfig {
        templates: vec![
            TemplateConfig {
                source: "local.txt".to_string(),
                destination: "~/output.txt".to_string(),
                mode: TemplateMode::Overwrite,
                condition: Some(TemplateCondition {
                    env: Some(EnvCondition::Logical(EnvConditionLogical {
                        not_any: Some(vec![[("CI".to_string(), "true".to_string())]
                            .into_iter()
                            .collect()]),
                        ..Default::default()
                    })),
                    ..Default::default()
                }),
            },
            TemplateConfig {
                source: "ci.txt".to_string(),
                destination: "~/output.txt".to_string(),
                mode: TemplateMode::Overwrite,
                condition: Some(TemplateCondition {
                    env: Some(EnvCondition::Simple(
                        [("CI".to_string(), "true".to_string())]
                            .into_iter()
                            .collect(),
                    )),
                    ..Default::default()
                }),
            },
        ],
        environment: vec![],
    };

    let processor = ConfigureProcessor::new(ext_dir.clone(), home_dir.clone());

    let result = processor.execute("test-ext", &config).await.unwrap();

    // Only CI template should be processed (local template skipped)
    assert_eq!(result.templates_processed, 1);

    // Verify CI content was written
    let output = fs::read_to_string(home_dir.join("output.txt")).unwrap();
    assert_eq!(output, "CI content");

    // Cleanup
    std::env::remove_var("CI");
}

#[tokio::test]
#[serial]
async fn test_conditional_template_local_mode() {
    let temp = TempDir::new().unwrap();
    let ext_dir = setup_test_extension(&temp, "test-ext").await;
    let home_dir = temp.path().join("home");
    fs::create_dir_all(&home_dir).unwrap();

    // Create two templates
    fs::write(ext_dir.join("local.txt"), "Local content").unwrap();
    fs::write(ext_dir.join("ci.txt"), "CI content").unwrap();

    // Ensure CI is not set
    std::env::remove_var("CI");
    std::env::remove_var("GITHUB_ACTIONS");

    let config = ConfigureConfig {
        templates: vec![
            TemplateConfig {
                source: "local.txt".to_string(),
                destination: "~/output.txt".to_string(),
                mode: TemplateMode::Overwrite,
                condition: Some(TemplateCondition {
                    env: Some(EnvCondition::Logical(EnvConditionLogical {
                        not_any: Some(vec![
                            [("CI".to_string(), "true".to_string())]
                                .into_iter()
                                .collect(),
                            [("GITHUB_ACTIONS".to_string(), "true".to_string())]
                                .into_iter()
                                .collect(),
                        ]),
                        ..Default::default()
                    })),
                    ..Default::default()
                }),
            },
            TemplateConfig {
                source: "ci.txt".to_string(),
                destination: "~/output.txt".to_string(),
                mode: TemplateMode::Overwrite,
                condition: Some(TemplateCondition {
                    env: Some(EnvCondition::Logical(EnvConditionLogical {
                        any: Some(vec![
                            [("CI".to_string(), "true".to_string())]
                                .into_iter()
                                .collect(),
                            [("GITHUB_ACTIONS".to_string(), "true".to_string())]
                                .into_iter()
                                .collect(),
                        ]),
                        ..Default::default()
                    })),
                    ..Default::default()
                }),
            },
        ],
        environment: vec![],
    };

    let processor = ConfigureProcessor::new(ext_dir.clone(), home_dir.clone());

    let result = processor.execute("test-ext", &config).await.unwrap();

    // Only local template should be processed (CI template skipped)
    assert_eq!(result.templates_processed, 1);

    // Verify local content was written
    let output = fs::read_to_string(home_dir.join("output.txt")).unwrap();
    assert_eq!(output, "Local content");
}

#[tokio::test]
async fn test_conditional_template_platform() {
    let temp = TempDir::new().unwrap();
    let ext_dir = setup_test_extension(&temp, "test-ext").await;
    let home_dir = temp.path().join("home");
    fs::create_dir_all(&home_dir).unwrap();

    fs::write(ext_dir.join("config.txt"), "Platform-specific").unwrap();

    let current_os = std::env::consts::OS;
    let os_name = match current_os {
        "linux" => "linux",
        "macos" => "macos",
        "windows" => "windows",
        _ => "unknown",
    };

    let config = ConfigureConfig {
        templates: vec![TemplateConfig {
            source: "config.txt".to_string(),
            destination: "~/config.txt".to_string(),
            mode: TemplateMode::Overwrite,
            condition: Some(TemplateCondition {
                platform: Some(PlatformCondition {
                    os: Some(vec![os_name.to_string()]),
                    arch: None,
                }),
                ..Default::default()
            }),
        }],
        environment: vec![],
    };

    let processor = ConfigureProcessor::new(ext_dir.clone(), home_dir.clone());

    let result = processor.execute("test-ext", &config).await.unwrap();

    // Should match current platform
    assert_eq!(result.templates_processed, 1);
    assert!(home_dir.join("config.txt").exists());
}

#[tokio::test]
async fn test_conditional_template_wrong_platform() {
    let temp = TempDir::new().unwrap();
    let ext_dir = setup_test_extension(&temp, "test-ext").await;
    let home_dir = temp.path().join("home");
    fs::create_dir_all(&home_dir).unwrap();

    fs::write(ext_dir.join("config.txt"), "Platform-specific").unwrap();

    let config = ConfigureConfig {
        templates: vec![TemplateConfig {
            source: "config.txt".to_string(),
            destination: "~/config.txt".to_string(),
            mode: TemplateMode::Overwrite,
            condition: Some(TemplateCondition {
                platform: Some(PlatformCondition {
                    os: Some(vec!["nonexistent_os".to_string()]),
                    arch: None,
                }),
                ..Default::default()
            }),
        }],
        environment: vec![],
    };

    let processor = ConfigureProcessor::new(ext_dir.clone(), home_dir.clone());

    let result = processor.execute("test-ext", &config).await.unwrap();

    // Template should be skipped due to platform mismatch
    assert_eq!(result.templates_processed, 0);
    assert!(!home_dir.join("config.txt").exists());
}

#[tokio::test]
async fn test_conditional_template_no_condition() {
    let temp = TempDir::new().unwrap();
    let ext_dir = setup_test_extension(&temp, "test-ext").await;
    let home_dir = temp.path().join("home");
    fs::create_dir_all(&home_dir).unwrap();

    fs::write(ext_dir.join("config.txt"), "Unconditional").unwrap();

    let config = ConfigureConfig {
        templates: vec![TemplateConfig {
            source: "config.txt".to_string(),
            destination: "~/config.txt".to_string(),
            mode: TemplateMode::Overwrite,
            condition: None, // No condition means always process
        }],
        environment: vec![],
    };

    let processor = ConfigureProcessor::new(ext_dir.clone(), home_dir.clone());

    let result = processor.execute("test-ext", &config).await.unwrap();

    // Should always process when no condition is specified
    assert_eq!(result.templates_processed, 1);
    assert!(home_dir.join("config.txt").exists());
}

#[tokio::test]
#[serial]
async fn test_conditional_template_complex_conditions() {
    let temp = TempDir::new().unwrap();
    let ext_dir = setup_test_extension(&temp, "test-ext").await;
    let home_dir = temp.path().join("home");
    fs::create_dir_all(&home_dir).unwrap();

    fs::write(ext_dir.join("config.txt"), "Complex conditions").unwrap();

    // Set up environment for complex condition
    std::env::set_var("CI", "true");
    std::env::set_var("DEPLOY_ENV", "staging");

    let current_os = std::env::consts::OS;
    let os_name = match current_os {
        "linux" => "linux",
        "macos" => "macos",
        "windows" => "windows",
        _ => "unknown",
    };

    let config = ConfigureConfig {
        templates: vec![TemplateConfig {
            source: "config.txt".to_string(),
            destination: "~/config.txt".to_string(),
            mode: TemplateMode::Overwrite,
            condition: Some(TemplateCondition {
                all: Some(vec![
                    TemplateCondition {
                        env: Some(EnvCondition::Simple(
                            [("CI".to_string(), "true".to_string())]
                                .into_iter()
                                .collect(),
                        )),
                        ..Default::default()
                    },
                    TemplateCondition {
                        platform: Some(PlatformCondition {
                            os: Some(vec![os_name.to_string()]),
                            arch: None,
                        }),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            }),
        }],
        environment: vec![],
    };

    let processor = ConfigureProcessor::new(ext_dir.clone(), home_dir.clone());

    let result = processor.execute("test-ext", &config).await.unwrap();

    // Should process because both CI=true AND platform matches
    assert_eq!(result.templates_processed, 1);
    assert!(home_dir.join("config.txt").exists());

    // Cleanup
    std::env::remove_var("CI");
    std::env::remove_var("DEPLOY_ENV");
}
