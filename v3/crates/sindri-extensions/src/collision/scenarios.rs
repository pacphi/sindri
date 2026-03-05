//! Scenario evaluation for collision handling

use super::detection::DetectedVersion;
use sindri_core::types::{CollisionScenario, ScenarioAction};

/// Outcome of evaluating scenarios against detected versions
#[derive(Debug, Clone)]
pub enum ScenarioOutcome {
    /// Continue with project-init
    Proceed,
    /// Skip this extension's project-init (non-error)
    Skip { message: String },
    /// Stop with an error/warning message
    Stop { message: String },
}

/// Evaluates collision scenarios against detected versions
pub struct ScenarioEvaluator;

impl ScenarioEvaluator {
    /// Match detected versions against extension's scenarios.
    /// First matching scenario wins. No match = Proceed.
    pub fn evaluate(
        detected: &[DetectedVersion],
        scenarios: &[CollisionScenario],
        installing_version: &str,
    ) -> ScenarioOutcome {
        for scenario in scenarios {
            // Check if any detected version matches this scenario's detected-version
            let version_match = detected
                .iter()
                .any(|d| d.version_label == scenario.detected_version);

            if !version_match {
                continue;
            }

            // Check if installing version matches (or wildcard)
            let install_match = scenario.installing_version == installing_version
                || scenario.installing_version == "*";

            if !install_match {
                continue;
            }

            // First match wins
            return match scenario.action {
                ScenarioAction::Proceed => ScenarioOutcome::Proceed,
                ScenarioAction::Skip => ScenarioOutcome::Skip {
                    message: scenario.message.clone(),
                },
                ScenarioAction::Stop => ScenarioOutcome::Stop {
                    message: scenario.message.clone(),
                },
                ScenarioAction::Backup => ScenarioOutcome::Proceed,
                ScenarioAction::Prompt => {
                    // In non-interactive context, treat prompt as skip
                    ScenarioOutcome::Skip {
                        message: format!(
                            "{}\n(Prompt not available in non-interactive mode, skipping)",
                            scenario.message
                        ),
                    }
                }
            };
        }

        // No matching scenario = proceed
        ScenarioOutcome::Proceed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_scenario(
        name: &str,
        detected_version: &str,
        installing_version: &str,
        action: ScenarioAction,
        message: &str,
    ) -> CollisionScenario {
        CollisionScenario {
            name: name.to_string(),
            detected_version: detected_version.to_string(),
            installing_version: installing_version.to_string(),
            action,
            message: message.to_string(),
            options: vec![],
        }
    }

    fn make_detected(version_label: &str) -> DetectedVersion {
        DetectedVersion {
            marker_path: "test".to_string(),
            version_label: version_label.to_string(),
        }
    }

    #[test]
    fn test_no_detected_versions_proceeds() {
        let scenarios = vec![make_scenario(
            "test",
            "v2",
            "3.5.2",
            ScenarioAction::Stop,
            "stop",
        )];
        let outcome = ScenarioEvaluator::evaluate(&[], &scenarios, "3.5.2");
        assert!(matches!(outcome, ScenarioOutcome::Proceed));
    }

    #[test]
    fn test_stop_scenario() {
        let detected = vec![make_detected("v2")];
        let scenarios = vec![make_scenario(
            "v2-to-ruflo-upgrade",
            "v2",
            "3.5.2",
            ScenarioAction::Stop,
            "V2 detected, manual migration required",
        )];
        let outcome = ScenarioEvaluator::evaluate(&detected, &scenarios, "3.5.2");
        assert!(matches!(outcome, ScenarioOutcome::Stop { .. }));
        if let ScenarioOutcome::Stop { message } = outcome {
            assert!(message.contains("V2 detected"));
        }
    }

    #[test]
    fn test_skip_scenario() {
        let detected = vec![make_detected("ruflo")];
        let scenarios = vec![make_scenario(
            "same-version-ruflo",
            "ruflo",
            "3.5.2",
            ScenarioAction::Skip,
            "Already initialized",
        )];
        let outcome = ScenarioEvaluator::evaluate(&detected, &scenarios, "3.5.2");
        assert!(matches!(outcome, ScenarioOutcome::Skip { .. }));
    }

    #[test]
    fn test_proceed_scenario() {
        let detected = vec![make_detected("agentic-qe")];
        let scenarios = vec![make_scenario(
            "agentic-qe-coexist",
            "agentic-qe",
            "3.5.2",
            ScenarioAction::Proceed,
            "Co-tenant detected",
        )];
        let outcome = ScenarioEvaluator::evaluate(&detected, &scenarios, "3.5.2");
        assert!(matches!(outcome, ScenarioOutcome::Proceed));
    }

    #[test]
    fn test_first_match_wins() {
        let detected = vec![make_detected("v2")];
        let scenarios = vec![
            make_scenario("first", "v2", "3.5.2", ScenarioAction::Skip, "skip wins"),
            make_scenario("second", "v2", "3.5.2", ScenarioAction::Stop, "stop loses"),
        ];
        let outcome = ScenarioEvaluator::evaluate(&detected, &scenarios, "3.5.2");
        assert!(matches!(outcome, ScenarioOutcome::Skip { .. }));
    }

    #[test]
    fn test_no_matching_scenario_proceeds() {
        let detected = vec![make_detected("unrecognized")];
        let scenarios = vec![make_scenario(
            "test",
            "v2",
            "3.5.2",
            ScenarioAction::Stop,
            "stop",
        )];
        let outcome = ScenarioEvaluator::evaluate(&detected, &scenarios, "3.5.2");
        assert!(matches!(outcome, ScenarioOutcome::Proceed));
    }

    #[test]
    fn test_backup_scenario() {
        let detected = vec![make_detected("old")];
        let scenarios = vec![make_scenario(
            "backup",
            "old",
            "1.0.0",
            ScenarioAction::Backup,
            "Backup and proceed",
        )];
        let outcome = ScenarioEvaluator::evaluate(&detected, &scenarios, "1.0.0");
        // Backup maps to Proceed (backup is handled by conflict rules)
        assert!(matches!(outcome, ScenarioOutcome::Proceed));
    }
}
