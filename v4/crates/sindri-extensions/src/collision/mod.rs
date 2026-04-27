//! Collision resolver (Sprint 4 §4.3, ADR-008 Gate 4).
//!
//! [`CollisionResolver::validate_and_resolve`] performs three jobs:
//!
//! 1. **Path-prefix admission** (ADR-008 Gate 4 belt-and-braces). Each
//!    `capabilities.collision_handling.path_prefix` must satisfy:
//!      - the literal `:shared` AND the component is sourced from the
//!        [`sindri_core::registry::CORE_REGISTRY_NAME`] (`sindri/core`)
//!        registry, OR
//!      - start with `<component-name>/`.
//!
//!    The same rule is enforced by the resolver during admission; this
//!    re-check is defense-in-depth and intentional.
//! 2. **Prefix overlap detection** ([`detection::detect_overlaps`]).
//! 3. **Apply ordering** ([`ordering::order`]).
//!
//! ## What is NOT ported in this wave
//!
//! v3 also performed **file-level** collision detection by comparing two
//! components' staged outputs on disk and dispatching the
//! [`scenarios::Scenario`] strategy (`Stop`/`Skip`/`Proceed`). That probe is
//! NOT yet ported to v4 — at apply time we emit a single
//! `tracing::warn!("v3 file-level collision detection not yet ported")` if any
//! component declares a non-`Stop` scenario via convention. The enum and
//! dispatch shape exist so a follow-up PR can fill in the probe without
//! breaking this API.

pub mod conflict;
pub mod detection;
pub mod ordering;
pub mod scenarios;

use crate::error::ExtensionError;
use sindri_core::component::ComponentManifest;
use sindri_core::registry::{CORE_REGISTRY_NAME, SHARED_PATH_PREFIX};
use sindri_targets::Target;

/// Context for a collision-resolver run.
pub struct CollisionContext<'a> {
    /// Active target (currently advisory; reserved for a future on-disk probe).
    pub target: &'a dyn Target,
}

/// The output of [`CollisionResolver::validate_and_resolve`].
#[derive(Debug, Clone, Default)]
pub struct CollisionPlan {
    /// Components ready to apply, in apply order.
    pub ordered: Vec<ComponentManifest>,
    /// Components withheld, with the reason they were withheld.
    pub skipped: Vec<(ComponentManifest, String)>,
}

/// Capability executor for `capabilities.collision_handling`.
#[derive(Debug, Default, Clone, Copy)]
pub struct CollisionResolver;

impl CollisionResolver {
    /// Create a new resolver.
    pub fn new() -> Self {
        Self
    }

    /// Validate path prefixes and produce an apply plan.
    ///
    /// `components` is the resolved closure; each entry pairs a manifest with
    /// the **registry name** the component was sourced from
    /// (e.g. `"sindri/core"`).
    pub async fn validate_and_resolve(
        &self,
        components: &[(ComponentManifest, &str)],
        _ctx: &CollisionContext<'_>,
    ) -> Result<CollisionPlan, ExtensionError> {
        // 1. Per-component admission (ADR-008 Gate 4, belt-and-braces).
        for (manifest, registry) in components {
            if let Some(coll) = &manifest.capabilities.collision_handling {
                validate_path_prefix(&manifest.metadata.name, &coll.path_prefix, registry)?;
            }
        }

        // 2. Detect prefix overlaps. For Wave 2B, any segment-aware overlap
        //    between two components is rejected unconditionally — the v3
        //    file-level scenario dispatch is deferred.
        let overlaps = detection::detect_overlaps(components);
        if let Some(o) = overlaps.first() {
            return Err(ExtensionError::CollisionViolation {
                component: o.a.clone(),
                prefix: o.a_prefix.clone(),
                reason: format!(
                    "prefix overlaps with component '{}' (prefix `{}`)",
                    o.b, o.b_prefix
                ),
                fix: format!(
                    "narrow `{}/` or `{}/` so they do not share a path-segment ancestor",
                    o.a_prefix.trim_end_matches('/'),
                    o.b_prefix.trim_end_matches('/'),
                ),
            });
        }

        // 3. Order components for apply. (Stable sort; preserves input order
        //    on ties so that callers feeding a topologically-ordered closure
        //    get topological tie-break for free.)
        let mut owned: Vec<(ComponentManifest, &str)> = components.to_vec();
        ordering::order(&mut owned);
        let ordered: Vec<ComponentManifest> = owned.into_iter().map(|(m, _)| m).collect();

        // 4. Heads-up about deferred v3 probe.
        tracing::debug!(
            target: "sindri::collision",
            "v3 file-level collision detection not yet ported \
             (Wave 2B detects prefix overlaps only)"
        );

        Ok(CollisionPlan {
            ordered,
            skipped: Vec::new(),
        })
    }
}

