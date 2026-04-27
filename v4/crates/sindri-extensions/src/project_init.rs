//! Project-init executor (Sprint 4 §4.3, ADR-024).
//!
//! [`ProjectInitExecutor`] orchestrates `capabilities.project_init` steps
//! across the resolved component closure. Steps are ordered by their declared
//! `priority` (ascending — lower priorities run first); ties are **broken by
//! the order the component appears in the input slice**, NOT alphabetically.
//! Callers are expected to feed the resolution closure in topological order so
//! that tie-broken priority matches dependency order.
//!
//! ## State markers
//!
//! Each step that completes successfully is recorded in
//! `<workdir>/.sindri/project-init.state.json`, a JSON document of the shape:
//!
//! ```json
//! { "completed": ["nodejs:0001-bootstrap", "nodejs:0002-link-bin"] }
//! ```
//!
//! Marker keys are `"<component-name>:<priority:04>-<command-hash-hex8>"`. On
//! re-run, any step whose marker key already exists is skipped without
//! re-dispatching the command.
//!
//! Failure of a step does **not** mark it complete — the executor returns
//! [`ExtensionError::ProjectInitFailed`] and the apply pipeline aborts. The
//! caller decides retry policy.

use crate::error::ExtensionError;
use sindri_core::component::{ComponentId, ProjectInitStep};
use sindri_targets::Target;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Stable reference to a component for state attribution.
///
/// We carry both the address ([`ComponentId`]) and the human-facing name so
/// the executor can attribute steps without re-resolving the component.
#[derive(Debug, Clone)]
pub struct ComponentRef {
    /// Backend-addressed identifier.
    pub component_id: ComponentId,
    /// Component metadata name (used in marker keys and error messages).
    pub name: String,
}

/// Context for a project-init run.
pub struct ProjectInitContext<'a> {
    /// Active target for command dispatch.
    pub target: &'a dyn Target,
    /// Project working directory; the state file lives at
    /// `<workdir>/.sindri/project-init.state.json`.
    pub workdir: &'a Path,
    /// Environment variables to expose to each step.
    pub env: &'a [(&'a str, &'a str)],
}

/// Capability executor for `capabilities.project_init` steps.
#[derive(Debug, Default, Clone, Copy)]
pub struct ProjectInitExecutor;

/// On-disk state document.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct StateFile {
    completed: BTreeSet<String>,
}

impl ProjectInitExecutor {
    /// Create a new executor.
    pub fn new() -> Self {
        Self
    }

    /// Run all project-init steps for the given (component, step) pairs.
    ///
    /// Steps are sorted by `step.priority` ascending; ties preserve input
    /// order (stable sort).
    pub async fn run(
        &self,
        steps: &[(ComponentRef, &ProjectInitStep)],
        ctx: &ProjectInitContext<'_>,
    ) -> Result<(), ExtensionError> {
        let state_path = state_path(ctx.workdir);
        let mut state = load_state(&state_path)?;

        // Stable sort: priority ascending, ties preserve input order.
        let mut ordered: Vec<(usize, &(ComponentRef, &ProjectInitStep))> =
            steps.iter().enumerate().collect();
        ordered.sort_by_key(|(idx, (_, step))| (step.priority, *idx));

        for (_idx, (cref, step)) in ordered {
            let key = marker_key(&cref.name, step);
            if state.completed.contains(&key) {
                tracing::debug!(
                    component = cref.name.as_str(),
                    priority = step.priority,
                    key = key.as_str(),
                    "project-init step already complete; skipping"
                );
                continue;
            }

            tracing::info!(
                component = cref.name.as_str(),
                priority = step.priority,
                command = step.command.as_str(),
                "running project-init step"
            );

            match ctx.target.exec(&step.command, ctx.env) {
                Ok(_) => {
                    state.completed.insert(key);
                    save_state(&state_path, &state)?;
                }
                Err(err) => {
                    return Err(ExtensionError::ProjectInitFailed {
                        component: cref.name.clone(),
                        priority: step.priority,
                        command: step.command.clone(),
                        detail: err.to_string(),
                    });
                }
            }
        }

        Ok(())
    }
}

/// Compute the on-disk path for the state file.
fn state_path(workdir: &Path) -> PathBuf {
    workdir.join(".sindri").join("project-init.state.json")
}

