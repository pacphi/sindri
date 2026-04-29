//! Auth-binding algorithm — observability-only resolver pass (ADR-027 §3).
//!
//! Phase 1 of the auth-aware implementation plan
//! (`v4/docs/plans/auth-aware-implementation-plan-2026-04-28.md`).
//!
//! # Algorithm (ADR-027 §3)
//!
//! For each [`AuthRequirement`]-shaped entry declared by each component
//! resolved against each [`Target`], we compute an [`AuthBinding`]:
//!
//! ```text
//! fn bind(req, target) -> Option<AuthBinding>:
//!     candidates = target.auth_capabilities()           // intrinsic
//!               ++ target.config.provides               // user overrides
//!               ++ requirement.discovery.* synthesised  // env/cli/oauth aliases
//!
//!     dedupe by (target_id, source.kind, source.params)
//!     sort by (priority desc, source.rank asc)
//!
//!     for cap in candidates:
//!         if cap.audience != req.audience: skip (audience-mismatch)
//!         if cap.source incompatible with req.scope: skip (scope-mismatch)
//!         return Bound(cap)
//!     None
//! ```
//!
//! # Scope of Phase 1
//!
//! - Pure dataflow over the manifests + target capabilities. **No values
//!   are read** (DDD-07 invariant 3 "no value capture").
//! - The apply path (`sindri-extensions::executor`) does **not** read the
//!   produced bindings yet — that is Phase 2.
//! - Built-in targets keep the trait default `auth_capabilities() = vec![]`;
//!   capabilities arrive via `TargetConfig.provides:` (Phase 1) and via
//!   per-target overrides (Phase 4).
//!
//! # Determinism
//!
//! Given identical input, the produced [`AuthBinding`] sequence is
//! byte-identical (same `id`, same selected source, same `considered`
//! list, same order). This is asserted by a property test
//! (`prop_determinism`).

use sha2::{Digest, Sha256};
use sindri_core::auth::{
    auth_source_kind, auth_source_rank, AuthBinding, AuthBindingStatus, AuthCapability,
    AuthRequirements, AuthScope, AuthSource, CertRequirement, OAuthRequirement, RejectedCandidate,
    SshKeyRequirement, TokenRequirement,
};

// =============================================================================
// Public API
// =============================================================================

/// One component's auth requirements paired with its address, as input to
/// [`bind_all`].
#[derive(Debug, Clone)]
pub struct ComponentAuthInput<'a> {
    /// Canonical component address (`backend:name[@qualifier]`).
    pub address: String,
    /// The component-declared requirements (cloned/borrowed from the
    /// manifest).
    pub auth: &'a AuthRequirements,
}

/// One target's identity paired with its full capability list, as input
/// to [`bind_all`].
#[derive(Debug, Clone)]
pub struct TargetAuthInput {
    /// Target name (key in `BomManifest.targets`).
    pub target_id: String,
    /// Capabilities = `Target::auth_capabilities()` ++
    /// `TargetConfig.provides`. The caller is responsible for stitching
    /// these together; this module treats the list as opaque.
    pub capabilities: Vec<AuthCapability>,
}

/// Outcome of the binding pass — the bindings to record in the lockfile,
/// plus aggregate counts for the CLI summary line and ledger emission.
#[derive(Debug, Clone, Default)]
pub struct BindingPass {
    /// All bindings, in stable order: per-component declaration order,
    /// then per-requirement declaration order (tokens → oauth → certs →
    /// ssh).
    pub bindings: Vec<AuthBinding>,
}

impl BindingPass {
    /// Number of [`AuthBindingStatus::Bound`] bindings.
    pub fn resolved(&self) -> usize {
        self.bindings
            .iter()
            .filter(|b| b.status == AuthBindingStatus::Bound)
            .count()
    }

    /// Number of [`AuthBindingStatus::Deferred`] bindings (optional, no
    /// source matched).
    pub fn deferred(&self) -> usize {
        self.bindings
            .iter()
            .filter(|b| b.status == AuthBindingStatus::Deferred)
            .count()
    }

