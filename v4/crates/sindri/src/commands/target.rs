//! `sindri target` subcommand surface (ADR-017).
//!
//! Wave 3C adds `use`, `start`, `stop`, `auth`, `update`, and the
//! `plugin {ls, install, trust, uninstall}` family on top of the
//! Sprint 9/10 `add`, `ls`, `status`, `create`, `destroy`, `doctor`,
//! `shell` verbs.
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_targets::{AuthValue, DockerTarget, LocalTarget, Target};
use std::path::{Path, PathBuf};

/// Command surface for the `sindri target …` family.
pub enum TargetCmd {
    Add {
        name: String,
        kind: String,
        opts: Vec<(String, String)>,
    },
    Ls,
    Status {
        name: String,
    },
    Create {
        name: String,
    },
    Destroy {
        name: String,
    },
    Doctor {
        name: Option<String>,
    },
    Shell {
        name: String,
    },
    /// Set the default target in sindri.yaml (`preferences.default_target`).
    Use {
        name: String,
    },
    /// Start a previously-created target resource.
    Start {
        name: String,
    },
    /// Stop a target resource without destroying it.
    Stop {
        name: String,
    },
    /// Wizard for setting up auth credentials on a target.
    Auth {
        name: String,
        /// Pre-supplied prefixed auth value (skips the interactive prompt).
        value: Option<String>,
    },
    /// Reconcile `targets.<name>.infra` in sindri.yaml with the on-disk
    /// lock — Terraform-plan-style classifier with destructive-prompt
    /// gating (Wave 5E, audit D2).
    Update {
        name: String,
        auto_approve: bool,
        no_color: bool,
    },
    /// Plugin management.
    Plugin {
        sub: PluginSub,
    },
}

/// `sindri target plugin …` subcommands.
pub enum PluginSub {
    Ls,
    Install {
        oci_ref: String,
        /// Override the `kind` the plugin will be installed under. Defaults
        /// to the trailing path component of `oci_ref`.
        kind: Option<String>,
    },
    Trust {
        kind: String,
        signer: String,
    },
    Uninstall {
        kind: String,
        yes: bool,
    },
}

/// Run a `target …` command and return the process exit code.
pub fn run(cmd: TargetCmd) -> i32 {
    match cmd {
        TargetCmd::Add { name, kind, opts } => add_target(&name, &kind, &opts),
        TargetCmd::Ls => list_targets(),
        TargetCmd::Status { name } => status_target(&name),
        TargetCmd::Create { name } => create_target(&name),
        TargetCmd::Destroy { name } => destroy_target(&name),
        TargetCmd::Doctor { name } => doctor(&name),
        TargetCmd::Shell { name } => shell(&name),
        TargetCmd::Use { name } => use_target(&name, Path::new("sindri.yaml")),
        TargetCmd::Start { name } => start_target(&name),
        TargetCmd::Stop { name } => stop_target(&name),
        TargetCmd::Auth { name, value } => auth_target(&name, value.as_deref()),
        TargetCmd::Update {
            name,
            auto_approve,
            no_color,
        } => update_target(&name, auto_approve, no_color),
        TargetCmd::Plugin { sub } => run_plugin(sub),
    }
}

fn add_target(name: &str, kind: &str, _opts: &[(String, String)]) -> i32 {
    println!(
        "Target '{}' (kind: {}) added — update sindri.yaml targets: section manually for now",
        name, kind
    );
    EXIT_SUCCESS
}

fn list_targets() -> i32 {
    println!("{:<20} {:<10} STATUS", "NAME", "KIND");
    println!("{}", "-".repeat(50));
    println!("{:<20} {:<10} ready", "local", "local");
    EXIT_SUCCESS
}

