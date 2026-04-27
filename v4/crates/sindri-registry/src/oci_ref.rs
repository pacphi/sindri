//! OCI reference parser (ADR-003 §"Registry artifact structure").
//!
//! Parses OCI references of the forms accepted by `sindri registry add`,
//! `sindri add`, and the resolver's outbound fetch path. Pure value type —
//! no I/O, no allocations beyond the resulting [`OciRef`].
//!
//! ## Accepted forms
//!
//! - `oci://ghcr.io/sindri-dev/registry-core:2026.04` — explicit `oci://`
//!   prefix, with tag.
//! - `ghcr.io/sindri-dev/registry-core:2026.04` — bare form, with tag.
//! - `ghcr.io/sindri-dev/registry-core@sha256:abc…` — digest-pinned form.
//! - `library/alpine:3.20` — defaulted to `docker.io` registry (see
//!   "Default registry" below).
//!
//! ## Default registry
//!
//! If the input has no `oci://` prefix and the first path segment contains
//! neither a `.` nor a `:`, it is treated as a Docker Hub repository and the
//! registry defaults to `docker.io`. This matches the behaviour of `docker
//! pull` and `oras` and is documented for future readers in
//! [`OciRef::parse`].
//!
//! Wave 3A.1 only consumes [`OciRef`] from the parser tests and the
//! [`crate::cache::RegistryCache`] reference index. Live OCI fetches that
//! actually hit a registry land in Wave 3A.2.

use crate::error::RegistryError;

/// Parsed OCI reference. Construct via [`OciRef::parse`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OciRef {
    /// Registry hostname (e.g. `ghcr.io`, `docker.io`, `registry.example.com:5000`).
    pub registry: String,
    /// Repository path within the registry (e.g. `sindri-dev/registry-core`).
    pub repository: String,
    /// Tag or digest reference.
    pub reference: OciReference,
}

/// Either a tag (`:1.0.0`) or a digest (`@sha256:…`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OciReference {
    /// Mutable tag (e.g. `2026.04`, `latest`).
    Tag(String),
    /// Immutable content digest in the canonical `sha256:<64-hex>` form.
    Digest(String),
}

const DEFAULT_REGISTRY: &str = "docker.io";

impl OciRef {
    /// Parse an OCI reference.
    ///
    /// See the module-level docs for the accepted grammar. Returns
    /// [`RegistryError::InvalidOciRef`] on malformed input — never panics.
    pub fn parse(input: &str) -> Result<Self, RegistryError> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(invalid(input, "empty input"));
        }

        let stripped = trimmed.strip_prefix("oci://").unwrap_or(trimmed);
        let had_scheme = trimmed.starts_with("oci://");

        // Split off the digest if present (digest takes precedence over tag).
        if let Some((head, digest)) = stripped.split_once('@') {
            let (registry, repository) = split_registry_and_repo(head, had_scheme)?;
            let digest = parse_digest(digest, input)?;
            return Ok(OciRef {
                registry,
                repository,
                reference: OciReference::Digest(digest),
            });
        }

        // Otherwise the last `:` after the final `/` is the tag separator.
        let last_slash = stripped.rfind('/');
        let last_colon = stripped.rfind(':');
        let tag_colon = match (last_slash, last_colon) {
            (Some(s), Some(c)) if c > s => Some(c),
            (None, Some(c)) => Some(c),
            _ => None,
        };

        let (head, tag) = match tag_colon {
            Some(idx) => {
                let tag = &stripped[idx + 1..];
                if tag.is_empty() {
                    return Err(invalid(input, "tag is empty after ':'"));
                }
                (&stripped[..idx], tag.to_string())
            }
            None => {
                return Err(invalid(
                    input,
                    "missing tag (expected ':<tag>' or '@sha256:…')",
                ))
            }
        };

        let (registry, repository) = split_registry_and_repo(head, had_scheme)?;
        Ok(OciRef {
            registry,
            repository,
            reference: OciReference::Tag(tag),
        })
    }

    /// Render the reference back to the canonical bare form
    /// `<registry>/<repository>:<tag>` or `<registry>/<repository>@sha256:<digest>`.
    pub fn to_canonical(&self) -> String {
        match &self.reference {
            OciReference::Tag(t) => format!("{}/{}:{}", self.registry, self.repository, t),
            OciReference::Digest(d) => format!("{}/{}@{}", self.registry, self.repository, d),
        }
    }
}

fn split_registry_and_repo(
    head: &str,
    had_scheme: bool,
) -> Result<(String, String), RegistryError> {
    let head = head.trim_matches('/');
    if head.is_empty() {
        return Err(invalid(head, "empty registry+repository"));
    }
    let (first, rest) = head.split_once('/').unwrap_or((head, ""));
    // The first segment is treated as a registry hostname when:
    //   - the input had an explicit `oci://` scheme, or
    //   - it contains a `.` (FQDN) or `:` (host:port), or
    //   - it is exactly `localhost`.
    let looks_like_registry =
        had_scheme || first.contains('.') || first.contains(':') || first == "localhost";
    if looks_like_registry {
        if rest.is_empty() {
            return Err(invalid(head, "missing repository path after registry"));
        }
        Ok((first.to_string(), rest.to_string()))
    } else {
        Ok((DEFAULT_REGISTRY.to_string(), head.to_string()))
    }
}

