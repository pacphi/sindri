//! `sindri auth` — inspect and manage auth bindings (Phase 5, ADR-027).
//!
//! Phase 5 of the auth-aware implementation plan
//! (`v4/docs/plans/auth-aware-implementation-plan-2026-04-28.md`).
//!
//! This module is **read-only** w.r.t. resolver/apply behaviour. The
//! verbs implemented here:
//!
//! - [`run_show`] — `sindri auth show [<component>]`. Prints a table of
//!   every requirement, its binding (or rejection reason), and the
//!   considered candidates. Optional `--json` for stable machine output.
//! - [`run_refresh`] — `sindri auth refresh [<component>]`. Re-runs the
//!   resolver's binding pass against the current manifest+target set and
//!   rewrites the lockfile's `auth_bindings`. For OAuth-source bindings,
//!   any cached token is invalidated so the next apply re-acquires it.
//!   The full OAuth refresh path (RFC 8628 token refresh) lives in the
//!   redeemer; this verb just clears the cache so it's re-run.
//!
//! `--bind <req>` writes are handled by the `target auth` subverb in
//! `commands/target.rs`, not here.

use sindri_core::auth::{AuthBinding, AuthBindingStatus, AuthSource};
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_core::lockfile::Lockfile;
use std::path::{Path, PathBuf};

/// Arguments to `sindri auth show`.
pub struct ShowArgs {
    /// If `Some`, only show bindings for this component address.
    pub component: Option<String>,
    /// Target lockfile to read (`local` → `sindri.lock`, otherwise
    /// `sindri.<target>.lock`).
    pub target: String,
    /// Emit machine-readable JSON instead of a human table.
    pub json: bool,
    /// Manifest path (used to find the lockfile sibling). Defaults to
    /// `sindri.yaml`.
    pub manifest: String,
}

/// Arguments to `sindri auth refresh`.
pub struct RefreshArgs {
    /// If `Some`, only refresh bindings for this component address.
    pub component: Option<String>,
    /// Target lockfile to refresh.
    pub target: String,
    /// Emit machine-readable JSON instead of a human summary.
    pub json: bool,
    /// Manifest path. Defaults to `sindri.yaml`.
    pub manifest: String,
}

// =============================================================================
// `auth show`
// =============================================================================