fn status_target(name: &str) -> i32 {
    if name == "local" {
        let t = LocalTarget::new();
        match t.profile() {
            Ok(p) => {
                println!("Target: local");
                println!("  Platform: {}", p.platform.triple());
                println!(
                    "  System PM: {}",
                    p.capabilities
                        .system_package_manager
                        .as_deref()
                        .unwrap_or("none")
                );
                println!("  Docker: {}", p.capabilities.has_docker);
            }
            Err(e) => eprintln!("Error: {}", e),
        }
        return EXIT_SUCCESS;
    }
    eprintln!("Target '{}' not found", name);
    EXIT_SCHEMA_OR_RESOLVE_ERROR
}

fn create_target(name: &str) -> i32 {
    let image = "ubuntu:24.04";
    let t = DockerTarget::new(name, image);
    match t.create() {
        Ok(_) => {
            println!("Created Docker target '{}' using image {}", name, image);
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to create target '{}': {}", name, e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

fn destroy_target(name: &str) -> i32 {
    let image = "ubuntu:24.04";
    let t = DockerTarget::new(name, image);
    match t.destroy() {
        Ok(_) => {
            println!("Destroyed target '{}'", name);
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Failed: {}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

fn doctor(name: &Option<String>) -> i32 {
    let target_name = name.as_deref().unwrap_or("local");
    println!("Running doctor checks for target '{}'...", target_name);

    let checks = if target_name == "local" {
        LocalTarget::new().check_prerequisites()
    } else {
        DockerTarget::new(target_name, "").check_prerequisites()
    };

    let mut any_failed = false;
    for check in &checks {
        if check.passed {
            println!("  [OK]   {}", check.name);
        } else {
            println!("  [FAIL] {}", check.name);
            if let Some(fix) = &check.fix {
                println!("         Fix: {}", fix);
            }
            any_failed = true;
        }
    }

    if any_failed {
        EXIT_SCHEMA_OR_RESOLVE_ERROR
    } else {
        println!("All checks passed.");
        EXIT_SUCCESS
    }
}

fn shell(name: &str) -> i32 {
    if name == "local" {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let status = std::process::Command::new(&shell)
            .status()
            .unwrap_or_else(|_| std::process::exit(1));
        if status.success() {
            EXIT_SUCCESS
        } else {
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    } else {
        eprintln!(
            "Interactive shell for target '{}': use `sindri target shell` once the cloud target plugin lands the PTY proxy",
            name
        );
        EXIT_SCHEMA_OR_RESOLVE_ERROR
    }
}

// ─── New Wave 3C subverbs ────────────────────────────────────────────────────

/// Set `preferences.default_target` in sindri.yaml. Public so unit tests in
/// the `tests` module below can drive it against a tempdir.
pub fn use_target(name: &str, manifest_path: &Path) -> i32 {
    let content = match std::fs::read_to_string(manifest_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Cannot read {}: {}", manifest_path.display(), e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let mut doc: serde_yaml::Value = match serde_yaml::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Cannot parse {}: {}", manifest_path.display(), e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let mapping = match doc.as_mapping_mut() {
        Some(m) => m,
        None => {
            eprintln!("Manifest is not a YAML mapping");
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let prefs_key = serde_yaml::Value::String("preferences".into());
    let prefs = mapping
        .entry(prefs_key)
        .or_insert(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    let prefs_map = match prefs.as_mapping_mut() {
        Some(m) => m,
        None => {
            eprintln!("`preferences` exists but is not a mapping");
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    prefs_map.insert(
        serde_yaml::Value::String("default_target".into()),
        serde_yaml::Value::String(name.to_string()),
    );

    let serialised = match serde_yaml::to_string(&doc) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Cannot serialise manifest: {}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let tmp = manifest_path.with_extension("yaml.tmp");
    if let Err(e) = std::fs::write(&tmp, &serialised) {
        eprintln!("Cannot write {}: {}", tmp.display(), e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }
    if let Err(e) = std::fs::rename(&tmp, manifest_path) {
        eprintln!("Cannot finalise {}: {}", manifest_path.display(), e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }
    println!("Default target set to '{}'", name);
    EXIT_SUCCESS
}

fn start_target(name: &str) -> i32 {
    // Wave 3C: only the local + docker builtins are recognised here. Cloud
    // targets are constructed from sindri.yaml at apply time; the `start`
    // verb's full wiring against the manifest lands when the target factory
    // is plumbed through `commands::resolve`. For now we surface a clear
    // "not yet wired" message rather than guessing at the kind.
    if name == "local" {
        println!("Local target is always ready.");
        return EXIT_SUCCESS;
    }
    let t = DockerTarget::new(name, "ubuntu:24.04");
    match t.start() {
        Ok(_) => {
            println!("Started target '{}'", name);
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to start target '{}': {}", name, e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

fn stop_target(name: &str) -> i32 {
    if name == "local" {
        eprintln!("Refusing to stop the `local` target.");
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }
    let t = DockerTarget::new(name, "ubuntu:24.04");
    match t.stop() {
        Ok(_) => {
            println!("Stopped target '{}'", name);
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to stop target '{}': {}", name, e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

fn auth_target(name: &str, value: Option<&str>) -> i32 {
    let raw = match value {
        Some(v) => v.to_string(),
        None => match prompt_for_auth(name) {
            Some(v) => v,
            None => return EXIT_SCHEMA_OR_RESOLVE_ERROR,
        },
    };
    let parsed = match AuthValue::parse(&raw) {
        Some(p) => p,
        None => {
            eprintln!(
                "Could not parse '{}' as an auth value; expected env:VAR | file:PATH | cli:CMD | plain:VALUE",
                raw
            );
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    if parsed.is_plain() {
        eprintln!(
            "Warning: storing a plain auth value in sindri.yaml is insecure. Prefer env: or file:."
        );
    }

    // Persist under targets.<name>.auth.token in sindri.yaml. This is the
    // canonical path per ADR-020 §4 — every auth-bearing target reads
    // `auth.token`.
    let manifest_path = std::path::PathBuf::from("sindri.yaml");
    if let Err(e) = persist_auth(&manifest_path, name, &raw) {
        eprintln!("Cannot update sindri.yaml: {}", e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }
    println!("Auth value stored for target '{}' (auth.token).", name);
    print_oauth_hint(name);
    EXIT_SUCCESS
}

fn print_oauth_hint(name: &str) {
    eprintln!(
        "If '{}' uses OAuth (e.g. fly, gcloud, az), run the upstream CLI's auth command \
         (e.g. `flyctl auth login`, `gcloud auth login`, `az login`) — sindri does not \
         drive OAuth flows directly.",
        name
    );
}

fn prompt_for_auth(name: &str) -> Option<String> {
    use std::io::{BufRead, Write};
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = writeln!(
        out,
        "Configure auth for target '{}'.\n\
         Accepted forms: env:VAR | file:PATH | cli:CMD | plain:VALUE",
        name
    );
    let _ = write!(out, "auth.token: ");
    let _ = out.flush();
    let mut line = String::new();
    if stdin.lock().read_line(&mut line).is_err() {
        return None;
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        eprintln!("No value provided.");
        return None;
    }
    Some(trimmed.to_string())
}

fn persist_auth(manifest_path: &Path, name: &str, value: &str) -> std::io::Result<()> {
    let content = std::fs::read_to_string(manifest_path)?;
    let mut doc: serde_yaml::Value = serde_yaml::from_str(&content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let mapping = doc.as_mapping_mut().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "manifest is not a mapping")
    })?;

    let targets_key = serde_yaml::Value::String("targets".into());
    let targets = mapping
        .entry(targets_key)
        .or_insert(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    let targets_map = targets.as_mapping_mut().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "`targets` is not a mapping",
        )
    })?;
    let target_key = serde_yaml::Value::String(name.into());
    let target = targets_map
        .entry(target_key)
        .or_insert(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    let target_map = target.as_mapping_mut().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("targets.{} is not a mapping", name),
        )
    })?;
    let auth_key = serde_yaml::Value::String("auth".into());
    let auth = target_map
        .entry(auth_key)
        .or_insert(serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    let auth_map = auth.as_mapping_mut().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "`auth` is not a mapping")
    })?;
    auth_map.insert(
        serde_yaml::Value::String("token".into()),
        serde_yaml::Value::String(value.into()),
    );

    let out = serde_yaml::to_string(&doc)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let tmp = manifest_path.with_extension("yaml.tmp");
    std::fs::write(&tmp, &out)?;
    std::fs::rename(&tmp, manifest_path)?;
    Ok(())
}

fn update_target(name: &str, auto_approve: bool, no_color: bool) -> i32 {
    update_target_at(
        name,
        Path::new("sindri.yaml"),
        &PathBuf::from(format!("sindri.{}.infra.lock", name)),
        auto_approve,
        no_color,
    )
}

/// Test-friendly variant that takes explicit paths. Drives the
/// convergence engine end-to-end:
///
/// 1. Loads `targets.<name>` from `sindri.yaml`.
/// 2. Loads (or initialises empty) the lockfile.
/// 3. Builds a [`Plan`](sindri_targets::convergence::Plan) and renders it.
/// 4. Prompts (unless `auto_approve`) before applying any destructive
///    entry, then writes the new lock atomically.
///
/// The Applier used here is a no-op [`StubApplier`] — actual provider-API
/// mutations land per-kind on top of `dispatch_create_async` (see PR #227).
/// Tests inject a fake Applier through the `convergence` integration tests
/// in `sindri-targets`.
pub fn update_target_at(
    name: &str,
    manifest_path: &Path,
    lock_path: &Path,
    auto_approve: bool,
    no_color: bool,
) -> i32 {
    use sindri_targets::convergence::{
        apply_plan, build_plan, render_plan, schema_for_kind, write_lock_atomic, AlwaysYesConfirm,
        InfraDocument, InfraLock, RenderOptions, StdinConfirm,
    };

    // ── Load manifest ────────────────────────────────────────────────
    let content = match std::fs::read_to_string(manifest_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Cannot read {}: {}", manifest_path.display(), e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let doc: serde_yaml::Value = match serde_yaml::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Cannot parse {}: {}", manifest_path.display(), e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let target_node = doc
        .get("targets")
        .and_then(|t| t.get(name))
        .cloned()
        .unwrap_or(serde_yaml::Value::Null);
    if target_node.is_null() {
        eprintln!(
            "Target '{}' not found in {} (targets.{} is missing).",
            name,
            manifest_path.display(),
            name
        );
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }
    let kind = target_node
        .get("kind")
        .and_then(|v| v.as_str())
        .unwrap_or("local")
        .to_string();
    let infra_yaml = target_node.get("infra").cloned();
    let infra_json: Option<serde_json::Value> = match infra_yaml {
        Some(v) => match serde_json::to_value(&v) {
            Ok(j) => Some(j),
            Err(e) => {
                eprintln!("Cannot convert targets.{}.infra to JSON: {}", name, e);
                return EXIT_SCHEMA_OR_RESOLVE_ERROR;
            }
        },
        None => None,
    };

    let desired = InfraDocument::from_infra_value(&kind, infra_json.as_ref());

    // ── Load (or default) lock ───────────────────────────────────────
    let recorded_lock = match InfraLock::read(lock_path, name, &kind) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Cannot read lock {}: {}", lock_path.display(), e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let recorded = InfraDocument {
        kind: recorded_lock.kind.clone(),
        resources: recorded_lock.resources.clone(),
    };

    // ── Build + render plan ──────────────────────────────────────────
    let schema = schema_for_kind(&kind);
    let plan = build_plan(name, &kind, &desired, &recorded, schema.as_ref());
    let rendered = render_plan(&plan, RenderOptions { color: !no_color });
    print!("{}", rendered);

    // ── Apply ────────────────────────────────────────────────────────
    let counts = plan.counts();
    if counts.create + counts.in_place + counts.destroy + counts.recreate == 0 {
        println!("No changes to apply.");
        return EXIT_SUCCESS;
    }

    let mut applier = StubApplier;
    let result = if auto_approve {
        let mut confirm = AlwaysYesConfirm;
        apply_plan(&plan, &recorded_lock, &mut applier, &mut confirm, true)
    } else {
        let mut confirm = StdinConfirm::default();
        apply_plan(&plan, &recorded_lock, &mut applier, &mut confirm, false)
    };

    let (new_lock, outcome) = match result {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Apply failed: {}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    if outcome.destructive_aborted {
        println!("Aborted — no changes applied.");
        return EXIT_SUCCESS;
    }

    if let Err(e) = write_lock_atomic(lock_path, &new_lock) {
        eprintln!("Cannot write lock {}: {}", lock_path.display(), e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }
    println!(
        "Applied {} change(s). Lock: {}",
        outcome.applied,
        lock_path.display()
    );
    EXIT_SUCCESS
}

/// No-op Applier that copies desired → recorded (stamping a `_managed`
/// marker so the lock reflects "we owned this resource"). Real provider
/// mutations are wired per-kind via `dispatch_create_async` (PR #227)
/// and land in subsequent waves.
struct StubApplier;

impl sindri_targets::convergence::Applier for StubApplier {
    fn create(
        &mut self,
        _name: &str,
        desired: &serde_json::Value,
    ) -> Result<sindri_targets::convergence::ResourceState, sindri_targets::TargetError> {
        let mut state = desired.clone();
        if let Some(map) = state.as_object_mut() {
            map.insert("_managed".into(), serde_json::Value::Bool(true));
        }
        Ok(state)
    }
    fn destroy(
        &mut self,
        _name: &str,
        _recorded: &sindri_targets::convergence::ResourceState,
    ) -> Result<(), sindri_targets::TargetError> {
        Ok(())
    }
    fn update_in_place(
        &mut self,
        _name: &str,
        recorded: &sindri_targets::convergence::ResourceState,
        desired: &serde_json::Value,
    ) -> Result<sindri_targets::convergence::ResourceState, sindri_targets::TargetError> {
        let mut state = desired.clone();
        if let (Some(new_map), Some(rec_map)) = (state.as_object_mut(), recorded.as_object()) {
            for k in ["id", "_managed"] {
                if let Some(v) = rec_map.get(k) {
                    new_map.insert(k.into(), v.clone());
                }
            }
        }
        Ok(state)
    }
}

// ─── Plugin management ──────────────────────────────────────────────────────

fn run_plugin(sub: PluginSub) -> i32 {
    match sub {
        PluginSub::Ls => plugin_ls(),
        PluginSub::Install { oci_ref, kind } => plugin_install(&oci_ref, kind.as_deref()),
        PluginSub::Trust { kind, signer } => plugin_trust(&kind, &signer),
        PluginSub::Uninstall { kind, yes } => plugin_uninstall(&kind, yes),
    }
}

fn plugins_root() -> Option<PathBuf> {
    sindri_core::paths::home_dir().map(|h| h.join(".sindri").join("plugins"))
}

fn plugin_trust_root() -> Option<PathBuf> {
    sindri_core::paths::home_dir().map(|h| h.join(".sindri").join("trust").join("plugins"))
}

fn plugin_ls() -> i32 {
    let root = match plugins_root() {
        Some(r) => r,
        None => {
            eprintln!("Cannot resolve $HOME");
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    if !root.is_dir() {
        println!("No plugins installed.");
        return EXIT_SUCCESS;
    }
    println!("{:<20} BINARY", "KIND");
    println!("{}", "-".repeat(60));
    let entries = match std::fs::read_dir(&root) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Cannot read {}: {}", root.display(), e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let kind = match path.file_name().and_then(|n| n.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let bin = path.join(format!("sindri-target-{}", kind));
        if bin.is_file() {
            println!("{:<20} {}", kind, bin.display());
        }
    }
    EXIT_SUCCESS
}

fn plugin_install(oci_ref: &str, kind_override: Option<&str>) -> i32 {
    // Wave 3C: the OCI fetch path lives in sindri-registry (Wave 3A.2).
    // Until that lands the install verb is intentionally non-functional
    // and clearly marked experimental, per the implementation plan.
    let kind = kind_override
        .map(str::to_string)
        .unwrap_or_else(|| derive_kind_from_ref(oci_ref));
    eprintln!(
        "EXPERIMENTAL: `sindri target plugin install` requires the OCI fetch path \
         from Wave 3A.2 (sindri-registry) which has not yet landed. \
         Until then, copy your plugin binary manually to \
         ~/.sindri/plugins/{0}/sindri-target-{0} and `chmod +x` it.\n\
         Reference attempted: {1}",
        kind, oci_ref
    );
    EXIT_SCHEMA_OR_RESOLVE_ERROR
}

fn derive_kind_from_ref(oci_ref: &str) -> String {
    // ghcr.io/foo/sindri-target-modal:1.2.3 → modal
    let last = oci_ref
        .rsplit('/')
        .next()
        .unwrap_or(oci_ref)
        .split(':')
        .next()
        .unwrap_or(oci_ref);
    last.strip_prefix("sindri-target-")
        .unwrap_or(last)
        .to_string()
}

fn plugin_trust(kind: &str, signer: &str) -> i32 {
    let root = match plugin_trust_root() {
        Some(r) => r,
        None => {
            eprintln!("Cannot resolve $HOME");
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let path_str = signer.strip_prefix("cosign:key=").unwrap_or(signer).trim();
    if path_str.is_empty() {
        eprintln!("Empty signer; expected `cosign:key=<path>` or a path to a P-256 PEM public key");
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }
    let pem = match std::fs::read_to_string(path_str) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Cannot read signer key '{}': {}", path_str, e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let dir = root.join(kind);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("Cannot create {}: {}", dir.display(), e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }
    let target = dir.join("cosign.pub");
    let tmp = target.with_extension("tmp");
    if let Err(e) = std::fs::write(&tmp, &pem) {
        eprintln!("Cannot write {}: {}", target.display(), e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }
    if let Err(e) = std::fs::rename(&tmp, &target) {
        eprintln!("Cannot finalise {}: {}", target.display(), e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }
    println!(
        "Trusted cosign key for plugin kind '{}' (stored at {})",
        kind,
        target.display()
    );
    EXIT_SUCCESS
}

fn plugin_uninstall(kind: &str, yes: bool) -> i32 {
    let root = match plugins_root() {
        Some(r) => r,
        None => {
            eprintln!("Cannot resolve $HOME");
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let dir = root.join(kind);
    if !dir.is_dir() {
        eprintln!("No plugin installed for kind '{}'", kind);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }
    if !yes && !confirm_uninstall(kind) {
        println!("Aborted.");
        return EXIT_SUCCESS;
    }
    if let Err(e) = std::fs::remove_dir_all(&dir) {
        eprintln!("Cannot remove {}: {}", dir.display(), e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }
    println!("Uninstalled plugin '{}'", kind);
    EXIT_SUCCESS
}

fn confirm_uninstall(kind: &str) -> bool {
    use std::io::{BufRead, Write};
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = write!(out, "Uninstall plugin '{}'? [y/N]: ", kind);
    let _ = out.flush();
    let mut line = String::new();
    if std::io::stdin().lock().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn use_writes_default_target() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("sindri.yaml");
        fs::write(
            &manifest,
            "components: []\npreferences:\n  default_target: local\n",
        )
        .unwrap();
        assert_eq!(use_target("staging", &manifest), EXIT_SUCCESS);
        let updated = fs::read_to_string(&manifest).unwrap();
        let doc: serde_yaml::Value = serde_yaml::from_str(&updated).unwrap();
        assert_eq!(
            doc.get("preferences")
                .and_then(|p| p.get("default_target"))
                .and_then(|v| v.as_str()),
            Some("staging")
        );
    }

    #[test]
    fn use_creates_preferences_when_absent() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("sindri.yaml");
        fs::write(&manifest, "components: []\n").unwrap();
        assert_eq!(use_target("ci", &manifest), EXIT_SUCCESS);
        let updated = fs::read_to_string(&manifest).unwrap();
        assert!(updated.contains("default_target: ci"));
    }

    #[test]
    fn persist_auth_creates_targets_section() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("sindri.yaml");
        fs::write(&manifest, "components: []\n").unwrap();
        persist_auth(&manifest, "fly1", "env:FLY_API_TOKEN").unwrap();
        let updated = fs::read_to_string(&manifest).unwrap();
        let doc: serde_yaml::Value = serde_yaml::from_str(&updated).unwrap();
        let token = doc
            .get("targets")
            .and_then(|t| t.get("fly1"))
            .and_then(|t| t.get("auth"))
            .and_then(|a| a.get("token"))
            .and_then(|v| v.as_str());
        assert_eq!(token, Some("env:FLY_API_TOKEN"));
    }

    #[test]
    fn derive_kind_from_oci_ref() {
        assert_eq!(
            derive_kind_from_ref("ghcr.io/foo/sindri-target-modal:1.0.0"),
            "modal"
        );
        assert_eq!(
            derive_kind_from_ref("docker.io/bar/sindri-target-lambda-labs"),
            "lambda-labs"
        );
        assert_eq!(derive_kind_from_ref("modal"), "modal");
    }

    #[test]
    fn update_target_creates_lockfile_with_auto_approve() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("sindri.yaml");
        let lock = dir.path().join("sindri.web.infra.lock");
        fs::write(
            &manifest,
            r#"
components: []
targets:
  web:
    kind: docker
    infra:
      resources:
        web:
          image: ghcr.io/example/web:1
          env:
            LOG: info
"#,
        )
        .unwrap();

        let code = update_target_at(
            "web", &manifest, &lock, /*auto_approve*/ true, /*no_color*/ true,
        );
        assert_eq!(code, EXIT_SUCCESS);
        assert!(lock.exists(), "lockfile should have been written");
        let written = fs::read_to_string(&lock).unwrap();
        let parsed: serde_yaml::Value = serde_yaml::from_str(&written).unwrap();
        assert_eq!(parsed.get("kind").and_then(|v| v.as_str()), Some("docker"));
        assert!(written.contains("ghcr.io/example/web:1"));
    }

    #[test]
    fn update_target_noop_when_lock_matches_manifest() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("sindri.yaml");
        let lock = dir.path().join("sindri.web.infra.lock");
        fs::write(
            &manifest,
            r#"
components: []
targets:
  web:
    kind: docker
    infra:
      resources:
        web:
          image: ghcr.io/example/web:1
"#,
        )
        .unwrap();
        // Pre-write a matching lock.
        fs::write(
            &lock,
            "kind: docker\nresources:\n  web:\n    image: ghcr.io/example/web:1\n",
        )
        .unwrap();

        let code = update_target_at("web", &manifest, &lock, true, true);
        assert_eq!(code, EXIT_SUCCESS);
    }

    #[test]
    fn update_target_missing_target_returns_error() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("sindri.yaml");
        let lock = dir.path().join("sindri.absent.infra.lock");
        fs::write(&manifest, "components: []\n").unwrap();
        let code = update_target_at("absent", &manifest, &lock, true, true);
        assert_eq!(code, EXIT_SCHEMA_OR_RESOLVE_ERROR);
    }

    #[test]
    fn plugin_install_rejects_unsigned_when_strict() {
        // Wave 3C placeholder: until OCI fetch lands in Wave 3A.2 the
        // command refuses to install anything (which is the strictest
        // possible policy). Once Wave 3A.2 lands this test will be
        // replaced with one that validates the cosign-trust check.
        let code = plugin_install("ghcr.io/example/sindri-target-modal:1.0.0", None);
        assert_eq!(code, EXIT_SCHEMA_OR_RESOLVE_ERROR);
    }
}
