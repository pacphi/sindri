// Condition evaluation for environment-based template selection

use anyhow::{Context, Result};
use regex::Regex;
use sindri_core::types::{
    EnvCondition, EnvConditionExpr, EnvConditionLogical, PlatformCondition, TemplateCondition,
};
use std::collections::HashMap;

/// Platform information for condition evaluation
#[derive(Debug, Clone)]
struct PlatformInfo {
    os: String,   // "linux", "macos", "windows"
    arch: String, // "x86_64", "aarch64", "arm64"
}

impl PlatformInfo {
    /// Detect current platform information
    fn detect() -> Self {
        let os = match std::env::consts::OS {
            "linux" => "linux",
            "macos" => "macos",
            "windows" => "windows",
            other => {
                tracing::warn!("Unknown OS: {}, defaulting to 'unknown'", other);
                "unknown"
            }
        };

        let arch = match std::env::consts::ARCH {
            "x86_64" => "x86_64",
            "aarch64" => "aarch64",
            "arm" => "arm64",
            other => {
                tracing::warn!("Unknown architecture: {}, defaulting to 'unknown'", other);
                "unknown"
            }
        };

        Self {
            os: os.to_string(),
            arch: arch.to_string(),
        }
    }
}

/// Condition evaluator for template selection
pub struct ConditionEvaluator {
    platform_info: PlatformInfo,
}

impl ConditionEvaluator {
    /// Create a new condition evaluator
    pub fn new() -> Self {
        Self {
            platform_info: PlatformInfo::detect(),
        }
    }

    /// Evaluate a template condition
    pub fn evaluate(&self, condition: &TemplateCondition) -> Result<bool> {
        // Evaluate env conditions
        if let Some(env_cond) = &condition.env {
            if !self.evaluate_env_condition(env_cond)? {
                return Ok(false);
            }
        }

        // Evaluate platform conditions
        if let Some(platform_cond) = &condition.platform {
            if !self.evaluate_platform_condition(platform_cond)? {
                return Ok(false);
            }
        }

        // Evaluate logical operators
        if let Some(any_conditions) = &condition.any {
            return self.evaluate_any(any_conditions);
        }

        if let Some(all_conditions) = &condition.all {
            return self.evaluate_all(all_conditions);
        }

        if let Some(not_condition) = &condition.not {
            return Ok(!self.evaluate(not_condition)?);
        }

        Ok(true)
    }

