use sindri_core::auth::{AuthBindingStatus, AuthCapability, AuthSource};
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_targets::{DockerTarget, LocalTarget, Target};

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
    /// `target auth <name>` — inspect and manage per-target auth.
    /// Phase 5 (ADR-027 §Phase 5).
    Auth(AuthSubArgs),
}

/// Arguments for the `target auth` subverb (Phase 5).
pub struct AuthSubArgs {
    /// Target name (key in `sindri.yaml.targets`).
    pub name: String,
    /// `--bind <req-id>`: write a `provides:` entry into the target
    /// manifest based on a previously-considered-but-rejected candidate
    /// (the `<req-id>` is the binding `id` shown by `auth show`).
    pub bind: Option<String>,
    /// Manifest path. Defaults to `sindri.yaml`.
    pub manifest: String,
    /// Override target lockfile (defaults to derived from `name`).
    pub target: String,
    /// Non-interactive: when `--bind` is set, choose this capability_id
    /// from the considered list automatically rather than prompting.
    pub capability_id: Option<String>,
    /// Override audience for the new `provides:` entry (defaults to the
    /// requirement's audience).
    pub audience: Option<String>,
    /// Override priority for the new `provides:` entry (defaults to 50).
    pub priority: Option<i32>,
    /// JSON output.
    pub json: bool,
}

pub fn run(cmd: TargetCmd) -> i32 {
    match cmd {
        TargetCmd::Add { name, kind, opts } => add_target(&name, &kind, &opts),
        TargetCmd::Ls => list_targets(),
        TargetCmd::Status { name } => status_target(&name),
        TargetCmd::Create { name } => create_target(&name),
        TargetCmd::Destroy { name } => destroy_target(&name),
        TargetCmd::Doctor { name } => doctor(&name),
        TargetCmd::Shell { name } => shell(&name),
        TargetCmd::Auth(args) => run_auth(args),
    }
}

fn add_target(name: &str, kind: &str, _opts: &[(String, String)]) -> i32 {
    // Sprint 9: write to sindri.yaml targets: section
    // Full implementation requires manifest read/write
    println!(
        "Target '{}' (kind: {}) added — update sindri.yaml targets: section manually for now",
        name, kind
    );
    EXIT_SUCCESS
}