    /// Number of [`AuthBindingStatus::Failed`] bindings (required, no
    /// source matched). Phase 2's Gate 5 will deny apply when this is
    /// non-zero.
    pub fn failed(&self) -> usize {
        self.bindings
            .iter()
            .filter(|b| b.status == AuthBindingStatus::Failed)
            .count()
    }
}

/// Run the binding algorithm across a Cartesian product of components and
/// targets.
///
/// The result is deterministic: callers will see the same `bindings`
/// vector for the same input. Bindings are emitted in stable order:
/// outer = `targets` order, inner = `components` order, innermost =
/// requirement-declaration order within each component (`tokens` first,
/// then `oauth`, then `certs`, then `ssh`).
pub fn bind_all(components: &[ComponentAuthInput<'_>], targets: &[TargetAuthInput]) -> BindingPass {
    let mut bindings = Vec::new();
    for tgt in targets {
        for comp in components {
            extend_bindings_for_component(comp, tgt, &mut bindings);
        }
    }
    BindingPass { bindings }
}

// =============================================================================
// Implementation
// =============================================================================

fn extend_bindings_for_component(
    comp: &ComponentAuthInput<'_>,
    tgt: &TargetAuthInput,
    out: &mut Vec<AuthBinding>,
) {
    for t in &comp.auth.tokens {
        out.push(bind_token(comp, tgt, t));
    }
    for o in &comp.auth.oauth {
        out.push(bind_oauth(comp, tgt, o));
    }
    for c in &comp.auth.certs {
        out.push(bind_cert(comp, tgt, c));
    }
    for s in &comp.auth.ssh {
        out.push(bind_ssh(comp, tgt, s));
    }
}

/// Common per-requirement view passed to [`bind_one`].
struct ReqView<'a> {
    name: &'a str,
    audience: &'a str,
    scope: AuthScope,
    optional: bool,
    /// Synthesised candidates from `DiscoveryHints` — appended at the end
    /// of the candidate list with priority `-100` so explicit target
    /// capabilities always win.
    discovered: Vec<AuthCapability>,
}

fn bind_token(
    comp: &ComponentAuthInput<'_>,
    tgt: &TargetAuthInput,
    t: &TokenRequirement,
) -> AuthBinding {
    let discovered = synthesise_from_discovery(&t.audience, &t.discovery);
    let view = ReqView {
        name: &t.name,
        audience: &t.audience,
        scope: t.scope,
        optional: t.optional,
        discovered,
    };
    bind_one(comp, tgt, &view)
}

fn bind_oauth(
    comp: &ComponentAuthInput<'_>,
    tgt: &TargetAuthInput,
    o: &OAuthRequirement,
) -> AuthBinding {
    // OAuth requirements declare their provider directly; synthesise a
    // single FromOAuth candidate keyed off `o.provider`.
    let discovered = vec![AuthCapability {
        id: format!("{}_oauth", o.provider),
        audience: o.audience.clone(),
        source: AuthSource::FromOAuth {
            provider: o.provider.clone(),
        },
        priority: -100,
    }];
    let view = ReqView {
        name: &o.name,
        audience: &o.audience,
        scope: o.scope,
        optional: o.optional,
        discovered,
    };
    bind_one(comp, tgt, &view)
}

fn bind_cert(
    comp: &ComponentAuthInput<'_>,
    tgt: &TargetAuthInput,
    c: &CertRequirement,
) -> AuthBinding {
    let view = ReqView {
        name: &c.name,
        audience: &c.audience,
        scope: c.scope,
        optional: c.optional,
        discovered: Vec::new(),
    };
    bind_one(comp, tgt, &view)
}

