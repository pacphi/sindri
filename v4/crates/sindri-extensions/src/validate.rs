//! Validate executor (DDD-01 §Validate, ADR-024).
//!
//! [`ValidateExecutor`] runs the post-install health checks declared by a
//! component's [`sindri_core::component::ValidateConfig`].
//!
//! Each [`sindri_core::component::ValidateCommand`] is dispatched through the
//! active [`Target`]; stdout is then matched against optional assertions:
//!
//! - `expected_output: Some(s)` — `s` must appear as a **substring** of stdout.
//! - `version_match: Some(spec)` — a semver-looking token (`vMAJOR.MINOR.PATCH`
//!   or bare `MAJOR.MINOR.PATCH`) is extracted from stdout and compared against
//!   the [`semver::VersionReq`].
//!
//! All assertions must succeed for the validate phase to pass. The first
//! failure surfaces as [`ExtensionError::ValidateFailed`] with the offending
//! command and a human-readable expected/got pair.

use crate::error::ExtensionError;
use sindri_core::component::{ValidateCommand, ValidateConfig};
use sindri_targets::Target;

/// Context for a validate run.
pub struct ValidateContext<'a> {
    /// Component metadata name (e.g. `"nodejs"`).
    pub component: &'a str,
    /// Active target for command dispatch.
    pub target: &'a dyn Target,
    /// Environment variables to expose to each validate command.
    pub env: &'a [(&'a str, &'a str)],
}

/// Capability executor for `validate` (ADR-024).
#[derive(Debug, Default, Clone, Copy)]
pub struct ValidateExecutor;

impl ValidateExecutor {
    /// Create a new executor.
    pub fn new() -> Self {
        Self
    }

    /// Run every command in `cfg`, asserting each one's expected_output
    /// and/or version_match.
    pub async fn run(
        &self,
        cfg: &ValidateConfig,
        ctx: &ValidateContext<'_>,
    ) -> Result<(), ExtensionError> {
        for cmd in &cfg.commands {
            self.run_one(cmd, ctx)?;
        }
        Ok(())
    }

    fn run_one(
        &self,
        cmd: &ValidateCommand,
        ctx: &ValidateContext<'_>,
    ) -> Result<(), ExtensionError> {
        tracing::info!(
            component = ctx.component,
            command = cmd.command.as_str(),
            "running validate command"
        );

        let (stdout, stderr) = ctx.target.exec(&cmd.command, ctx.env).map_err(|err| {
            ExtensionError::ValidateFailed {
                component: ctx.component.to_string(),
                command: cmd.command.clone(),
                expected: "command to exit 0".to_string(),
                got: err.to_string(),
            }
        })?;

        if let Some(expected) = &cmd.expected_output {
            if !stdout.contains(expected.as_str()) {
                return Err(ExtensionError::ValidateFailed {
                    component: ctx.component.to_string(),
                    command: cmd.command.clone(),
                    expected: format!("stdout to contain `{}`", expected),
                    got: truncate(&stdout, 256),
                });
            }
        }

        if let Some(spec) = &cmd.version_match {
            let req =
                semver::VersionReq::parse(spec).map_err(|e| ExtensionError::ValidateFailed {
                    component: ctx.component.to_string(),
                    command: cmd.command.clone(),
                    expected: format!("valid semver requirement (`{}`)", spec),
                    got: format!("parse error: {}", e),
                })?;

            let token = extract_semver(&stdout).ok_or_else(|| ExtensionError::ValidateFailed {
                component: ctx.component.to_string(),
                command: cmd.command.clone(),
                expected: format!("a semver-looking token matching `{}`", spec),
                got: format!(
                    "no MAJOR.MINOR.PATCH token in stdout: {}",
                    truncate(&stdout, 256)
                ),
            })?;

            let actual =
                semver::Version::parse(&token).map_err(|e| ExtensionError::ValidateFailed {
                    component: ctx.component.to_string(),
                    command: cmd.command.clone(),
                    expected: format!("a parseable semver in stdout matching `{}`", spec),
                    got: format!("`{}`: {}", token, e),
                })?;

            if !req.matches(&actual) {
                return Err(ExtensionError::ValidateFailed {
                    component: ctx.component.to_string(),
                    command: cmd.command.clone(),
                    expected: format!("version matching `{}`", spec),
                    got: format!("found `{}` (stderr: `{}`)", actual, truncate(&stderr, 128)),
                });
            }
        }

        Ok(())
    }
}

/// Extract the first `MAJOR.MINOR.PATCH` token from `stdout`, ignoring an
/// optional leading `v` (so `node --version` → `v22.5.1` → `22.5.1` works).
fn extract_semver(stdout: &str) -> Option<String> {
    let bytes = stdout.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        // Optional leading `v`/`V`.
        let mut j = i;
        if bytes[j] == b'v' || bytes[j] == b'V' {
            j += 1;
        }
        if let Some((token, end)) = match_semver_at(bytes, j) {
            // Make sure the token isn't preceded by an alphanumeric (so we
            // don't grab `1.2.3` out of `node1.2.3pre`).
            if i == 0 || !bytes[i - 1].is_ascii_alphanumeric() {
                let _ = end;
                return Some(token);
            }
        }
        i += 1;
    }
    None
}

