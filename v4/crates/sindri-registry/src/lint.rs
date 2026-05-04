use crate::error::RegistryError;
use sindri_core::component::ComponentManifest;
use std::fs;
use std::path::Path;

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
        LintResult {
            passed: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
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
    let manifest: ComponentManifest =
        serde_yaml::from_str(&content).map_err(|e| RegistryError::SchemaError(e.to_string()))?;

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
                message: "`binary` components must have `checksums` for all listed platforms"
                    .into(),
                fix: Some("Run `sindri registry fetch-checksums` to populate checksums".into()),
            });
        }
    }

    // collision-handling path prefix — Gate 4 (ADR-008). The rule lives in
    // `sindri-policy::capability_trust` and is also evaluated at resolve
    // time by `sindri-resolver::admission`. The lint sees the file in
    // isolation (no source registry), so we pass `sindri/core` to admit the
    // `:shared` escape hatch — the resolve-time gate is the authority that
    // rejects `:shared` from non-core registries.
    let prefix = manifest
        .capabilities
        .collision_handling
        .as_ref()
        .map(|c| c.path_prefix.as_str());
    if let Err(violation) = sindri_policy::check_collision_prefix(
        manifest.metadata.name.as_str(),
        sindri_core::registry::CORE_REGISTRY_NAME,
        prefix,
    ) {
        errors.push(LintDiagnostic {
            code: "LINT_COLLISION_PREFIX".into(),
            message: violation.message(),
            fix: Some(violation.fix().into()),
        });
    }

    // description empty is a warning, not error
    if manifest.metadata.description.trim().is_empty() {
        warnings.push(LintDiagnostic {
            code: "LINT_MISSING_DESCRIPTION".into(),
            message: "`metadata.description` is recommended".into(),
            fix: None,
        });
    }

    // Lifecycle hooks (ADR-030) — three publish-time warnings. The
    // dispatcher's runtime contract gate is the actual enforcement
    // boundary; these surface issues early so authors notice before
    // a `sindri apply` user does.
    let pkg_root = path.parent().unwrap_or(Path::new("."));
    if let Some(hooks) = manifest.capabilities.hooks.as_ref() {
        let phases = [
            ("pre-install", hooks.pre_install.as_ref()),
            ("install", hooks.install.as_ref()),
            ("post-install", hooks.post_install.as_ref()),
            ("configure", hooks.configure.as_ref()),
            ("validate", hooks.validate.as_ref()),
            ("upgrade", hooks.upgrade.as_ref()),
            ("uninstall", hooks.uninstall.as_ref()),
            ("project-init", hooks.project_init.as_ref()),
        ];
        for (phase, sref) in phases {
            let Some(sref) = sref else { continue };
            if let Some(rel_sh) = sref.sh.as_ref() {
                lint_hook_sh(pkg_root, phase, rel_sh, &mut warnings);
            }
        }
    }

    let passed = errors.is_empty();
    Ok(LintResult {
        passed,
        errors,
        warnings,
    })
}

/// Three lifecycle-hook warnings per ADR-030 §"Lint rules":
///   * LINT_HOOK_MISSING_SHEBANG       — script doesn't start with `#!/usr/bin/env bash` (or `#!/bin/bash`).
///   * LINT_HOOK_NON_EXECUTABLE        — script lacks any +x bit on POSIX.
///   * LINT_HOOK_MISSING_HELPERS_SOURCE — script doesn't source `sindri-helpers.sh`.
fn lint_hook_sh(pkg_root: &Path, phase: &str, rel_sh: &Path, warnings: &mut Vec<LintDiagnostic>) {
    let abs = pkg_root.join(rel_sh);
    let display = rel_sh.display().to_string();
    let Ok(content) = fs::read_to_string(&abs) else {
        // Missing file is the dispatcher's job to surface at runtime;
        // the lint focuses on present-but-malformed scripts.
        return;
    };
    let first_line = content.lines().next().unwrap_or("");
    if !first_line.starts_with("#!/usr/bin/env bash")
        && !first_line.starts_with("#!/bin/bash")
        && !first_line.starts_with("#!/usr/bin/bash")
    {
        warnings.push(LintDiagnostic {
            code: "LINT_HOOK_MISSING_SHEBANG".into(),
            message: format!(
                "{} hook `{}` should start with `#!/usr/bin/env bash`",
                phase, display
            ),
            fix: Some("Add `#!/usr/bin/env bash` as the first line".into()),
        });
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&abs) {
            let mode = meta.permissions().mode();
            if mode & 0o111 == 0 {
                warnings.push(LintDiagnostic {
                    code: "LINT_HOOK_NON_EXECUTABLE".into(),
                    message: format!("{} hook `{}` is not executable", phase, display),
                    fix: Some(format!("Run `chmod +x {}`", display)),
                });
            }
        }
    }

    if !content.contains("sindri-helpers.sh") {
        warnings.push(LintDiagnostic {
            code: "LINT_HOOK_MISSING_HELPERS_SOURCE".into(),
            message: format!(
                "{} hook `{}` doesn't source `sindri-helpers.sh`",
                phase, display
            ),
            fix: Some(
                "Add `. \"$(dirname \"$0\")/../../../support/scripts/sindri-helpers.sh\"; sindri::init`"
                    .into(),
            ),
        });
    }
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
    Ok(LintResult {
        passed,
        errors: all_errors,
        warnings: all_warnings,
    })
}