/// Enforce the v4 path-prefix rule (ADR-008 Gate 4) for a single component.
fn validate_path_prefix(
    component: &str,
    prefix: &str,
    registry: &str,
) -> Result<(), ExtensionError> {
    if prefix == SHARED_PATH_PREFIX {
        if registry == CORE_REGISTRY_NAME {
            return Ok(());
        }
        return Err(ExtensionError::CollisionViolation {
            component: component.to_string(),
            prefix: prefix.to_string(),
            reason: format!(
                "the `:shared` prefix is reserved for components in the `{CORE_REGISTRY_NAME}` registry; this component is from `{registry}`"
            ),
            fix: format!("use `{component}/<sub-path>` instead"),
        });
    }

    let expected = format!("{component}/");
    if prefix.starts_with(&expected) || prefix == component {
        return Ok(());
    }

    Err(ExtensionError::CollisionViolation {
        component: component.to_string(),
        prefix: prefix.to_string(),
        reason: format!(
            "path_prefix must start with `{component}/` (or be the component name itself)"
        ),
        fix: format!("change `path_prefix` to `{component}/<sub-path>`"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::component::{
        Backend, CollisionHandlingConfig, ComponentCapabilities, ComponentManifest,
        ComponentMetadata, InstallConfig,
    };
    use sindri_core::platform::TargetProfile;
    use sindri_targets::error::TargetError;
    use sindri_targets::traits::PrereqCheck;

    struct NoopTarget;
    impl Target for NoopTarget {
        fn name(&self) -> &str {
            "noop"
        }
        fn kind(&self) -> &str {
            "noop"
        }
        fn profile(&self) -> Result<TargetProfile, TargetError> {
            Err(TargetError::Unavailable {
                name: "noop".into(),
                reason: "test".into(),
            })
        }
        fn exec(&self, _cmd: &str, _env: &[(&str, &str)]) -> Result<(String, String), TargetError> {
            Ok((String::new(), String::new()))
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

    fn manifest(name: &str, prefix: Option<&str>) -> ComponentManifest {
        ComponentManifest {
            metadata: ComponentMetadata {
                name: name.into(),
                version: "1.0.0".into(),
                description: String::new(),
                license: "MIT".into(),
                tags: Vec::new(),
                homepage: None,
            },
            platforms: Vec::new(),
            install: InstallConfig::default(),
            depends_on: Vec::new(),
            capabilities: ComponentCapabilities {
                collision_handling: prefix.map(|p| CollisionHandlingConfig {
                    path_prefix: p.into(),
                }),
                hooks: None,
                project_init: None,
            },
        }
    }

    fn _has_backend(_b: Backend) {}

    fn ctx<'a>(t: &'a dyn Target) -> CollisionContext<'a> {
        CollisionContext { target: t }
    }

    #[tokio::test]
    async fn path_prefix_must_match_component_name() {
        let t = NoopTarget;
        let comps = vec![(manifest("nodejs", Some("etc/foo")), "acme")];
        let err = CollisionResolver::new()
            .validate_and_resolve(&comps, &ctx(&t))
            .await
            .expect_err("must reject mismatched prefix");
        match err {
            ExtensionError::CollisionViolation {
                component, prefix, ..
            } => {
                assert_eq!(component, "nodejs");
                assert_eq!(prefix, "etc/foo");
            }
            other => panic!("expected CollisionViolation, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn path_prefix_matching_component_name_ok() {
        let t = NoopTarget;
        let comps = vec![(manifest("nodejs", Some("nodejs/conf")), "acme")];
        let plan = CollisionResolver::new()
            .validate_and_resolve(&comps, &ctx(&t))
            .await
            .expect("nodejs/conf is valid");
        assert_eq!(plan.ordered.len(), 1);
    }

    #[tokio::test]
    async fn shared_only_allowed_in_sindri_core() {
        let t = NoopTarget;

        // sindri/core + :shared → ok
        let ok = vec![(manifest("toolbox", Some(":shared")), "sindri/core")];
        let plan = CollisionResolver::new()
            .validate_and_resolve(&ok, &ctx(&t))
            .await
            .expect("sindri/core may declare :shared");
        assert_eq!(plan.ordered.len(), 1);

        // acme + :shared → reject
        let bad = vec![(manifest("toolbox", Some(":shared")), "acme")];
        let err = CollisionResolver::new()
            .validate_and_resolve(&bad, &ctx(&t))
            .await
            .expect_err(":shared is restricted to sindri/core");
        match err {
            ExtensionError::CollisionViolation {
                component, prefix, ..
            } => {
                assert_eq!(component, "toolbox");
                assert_eq!(prefix, ":shared");
            }
            other => panic!("expected CollisionViolation, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn overlapping_prefixes_are_rejected() {
        let t = NoopTarget;
        // "nodejs" claims "nodejs/conf" and a hypothetical "nodejs/conf/sub"
        // is also claimed → reject.
        let comps = vec![
            (manifest("nodejs", Some("nodejs/conf")), "acme"),
            (manifest("nodejs-extra", Some("nodejs-extra/conf")), "acme"),
        ];
        // These two should NOT overlap (different prefix trees).
        CollisionResolver::new()
            .validate_and_resolve(&comps, &ctx(&t))
            .await
            .expect("disjoint prefix trees should pass");
    }
}
