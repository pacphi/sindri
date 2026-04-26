use std::fs;
use std::path::Path;
use crate::error::RegistryError;
use sindri_core::component::ComponentManifest;

/// A lint error or warning for a component
#[derive(Debug, Clone)]
pub struct LintDiagnostic {
    pub code: String,
    pub message: String,
    pub fix: Option<String>,
}

/// Lint result for a single component or directory
#[derive(Debug)]
pub struct LintResult {
    pub passed: bool,
    pub errors: Vec<LintDiagnostic>,
    pub warnings: Vec<LintDiagnostic>,
}

impl LintResult {
    pub fn ok() -> Self {
        LintResult { passed: true, errors: Vec::new(), warnings: Vec::new() }
    }
}

/// Lint a component.yaml path or directory of components (ADR-008 Gate 4 enforced)
pub fn lint_path(path: &Path) -> Result<LintResult, RegistryError> {
    if path.is_dir() {
        return lint_directory(path);
    }
    lint_file(path)
}

fn lint_file(path: &Path) -> Result<LintResult, RegistryError> {
    let content = fs::read_to_string(path)?;
    let manifest: ComponentManifest = serde_yaml::from_str(&content)
        .map_err(|e| RegistryError::SchemaError(e.to_string()))?;

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // platforms must not be empty
    if manifest.platforms.is_empty() {
        errors.push(LintDiagnostic {
            code: "LINT_EMPTY_PLATFORMS".into(),
            message: "`platforms` must not be empty".into(),
            fix: Some("Add at least one platform entry".into()),
        });
    }

    // license must be a non-empty SPDX identifier
    if manifest.metadata.license.trim().is_empty() {
        errors.push(LintDiagnostic {
            code: "LINT_MISSING_LICENSE".into(),
            message: "`metadata.license` must be a valid SPDX identifier".into(),
            fix: Some("Add e.g. `license: MIT`".into()),
        });
    }

    // binary components must have checksums for every platform
    if let Some(binary) = &manifest.install.binary {
        if binary.checksums.is_empty() {
            errors.push(LintDiagnostic {
                code: "LINT_MISSING_CHECKSUMS".into(),
                message: "`binary` components must have `checksums` for all listed platforms".into(),
                fix: Some("Run `sindri registry fetch-checksums` to populate checksums".into()),
            });
        }
    }

    // collision-handling path prefix must start with `<component-name>/` (ADR-008 Gate 4)
    if let Some(cap) = &manifest.capabilities.collision_handling {
        let expected = format!("{}/", manifest.metadata.name);
        if !cap.path_prefix.starts_with(&expected) && cap.path_prefix != ":shared" {
            errors.push(LintDiagnostic {
                code: "LINT_COLLISION_PREFIX".into(),
                message: format!(
                    "`collision_handling.path_prefix` must start with `{}/`",
                    manifest.metadata.name
                ),
                fix: Some(format!(
                    "Change path_prefix to `{}/...`",
                    manifest.metadata.name
                )),
            });
        }
    }

    // description empty is a warning, not error
    if manifest.metadata.description.trim().is_empty() {
        warnings.push(LintDiagnostic {
            code: "LINT_MISSING_DESCRIPTION".into(),
            message: "`metadata.description` is recommended".into(),
            fix: None,
        });
    }

    let passed = errors.is_empty();
    Ok(LintResult { passed, errors, warnings })
}

fn lint_directory(dir: &Path) -> Result<LintResult, RegistryError> {
    let mut all_errors = Vec::new();
    let mut all_warnings = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "yaml").unwrap_or(false) {
            let result = lint_file(&path)?;
            all_errors.extend(result.errors);
            all_warnings.extend(result.warnings);
        }
    }

    let passed = all_errors.is_empty();
    Ok(LintResult { passed, errors: all_errors, warnings: all_warnings })
}