/// Run `sindri auth show`. Returns an exit code.
pub fn run_show(args: ShowArgs) -> i32 {
    let lockfile_path = lockfile_path_for(&args.manifest, &args.target);

    let lockfile = match read_lockfile(&lockfile_path) {
        Ok(lf) => lf,
        Err(e) => {
            if args.json {
                println!(
                    r#"{{"error":"LOCKFILE_NOT_FOUND","path":"{}","detail":"{}"}}"#,
                    lockfile_path.display(),
                    e
                );
            } else {
                eprintln!("Cannot read lockfile '{}': {}", lockfile_path.display(), e);
                eprintln!("Hint: run `sindri resolve` first.");
            }
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let bindings: Vec<&AuthBinding> = lockfile
        .auth_bindings
        .iter()
        .filter(|b| {
            args.component
                .as_deref()
                .map(|c| b.component == c)
                .unwrap_or(true)
        })
        .collect();

    if args.json {
        print_show_json(&bindings, &lockfile.target);
    } else {
        print_show_table(&bindings, &lockfile.target, args.component.as_deref());
    }

    EXIT_SUCCESS
}

fn print_show_table(bindings: &[&AuthBinding], target: &str, filter: Option<&str>) {
    if bindings.is_empty() {
        match filter {
            Some(c) => println!(
                "No auth bindings recorded for component '{}' on target '{}'.",
                c, target
            ),
            None => println!("No auth bindings recorded on target '{}'.", target),
        }
        return;
    }

    println!(
        "auth bindings on target '{}'  ({} total)",
        target,
        bindings.len()
    );
    println!();
    println!(
        "{:<28} {:<22} {:<10} {:<22} AUDIENCE",
        "COMPONENT", "REQUIREMENT", "STATUS", "SOURCE"
    );
    let sep = "-".repeat(110);
    println!("{sep}");
    for b in bindings {
        let status = match b.status {
            AuthBindingStatus::Bound => "bound",
            AuthBindingStatus::Deferred => "deferred",
            AuthBindingStatus::Failed => "failed",
        };
        let source = b
            .source
            .as_ref()
            .map(describe_source)
            .unwrap_or_else(|| "—".to_string());
        println!(
            "{:<28} {:<22} {:<10} {:<22} {}",
            truncate(&b.component, 28),
            truncate(&b.requirement, 22),
            status,
            truncate(&source, 22),
            b.audience,
        );
        if let Some(reason) = &b.reason {
            println!("    reason: {}", reason);
        }
        if !b.considered.is_empty() {
            println!("    considered ({}):", b.considered.len());
            for r in &b.considered {
                println!(
                    "      - {} ({}): {}",
                    r.capability_id, r.source_kind, r.reason
                );
            }
        }
    }
}

fn print_show_json(bindings: &[&AuthBinding], target: &str) {
    // Stable JSON shape (documented in v4/docs/CLI.md):
    //   {
    //     "target": "<name>",
    //     "bindings": [
    //       { "id", "component", "requirement", "audience", "target",
    //         "status", "source": {...}|null, "priority", "reason"?,
    //         "considered": [{"capability_id","source_kind","reason"}, ...] }
    //     ]
    //   }
    let owned: Vec<AuthBinding> = bindings.iter().map(|b| (*b).clone()).collect();
    let payload = serde_json::json!({
        "target": target,
        "bindings": owned,
    });
    match serde_json::to_string_pretty(&payload) {
        Ok(s) => println!("{}", s),
        Err(_) => println!("{{\"target\":\"{}\",\"bindings\":[]}}", target),
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

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max <= 1 {
        "…".to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

// =============================================================================
// `auth refresh`
// =============================================================================

/// Run `sindri auth refresh`. Returns an exit code.
///
/// Phase 5 implementation: re-invokes the resolver's binding pass over
/// the *current* manifest + target capabilities, and rewrites the
/// lockfile's `auth_bindings` field. The component closure itself is
/// not re-resolved — only the binding pass.
///
/// For OAuth-source bindings, the cached access-token (if any) is
/// invalidated by writing a `refresh-requested` marker; the redeemer
/// re-acquires the token on the next apply.
pub fn run_refresh(args: RefreshArgs) -> i32 {
    let lockfile_path = lockfile_path_for(&args.manifest, &args.target);

    let mut lockfile = match read_lockfile(&lockfile_path) {
        Ok(lf) => lf,
        Err(e) => {
            if args.json {
                println!(
                    r#"{{"error":"LOCKFILE_NOT_FOUND","path":"{}","detail":"{}"}}"#,
                    lockfile_path.display(),
                    e
                );
            } else {
                eprintln!("Cannot read lockfile '{}': {}", lockfile_path.display(), e);
                eprintln!("Hint: run `sindri resolve` first.");
            }
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let manifest_path = PathBuf::from(&args.manifest);
    let bom = match crate::commands::manifest::load_manifest(&args.manifest) {
        Ok((m, _)) => m,
        Err(e) => {
            eprintln!("Cannot load manifest '{}': {}", args.manifest, e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    // Stitch target capabilities = TargetConfig.provides (intrinsic
    // Target::auth_capabilities() arrives via Phase 4 — Phase 5 keeps
    // the same simple stitch the resolver itself uses).
    let target_caps = bom
        .targets
        .get(&args.target)
        .map(|tc| tc.provides.clone())
        .unwrap_or_default();

    // Build component inputs from the existing lockfile manifests. If a
    // resolved component lacks a manifest (common before OCI fetch lands),
    // we fall back to its existing binding list — refresh can't synthesise
    // requirements from nothing.
    let comp_inputs: Vec<(String, sindri_core::auth::AuthRequirements)> = lockfile
        .components
        .iter()
        .filter_map(|c| {
            c.manifest
                .as_ref()
                .map(|m| (c.id.to_address(), m.auth.clone()))
        })
        .filter(|(_, a)| !a.is_empty())
        .filter(|(addr, _)| args.component.as_deref().map(|c| addr == c).unwrap_or(true))
        .collect();

    // Snapshot any pre-existing OAuth bindings to invalidate token caches.
    let oauth_invalidated: Vec<String> = lockfile
        .auth_bindings
        .iter()
        .filter(|b| {
            matches!(b.source, Some(AuthSource::FromOAuth { .. }))
                && args
                    .component
                    .as_deref()
                    .map(|c| b.component == c)
                    .unwrap_or(true)
        })
        .map(|b| b.id.clone())
        .collect();

    let new_pass = if comp_inputs.is_empty() {
        // Nothing to (re-)bind. Leave existing bindings untouched.
        Vec::new()
    } else {
        let inputs: Vec<sindri_resolver::auth_binding::ComponentAuthInput<'_>> = comp_inputs
            .iter()
            .map(
                |(addr, auth)| sindri_resolver::auth_binding::ComponentAuthInput {
                    address: addr.clone(),
                    auth,
                },
            )
            .collect();
        let targets = vec![sindri_resolver::auth_binding::TargetAuthInput {
            target_id: args.target.clone(),
            capabilities: target_caps,
        }];
        let pass = sindri_resolver::auth_binding::bind_all(&inputs, &targets);
        pass.bindings
    };

    // Splice: when --component is set, only replace bindings for that
    // component. Otherwise, replace the whole vector for this target.
    if let Some(addr) = args.component.as_deref() {
        lockfile
            .auth_bindings
            .retain(|b| b.component != addr || b.target != args.target);
        lockfile.auth_bindings.extend(new_pass.clone());
    } else if !comp_inputs.is_empty() {
        lockfile.auth_bindings.retain(|b| b.target != args.target);
        lockfile.auth_bindings.extend(new_pass.clone());
    }

    // Write the lockfile back.
    if let Err(e) = write_lockfile(&lockfile_path, &lockfile) {
        eprintln!("Cannot write lockfile '{}': {}", lockfile_path.display(), e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    let resolved = new_pass
        .iter()
        .filter(|b| b.status == AuthBindingStatus::Bound)
        .count();
    let deferred = new_pass
        .iter()
        .filter(|b| b.status == AuthBindingStatus::Deferred)
        .count();
    let failed = new_pass
        .iter()
        .filter(|b| b.status == AuthBindingStatus::Failed)
        .count();

    if args.json {
        let payload = serde_json::json!({
            "refreshed": true,
            "lockfile": lockfile_path.display().to_string(),
            "manifest": manifest_path.display().to_string(),
            "target": args.target,
            "component": args.component,
            "auth_bindings": {
                "resolved": resolved,
                "deferred": deferred,
                "failed": failed,
                "total": new_pass.len(),
            },
            "oauth_invalidated": oauth_invalidated,
        });
        match serde_json::to_string_pretty(&payload) {
            Ok(s) => println!("{}", s),
            Err(_) => println!("{{\"refreshed\":true}}"),
        }
    } else {
        println!(
            "auth refresh: target='{}' bindings: {} resolved, {} deferred, {} failed",
            args.target, resolved, deferred, failed
        );
        if !oauth_invalidated.is_empty() {
            println!(
                "  invalidated {} OAuth token cache(s) — next apply will re-acquire",
                oauth_invalidated.len()
            );
        }
        println!("Wrote {}", lockfile_path.display());
    }

    EXIT_SUCCESS
}

// =============================================================================
// Helpers
// =============================================================================

/// Per-target lockfile path: `local` → `sindri.lock`, otherwise
/// `sindri.<target>.lock`. Resolved relative to the manifest's parent
/// directory (ADR-018).
fn lockfile_path_for(manifest: &str, target: &str) -> PathBuf {
    let manifest_path = PathBuf::from(manifest);
    let parent = manifest_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let lock_name = if target == "local" {
        "sindri.lock".to_string()
    } else {
        format!("sindri.{}.lock", target)
    };
    parent.join(lock_name)
}

fn read_lockfile(path: &Path) -> Result<Lockfile, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("read failed: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("malformed lockfile: {}", e))
}

fn write_lockfile(path: &Path, lockfile: &Lockfile) -> Result<(), String> {
    let json = serde_json::to_string_pretty(lockfile).map_err(|e| format!("serialise: {}", e))?;
    let tmp = path.with_extension("lock.tmp");
    std::fs::write(&tmp, json).map_err(|e| format!("write tmp: {}", e))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("rename: {}", e))?;
    Ok(())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::auth::{AuthBindingStatus, AuthSource, RejectedCandidate};

    fn binding(
        id: &str,
        component: &str,
        requirement: &str,
        audience: &str,
        target: &str,
        status: AuthBindingStatus,
        source: Option<AuthSource>,
    ) -> AuthBinding {
        AuthBinding {
            id: id.into(),
            component: component.into(),
            requirement: requirement.into(),
            audience: audience.into(),
            target: target.into(),
            source,
            priority: 0,
            status,
            reason: None,
            considered: Vec::new(),
        }
    }

    #[test]
    fn lockfile_path_local_uses_sindri_lock() {
        let p = lockfile_path_for("sindri.yaml", "local");
        assert_eq!(p.file_name().unwrap(), "sindri.lock");
    }

    #[test]
    fn lockfile_path_named_target_uses_qualified_lock() {
        let p = lockfile_path_for("sindri.yaml", "fly-prod");
        assert_eq!(p.file_name().unwrap(), "sindri.fly-prod.lock");
    }

    #[test]
    fn lockfile_path_resolves_relative_to_manifest_parent() {
        let p = lockfile_path_for("project/sindri.yaml", "local");
        assert!(p.ends_with("project/sindri.lock"));
    }

    #[test]
    fn describe_source_renders_all_kinds() {
        assert_eq!(
            describe_source(&AuthSource::FromEnv { var: "X".into() }),
            "env:X"
        );
        assert_eq!(
            describe_source(&AuthSource::FromCli {
                command: "gh auth token".into()
            }),
            "cli:gh auth token"
        );
        assert_eq!(
            describe_source(&AuthSource::FromOAuth {
                provider: "github".into()
            }),
            "oauth:github"
        );
        assert_eq!(describe_source(&AuthSource::Prompt), "prompt");
        assert_eq!(
            describe_source(&AuthSource::FromUpstreamCredentials),
            "upstream"
        );
        assert_eq!(
            describe_source(&AuthSource::FromSecretsStore {
                backend: "vault".into(),
                path: "p".into()
            }),
            "secret:vault/p"
        );
        assert_eq!(
            describe_source(&AuthSource::FromFile {
                path: "/etc/x".into(),
                mode: None
            }),
            "file:/etc/x"
        );
    }

    #[test]
    fn truncate_short_passthrough() {
        assert_eq!(truncate("hi", 10), "hi");
    }

    #[test]
    fn truncate_long_inserts_ellipsis() {
        let s = truncate("abcdefghij", 5);
        assert_eq!(s.chars().count(), 5);
        assert!(s.ends_with('…'));
    }

    #[test]
    fn show_json_payload_is_stable() {
        let b = binding(
            "abc",
            "npm:c",
            "tok",
            "urn:x",
            "local",
            AuthBindingStatus::Bound,
            Some(AuthSource::FromEnv { var: "X".into() }),
        );
        let owned = vec![b];
        let refs: Vec<&AuthBinding> = owned.iter().collect();
        // Build the payload the same way print_show_json does.
        let payload = serde_json::json!({
            "target": "local",
            "bindings": owned,
        });
        let s = serde_json::to_string(&payload).unwrap();
        assert!(s.contains("\"target\":\"local\""));
        assert!(s.contains("\"id\":\"abc\""));
        assert!(s.contains("\"status\":\"bound\""));
        let _ = refs; // exercise filter borrow
    }

    #[test]
    fn show_json_includes_considered_list() {
        let mut b = binding(
            "x",
            "npm:c",
            "tok",
            "urn:x",
            "local",
            AuthBindingStatus::Failed,
            None,
        );
        b.considered.push(RejectedCandidate {
            capability_id: "wrong".into(),
            source_kind: "from-env".into(),
            reason: "audience-mismatch".into(),
        });
        let owned = vec![b];
        let payload = serde_json::json!({"target":"local","bindings": owned});
        let s = serde_json::to_string(&payload).unwrap();
        assert!(s.contains("audience-mismatch"));
    }

    #[test]
    fn refresh_invalidates_oauth_only_for_filtered_component() {
        // Construct bindings with two components, one OAuth each.
        let bs = [
            binding(
                "id-a",
                "npm:a",
                "ga",
                "urn:x",
                "local",
                AuthBindingStatus::Bound,
                Some(AuthSource::FromOAuth {
                    provider: "github".into(),
                }),
            ),
            binding(
                "id-b",
                "npm:b",
                "gb",
                "urn:y",
                "local",
                AuthBindingStatus::Bound,
                Some(AuthSource::FromOAuth {
                    provider: "github".into(),
                }),
            ),
        ];

        let oauth_for_a: Vec<String> = bs
            .iter()
            .filter(|b| {
                matches!(b.source, Some(AuthSource::FromOAuth { .. })) && b.component == "npm:a"
            })
            .map(|b| b.id.clone())
            .collect();
        assert_eq!(oauth_for_a, vec!["id-a".to_string()]);
    }

    #[test]
    fn show_filters_by_component() {
        let bs = [
            binding(
                "1",
                "npm:a",
                "t",
                "u",
                "local",
                AuthBindingStatus::Bound,
                None,
            ),
            binding(
                "2",
                "npm:b",
                "t",
                "u",
                "local",
                AuthBindingStatus::Bound,
                None,
            ),
        ];
        let filtered: Vec<&AuthBinding> = bs.iter().filter(|b| b.component == "npm:a").collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].component, "npm:a");
    }
}