    /// Evaluate environment variable condition
    fn evaluate_env_condition(&self, env_cond: &EnvCondition) -> Result<bool> {
        match env_cond {
            EnvCondition::Simple(map) => {
                // Simple key-value matching: { CI: "true" }
                for (key, expected_value) in map {
                    let actual_value = self.get_env_var(key);
                    if actual_value.as_deref() != Some(expected_value.as_str()) {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            EnvCondition::Complex(map) => {
                // Complex with operators: { CI: { equals: "true" } }
                for (key, expr) in map {
                    if !self.evaluate_env_expr(key, expr)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            EnvCondition::Logical(logical) => {
                // Logical operators: { any: [...], all: [...], not_any: [...] }
                self.evaluate_env_logical(logical)
            }
        }
    }

    /// Evaluate environment variable expression
    fn evaluate_env_expr(&self, key: &str, expr: &EnvConditionExpr) -> Result<bool> {
        let actual_value = self.get_env_var(key);

        // Check exists condition
        if let Some(should_exist) = expr.exists {
            return Ok(actual_value.is_some() == should_exist);
        }

        // For other operators, we need a value
        let value = match actual_value {
            Some(v) => v,
            None => return Ok(false), // Variable doesn't exist
        };

        // Check equals
        if let Some(expected) = &expr.equals {
            if value != *expected {
                return Ok(false);
            }
        }

        // Check not_equals
        if let Some(not_expected) = &expr.not_equals {
            if value == *not_expected {
                return Ok(false);
            }
        }

        // Check regex match
        if let Some(pattern) = &expr.matches {
            let regex = Regex::new(pattern)
                .with_context(|| format!("Invalid regex pattern: {}", pattern))?;
            if !regex.is_match(&value) {
                return Ok(false);
            }
        }

        // Check in_list
        if let Some(list) = &expr.in_list {
            if !list.contains(&value) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Evaluate logical environment conditions
    fn evaluate_env_logical(&self, logical: &EnvConditionLogical) -> Result<bool> {
        // Check any (OR logic)
        if let Some(any_conditions) = &logical.any {
            for condition_map in any_conditions {
                if self.evaluate_simple_env_map(condition_map) {
                    return Ok(true);
                }
            }
            return Ok(false);
        }

        // Check all (AND logic)
        if let Some(all_conditions) = &logical.all {
            for condition_map in all_conditions {
                if !self.evaluate_simple_env_map(condition_map) {
                    return Ok(false);
                }
            }
            return Ok(true);
        }

        // Check not_any (NOR logic)
        if let Some(not_any_conditions) = &logical.not_any {
            for condition_map in not_any_conditions {
                if self.evaluate_simple_env_map(condition_map) {
                    return Ok(false);
                }
            }
            return Ok(true);
        }

        // Check not_all (NAND logic)
        if let Some(not_all_conditions) = &logical.not_all {
            for condition_map in not_all_conditions {
                if !self.evaluate_simple_env_map(condition_map) {
                    return Ok(true);
                }
            }
            return Ok(false);
        }

        Ok(true)
    }

    /// Evaluate simple environment variable map (key-value pairs)
    fn evaluate_simple_env_map(&self, map: &HashMap<String, String>) -> bool {
        for (key, expected_value) in map {
            let actual_value = self.get_env_var(key);
            if actual_value.as_deref() != Some(expected_value.as_str()) {
                return false;
            }
        }
        true
    }

    /// Evaluate platform condition
    fn evaluate_platform_condition(&self, platform: &PlatformCondition) -> Result<bool> {
        // Check OS
        if let Some(allowed_os) = &platform.os {
            if !allowed_os.contains(&self.platform_info.os) {
                return Ok(false);
            }
        }

        // Check architecture
        if let Some(allowed_arch) = &platform.arch {
            if !allowed_arch.contains(&self.platform_info.arch) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Evaluate any (OR) conditions
    fn evaluate_any(&self, conditions: &[TemplateCondition]) -> Result<bool> {
        for condition in conditions {
            if self.evaluate(condition)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Evaluate all (AND) conditions
    fn evaluate_all(&self, conditions: &[TemplateCondition]) -> Result<bool> {
        for condition in conditions {
            if !self.evaluate(condition)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Get environment variable value
    fn get_env_var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

impl Default for ConditionEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    #[serial]
    fn test_evaluate_env_simple() {
        let evaluator = ConditionEvaluator::new();
        env::set_var("TEST_VAR", "true");

        let condition = TemplateCondition {
            env: Some(EnvCondition::Simple(
                [("TEST_VAR".to_string(), "true".to_string())]
                    .into_iter()
                    .collect(),
            )),
            ..Default::default()
        };

        assert!(evaluator.evaluate(&condition).unwrap());
        env::remove_var("TEST_VAR");
    }

    #[test]
    #[serial]
    fn test_evaluate_env_simple_mismatch() {
        let evaluator = ConditionEvaluator::new();
        env::set_var("TEST_VAR", "false");

        let condition = TemplateCondition {
            env: Some(EnvCondition::Simple(
                [("TEST_VAR".to_string(), "true".to_string())]
                    .into_iter()
                    .collect(),
            )),
            ..Default::default()
        };

        assert!(!evaluator.evaluate(&condition).unwrap());
        env::remove_var("TEST_VAR");
    }

    #[test]
    #[serial]
    fn test_evaluate_env_not_equals() {
        let evaluator = ConditionEvaluator::new();
        env::set_var("CI", "false");

        let condition = TemplateCondition {
            env: Some(EnvCondition::Complex(
                [(
                    "CI".to_string(),
                    EnvConditionExpr {
                        not_equals: Some("true".to_string()),
                        ..Default::default()
                    },
                )]
                .into_iter()
                .collect(),
            )),
            ..Default::default()
        };

        assert!(evaluator.evaluate(&condition).unwrap());
        env::remove_var("CI");
    }

    #[test]
    #[serial]
    fn test_evaluate_env_exists() {
        let evaluator = ConditionEvaluator::new();
        env::set_var("EXISTING_VAR", "some_value");

        let condition_exists = TemplateCondition {
            env: Some(EnvCondition::Complex(
                [(
                    "EXISTING_VAR".to_string(),
                    EnvConditionExpr {
                        exists: Some(true),
                        ..Default::default()
                    },
                )]
                .into_iter()
                .collect(),
            )),
            ..Default::default()
        };

        assert!(evaluator.evaluate(&condition_exists).unwrap());

        let condition_not_exists = TemplateCondition {
            env: Some(EnvCondition::Complex(
                [(
                    "NON_EXISTING_VAR".to_string(),
                    EnvConditionExpr {
                        exists: Some(false),
                        ..Default::default()
                    },
                )]
                .into_iter()
                .collect(),
            )),
            ..Default::default()
        };

        assert!(evaluator.evaluate(&condition_not_exists).unwrap());
        env::remove_var("EXISTING_VAR");
    }

    #[test]
    fn test_evaluate_platform() {
        let evaluator = ConditionEvaluator::new();

        let condition = TemplateCondition {
            platform: Some(PlatformCondition {
                os: Some(vec![evaluator.platform_info.os.clone()]),
                arch: None,
            }),
            ..Default::default()
        };

        assert!(evaluator.evaluate(&condition).unwrap());
    }

    #[test]
    fn test_evaluate_platform_wrong_os() {
        let evaluator = ConditionEvaluator::new();

        let condition = TemplateCondition {
            platform: Some(PlatformCondition {
                os: Some(vec!["nonexistent_os".to_string()]),
                arch: None,
            }),
            ..Default::default()
        };

        assert!(!evaluator.evaluate(&condition).unwrap());
    }

    #[test]
    #[serial]
    fn test_evaluate_any() {
        let evaluator = ConditionEvaluator::new();
        env::set_var("VAR1", "true");
        env::remove_var("VAR2");

        let condition = TemplateCondition {
            any: Some(vec![
                TemplateCondition {
                    env: Some(EnvCondition::Simple(
                        [("VAR1".to_string(), "true".to_string())]
                            .into_iter()
                            .collect(),
                    )),
                    ..Default::default()
                },
                TemplateCondition {
                    env: Some(EnvCondition::Simple(
                        [("VAR2".to_string(), "true".to_string())]
                            .into_iter()
                            .collect(),
                    )),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };

        // Should be true because VAR1 matches
        assert!(evaluator.evaluate(&condition).unwrap());
        env::remove_var("VAR1");
    }

    #[test]
    #[serial]
    fn test_evaluate_all() {
        let evaluator = ConditionEvaluator::new();
        env::set_var("VAR1", "true");
        env::set_var("VAR2", "true");

        let condition = TemplateCondition {
            all: Some(vec![
                TemplateCondition {
                    env: Some(EnvCondition::Simple(
                        [("VAR1".to_string(), "true".to_string())]
                            .into_iter()
                            .collect(),
                    )),
                    ..Default::default()
                },
                TemplateCondition {
                    env: Some(EnvCondition::Simple(
                        [("VAR2".to_string(), "true".to_string())]
                            .into_iter()
                            .collect(),
                    )),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };

        // Should be true because both conditions match
        assert!(evaluator.evaluate(&condition).unwrap());
        env::remove_var("VAR1");
        env::remove_var("VAR2");
    }

    #[test]
    #[serial]
    fn test_evaluate_not() {
        let evaluator = ConditionEvaluator::new();
        env::set_var("VAR1", "false");

        let condition = TemplateCondition {
            not: Some(Box::new(TemplateCondition {
                env: Some(EnvCondition::Simple(
                    [("VAR1".to_string(), "true".to_string())]
                        .into_iter()
                        .collect(),
                )),
                ..Default::default()
            })),
            ..Default::default()
        };

        // Should be true because VAR1 is "false", not "true"
        assert!(evaluator.evaluate(&condition).unwrap());
        env::remove_var("VAR1");
    }

    #[test]
    #[serial]
    fn test_evaluate_regex_match() {
        let evaluator = ConditionEvaluator::new();
        env::set_var("WORKSPACE", "/home/user/workspace");

        let condition = TemplateCondition {
            env: Some(EnvCondition::Complex(
                [(
                    "WORKSPACE".to_string(),
                    EnvConditionExpr {
                        matches: Some("^/home/.*/workspace$".to_string()),
                        ..Default::default()
                    },
                )]
                .into_iter()
                .collect(),
            )),
            ..Default::default()
        };

        assert!(evaluator.evaluate(&condition).unwrap());
        env::remove_var("WORKSPACE");
    }

    #[test]
    #[serial]
    fn test_evaluate_in_list() {
        let evaluator = ConditionEvaluator::new();
        env::set_var("ENVIRONMENT", "staging");

        let condition = TemplateCondition {
            env: Some(EnvCondition::Complex(
                [(
                    "ENVIRONMENT".to_string(),
                    EnvConditionExpr {
                        in_list: Some(vec![
                            "development".to_string(),
                            "staging".to_string(),
                            "production".to_string(),
                        ]),
                        ..Default::default()
                    },
                )]
                .into_iter()
                .collect(),
            )),
            ..Default::default()
        };

        assert!(evaluator.evaluate(&condition).unwrap());
        env::remove_var("ENVIRONMENT");
    }

    #[test]
    #[serial]
    fn test_evaluate_logical_any() {
        let evaluator = ConditionEvaluator::new();
        env::set_var("CI", "true");

        let condition = TemplateCondition {
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
        };

        assert!(evaluator.evaluate(&condition).unwrap());
        env::remove_var("CI");
    }

    #[test]
    #[serial]
    fn test_evaluate_logical_not_any() {
        let evaluator = ConditionEvaluator::new();
        env::remove_var("CI");
        env::remove_var("GITHUB_ACTIONS");

        let condition = TemplateCondition {
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
        };

        assert!(evaluator.evaluate(&condition).unwrap());
    }

    #[test]
    fn test_default_implementation() {
        let evaluator = ConditionEvaluator::default();
        assert!(!evaluator.platform_info.os.is_empty());
        assert!(!evaluator.platform_info.arch.is_empty());
    }
}