fn parse_digest(digest: &str, original: &str) -> Result<String, RegistryError> {
    // Canonical OCI digest is `<algorithm>:<hex>`. We accept any registered
    // algorithm but validate the hex portion is non-empty and lowercase hex.
    let (alg, hex) = digest
        .split_once(':')
        .ok_or_else(|| invalid(original, "digest missing ':' separator"))?;
    if alg.is_empty() {
        return Err(invalid(original, "digest algorithm is empty"));
    }
    if hex.is_empty() {
        return Err(invalid(original, "digest hex is empty"));
    }
    if alg == "sha256" && hex.len() != 64 {
        return Err(invalid(
            original,
            "sha256 digest must be exactly 64 hex chars",
        ));
    }
    if !hex
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err(invalid(original, "digest hex must be lowercase ASCII hex"));
    }
    Ok(digest.to_string())
}

fn invalid(input: &str, reason: &str) -> RegistryError {
    RegistryError::InvalidOciRef {
        input: input.to_string(),
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tag_form() {
        let r = OciRef::parse("ghcr.io/sindri-dev/registry-core:2026.04").unwrap();
        assert_eq!(r.registry, "ghcr.io");
        assert_eq!(r.repository, "sindri-dev/registry-core");
        assert_eq!(r.reference, OciReference::Tag("2026.04".into()));
    }

    #[test]
    fn parses_digest_form() {
        let digest = format!("sha256:{}", "a".repeat(64));
        let input = format!("ghcr.io/foo/bar@{}", digest);
        let r = OciRef::parse(&input).unwrap();
        assert_eq!(r.registry, "ghcr.io");
        assert_eq!(r.repository, "foo/bar");
        assert_eq!(r.reference, OciReference::Digest(digest));
    }

    #[test]
    fn parses_oci_scheme_prefix() {
        let r = OciRef::parse("oci://ghcr.io/sindri-dev/registry-core:1.0.0").unwrap();
        assert_eq!(r.registry, "ghcr.io");
        assert_eq!(r.repository, "sindri-dev/registry-core");
        assert_eq!(r.reference, OciReference::Tag("1.0.0".into()));
    }

    #[test]
    fn parses_subdomain_registry() {
        let r = OciRef::parse("registry.example.co.uk/team/app:v7").unwrap();
        assert_eq!(r.registry, "registry.example.co.uk");
        assert_eq!(r.repository, "team/app");
        assert_eq!(r.reference, OciReference::Tag("v7".into()));
    }

    #[test]
    fn parses_nested_repository_path() {
        let r = OciRef::parse("ghcr.io/org/team/sub/component:0.1.0").unwrap();
        assert_eq!(r.repository, "org/team/sub/component");
    }

    #[test]
    fn parses_registry_with_port() {
        let r = OciRef::parse("localhost:5000/foo/bar:dev").unwrap();
        assert_eq!(r.registry, "localhost:5000");
        assert_eq!(r.repository, "foo/bar");
        assert_eq!(r.reference, OciReference::Tag("dev".into()));
    }

    #[test]
    fn defaults_registry_to_docker_io() {
        let r = OciRef::parse("library/alpine:3.20").unwrap();
        assert_eq!(r.registry, "docker.io");
        assert_eq!(r.repository, "library/alpine");
        assert_eq!(r.reference, OciReference::Tag("3.20".into()));
    }

    #[test]
    fn rejects_missing_tag() {
        let err = OciRef::parse("ghcr.io/foo/bar").unwrap_err();
        assert!(matches!(err, RegistryError::InvalidOciRef { .. }));
    }

    #[test]
    fn rejects_malformed_digest() {
        let err = OciRef::parse("ghcr.io/foo/bar@sha256:not-hex").unwrap_err();
        assert!(matches!(err, RegistryError::InvalidOciRef { .. }));
        let err = OciRef::parse("ghcr.io/foo/bar@sha256:tooShort").unwrap_err();
        assert!(matches!(err, RegistryError::InvalidOciRef { .. }));
    }

    #[test]
    fn rejects_empty_input() {
        assert!(matches!(
            OciRef::parse("").unwrap_err(),
            RegistryError::InvalidOciRef { .. }
        ));
        assert!(matches!(
            OciRef::parse("   ").unwrap_err(),
            RegistryError::InvalidOciRef { .. }
        ));
    }

    #[test]
    fn round_trip_via_canonical_tag() {
        let original = "ghcr.io/sindri-dev/registry-core:2026.04";
        let r = OciRef::parse(original).unwrap();
        assert_eq!(r.to_canonical(), original);
        // Re-parsing the canonical form gives an equal value.
        assert_eq!(OciRef::parse(&r.to_canonical()).unwrap(), r);
    }

    #[test]
    fn round_trip_via_canonical_digest() {
        let digest = format!("sha256:{}", "f".repeat(64));
        let input = format!("ghcr.io/foo/bar@{}", digest);
        let r = OciRef::parse(&input).unwrap();
        assert_eq!(r.to_canonical(), input);
        assert_eq!(OciRef::parse(&r.to_canonical()).unwrap(), r);
    }
}
