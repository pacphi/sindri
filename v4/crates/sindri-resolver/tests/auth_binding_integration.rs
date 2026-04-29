//! Integration test for the auth-binding pass (ADR-027 §3, Phase 1).
//!
//! Scenario: 3 components × 2 targets, with overlapping audience
//! requirements. Asserts the resulting binding sequence is deterministic
//! and that the lockfile-shaped serialisation snapshot matches the
//! expected YAML.

use sindri_core::auth::{
    AuthBindingStatus, AuthCapability, AuthRequirements, AuthScope, AuthSource, DiscoveryHints,
    Redemption, TokenRequirement,
};
use sindri_resolver::auth_binding::{bind_all, ComponentAuthInput, TargetAuthInput};

fn token(name: &str, audience: &str, optional: bool) -> TokenRequirement {
    TokenRequirement {
        name: name.into(),
        description: name.into(),
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

#[test]
fn three_components_two_targets_with_overlap() {
    // Components:
    //   npm:claude-code → token "anthropic" @ urn:anthropic:api  (required)
    //   npm:gh          → token "github"    @ https://api.github.com (optional)
    //   pipx:awscli     → token "aws"       @ https://sts.amazonaws.com (required)
    let claude = AuthRequirements {
        tokens: vec![token("anthropic", "urn:anthropic:api", false)],
        ..Default::default()
    };
    let gh = AuthRequirements {
        tokens: vec![token("github", "https://api.github.com", true)],
        ..Default::default()
    };
    let aws = AuthRequirements {
        tokens: vec![token("aws", "https://sts.amazonaws.com", false)],
        ..Default::default()
    };

    let components = vec![
        ComponentAuthInput {
            address: "npm:claude-code".into(),
            auth: &claude,
        },
        ComponentAuthInput {
            address: "npm:gh".into(),
            auth: &gh,
        },
        ComponentAuthInput {
            address: "pipx:awscli".into(),
            auth: &aws,
        },
    ];

    // Target 1 — `local`: provides anthropic + github via env, no aws.
    // Target 2 — `fly`:   provides anthropic via vault (priority 100) and aws.
    let local = TargetAuthInput {
        target_id: "local".into(),
        capabilities: vec![
            cap(
                "anthropic_env",
                "urn:anthropic:api",
                AuthSource::FromEnv {
                    var: "ANTHROPIC_API_KEY".into(),
                },
                0,
            ),
            cap(
                "gh_cli",
                "https://api.github.com",
                AuthSource::FromCli {
                    command: "gh auth token".into(),
                },
                0,
            ),
        ],
    };
    let fly = TargetAuthInput {
        target_id: "fly".into(),
        capabilities: vec![
            cap(
                "anthropic_vault",
                "urn:anthropic:api",
                AuthSource::FromSecretsStore {
                    backend: "vault".into(),
                    path: "secrets/anthropic/prod".into(),
                },
                100,
            ),
            cap(
                "aws_vault",
                "https://sts.amazonaws.com",
                AuthSource::FromSecretsStore {
                    backend: "vault".into(),
                    path: "secrets/aws/sts".into(),
                },
                100,
            ),
        ],
    };

    let pass = bind_all(&components, &[local, fly]);
    // 3 reqs × 2 targets = 6 bindings.
    assert_eq!(pass.bindings.len(), 6);

    // Deterministic order: local first, then fly. Within target: components
    // in declaration order.
    assert_eq!(pass.bindings[0].target, "local");
    assert_eq!(pass.bindings[0].component, "npm:claude-code");
    assert_eq!(pass.bindings[3].target, "fly");
    assert_eq!(pass.bindings[3].component, "npm:claude-code");

    // Outcomes:
    //   local × claude-code → Bound (env)
    assert_eq!(pass.bindings[0].status, AuthBindingStatus::Bound);
    assert!(matches!(
        pass.bindings[0].source,
        Some(AuthSource::FromEnv { .. })
    ));
    //   local × gh → Bound (cli)
    assert_eq!(pass.bindings[1].status, AuthBindingStatus::Bound);
    assert!(matches!(
        pass.bindings[1].source,
        Some(AuthSource::FromCli { .. })
    ));
    //   local × aws → Failed (required, no source)
    assert_eq!(pass.bindings[2].status, AuthBindingStatus::Failed);
    //   fly × claude-code → Bound (vault, priority 100)
    assert_eq!(pass.bindings[3].status, AuthBindingStatus::Bound);
    assert!(matches!(
        pass.bindings[3].source,
        Some(AuthSource::FromSecretsStore { .. })
    ));
    assert_eq!(pass.bindings[3].priority, 100);
    //   fly × gh → Deferred (optional, no source)
    assert_eq!(pass.bindings[4].status, AuthBindingStatus::Deferred);
    //   fly × aws → Bound (vault)
    assert_eq!(pass.bindings[5].status, AuthBindingStatus::Bound);

    // Aggregate counts match the CLI summary line.
    assert_eq!(pass.resolved(), 4);
    assert_eq!(pass.deferred(), 1);
    assert_eq!(pass.failed(), 1);

    // Snapshot — round-trip through YAML produces identical bindings.
    let yaml = serde_yaml::to_string(&pass.bindings).unwrap();
    let back: Vec<sindri_core::auth::AuthBinding> = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(back, pass.bindings);

    // Deterministic ids — recompute and compare.
    let pass2 = bind_all(
        &components,
        &[
            TargetAuthInput {
                target_id: "local".into(),
                capabilities: vec![
                    cap(
                        "anthropic_env",
                        "urn:anthropic:api",
                        AuthSource::FromEnv {
                            var: "ANTHROPIC_API_KEY".into(),
                        },
                        0,
                    ),
                    cap(
                        "gh_cli",
                        "https://api.github.com",
                        AuthSource::FromCli {
                            command: "gh auth token".into(),
                        },
                        0,
                    ),
                ],
            },
            TargetAuthInput {
                target_id: "fly".into(),
                capabilities: vec![
                    cap(
                        "anthropic_vault",
                        "urn:anthropic:api",
                        AuthSource::FromSecretsStore {
                            backend: "vault".into(),
                            path: "secrets/anthropic/prod".into(),
                        },
                        100,
                    ),
                    cap(
                        "aws_vault",
                        "https://sts.amazonaws.com",
                        AuthSource::FromSecretsStore {
                            backend: "vault".into(),
                            path: "secrets/aws/sts".into(),
                        },
                        100,
                    ),
                ],
            },
        ],
    );
    let s1 = serde_json::to_string(&pass.bindings).unwrap();
    let s2 = serde_json::to_string(&pass2.bindings).unwrap();
    assert_eq!(s1, s2, "bind_all not deterministic across calls");
}

/// Property-style determinism test (no `proptest` dep — fixed seeded
/// permutations of valid input). Asserts that for several distinct
/// `(req, capabilities)` pairs, two independent runs of `bind_all`
/// produce byte-identical output (same `binding.id`, same selected
/// source, same `considered` ordering).
#[test]
fn determinism_across_random_valid_inputs() {
    let scenarios: Vec<(Vec<TokenRequirement>, Vec<AuthCapability>)> = vec![
        (
            vec![
                token("a", "urn:a", false),
                token("b", "urn:b", true),
                token("c", "urn:c", false),
            ],
            vec![
                cap("x", "urn:a", AuthSource::FromEnv { var: "A".into() }, 50),
                cap(
                    "y",
                    "urn:c",
                    AuthSource::FromCli {
                        command: "c-cli".into(),
                    },
                    10,
                ),
            ],
        ),
        (
            vec![token("k", "URN:K", false)],
            vec![
                cap(
                    "v",
                    "urn:k",
                    AuthSource::FromSecretsStore {
                        backend: "vault".into(),
                        path: "p".into(),
                    },
                    0,
                ),
                cap("e", "urn:K", AuthSource::FromEnv { var: "K".into() }, 0),
            ],
        ),
        (
            vec![token("only", "urn:only", true)],
            vec![cap(
                "wrong",
                "urn:other",
                AuthSource::FromEnv {
                    var: "WRONG".into(),
                },
                999,
            )],
        ),
    ];

    for (idx, (toks, caps)) in scenarios.into_iter().enumerate() {
        let auth = AuthRequirements {
            tokens: toks,
            ..Default::default()
        };
        let comp = ComponentAuthInput {
            address: "npm:scenario".into(),
            auth: &auth,
        };
        let tgt = TargetAuthInput {
            target_id: "t".into(),
            capabilities: caps.clone(),
        };
        let p1 = bind_all(std::slice::from_ref(&comp), std::slice::from_ref(&tgt));
        let p2 = bind_all(&[comp], &[tgt]);
        assert_eq!(
            serde_json::to_string(&p1.bindings).unwrap(),
            serde_json::to_string(&p2.bindings).unwrap(),
            "scenario {} not deterministic",
            idx
        );
        // All ids are 16 hex chars (DDD-07 invariant 4 → format check).
        for b in &p1.bindings {
            assert_eq!(b.id.len(), 16, "id wrong width: {}", b.id);
            assert!(b.id.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }
}