fn bind_ssh(
    comp: &ComponentAuthInput<'_>,
    tgt: &TargetAuthInput,
    s: &SshKeyRequirement,
) -> AuthBinding {
    let view = ReqView {
        name: &s.name,
        audience: &s.audience,
        scope: s.scope,
        optional: s.optional,
        discovered: Vec::new(),
    };
    bind_one(comp, tgt, &view)
}

fn synthesise_from_discovery(
    audience: &str,
    d: &sindri_core::auth::DiscoveryHints,
) -> Vec<AuthCapability> {
    let mut out = Vec::new();
    for var in &d.env_aliases {
        out.push(AuthCapability {
            id: format!("env-alias:{}", var),
            audience: audience.to_string(),
            source: AuthSource::FromEnv { var: var.clone() },
            priority: -100,
        });
    }
    for cmd in &d.cli_aliases {
        out.push(AuthCapability {
            id: format!("cli-alias:{}", cmd),
            audience: audience.to_string(),
            source: AuthSource::FromCli {
                command: cmd.clone(),
            },
            priority: -100,
        });
    }
    if let Some(p) = &d.oauth_provider {
        out.push(AuthCapability {
            id: format!("oauth-provider:{}", p),
            audience: audience.to_string(),
            source: AuthSource::FromOAuth {
                provider: p.clone(),
            },
            priority: -100,
        });
    }
    out
}

fn bind_one(
    comp: &ComponentAuthInput<'_>,
    tgt: &TargetAuthInput,
    view: &ReqView<'_>,
) -> AuthBinding {
    let id = compute_binding_id(&comp.address, view.name, &tgt.target_id);

    // 1. Build candidate list: target capabilities first (priority by user),
    //    then synthesised discovery candidates (priority -100).
    let mut candidates: Vec<AuthCapability> = tgt.capabilities.clone();
    candidates.extend(view.discovered.clone());

    // 2. Dedupe by (source_kind, source_params). Stable: keep first.
    candidates = dedupe_candidates(candidates);

    // 3. Sort by (priority desc, source_rank asc, id asc).
    candidates.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then_with(|| auth_source_rank(&a.source).cmp(&auth_source_rank(&b.source)))
            .then_with(|| a.id.cmp(&b.id))
    });

    // 4. Walk candidates, first match wins; record rejections.
    let canon_audience = canon(view.audience);
    let mut considered: Vec<RejectedCandidate> = Vec::new();
    let mut chosen: Option<AuthCapability> = None;

    for cap in candidates {
        if canon(&cap.audience) != canon_audience {
            considered.push(RejectedCandidate {
                capability_id: cap.id.clone(),
                source_kind: auth_source_kind(&cap.source).to_string(),
                reason: "audience-mismatch".into(),
            });
            continue;
        }
        if !scope_compatible(view.scope, &cap.source) {
            considered.push(RejectedCandidate {
                capability_id: cap.id.clone(),
                source_kind: auth_source_kind(&cap.source).to_string(),
                reason: "scope-mismatch".into(),
            });
            continue;
        }
        chosen = Some(cap);
        break;
    }

    match chosen {
        Some(cap) => AuthBinding {
            id,
            component: comp.address.clone(),
            requirement: view.name.to_string(),
            audience: canon_audience,
            target: tgt.target_id.clone(),
            source: Some(cap.source),
            priority: cap.priority,
            status: AuthBindingStatus::Bound,
            reason: None,
            considered,
        },
        None => {
            let (status, reason) = if view.optional {
                (
                    AuthBindingStatus::Deferred,
                    Some("no source matched (optional)".to_string()),
                )
            } else {
                (
                    AuthBindingStatus::Failed,
                    Some("no source matched (required)".to_string()),
                )
            };
            AuthBinding {
                id,
                component: comp.address.clone(),
                requirement: view.name.to_string(),
                audience: canon_audience,
                target: tgt.target_id.clone(),
                source: None,
                priority: 0,
                status,
                reason,
                considered,
            }
        }
    }
}

