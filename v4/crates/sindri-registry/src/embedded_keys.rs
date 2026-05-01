//! Embedded cosign trust keys for first-party Sindri registries (ADR-014).
//!
//! Phase 3 of the 2026-04-30 reconciliation plan (F-REG-01) ships the
//! `EmbeddedKey[]` infrastructure — the same model cosign and Sigstore
//! use for their TUF root distribution. The production [`EMBEDDED_KEYS`]
//! slice is **empty today** because the `sindri-core` cosign signing
//! infrastructure is a prerequisite track that must land out-of-band
//! before any real key bytes can be embedded. Once the keypair exists,
//! its public-key SPKI PEM gets `include_bytes!`-ed into a new entry.
//!
//! ## Rotation model (TUF-inspired)
//!
//! [`EMBEDDED_KEYS`] is a slice. New keys are appended; old keys remain
//! during a documented overlap window so that a binary built before
//! rotation can still verify signatures produced under the new key. After
//! the window expires the old entry is removed in a release.
//!
//! Each entry carries a `valid_until: Option<&'static str>` (RFC3339).
//! `None` means "current key, no expiry." A key whose `valid_until` is
//! in the past is filtered out by [`active_keys_for`].
//!
//! ## Why this exists
//!
//! Without an embedded trust root, every fresh install requires the user
//! to run `sindri registry trust sindri-core --signer cosign:key=...`
//! before fetching anything. The very first download is therefore an
//! attackable TOFU step. Embedded keys eliminate that step for the
//! first-party registry — `sindri install foo` works on a fresh box.
//!
//! See ADR-014 (signed registries with cosign) and the 2026-04-30
//! Phase 3 trust research note for the field-survey rationale.

use crate::error::RegistryError;
use crate::signing::TrustedKey;

/// Single embedded trust-key entry. Static lifetime — entries are baked
/// into the binary at build time.
#[derive(Debug, Clone, Copy)]
pub struct EmbeddedKey {
    /// Logical registry name (e.g. `"sindri-core"`). Matches the alias
    /// callers pass to `CosignVerifier::keys_for(name)`.
    pub registry_alias: &'static str,
    /// Stable 8-char key id derived from SHA-256 of the SPKI bytes,
    /// produced by the cosign signing flow at key-generation time.
    pub key_id: &'static str,
    /// SPKI PEM bytes (P-256 ECDSA public key).
    pub spki_pem: &'static [u8],
    /// Optional RFC3339 expiry. `None` = current, no expiry. A key whose
    /// expiry is in the past is filtered out by [`active_keys_for`].
    pub valid_until: Option<&'static str>,
    /// Generation counter — `1` for the original published key, `2` for
    /// its first rotation, etc. Useful for diagnostic logs.
    pub generation: u32,
}

/// Production embedded-key set.
///
/// **Empty today** — production-grade `sindri-core` cosign signing
/// infrastructure is a prerequisite track. Once the keypair exists,
/// the public key gets a new entry here:
///
/// ```ignore
/// EmbeddedKey {
///     registry_alias: "sindri-core",
///     key_id: "<8-char id>",
///     spki_pem: include_bytes!("../../trust/sindri-core-2026.pub"),
///     valid_until: None,
///     generation: 1,
/// }
/// ```
///
/// Old keys remain in the slice for the rotation overlap window then
/// are removed in a release. The verifier accepts signatures from any
/// active embedded key for the requested registry.
pub static EMBEDDED_KEYS: &[EmbeddedKey] = &[];

/// Filter `keys` to those actively trusted for `registry_alias` at `now`.
///
/// `now` should be an RFC3339 timestamp string; tests can pass any
/// fixed value to make assertions deterministic. In production callers
/// should pass `chrono::Utc::now().to_rfc3339()` (or equivalent).
///
/// Active means: `registry_alias` matches AND (`valid_until` is `None`
/// OR `now <= valid_until` lexicographically — RFC3339 sorts).
pub fn active_keys_for<'a>(
    keys: &'a [EmbeddedKey],
    registry_alias: &str,
    now: &str,
) -> Vec<&'a EmbeddedKey> {
    keys.iter()
        .filter(|k| k.registry_alias == registry_alias)
        .filter(|k| match k.valid_until {
            None => true,
            Some(expiry) => now <= expiry,
        })
        .collect()
}

/// Convert an [`EmbeddedKey`] into a runtime [`TrustedKey`] for the
/// verifier. Returns the same parse error as a disk-loaded key would
/// produce on bad PEM bytes — but in production this should never
/// happen because the bytes were vetted at build time.
pub fn embedded_to_trusted(key: &EmbeddedKey) -> Result<TrustedKey, RegistryError> {
    let pem =
        std::str::from_utf8(key.spki_pem).map_err(|e| RegistryError::TrustKeyParseFailed {
            path: format!("<embedded:{}/{}>", key.registry_alias, key.key_id),
            detail: format!("SPKI PEM bytes are not valid UTF-8: {e}"),
        })?;
    TrustedKey::from_pem(pem).map_err(|e| match e {
        RegistryError::TrustKeyParseFailed { detail, .. } => RegistryError::TrustKeyParseFailed {
            path: format!("<embedded:{}/{}>", key.registry_alias, key.key_id),
            detail,
        },
        other => other,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(alias: &'static str, expiry: Option<&'static str>) -> EmbeddedKey {
        EmbeddedKey {
            registry_alias: alias,
            key_id: "deadbeef",
            spki_pem: b"",
            valid_until: expiry,
            generation: 1,
        }
    }

    #[test]
    fn production_key_set_is_empty_today() {
        // Sanity guard: until production signing infra lands, the array
        // is empty. Adding a real key without removing this guard is a
        // PR-time signal to update the test.
        assert!(
            EMBEDDED_KEYS.is_empty(),
            "EMBEDDED_KEYS gained an entry; update embedded_keys::tests::production_key_set_is_empty_today and ADR-014"
        );
    }

    #[test]
    fn active_keys_for_filters_by_alias() {
        let set = [k("sindri-core", None), k("acme/internal", None)];
        let active = active_keys_for(&set, "sindri-core", "2026-05-01T00:00:00Z");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].registry_alias, "sindri-core");
    }

    #[test]
    fn active_keys_for_excludes_expired() {
        let set = [
            k("sindri-core", Some("2026-04-01T00:00:00Z")),
            k("sindri-core", Some("2027-01-01T00:00:00Z")),
            k("sindri-core", None),
        ];
        let active = active_keys_for(&set, "sindri-core", "2026-05-01T00:00:00Z");
        assert_eq!(active.len(), 2, "one expired key should be filtered out");
    }

    #[test]
    fn active_keys_for_includes_current_window() {
        let set = [k("sindri-core", Some("2027-01-01T00:00:00Z"))];
        let active = active_keys_for(&set, "sindri-core", "2026-05-01T00:00:00Z");
        assert_eq!(
            active.len(),
            1,
            "key inside its valid_until window is active"
        );
    }

    #[test]
    fn active_keys_for_no_expiry_always_active() {
        let set = [k("sindri-core", None)];
        let active = active_keys_for(&set, "sindri-core", "2099-12-31T23:59:59Z");
        assert_eq!(active.len(), 1, "None expiry = always active");
    }

    #[test]
    fn embedded_to_trusted_rejects_invalid_pem() {
        let bad = EmbeddedKey {
            registry_alias: "sindri-core",
            key_id: "badkey00",
            spki_pem: b"not a pem",
            valid_until: None,
            generation: 1,
        };
        let r = embedded_to_trusted(&bad);
        assert!(r.is_err());
    }
}