fn list_targets() -> i32 {
    // Sprint 9: show local as the always-present default (ADR-023)
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
    // Sprint 9: only docker targets are provisionable here
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

// =============================================================================
// `target auth <name>` — Phase 5 (ADR-027 §Phase 5)
// =============================================================================

/// Run `sindri target auth <name>` (Phase 5).
///
/// Without `--bind`, prints the per-target `provides:` capability list
/// from the manifest. With `--bind <req-id>`, looks up the binding by
/// id in the lockfile and writes a `provides:` entry into the manifest
/// derived from one of its considered-but-rejected candidates.
pub fn run_auth(args: AuthSubArgs) -> i32 {
    let manifest_path = std::path::PathBuf::from(&args.manifest);
    let bom_result = crate::commands::manifest::load_manifest(&args.manifest);
    let mut bom = match bom_result {
        Ok((m, _)) => m,
        Err(e) => {
            eprintln!("Cannot load manifest '{}': {}", args.manifest, e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    if !bom.targets.contains_key(&args.name) && !bom.targets.contains_key(&args.target) {
        if args.json {
            println!(r#"{{"error":"TARGET_NOT_FOUND","target":"{}"}}"#, args.name);
        } else {
            eprintln!("Target '{}' not found in {}", args.name, args.manifest);
        }
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }
    let key = if bom.targets.contains_key(&args.name) {
        args.name.clone()
    } else {
        args.target.clone()
    };

    if let Some(req_id) = &args.bind {
        return run_auth_bind(&mut bom, &manifest_path, &key, req_id, &args);
    }

    // Inspection mode: print the existing provides: list.
    let target_cfg = bom.targets.get(&key).expect("checked above");
    let provides = &target_cfg.provides;
    if args.json {
        let payload = serde_json::json!({
            "target": key,
            "kind": target_cfg.kind,
            "provides": provides,
        });
        match serde_json::to_string_pretty(&payload) {
            Ok(s) => println!("{}", s),
            Err(_) => println!("{{}}"),
        }
    } else {
        println!("target '{}' (kind: {})", key, target_cfg.kind);
        if provides.is_empty() {
            println!("  no `provides:` entries declared.");
            println!("  Add one with: sindri target auth {} --bind <req-id>", key);
        } else {
            println!(
                "  {:<20} {:<20} {:<30} PRIORITY",
                "ID", "AUDIENCE", "SOURCE"
            );
            for c in provides {
                println!(
                    "  {:<20} {:<20} {:<30} {}",
                    c.id,
                    c.audience,
                    describe_source(&c.source),
                    c.priority,
                );
            }
        }
    }
    EXIT_SUCCESS
}

fn run_auth_bind(
    bom: &mut sindri_core::manifest::BomManifest,
    manifest_path: &std::path::Path,
    target_name: &str,
    req_id: &str,
    args: &AuthSubArgs,
) -> i32 {
    // Locate the binding in the per-target lockfile.
    let lockfile_path = if target_name == "local" {
        manifest_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("sindri.lock")
    } else {
        manifest_path
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join(format!("sindri.{}.lock", target_name))
    };
    let content = match std::fs::read_to_string(&lockfile_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Cannot read lockfile '{}': {}", lockfile_path.display(), e);
            eprintln!("Hint: run `sindri resolve` first.");
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };
    let lockfile: sindri_core::lockfile::Lockfile = match serde_json::from_str(&content) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Malformed lockfile: {}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let binding = lockfile
        .auth_bindings
        .iter()
        .find(|b| b.id == req_id || b.requirement == req_id);
    let binding = match binding {
        Some(b) => b,
        None => {
            eprintln!(
                "No binding with id or requirement-name '{}' on target '{}'.",
                req_id, target_name
            );
            eprintln!(
                "Tip: `sindri auth show --target {}` lists all bindings.",
                target_name
            );
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    if binding.status == AuthBindingStatus::Bound {
        eprintln!(
            "Binding '{}' is already Bound (source: {}). Nothing to do.",
            binding.id,
            binding
                .source
                .as_ref()
                .map(describe_source)
                .unwrap_or_else(|| "—".into())
        );
        return EXIT_SUCCESS;
    }

    if binding.considered.is_empty() {
        eprintln!(
            "Binding '{}' has no considered-but-rejected candidates to bind.",
            binding.id
        );
        eprintln!(
            "Hint: declare a source via `targets.{}.provides:` directly, or set the \
             component requirement `optional: true`.",
            target_name
        );
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    // Choose a candidate: --capability-id wins, else if exactly one
    // candidate exists pick it, else error.
    let chosen_idx = if let Some(cid) = &args.capability_id {
        binding
            .considered
            .iter()
            .position(|r| r.capability_id == *cid)
    } else if binding.considered.len() == 1 {
        Some(0)
    } else {
        None
    };
    let chosen = match chosen_idx {
        Some(i) => &binding.considered[i],
        None => {
            eprintln!(
                "Binding '{}' has {} considered candidates; pass --capability-id to choose.",
                binding.id,
                binding.considered.len()
            );
            for r in &binding.considered {
                eprintln!("  - {} ({}): {}", r.capability_id, r.source_kind, r.reason);
            }
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    // Synthesise an AuthSource from the rejected candidate's
    // `source_kind` discriminant. The candidate carries no parameters
    // (we only persisted the kind), so we pick safe defaults that the
    // user is expected to edit. The `--bind` flow's value is in writing
    // a *valid* skeleton; users tweak fields after.
    let new_source = source_from_kind(&chosen.source_kind, &binding.requirement);
    let new_audience = args
        .audience
        .clone()
        .unwrap_or_else(|| binding.audience.clone());
    let new_priority = args.priority.unwrap_or(50);
    let new_id = chosen.capability_id.clone();

    // Write the provides entry into the target config.
    let cfg = bom
        .targets
        .get_mut(target_name)
        .expect("checked existence above");
    // Remove any existing provides with the same id (idempotent).
    cfg.provides.retain(|c| c.id != new_id);
    let new_cap = AuthCapability {
        id: new_id.clone(),
        audience: new_audience.clone(),
        source: new_source.clone(),
        priority: new_priority,
    };
    cfg.provides.push(new_cap.clone());

    // Persist.
    if let Err(e) = crate::commands::manifest::save_manifest(
        manifest_path.to_str().unwrap_or("sindri.yaml"),
        bom,
    ) {
        eprintln!("Cannot write manifest: {}", e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    if args.json {
        let payload = serde_json::json!({
            "bound": true,
            "manifest": manifest_path.display().to_string(),
            "target": target_name,
            "binding_id": binding.id,
            "added_capability": new_cap,
            "next_steps": ["sindri resolve", "sindri auth show", "sindri apply"],
        });
        match serde_json::to_string_pretty(&payload) {
            Ok(s) => println!("{}", s),
            Err(_) => println!("{{\"bound\":true}}"),
        }
    } else {
        println!(
            "Wrote provides entry '{}' (audience='{}', source={}, priority={}) \
             to targets.{} in {}",
            new_id,
            new_audience,
            describe_source(&new_source),
            new_priority,
            target_name,
            manifest_path.display(),
        );
        println!("Next: `sindri resolve` to re-bind, then `sindri auth show` to verify.");
    }
    EXIT_SUCCESS
}

/// Produce a syntactically valid [`AuthSource`] skeleton from a
/// candidate's `source_kind` discriminant. Users are expected to edit
/// the placeholder fields; this just gets a parseable manifest written
/// so subsequent `sindri resolve` runs without manifest-edit churn.
fn source_from_kind(kind: &str, hint: &str) -> AuthSource {
    match kind {
        "from-secrets-store" => AuthSource::FromSecretsStore {
            backend: "vault".into(),
            path: format!("secrets/{}", hint),
        },
        "from-env" => AuthSource::FromEnv {
            var: hint.to_uppercase(),
        },
        "from-file" => AuthSource::FromFile {
            path: format!("/etc/sindri/{}.pem", hint),
            mode: Some(0o600),
        },
        "from-cli" => AuthSource::FromCli {
            command: format!("# replace: command that prints {}", hint),
        },
        "from-upstream-credentials" => AuthSource::FromUpstreamCredentials,
        "from-oauth" => AuthSource::FromOAuth {
            provider: "github".into(),
        },
        "prompt" => AuthSource::Prompt,
        _ => AuthSource::FromEnv {
            var: hint.to_uppercase(),
        },
    }
}

fn describe_source(s: &AuthSource) -> String {
    match s {
        AuthSource::FromSecretsStore { backend, path } => {
            format!("secret:{}/{}", backend, path)
        }
        AuthSource::FromEnv { var } => format!("env:{}", var),
        AuthSource::FromFile { path, .. } => format!("file:{}", path),
        AuthSource::FromCli { command } => format!("cli:{}", command),
        AuthSource::FromUpstreamCredentials => "upstream".to_string(),
        AuthSource::FromOAuth { provider } => format!("oauth:{}", provider),
        AuthSource::Prompt => "prompt".to_string(),
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
            "Interactive shell for target '{}': sprint 10 (cloud targets)",
            name
        );
        EXIT_SCHEMA_OR_RESOLVE_ERROR
    }
}

#[cfg(test)]
mod auth_subverb_tests {
    use super::*;

    #[test]
    fn source_from_kind_env_uses_uppercased_hint() {
        match source_from_kind("from-env", "github_token") {
            AuthSource::FromEnv { var } => assert_eq!(var, "GITHUB_TOKEN"),
            other => panic!("got {:?}", other),
        }
    }

    #[test]
    fn source_from_kind_secrets_store_default_vault() {
        match source_from_kind("from-secrets-store", "tok") {
            AuthSource::FromSecretsStore { backend, path } => {
                assert_eq!(backend, "vault");
                assert_eq!(path, "secrets/tok");
            }
            other => panic!("got {:?}", other),
        }
    }

    #[test]
    fn source_from_kind_unknown_falls_back_to_env() {
        match source_from_kind("never-heard-of", "tok") {
            AuthSource::FromEnv { var } => assert_eq!(var, "TOK"),
            other => panic!("got {:?}", other),
        }
    }

    #[test]
    fn source_from_kind_file_uses_etc_sindri() {
        match source_from_kind("from-file", "client_cert") {
            AuthSource::FromFile { path, mode } => {
                assert!(path.contains("client_cert"));
                assert_eq!(mode, Some(0o600));
            }
            other => panic!("got {:?}", other),
        }
    }

    #[test]
    fn target_auth_bind_round_trips_through_manifest() {
        // Smoke: build a TargetConfig, append a provides via the same
        // path as run_auth_bind, serialise + parse, assert equality.
        use sindri_core::manifest::TargetConfig;
        let mut tc = TargetConfig {
            kind: "fly".into(),
            infra: None,
            auth: None,
            provides: vec![],
        };
        tc.provides.push(AuthCapability {
            id: "github_token".into(),
            audience: "https://api.github.com".into(),
            source: AuthSource::FromEnv { var: "GH".into() },
            priority: 50,
        });
        let s = serde_yaml::to_string(&tc).unwrap();
        let back: TargetConfig = serde_yaml::from_str(&s).unwrap();
        assert_eq!(back.provides.len(), 1);
        assert_eq!(back.provides[0].id, "github_token");
    }
}
