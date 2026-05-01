//! Well-known constants — filenames, paths, and URLs that several crates
//! reference. Centralized here so a rename or URL flip is a one-file change.
//!
//! Schema URL policy (F-XCUT-02 / ADR-013): the BOM/policy/component schemas
//! are *currently* served from `raw.githubusercontent.com` while the
//! `https://schemas.sindri.dev/v4/` host is being stood up. The
//! [`bom_schema_url`] accessor returns the value of the `SINDRI_SCHEMA_BASE_URL`
//! environment variable at *build time* (via `option_env!`), with the
//! transitional GitHub-raw URL as the compiled-in fallback. Setting
//! `SINDRI_SCHEMA_BASE_URL=https://schemas.sindri.dev/v4` at build time
//! flips the published binaries to the canonical host without a code edit.

/// Default project-scoped manifest filename.
pub const PROJECT_MANIFEST_FILENAME: &str = "sindri.yaml";

/// Default project-scoped install-policy filename.
pub const PROJECT_POLICY_FILENAME: &str = "sindri.policy.yaml";

/// Build-time-overridable base URL for v4 JSON Schemas. See module docs.
///
/// Trailing slash is *not* included; combine via [`schema_url`].
pub const SCHEMA_BASE_URL: &str = match option_env!("SINDRI_SCHEMA_BASE_URL") {
    Some(v) => v,
    None => "https://raw.githubusercontent.com/pacphi/sindri/v4/v4/schemas",
};

/// Build a fully-qualified schema URL for `<name>.json` under the configured
/// base. Examples: `bom`, `policy`, `component`, `registry-index`.
pub fn schema_url(name: &str) -> String {
    format!("{}/{}.json", SCHEMA_BASE_URL, name)
}

/// Convenience accessor for the BOM-manifest schema URL.
pub fn bom_schema_url() -> String {
    schema_url("bom")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bom_url_is_well_formed() {
        let url = bom_schema_url();
        assert!(url.starts_with("http"));
        assert!(url.ends_with("/bom.json"));
    }

    #[test]
    fn schema_url_appends_name() {
        assert!(schema_url("policy").ends_with("/policy.json"));
        assert!(schema_url("component").ends_with("/component.json"));
    }

    #[test]
    fn filenames_are_canonical() {
        assert_eq!(PROJECT_MANIFEST_FILENAME, "sindri.yaml");
        assert_eq!(PROJECT_POLICY_FILENAME, "sindri.policy.yaml");
    }
}