/// Deterministic 16-hex-char id for an [`AuthBinding`] (DDD-07 invariant 4).
fn compute_binding_id(component: &str, requirement: &str, target: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"auth-binding:v1\n");
    h.update(component.as_bytes());
    h.update(b"\n");
    h.update(requirement.as_bytes());
    h.update(b"\n");
    h.update(target.as_bytes());
    let digest = h.finalize();
    hex::encode(&digest[..8])
}

/// Canonical audience matching: lower-cased, trimmed (no globs — DDD-07
/// "Audience" definition).
fn canon(s: &str) -> String {
    s.trim().to_ascii_lowercase()
}

/// Phase 1 scope/source compatibility:
///
/// - `Prompt` is interactive and cannot satisfy `scope: install` (in a
///   `--ci` invocation Phase 2's Gate 5 will reject it; the binding
///   stage already excludes the obviously-wrong combination so the
///   `considered` list shows the rejection).
/// - All other source kinds are scope-compatible at this phase.
fn scope_compatible(scope: AuthScope, source: &AuthSource) -> bool {
    !matches!((scope, source), (AuthScope::Install, AuthSource::Prompt))
}

/// Stable-keep dedupe: first occurrence wins, so user-supplied
/// `provides:` entries (which the caller is expected to put first) take
/// precedence over the trait's intrinsic capabilities for the same key.
fn dedupe_candidates(in_caps: Vec<AuthCapability>) -> Vec<AuthCapability> {
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut out = Vec::with_capacity(in_caps.len());
    for cap in in_caps {
        let key = source_dedupe_key(&cap.source);
        if seen.insert(key) {
            out.push(cap);
        }
    }
    out
}