/// Marker key for a single step. Stable across reruns.
///
/// Format: `{component}:{priority:04}-{fnv8(command)}`. We hash the command
/// (rather than embedding it raw) so keys stay short and filesystem-safe even
/// if a step's command contains spaces, quotes, or path separators.
fn marker_key(component: &str, step: &ProjectInitStep) -> String {
    let h = fnv1a_64(step.command.as_bytes());
    format!("{component}:{:04}-{:016x}", step.priority, h)
}

/// FNV-1a 64-bit. Small dependency-free hash sufficient for marker keys
/// (collision resistance is not security-critical here — it only disambiguates
/// two steps with the same priority on the same component).
fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn load_state(path: &Path) -> Result<StateFile, ExtensionError> {
    if !path.exists() {
        return Ok(StateFile::default());
    }
    let bytes = std::fs::read(path)?;
    if bytes.is_empty() {
        return Ok(StateFile::default());
    }
    let state: StateFile = serde_json::from_slice(&bytes)?;
    Ok(state)
}

fn save_state(path: &Path, state: &StateFile) -> Result<(), ExtensionError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(state)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::component::{Backend, ComponentId};
    use sindri_core::platform::TargetProfile;
    use sindri_targets::error::TargetError;
    use sindri_targets::traits::PrereqCheck;
    use std::sync::Mutex;
    use tempfile::TempDir;

    struct RecordingTarget {
        commands: Mutex<Vec<String>>,
    }

    impl RecordingTarget {
        fn new() -> Self {
            Self {
                commands: Mutex::new(Vec::new()),
            }
        }
        fn captured(&self) -> Vec<String> {
            self.commands.lock().unwrap().clone()
        }
    }

    impl Target for RecordingTarget {
        fn name(&self) -> &str {
            "rec"
        }
        fn kind(&self) -> &str {
            "rec"
        }
        fn profile(&self) -> Result<TargetProfile, TargetError> {
            Err(TargetError::Unavailable {
                name: "rec".into(),
                reason: "test".into(),
            })
        }
        fn exec(&self, cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
            self.commands.lock().unwrap().push(cmd.to_string());
            Ok((String::new(), String::new()))
        }
        fn upload(&self, _local: &std::path::Path, _remote: &str) -> Result<(), TargetError> {
            Ok(())
        }
        fn download(&self, _remote: &str, _local: &std::path::Path) -> Result<(), TargetError> {
            Ok(())
        }
        fn check_prerequisites(&self) -> Vec<PrereqCheck> {
            Vec::new()
        }
    }

    fn cref(name: &str) -> ComponentRef {
        ComponentRef {
            component_id: ComponentId {
                backend: Backend::Mise,
                name: name.into(),
            },
            name: name.into(),
        }
    }

    #[tokio::test]
    async fn priority_order_respected() {
        let tmp = TempDir::new().unwrap();
        let target = RecordingTarget::new();
        let s30 = ProjectInitStep {
            command: "echo thirty".into(),
            priority: 30,
        };
        let s10 = ProjectInitStep {
            command: "echo ten".into(),
            priority: 10,
        };
        let s20 = ProjectInitStep {
            command: "echo twenty".into(),
            priority: 20,
        };
        let c = cref("nodejs");
        let steps = vec![(c.clone(), &s30), (c.clone(), &s10), (c.clone(), &s20)];
        let ctx = ProjectInitContext {
            target: &target,
            workdir: tmp.path(),
            env: &[],
        };
        ProjectInitExecutor::new()
            .run(&steps, &ctx)
            .await
            .expect("run should succeed");
        assert_eq!(
            target.captured(),
            vec![
                "echo ten".to_string(),
                "echo twenty".to_string(),
                "echo thirty".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn completed_steps_skipped_on_rerun() {
        let tmp = TempDir::new().unwrap();
        let target = RecordingTarget::new();
        let step = ProjectInitStep {
            command: "echo once".into(),
            priority: 10,
        };
        let c = cref("nodejs");
        let steps = vec![(c.clone(), &step)];
        let ctx = ProjectInitContext {
            target: &target,
            workdir: tmp.path(),
            env: &[],
        };
        ProjectInitExecutor::new().run(&steps, &ctx).await.unwrap();
        ProjectInitExecutor::new().run(&steps, &ctx).await.unwrap();
        // Second run should not re-dispatch the command.
        assert_eq!(target.captured(), vec!["echo once".to_string()]);
        // State file exists.
        assert!(tmp.path().join(".sindri/project-init.state.json").exists());
    }
}