/// Try to match `<digits>.<digits>.<digits>` starting at `start`. Returns
/// the matched token and the index just past it.
fn match_semver_at(bytes: &[u8], start: usize) -> Option<(String, usize)> {
    let n = bytes.len();
    let (a_start, a_end) = scan_digits(bytes, start)?;
    if a_end >= n || bytes[a_end] != b'.' {
        return None;
    }
    let (_, b_end) = scan_digits(bytes, a_end + 1)?;
    if b_end >= n || bytes[b_end] != b'.' {
        return None;
    }
    let (_, c_end) = scan_digits(bytes, b_end + 1)?;
    let token = std::str::from_utf8(&bytes[a_start..c_end])
        .ok()?
        .to_string();
    Some((token, c_end))
}

fn scan_digits(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
    let n = bytes.len();
    if start >= n || !bytes[start].is_ascii_digit() {
        return None;
    }
    let mut end = start;
    while end < n && bytes[end].is_ascii_digit() {
        end += 1;
    }
    Some((start, end))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::platform::TargetProfile;
    use sindri_targets::error::TargetError;
    use sindri_targets::traits::PrereqCheck;
    use std::sync::Mutex;

    /// Mock target that returns scripted stdout for each command.
    struct ScriptedTarget {
        responses: Mutex<Vec<(String, String)>>, // (stdout, stderr) per call
    }

    impl ScriptedTarget {
        fn with(stdouts: &[&str]) -> Self {
            let v = stdouts
                .iter()
                .map(|s| ((*s).to_string(), String::new()))
                .collect::<Vec<_>>();
            Self {
                responses: Mutex::new(v),
            }
        }
    }

    impl Target for ScriptedTarget {
        fn name(&self) -> &str {
            "scripted"
        }
        fn kind(&self) -> &str {
            "scripted"
        }
        fn profile(&self) -> Result<TargetProfile, TargetError> {
            Err(TargetError::Unavailable {
                name: "scripted".into(),
                reason: "test".into(),
            })
        }
        fn exec(&self, _cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
            let mut g = self.responses.lock().unwrap();
            if g.is_empty() {
                return Ok((String::new(), String::new()));
            }
            Ok(g.remove(0))
        }
        fn upload(&self, _l: &std::path::Path, _r: &str) -> Result<(), TargetError> {
            Ok(())
        }
        fn download(&self, _r: &str, _l: &std::path::Path) -> Result<(), TargetError> {
            Ok(())
        }
        fn check_prerequisites(&self) -> Vec<PrereqCheck> {
            Vec::new()
        }
    }

    fn ctx<'a>(target: &'a dyn Target) -> ValidateContext<'a> {
        ValidateContext {
            component: "nodejs",
            target,
            env: &[],
        }
    }

    #[tokio::test]
    async fn version_match_passes_for_compatible_version() {
        let t = ScriptedTarget::with(&["v22.5.1\n"]);
        let cfg = ValidateConfig {
            commands: vec![ValidateCommand {
                command: "node --version".into(),
                expected_output: None,
                version_match: Some(">=22.0.0".into()),
            }],
        };
        ValidateExecutor::new()
            .run(&cfg, &ctx(&t))
            .await
            .expect("v22.5.1 satisfies >=22.0.0");
    }

    #[tokio::test]
    async fn version_match_fails_for_incompatible_version() {
        let t = ScriptedTarget::with(&["v18.20.0\n"]);
        let cfg = ValidateConfig {
            commands: vec![ValidateCommand {
                command: "node --version".into(),
                expected_output: None,
                version_match: Some(">=22.0.0".into()),
            }],
        };
        let err = ValidateExecutor::new()
            .run(&cfg, &ctx(&t))
            .await
            .expect_err("v18.20.0 must not satisfy >=22.0.0");
        match err {
            ExtensionError::ValidateFailed {
                component, command, ..
            } => {
                assert_eq!(component, "nodejs");
                assert_eq!(command, "node --version");
            }
            other => panic!("expected ValidateFailed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn expected_output_substring_match() {
        let t = ScriptedTarget::with(&["hello world from sindri\n"]);
        let cfg = ValidateConfig {
            commands: vec![ValidateCommand {
                command: "echo hello".into(),
                expected_output: Some("from sindri".into()),
                version_match: None,
            }],
        };
        ValidateExecutor::new()
            .run(&cfg, &ctx(&t))
            .await
            .expect("substring should match");
    }

    #[tokio::test]
    async fn expected_output_failure_yields_validate_failed() {
        let t = ScriptedTarget::with(&["nope\n"]);
        let cfg = ValidateConfig {
            commands: vec![ValidateCommand {
                command: "echo nope".into(),
                expected_output: Some("yes".into()),
                version_match: None,
            }],
        };
        let err = ValidateExecutor::new()
            .run(&cfg, &ctx(&t))
            .await
            .expect_err("substring miss must fail");
        assert!(matches!(err, ExtensionError::ValidateFailed { .. }));
    }

    #[test]
    fn extract_semver_handles_v_prefix_and_trailing_text() {
        assert_eq!(extract_semver("v22.5.1\n").as_deref(), Some("22.5.1"));
        assert_eq!(extract_semver("v22.5.1").as_deref(), Some("22.5.1"));
        assert_eq!(
            extract_semver("Python 3.12.4 (...)\n").as_deref(),
            Some("3.12.4")
        );
        assert_eq!(extract_semver("garbage").as_deref(), None);
    }
}