fn source_dedupe_key(s: &AuthSource) -> String {
    match s {
        AuthSource::FromSecretsStore { backend, path } => {
            format!("from-secrets-store|{}|{}", backend, path)
        }
        AuthSource::FromEnv { var } => format!("from-env|{}", var),
        AuthSource::FromFile { path, mode } => {
            format!("from-file|{}|{:?}", path, mode)
        }
        AuthSource::FromCli { command } => format!("from-cli|{}", command),
        AuthSource::FromUpstreamCredentials => "from-upstream-credentials".to_string(),
        AuthSource::FromOAuth { provider } => format!("from-oauth|{}", provider),
        AuthSource::Prompt => "prompt".to_string(),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::auth::{DiscoveryHints, Redemption};

    fn token_req(name: &str, audience: &str, optional: bool) -> TokenRequirement {
        TokenRequirement {
            name: name.into(),
            description: format!("token {}", name),
            scope: AuthScope::Both,
            optional,
            audience: audience.into(),
            redemption: Redemption::EnvVar {
                env_name: name.to_uppercase(),
            },
            discovery: DiscoveryHints::default(),
        }
    }

    fn cap(id: &str, audience: &str, src: AuthSource, prio: i32) -> AuthCapability {
        AuthCapability {
            id: id.into(),
            audience: audience.into(),
            source: src,
            priority: prio,
        }
    }

    fn req_set(tokens: Vec<TokenRequirement>) -> AuthRequirements {
        AuthRequirements {
            tokens,
            ..Default::default()
        }
    }

    fn comp_input<'a>(addr: &str, auth: &'a AuthRequirements) -> ComponentAuthInput<'a> {
        ComponentAuthInput {
            address: addr.into(),
            auth,
        }
    }

    fn tgt_input(name: &str, caps: Vec<AuthCapability>) -> TargetAuthInput {
        TargetAuthInput {
            target_id: name.into(),
            capabilities: caps,
        }
    }

    // 1. Audience match — same audience binds.
    #[test]
    fn audience_match_binds() {
        let auth = req_set(vec![token_req("gh", "https://api.github.com", false)]);
        let caps = vec![cap(
            "gh_token",
            "https://api.github.com",
            AuthSource::FromEnv {
                var: "GITHUB_TOKEN".into(),
            },
            0,
        )];
        let pass = bind_all(&[comp_input("npm:gh", &auth)], &[tgt_input("local", caps)]);
        assert_eq!(pass.resolved(), 1);
        assert_eq!(pass.failed(), 0);
        let b = &pass.bindings[0];
        assert_eq!(b.status, AuthBindingStatus::Bound);
        assert_eq!(b.target, "local");
        assert!(matches!(b.source, Some(AuthSource::FromEnv { .. })));
    }

    // 2. Audience mismatch — does not bind, recorded as rejected.
    #[test]
    fn audience_mismatch_does_not_bind() {
        let auth = req_set(vec![token_req("gh", "https://api.github.com", false)]);
        let caps = vec![cap(
            "wrong",
            "https://gitlab.com/api",
            AuthSource::FromEnv {
                var: "GITLAB_TOKEN".into(),
            },
            0,
        )];
        let pass = bind_all(&[comp_input("npm:gh", &auth)], &[tgt_input("local", caps)]);
        assert_eq!(pass.resolved(), 0);
        assert_eq!(pass.failed(), 1);
        let b = &pass.bindings[0];
        assert_eq!(b.status, AuthBindingStatus::Failed);
        assert_eq!(b.considered.len(), 1);
        assert_eq!(b.considered[0].reason, "audience-mismatch");
    }

    // 3. Scope mismatch — Prompt rejected for Install scope.
    #[test]
    fn prompt_rejected_for_install_scope() {
        let mut t = token_req("k", "urn:x", false);
        t.scope = AuthScope::Install;
        let auth = req_set(vec![t]);
        let caps = vec![
            cap("p", "urn:x", AuthSource::Prompt, 100),
            cap("e", "urn:x", AuthSource::FromEnv { var: "X".into() }, 0),
        ];
        let pass = bind_all(&[comp_input("npm:k", &auth)], &[tgt_input("local", caps)]);
        let b = &pass.bindings[0];
        assert_eq!(b.status, AuthBindingStatus::Bound);
        // Prompt was first (priority 100) but rejected for scope, env wins.
        assert!(matches!(b.source, Some(AuthSource::FromEnv { .. })));
        assert!(b
            .considered
            .iter()
            .any(|r| r.reason == "scope-mismatch" && r.source_kind == "prompt"));
    }

    // 4. Priority order — higher priority wins among same-audience candidates.
    #[test]
    fn higher_priority_wins() {
        let auth = req_set(vec![token_req("k", "urn:x", false)]);
        let caps = vec![
            cap("low", "urn:x", AuthSource::FromEnv { var: "LOW".into() }, 0),
            cap(
                "high",
                "urn:x",
                AuthSource::FromEnv { var: "HIGH".into() },
                100,
            ),
        ];
        let pass = bind_all(&[comp_input("npm:k", &auth)], &[tgt_input("t", caps)]);
        let b = &pass.bindings[0];
        match b.source.as_ref().unwrap() {
            AuthSource::FromEnv { var } => assert_eq!(var, "HIGH"),
            other => panic!("got {:?}", other),
        }
        assert_eq!(b.priority, 100);
    }

    // 5. Source-rank tie-breaker — equal priority, secrets-store beats env.
    #[test]
    fn source_rank_tiebreaker() {
        let auth = req_set(vec![token_req("k", "urn:x", false)]);
        let caps = vec![
            cap("env", "urn:x", AuthSource::FromEnv { var: "X".into() }, 0),
            cap(
                "vault",
                "urn:x",
                AuthSource::FromSecretsStore {
                    backend: "vault".into(),
                    path: "p".into(),
                },
                0,
            ),
        ];
        let pass = bind_all(&[comp_input("npm:k", &auth)], &[tgt_input("t", caps)]);
        let b = &pass.bindings[0];
        assert!(matches!(
            b.source,
            Some(AuthSource::FromSecretsStore { .. })
        ));
    }

    // 6. Considered-but-rejected list captures all skipped candidates.
    #[test]
    fn considered_list_records_all_skips() {
        let auth = req_set(vec![token_req("k", "urn:x", false)]);
        let caps = vec![
            cap(
                "wrong1",
                "urn:y",
                AuthSource::FromEnv { var: "A".into() },
                10,
            ),
            cap(
                "wrong2",
                "urn:z",
                AuthSource::FromEnv { var: "B".into() },
                5,
            ),
            cap("right", "urn:x", AuthSource::FromEnv { var: "C".into() }, 0),
        ];
        let pass = bind_all(&[comp_input("npm:k", &auth)], &[tgt_input("t", caps)]);
        let b = &pass.bindings[0];
        assert_eq!(b.status, AuthBindingStatus::Bound);
        assert_eq!(b.considered.len(), 2);
        assert!(b.considered.iter().all(|r| r.reason == "audience-mismatch"));
    }

    // 7. Deduplication — identical (kind, params) appears once.
    #[test]
    fn dedupe_drops_identical_candidates() {
        let auth = req_set(vec![token_req("k", "urn:x", false)]);
        let caps = vec![
            cap(
                "first",
                "urn:x",
                AuthSource::FromEnv { var: "X".into() },
                10,
            ),
            cap(
                "duplicate",
                "urn:x",
                AuthSource::FromEnv { var: "X".into() },
                100, // higher priority — but duped away by stable-keep
            ),
        ];
        let pass = bind_all(&[comp_input("npm:k", &auth)], &[tgt_input("t", caps)]);
        let b = &pass.bindings[0];
        assert_eq!(b.priority, 10, "first occurrence wins on dedupe");
    }

    // 8. Deterministic id — same inputs → same id, different inputs → different id.
    #[test]
    fn binding_id_is_deterministic() {
        let id1 = compute_binding_id("npm:k", "tok", "local");
        let id2 = compute_binding_id("npm:k", "tok", "local");
        assert_eq!(id1, id2);
        assert_ne!(id1, compute_binding_id("npm:k", "tok", "remote"));
        assert_ne!(id1, compute_binding_id("npm:k", "other", "local"));
        assert_eq!(id1.len(), 16);
    }

    // 9. Optional + no source → Deferred (not Failed).
    #[test]
    fn optional_unmatched_is_deferred() {
        let auth = req_set(vec![token_req("k", "urn:x", true)]); // optional
        let pass = bind_all(&[comp_input("npm:k", &auth)], &[tgt_input("t", vec![])]);
        assert_eq!(pass.deferred(), 1);
        assert_eq!(pass.failed(), 0);
    }

    // 10. Required + no source → Failed.
    #[test]
    fn required_unmatched_is_failed() {
        let auth = req_set(vec![token_req("k", "urn:x", false)]);
        let pass = bind_all(&[comp_input("npm:k", &auth)], &[tgt_input("t", vec![])]);
        assert_eq!(pass.failed(), 1);
    }

    // 11. Discovery hints synthesize fallback candidates.
    #[test]
    fn discovery_hints_synthesize_candidates() {
        let mut t = token_req("k", "urn:x", false);
        t.discovery = DiscoveryHints {
            env_aliases: vec!["MY_TOKEN".into()],
            ..Default::default()
        };
        let auth = req_set(vec![t]);
        let pass = bind_all(&[comp_input("npm:k", &auth)], &[tgt_input("t", vec![])]);
        let b = &pass.bindings[0];
        assert_eq!(b.status, AuthBindingStatus::Bound);
        match b.source.as_ref().unwrap() {
            AuthSource::FromEnv { var } => assert_eq!(var, "MY_TOKEN"),
            other => panic!("got {:?}", other),
        }
        assert_eq!(b.priority, -100, "synthesised candidates use -100");
    }

    // 12. Audience canonicalisation — case-insensitive equality.
    #[test]
    fn audience_match_is_case_insensitive() {
        let auth = req_set(vec![token_req("k", "Urn:X", false)]);
        let caps = vec![cap(
            "c",
            "URN:x",
            AuthSource::FromEnv { var: "X".into() },
            0,
        )];
        let pass = bind_all(&[comp_input("npm:k", &auth)], &[tgt_input("t", caps)]);
        assert_eq!(pass.resolved(), 1);
        let b = &pass.bindings[0];
        assert_eq!(b.audience, "urn:x");
    }

    // 13. Cross product — N components × M targets emits N*M (per-req) bindings
    //     in stable order (target outer, component inner).
    #[test]
    fn cross_product_emits_bindings_in_stable_order() {
        let a1 = req_set(vec![token_req("a", "urn:a", false)]);
        let a2 = req_set(vec![token_req("b", "urn:b", false)]);
        let pass = bind_all(
            &[comp_input("npm:c1", &a1), comp_input("npm:c2", &a2)],
            &[tgt_input("t1", vec![]), tgt_input("t2", vec![])],
        );
        // 2 targets * 2 components * 1 req each = 4 bindings.
        assert_eq!(pass.bindings.len(), 4);
        assert_eq!(pass.bindings[0].target, "t1");
        assert_eq!(pass.bindings[0].component, "npm:c1");
        assert_eq!(pass.bindings[1].target, "t1");
        assert_eq!(pass.bindings[1].component, "npm:c2");
        assert_eq!(pass.bindings[2].target, "t2");
        assert_eq!(pass.bindings[2].component, "npm:c1");
        assert_eq!(pass.bindings[3].target, "t2");
        assert_eq!(pass.bindings[3].component, "npm:c2");
    }

    // 14. Determinism property test — same input → byte-identical output.
    //     Pure logic determinism (no `proptest` crate dep needed): we sample
    //     several non-trivial inputs and assert byte-equality across two runs.
    #[test]
    fn prop_determinism_byte_identical() {
        let inputs: Vec<(Vec<TokenRequirement>, Vec<AuthCapability>)> = vec![
            (
                vec![
                    token_req("a", "urn:a", false),
                    token_req("b", "urn:b", true),
                ],
                vec![
                    cap("c1", "urn:a", AuthSource::FromEnv { var: "A".into() }, 10),
                    cap(
                        "c2",
                        "urn:b",
                        AuthSource::FromCli {
                            command: "x".into(),
                        },
                        5,
                    ),
                ],
            ),
            (
                vec![token_req("z", "Z", false)],
                vec![
                    cap(
                        "c1",
                        "z",
                        AuthSource::FromSecretsStore {
                            backend: "vault".into(),
                            path: "p".into(),
                        },
                        0,
                    ),
                    cap("c2", "z", AuthSource::FromEnv { var: "Z".into() }, 100),
                ],
            ),
        ];
        for (toks, caps) in inputs {
            let auth = req_set(toks);
            let p1 = bind_all(
                &[comp_input("npm:k", &auth)],
                &[tgt_input("t", caps.clone())],
            );
            let p2 = bind_all(&[comp_input("npm:k", &auth)], &[tgt_input("t", caps)]);
            // Serialise and compare bytes — strongest determinism signal.
            let s1 = serde_json::to_string(&p1.bindings).unwrap();
            let s2 = serde_json::to_string(&p2.bindings).unwrap();
            assert_eq!(s1, s2, "bind_all not deterministic");
        }
    }

    // 15. Serialisation round-trip for AuthBinding inside a lockfile-like blob.
    #[test]
    fn binding_round_trips_through_yaml() {
        let auth = req_set(vec![token_req("k", "urn:x", false)]);
        let caps = vec![cap(
            "c",
            "urn:x",
            AuthSource::FromEnv { var: "X".into() },
            0,
        )];
        let pass = bind_all(&[comp_input("npm:k", &auth)], &[tgt_input("t", caps)]);
        let s = serde_yaml::to_string(&pass.bindings).unwrap();
        let back: Vec<AuthBinding> = serde_yaml::from_str(&s).unwrap();
        assert_eq!(pass.bindings, back);
    }
}
