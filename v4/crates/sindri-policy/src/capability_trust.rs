//! Centralised capability-trust rule (Gate 4 of ADR-008).
//!
//! Phase 2 of the 2026-04-30 reconciliation plan (F-REG-06): the
//! collision-handling path-prefix rule used to be implemented twice —
//! once in `sindri-registry::lint` (publish-time check) and once in
//! `sindri-resolver::admission` (resolve-time check). Both call sites
//! now delegate to [`check_collision_prefix`] so the rule has a single
//! source of truth.
//!
//! The rule, restated:
//!
//! 1. If a component declares `capabilities.collisionHandling.pathPrefix`,
//!    the prefix must either:
//!    - be `:shared` and be sourced from `sindri/core` (the only registry
//!      that may claim cross-component shared paths), OR
//!    - have its first path segment exactly equal the component's name.
//! 2. Components without a `collisionHandling` block are unconstrained.

use sindri_core::registry::{CORE_REGISTRY_NAME, SHARED_PATH_PREFIX};

/// Why a collision-prefix declaration was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollisionPrefixViolationKind {
    /// `:shared` was declared by a component sourced outside `sindri/core`.
    SharedReservedForCore,
    /// The first path segment did not equal the component name.
    PrefixDoesNotMatchName,
}

/// Structured violation returned by [`check_collision_prefix`].
///
/// Call sites format their own message — `lint.rs` produces a
/// publish-time `LINT_COLLISION_PREFIX` diagnostic; `admission.rs`
/// produces a resolve-time `ADM_CAPABILITY_TRUST_VIOLATION` deny. Both
/// derive their text from the same fields here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollisionPrefixViolation {
    pub kind: CollisionPrefixViolationKind,
    pub component_name: String,
    pub registry_name: String,
    pub declared_prefix: String,
}

impl CollisionPrefixViolation {
    /// Human-readable explanation of the violation, suitable for a CLI message.
    pub fn message(&self) -> String {
        match self.kind {
            CollisionPrefixViolationKind::SharedReservedForCore => format!(
                "Component `{}` from registry `{}` declares the `:shared` \
                 collision-handling prefix, which is reserved for `{}`",
                self.component_name, self.registry_name, CORE_REGISTRY_NAME
            ),
            CollisionPrefixViolationKind::PrefixDoesNotMatchName => format!(
                "Component `{}` declares collision-handling prefix `{}` whose \
                 first segment must equal the component name `{}`",
                self.component_name, self.declared_prefix, self.component_name
            ),
        }
    }

    /// Suggested fix string, suitable for a CLI message.
    pub fn fix(&self) -> &'static str {
        match self.kind {
            CollisionPrefixViolationKind::SharedReservedForCore => {
                "Replace `:shared` with `{component-name}/...` or publish via the core registry"
            }
            CollisionPrefixViolationKind::PrefixDoesNotMatchName => {
                "Use a path of the form `{component-name}/...` (or publish via the core registry to use `:shared`)"
            }
        }
    }
}

/// Validate a single component's collision-handling prefix.
///
/// Returns `Ok(())` if the prefix is well-formed (or if the component
/// declares no prefix — pass `None` for `declared_prefix`). Returns
/// `Err(CollisionPrefixViolation)` otherwise.
///
/// Both `sindri-registry::lint` (publish time) and
/// `sindri-resolver::admission::check_capability_trust` (resolve time)
/// call this — defense in depth across the publish/install boundary.
pub fn check_collision_prefix(
    component_name: &str,
    registry_name: &str,
    declared_prefix: Option<&str>,
) -> Result<(), CollisionPrefixViolation> {
    let prefix = match declared_prefix {
        None => return Ok(()),
        Some(p) => p.trim(),
    };

    // The :shared escape hatch — only the core registry may use it.
    if prefix == SHARED_PATH_PREFIX {
        if registry_name == CORE_REGISTRY_NAME {
            return Ok(());
        }
        return Err(CollisionPrefixViolation {
            kind: CollisionPrefixViolationKind::SharedReservedForCore,
            component_name: component_name.into(),
            registry_name: registry_name.into(),
            declared_prefix: prefix.into(),
        });
    }

    // Otherwise the first path segment must equal the component's name.
    let normalized = prefix.trim_start_matches('/');
    let first_segment = normalized.split('/').next().unwrap_or("");

    if first_segment == component_name {
        Ok(())
    } else {
        Err(CollisionPrefixViolation {
            kind: CollisionPrefixViolationKind::PrefixDoesNotMatchName,
            component_name: component_name.into(),
            registry_name: registry_name.into(),
            declared_prefix: prefix.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_prefix_is_unconstrained() {
        assert!(check_collision_prefix("foo", "any-registry", None).is_ok());
    }

    #[test]
    fn shared_allowed_for_core_registry() {
        assert!(
            check_collision_prefix("nodejs", CORE_REGISTRY_NAME, Some(SHARED_PATH_PREFIX)).is_ok()
        );
    }

    #[test]
    fn shared_rejected_for_third_party_registry() {
        let err = check_collision_prefix("nodejs", "acme/internal", Some(SHARED_PATH_PREFIX))
            .expect_err("shared from non-core must reject");
        assert_eq!(
            err.kind,
            CollisionPrefixViolationKind::SharedReservedForCore
        );
        assert!(err.message().contains(CORE_REGISTRY_NAME));
    }

    #[test]
    fn matching_first_segment_admitted() {
        assert!(check_collision_prefix("nodejs", "any-registry", Some("nodejs/bin")).is_ok());
    }

    #[test]
    fn matching_first_segment_with_leading_slash_admitted() {
        assert!(check_collision_prefix("nodejs", "any-registry", Some("/nodejs/bin")).is_ok());
    }

    #[test]
    fn mismatched_first_segment_rejected() {
        let err = check_collision_prefix("nodejs", "any-registry", Some("etc/passwd"))
            .expect_err("mismatched prefix must reject");
        assert_eq!(
            err.kind,
            CollisionPrefixViolationKind::PrefixDoesNotMatchName
        );
        assert!(err.message().contains("first segment"));
    }

    #[test]
    fn empty_prefix_rejected_as_mismatch() {
        let err = check_collision_prefix("nodejs", "any-registry", Some(""))
            .expect_err("empty prefix must reject");
        assert_eq!(
            err.kind,
            CollisionPrefixViolationKind::PrefixDoesNotMatchName
        );
    }

    #[test]
    fn whitespace_prefix_trimmed_then_rejected() {
        let err = check_collision_prefix("nodejs", "any-registry", Some("   "))
            .expect_err("whitespace-only prefix must reject");
        assert_eq!(
            err.kind,
            CollisionPrefixViolationKind::PrefixDoesNotMatchName
        );
    }

    #[test]
    fn fix_strings_are_non_empty() {
        let v_shared = CollisionPrefixViolation {
            kind: CollisionPrefixViolationKind::SharedReservedForCore,
            component_name: "nodejs".into(),
            registry_name: "acme".into(),
            declared_prefix: ":shared".into(),
        };
        assert!(!v_shared.fix().is_empty());

        let v_mismatch = CollisionPrefixViolation {
            kind: CollisionPrefixViolationKind::PrefixDoesNotMatchName,
            component_name: "nodejs".into(),
            registry_name: "acme".into(),
            declared_prefix: "etc/passwd".into(),
        };
        assert!(!v_mismatch.fix().is_empty());
    }
}
