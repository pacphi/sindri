//! SBOM generation (ADR-007).
//!
//! Reads the resolved lockfile (`sindri.lock` or `sindri.<target>.lock`) and
//! emits one of:
//!
//! * **SPDX 2.3 JSON** — the default. Hand-built against the SPDX 2.3
//!   specification using `serde_json` because the Rust SPDX crate
//!   (`spdx-rs` 0.5.x) is parsing-focused and does not provide a stable
//!   document-builder API at v1.0+.
//! * **CycloneDX 1.6 JSON** — also hand-built. The `cyclonedx-bom` crate
//!   (0.8.1) currently supports specs 1.3–1.5 only; rather than emit an
//!   older spec, we serialize 1.6-conformant JSON directly.
//!
//! Both writers are deterministic for a fixed lockfile + a fixed `created`
//! timestamp + a fixed document UUID. The UUID and timestamp are sourced
//! from the host clock / a v4 RNG, so two consecutive runs differ only in
//! those two fields.
//!
//! ## PURL conventions
//!
//! Per the [Package URL spec](https://github.com/package-url/purl-spec) we
//! map each backend to a PURL `type` as follows:
//!
//! | Backend     | PURL type | Example                                                |
//! | ----------- | --------- | ------------------------------------------------------ |
//! | `mise`      | `mise`    | `pkg:mise/nodejs@22.0.0`                               |
//! | `npm`       | `npm`     | `pkg:npm/typescript@5.4.5`                             |
//! | `cargo`     | `cargo`   | `pkg:cargo/ripgrep@14.1.0`                             |
//! | `pipx`      | `pypi`    | `pkg:pypi/black@24.4.2`                                |
//! | `go-install`| `golang`  | `pkg:golang/sigs.k8s.io/kind@v0.22.0`                  |
//! | `brew`      | `brew`    | `pkg:brew/git@2.45.0`                                  |
//! | `apt`/`dnf`/`zypper`/`pacman`/`apk` | `<backend>` | `pkg:apt/curl@8.5.0` |
//! | `winget`    | `winget`  | `pkg:winget/Git.Git@2.45.0`                            |
//! | `scoop`     | `scoop`   | `pkg:scoop/git@2.45.0`                                 |
//! | `sdkman`    | `sdkman`  | `pkg:sdkman/java@21.0.5-tem`                           |
//! | `binary`    | `generic` | `pkg:generic/foo@1.2.3?download_url=https://...`       |
//! | `script`    | `generic` | `pkg:generic/foo@1.0.0`                                |
//! | `collection`| `generic` | `pkg:generic/<name>@<version>`                         |
//!
//! `mise`, `brew`, `winget`, `scoop`, and `sdkman` are not in the official
//! PURL type list; we use the backend name verbatim and document this here.
//! Tools that need to round-trip these will treat them as opaque types.
//!
//! ## OCI digests
//!
//! When `ResolvedComponent::manifest_digest` is populated (registry-level
//! OCI digest, ADR-003 / ADR-014), it is emitted as:
//!
//! * SPDX 2.3 — an `externalRefs` entry with
//!   `referenceCategory: "PERSISTENT-ID"`, `referenceType: "oci"`.
//! * CycloneDX 1.6 — an extra `hashes[]` entry with `alg: "OCI-DIGEST"`.
//!   `OCI-DIGEST` is **not** a CycloneDX-defined hash algorithm; we use it
//!   as a custom marker. Consumers that strictly validate against the
//!   CycloneDX schema will reject it, so a stricter mode may be added later.

use sindri_core::component::{Backend, ComponentManifest};
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_STALE_LOCKFILE, EXIT_SUCCESS};
use sindri_core::lockfile::{Lockfile, ResolvedComponent};
use std::path::{Path, PathBuf};

/// Output format selector, parsed from `--format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BomFormat {
    /// SPDX 2.3 JSON.
    Spdx,
    /// CycloneDX 1.6 JSON.
    CycloneDx,
}

impl BomFormat {
    /// Parse `--format` value. Accepts `spdx` and `cyclonedx`.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "spdx" | "spdx-json" => Ok(BomFormat::Spdx),
            "cyclonedx" | "cyclonedx-json" | "cdx" => Ok(BomFormat::CycloneDx),
            other => Err(format!(
                "unknown SBOM format '{}': expected 'spdx' or 'cyclonedx'",
                other
            )),
        }
    }

    fn default_extension(self) -> &'static str {
        match self {
            BomFormat::Spdx => "spdx.json",
            BomFormat::CycloneDx => "cdx.json",
        }
    }
}

/// CLI args for `sindri bom`.
pub struct BomArgs {
    /// Output format (`spdx` or `cyclonedx`).
    pub format: String,
    /// Target name (`local` or a configured remote).
    pub target: String,
    /// Output path. Defaults to `sindri.<target>.bom.<ext>`.
    pub output: Option<String>,
}

/// Run `sindri bom`.
pub fn run(args: BomArgs) -> i32 {
    let format = match BomFormat::parse(&args.format) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let lock_name = lockfile_name(&args.target);
    let lockfile_path = PathBuf::from(&lock_name);
    if !lockfile_path.exists() {
        eprintln!(
            "Lockfile '{}' not found. Run `sindri resolve` first.",
            lock_name
        );
        return EXIT_STALE_LOCKFILE;
    }

    let content = match std::fs::read_to_string(&lockfile_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Cannot read lockfile: {}", e);
            return EXIT_STALE_LOCKFILE;
        }
    };

    let lockfile: Lockfile = match serde_json::from_str(&content) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Malformed lockfile: {}", e);
            return EXIT_STALE_LOCKFILE;
        }
    };

    let sbom = render(&lockfile, format);

    let default = format!(
        "sindri.{}.bom.{}",
        lockfile.target,
        format.default_extension()
    );
    let output_path = args.output.unwrap_or(default);

    match std::fs::write(&output_path, &sbom) {
        Ok(_) => {
            println!("SBOM written to {}", output_path);
            EXIT_SUCCESS
        }
        Err(e) => {
            eprintln!("Failed to write SBOM: {}", e);
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}

/// Lockfile filename for a target (matches `apply.rs` / `resolve.rs`).
fn lockfile_name(target: &str) -> String {
    if target == "local" {
        "sindri.lock".to_string()
    } else {
        format!("sindri.{}.lock", target)
    }
}

/// Render the SBOM for the given format. The output is deterministic except
/// for the document UUID and `created` timestamp.
pub fn render(lockfile: &Lockfile, format: BomFormat) -> String {
    match format {
        BomFormat::Spdx => render_spdx(lockfile),
        BomFormat::CycloneDx => render_cyclonedx(lockfile),
    }
}

/// Auto-emit hook: write `sindri.<target>.bom.spdx.json` next to the
/// lockfile after a successful `apply`. Idempotent — overwrites the file.
///
/// `cwd` is the working directory the file should be written in (typically
/// `std::env::current_dir()`); allowing it to be passed makes the tests
/// hermetic.
pub fn auto_emit_after_apply(lockfile: &Lockfile, cwd: &Path) -> std::io::Result<PathBuf> {
    let path = cwd.join(format!(
        "sindri.{}.bom.{}",
        lockfile.target,
        BomFormat::Spdx.default_extension()
    ));
    let body = render_spdx(lockfile);
    std::fs::write(&path, body)?;
    Ok(path)
}

// ---------------------------------------------------------------------------
// SPDX 2.3
// ---------------------------------------------------------------------------

fn render_spdx(lockfile: &Lockfile) -> String {
    let doc_uuid = uuid::Uuid::new_v4();
    let created = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let tool_id = format!("Tool: sindri-{}", env!("CARGO_PKG_VERSION"));

    let packages: Vec<serde_json::Value> = lockfile.components.iter().map(spdx_package).collect();

    let document_describes: Vec<String> = packages
        .iter()
        .filter_map(|p| p.get("SPDXID").and_then(|v| v.as_str()).map(String::from))
        .collect();

    let mut relationships: Vec<serde_json::Value> = document_describes
        .iter()
        .map(|spdxid| {
            serde_json::json!({
                "spdxElementId": "SPDXRef-DOCUMENT",
                "relationshipType": "DESCRIBES",
                "relatedSpdxElement": spdxid,
            })
        })
        .collect();

    // DEPENDS_ON: derived from `ResolvedComponent::depends_on`. Each entry is
    // a `backend:name[@qualifier]` address; we resolve it to the SPDXID of
    // the matching component, if present in the lockfile.
    let address_to_spdxid: std::collections::HashMap<String, String> = lockfile
        .components
        .iter()
        .map(|c| (c.id.to_address(), spdx_id_for(c)))
        .collect();

    for comp in &lockfile.components {
        let from = spdx_id_for(comp);
        for dep_addr in &comp.depends_on {
            if let Some(to) = address_to_spdxid.get(dep_addr) {
                relationships.push(serde_json::json!({
                    "spdxElementId": from,
                    "relationshipType": "DEPENDS_ON",
                    "relatedSpdxElement": to,
                }));
            }
        }
    }

    let doc = serde_json::json!({
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": format!("sindri-bom-{}", lockfile.target),
        "documentNamespace": format!("https://sindri.dev/spdxdocs/{}", doc_uuid),
        "creationInfo": {
            "creators": [tool_id],
            "created": created,
        },
        "packages": packages,
        "relationships": relationships,
    });

    serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".to_string())
}

fn spdx_id_for(comp: &ResolvedComponent) -> String {
    let safe_name = comp.id.name.replace(['/', '@', ':', '.'], "-");
    let safe_qual = comp
        .id
        .qualifier
        .as_deref()
        .map(|q| format!("-{}", q.replace(['/', '@', ':', '.'], "-")))
        .unwrap_or_default();
    format!(
        "SPDXRef-Package-{}-{}{}",
        comp.id.backend.as_str(),
        safe_name,
        safe_qual
    )
}

fn spdx_package(comp: &ResolvedComponent) -> serde_json::Value {
    let license = license_for(comp);
    let purl = purl_for(comp);
    let download = download_location(comp).unwrap_or_else(|| "NOASSERTION".to_string());

    let mut external_refs = vec![serde_json::json!({
        "referenceCategory": "PACKAGE-MANAGER",
        "referenceType": "purl",
        "referenceLocator": purl,
    })];
    if let Some(digest) = &comp.manifest_digest {
        // ADR-003 / ADR-014: registry-level OCI digest. Emit as a
        // PERSISTENT-ID/oci external reference (SPDX 2.3 supports custom
        // referenceTypes under the PERSISTENT-ID category).
        external_refs.push(serde_json::json!({
            "referenceCategory": "PERSISTENT-ID",
            "referenceType": "oci",
            "referenceLocator": digest,
        }));
    }

    let mut checksums: Vec<serde_json::Value> = comp
        .checksums
        .iter()
        .map(|(algo, val)| {
            serde_json::json!({
                "algorithm": algo.to_uppercase(),
                "checksumValue": val,
            })
        })
        .collect();
    // Stable order for deterministic output.
    checksums.sort_by(|a, b| {
        a.get("algorithm")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .cmp(b.get("algorithm").and_then(|v| v.as_str()).unwrap_or(""))
    });

    let mut pkg = serde_json::json!({
        "SPDXID": spdx_id_for(comp),
        "name": comp.id.to_address(),
        "versionInfo": comp.version.0,
        "downloadLocation": download,
        "filesAnalyzed": false,
        "licenseConcluded": "NOASSERTION",
        "licenseDeclared": license,
        "copyrightText": "NOASSERTION",
        "externalRefs": external_refs,
    });
    if !checksums.is_empty() {
        pkg.as_object_mut()
            .expect("json object")
            .insert("checksums".to_string(), serde_json::Value::Array(checksums));
    }
    pkg
}

// ---------------------------------------------------------------------------
// CycloneDX 1.6
// ---------------------------------------------------------------------------

fn render_cyclonedx(lockfile: &Lockfile) -> String {
    let serial = format!("urn:uuid:{}", uuid::Uuid::new_v4());
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let tool_version = env!("CARGO_PKG_VERSION");

    let components: Vec<serde_json::Value> =
        lockfile.components.iter().map(cdx_component).collect();

    let mut dependencies: Vec<serde_json::Value> = Vec::new();
    let address_to_ref: std::collections::HashMap<String, String> = lockfile
        .components
        .iter()
        .map(|c| (c.id.to_address(), cdx_bom_ref(c)))
        .collect();
    for comp in &lockfile.components {
        let bref = cdx_bom_ref(comp);
        let deps: Vec<serde_json::Value> = comp
            .depends_on
            .iter()
            .filter_map(|addr| address_to_ref.get(addr).cloned())
            .map(|r| serde_json::json!({ "ref": r }))
            .collect();
        dependencies.push(serde_json::json!({
            "ref": bref,
            "dependsOn": deps,
        }));
    }

    let doc = serde_json::json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "serialNumber": serial,
        "version": 1,
        "metadata": {
            "timestamp": timestamp,
            "tools": {
                "components": [{
                    "type": "application",
                    "name": "sindri",
                    "version": tool_version,
                }]
            },
            "component": {
                "type": "application",
                "bom-ref": format!("sindri-bom-{}", lockfile.target),
                "name": format!("sindri-bom-{}", lockfile.target),
            },
        },
        "components": components,
        "dependencies": dependencies,
    });

    serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".to_string())
}

fn cdx_bom_ref(comp: &ResolvedComponent) -> String {
    format!("{}@{}", comp.id.to_address(), comp.version.0)
}

fn cdx_component(comp: &ResolvedComponent) -> serde_json::Value {
    let license = license_for(comp);
    let licenses = if license == "NOASSERTION" {
        serde_json::json!([])
    } else {
        serde_json::json!([{ "license": { "id": license } }])
    };

    let mut hashes: Vec<serde_json::Value> = comp
        .checksums
        .iter()
        .map(|(algo, val)| {
            serde_json::json!({
                "alg": cdx_hash_alg(algo),
                "content": val,
            })
        })
        .collect();
    hashes.sort_by(|a, b| {
        a.get("alg")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .cmp(b.get("alg").and_then(|v| v.as_str()).unwrap_or(""))
    });
    if let Some(digest) = &comp.manifest_digest {
        // Custom hash algorithm — see module-level docs.
        hashes.push(serde_json::json!({
            "alg": "OCI-DIGEST",
            "content": digest,
        }));
    }

    let mut comp_json = serde_json::json!({
        "type": "library",
        "bom-ref": cdx_bom_ref(comp),
        "name": comp.id.to_address(),
        "version": comp.version.0,
        "purl": purl_for(comp),
        "licenses": licenses,
    });
    if !hashes.is_empty() {
        comp_json
            .as_object_mut()
            .expect("json object")
            .insert("hashes".to_string(), serde_json::Value::Array(hashes));
    }
    comp_json
}

fn cdx_hash_alg(algo: &str) -> String {
    // CycloneDX 1.6 hash-alg enum uses SHA-256 / SHA-512 / etc.
    match algo.to_ascii_lowercase().as_str() {
        "sha256" | "sha-256" => "SHA-256".to_string(),
        "sha384" | "sha-384" => "SHA-384".to_string(),
        "sha512" | "sha-512" => "SHA-512".to_string(),
        "md5" => "MD5".to_string(),
        "sha1" | "sha-1" => "SHA-1".to_string(),
        other => other.to_uppercase(),
    }
}

// ---------------------------------------------------------------------------
// Shared mappers
// ---------------------------------------------------------------------------

/// SPDX license expression for a component. Falls back to `NOASSERTION`
/// (an SPDX-recognised value meaning "not enough information") when the
/// manifest is absent or has an empty license string.
fn license_for(comp: &ResolvedComponent) -> String {
    comp.manifest
        .as_ref()
        .map(|m: &ComponentManifest| m.metadata.license.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "NOASSERTION".to_string())
}

/// Best-effort download URL or source identifier, when one can be derived
/// from the manifest's install config.
fn download_location(comp: &ResolvedComponent) -> Option<String> {
    let manifest = comp.manifest.as_ref()?;
    if let Some(b) = &manifest.install.binary {
        return Some(b.url_template.clone());
    }
    None
}

/// Build the canonical Package URL for a component.
///
/// See the module-level table for the full backend → PURL `type` mapping.
fn purl_for(comp: &ResolvedComponent) -> String {
    let ty = match comp.backend {
        Backend::Mise => "mise",
        Backend::Apt => "apt",
        Backend::Dnf => "dnf",
        Backend::Zypper => "zypper",
        Backend::Pacman => "pacman",
        Backend::Apk => "apk",
        Backend::Brew => "brew",
        Backend::Winget => "winget",
        Backend::Scoop => "scoop",
        Backend::Npm => "npm",
        Backend::Pipx => "pypi",
        Backend::Cargo => "cargo",
        Backend::GoInstall => "golang",
        Backend::Sdkman => "sdkman",
        Backend::Binary => "generic",
        Backend::Script => "generic",
        Backend::Collection => "generic",
    };

    // PURL forbids scheme/host punctuation in the name/version segments;
    // encode `/` only when present (e.g. golang module paths legitimately
    // contain `/` and PURL keeps them literal). For all other backends
    // names are simple identifiers, so a literal copy is fine.
    let name = &comp.id.name;
    let version = &comp.version.0;

    let qualifier_suffix = match comp.backend {
        Backend::Binary => comp
            .manifest
            .as_ref()
            .and_then(|m| m.install.binary.as_ref())
            .map(|b| format!("?download_url={}", b.url_template))
            .unwrap_or_default(),
        _ => String::new(),
    };

    if version.is_empty() {
        format!("pkg:{}/{}{}", ty, name, qualifier_suffix)
    } else {
        format!("pkg:{}/{}@{}{}", ty, name, version, qualifier_suffix)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::component::{
        Backend, BinaryInstallConfig, ComponentId, ComponentManifest, ComponentMetadata,
        InstallConfig,
    };
    use sindri_core::lockfile::Lockfile;
    use sindri_core::platform::{Arch, Os, Platform};
    use sindri_core::version::Version;
    use std::collections::HashMap;

    fn manifest_with(license: &str, binary_url: Option<&str>) -> ComponentManifest {
        ComponentManifest {
            metadata: ComponentMetadata {
                name: "x".into(),
                version: "1.0.0".into(),
                description: "x".into(),
                license: license.into(),
                tags: vec![],
                homepage: None,
            },
            platforms: vec![Platform {
                os: Os::Linux,
                arch: Arch::X86_64,
            }],
            install: InstallConfig {
                binary: binary_url.map(|u| BinaryInstallConfig {
                    url_template: u.to_string(),
                    checksums: HashMap::new(),
                    install_path: "/usr/local/bin/x".into(),
                }),
                ..InstallConfig::default()
            },
            depends_on: vec![],
            capabilities: Default::default(),
            options: Default::default(),
            validate: None,
            configure: None,
            remove: None,
            overrides: HashMap::new(),
            auth: Default::default(),
        }
    }

    fn comp(
        backend: Backend,
        name: &str,
        version: &str,
        manifest: Option<ComponentManifest>,
    ) -> ResolvedComponent {
        ResolvedComponent {
            id: ComponentId {
                backend: backend.clone(),
                name: name.into(),
                qualifier: None,
            },
            version: Version::new(version),
            backend,
            oci_digest: None,
            checksums: HashMap::new(),
            depends_on: vec![],
            manifest,
            manifest_digest: None,
            component_digest: None,
            platforms: None,
            source: None,
        }
    }

    fn lockfile_with(components: Vec<ResolvedComponent>) -> Lockfile {
        Lockfile {
            version: 1,
            bom_hash: "deadbeef".into(),
            target: "local".into(),
            components,
            auth_bindings: Vec::new(),
        }
    }

    #[test]
    fn spdx_output_has_required_fields() {
        let lf = lockfile_with(vec![comp(Backend::Mise, "nodejs", "22.0.0", None)]);
        let s = render_spdx(&lf);
        let v: serde_json::Value = serde_json::from_str(&s).expect("valid json");
        assert_eq!(v["spdxVersion"], "SPDX-2.3");
        assert_eq!(v["dataLicense"], "CC0-1.0");
        assert_eq!(v["SPDXID"], "SPDXRef-DOCUMENT");
        assert_eq!(v["name"], "sindri-bom-local");
        assert!(v["documentNamespace"]
            .as_str()
            .unwrap()
            .starts_with("https://sindri.dev/spdxdocs/"));
        assert!(v["creationInfo"]["created"].is_string());
        let creators = v["creationInfo"]["creators"].as_array().unwrap();
        assert!(creators
            .iter()
            .any(|c| c.as_str().unwrap_or("").starts_with("Tool: sindri-")));
        assert_eq!(v["packages"].as_array().unwrap().len(), 1);
        let rels = v["relationships"].as_array().unwrap();
        // DESCRIBES from DOCUMENT to the package.
        assert!(rels
            .iter()
            .any(|r| r["spdxElementId"] == "SPDXRef-DOCUMENT"
                && r["relationshipType"] == "DESCRIBES"));
    }

    #[test]
    fn cyclonedx_output_has_required_fields() {
        let lf = lockfile_with(vec![comp(Backend::Npm, "typescript", "5.4.5", None)]);
        let s = render_cyclonedx(&lf);
        let v: serde_json::Value = serde_json::from_str(&s).expect("valid json");
        assert_eq!(v["bomFormat"], "CycloneDX");
        assert_eq!(v["specVersion"], "1.6");
        assert_eq!(v["version"], 1);
        assert!(v["serialNumber"].as_str().unwrap().starts_with("urn:uuid:"));
        assert!(v["metadata"]["timestamp"].is_string());
        let tools = v["metadata"]["tools"]["components"].as_array().unwrap();
        assert_eq!(tools[0]["name"], "sindri");
        let comps = v["components"].as_array().unwrap();
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0]["type"], "library");
        assert_eq!(comps[0]["purl"], "pkg:npm/typescript@5.4.5");
    }

    #[test]
    fn purl_for_mise_backend_is_canonical() {
        let c = comp(Backend::Mise, "nodejs", "22.0.0", None);
        assert_eq!(purl_for(&c), "pkg:mise/nodejs@22.0.0");
    }

    #[test]
    fn purl_for_npm_backend_is_canonical() {
        let c = comp(Backend::Npm, "typescript", "5.4.5", None);
        assert_eq!(purl_for(&c), "pkg:npm/typescript@5.4.5");
    }

    #[test]
    fn purl_for_cargo_backend_is_canonical() {
        let c = comp(Backend::Cargo, "ripgrep", "14.1.0", None);
        assert_eq!(purl_for(&c), "pkg:cargo/ripgrep@14.1.0");
    }

    #[test]
    fn purl_for_binary_backend_includes_download_url() {
        let m = manifest_with("MIT", Some("https://example.com/foo-{version}.tgz"));
        let c = comp(Backend::Binary, "foo", "1.2.3", Some(m));
        let p = purl_for(&c);
        assert!(p.starts_with("pkg:generic/foo@1.2.3"));
        assert!(p.contains("download_url=https://example.com/foo-{version}.tgz"));
    }

    #[test]
    fn license_falls_back_to_noassertion_when_manifest_absent() {
        let c = comp(Backend::Mise, "nodejs", "22.0.0", None);
        assert_eq!(license_for(&c), "NOASSERTION");
    }

    #[test]
    fn license_uses_manifest_value_when_present() {
        let m = manifest_with("Apache-2.0", None);
        let c = comp(Backend::Mise, "x", "1.0.0", Some(m));
        assert_eq!(license_for(&c), "Apache-2.0");
    }

    #[test]
    fn oci_digest_emitted_as_external_reference_in_spdx() {
        let mut c = comp(Backend::Brew, "git", "2.45.0", None);
        c.manifest_digest = Some("sha256:abc123".into());
        let lf = lockfile_with(vec![c]);
        let s = render_spdx(&lf);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        let pkg = &v["packages"][0];
        let refs = pkg["externalRefs"].as_array().unwrap();
        let oci_ref = refs
            .iter()
            .find(|r| r["referenceType"] == "oci")
            .expect("oci external ref");
        assert_eq!(oci_ref["referenceCategory"], "PERSISTENT-ID");
        assert_eq!(oci_ref["referenceLocator"], "sha256:abc123");
    }

    #[test]
    fn oci_digest_emitted_as_hash_in_cyclonedx() {
        let mut c = comp(Backend::Brew, "git", "2.45.0", None);
        c.manifest_digest = Some("sha256:def456".into());
        let lf = lockfile_with(vec![c]);
        let s = render_cyclonedx(&lf);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        let hashes = v["components"][0]["hashes"].as_array().unwrap();
        assert!(hashes
            .iter()
            .any(|h| h["alg"] == "OCI-DIGEST" && h["content"] == "sha256:def456"));
    }

    #[test]
    fn depends_on_emits_dependson_relationships_in_spdx() {
        let mut a = comp(Backend::Mise, "a", "1.0.0", None);
        a.depends_on = vec!["mise:b".into()];
        let b = comp(Backend::Mise, "b", "2.0.0", None);
        let lf = lockfile_with(vec![a, b]);
        let s = render_spdx(&lf);
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        let rels = v["relationships"].as_array().unwrap();
        let dep = rels
            .iter()
            .find(|r| r["relationshipType"] == "DEPENDS_ON")
            .expect("DEPENDS_ON relationship");
        assert_eq!(dep["spdxElementId"], "SPDXRef-Package-mise-a");
        assert_eq!(dep["relatedSpdxElement"], "SPDXRef-Package-mise-b");
    }

    #[test]
    fn checksums_emitted_in_both_formats() {
        let mut c = comp(Backend::Binary, "tool", "1.0.0", None);
        c.checksums
            .insert("sha256".into(), "0123456789abcdef".into());
        let lf = lockfile_with(vec![c]);
        let s_spdx = render_spdx(&lf);
        let v: serde_json::Value = serde_json::from_str(&s_spdx).unwrap();
        let cks = v["packages"][0]["checksums"].as_array().unwrap();
        assert_eq!(cks[0]["algorithm"], "SHA256");
        assert_eq!(cks[0]["checksumValue"], "0123456789abcdef");

        let s_cdx = render_cyclonedx(&lf);
        let v: serde_json::Value = serde_json::from_str(&s_cdx).unwrap();
        let hashes = v["components"][0]["hashes"].as_array().unwrap();
        assert!(hashes
            .iter()
            .any(|h| h["alg"] == "SHA-256" && h["content"] == "0123456789abcdef"));
    }

    #[test]
    fn auto_emit_writes_file_after_apply() {
        let dir = tempfile::tempdir().unwrap();
        let lf = lockfile_with(vec![comp(Backend::Mise, "nodejs", "22.0.0", None)]);
        let path = auto_emit_after_apply(&lf, dir.path()).expect("auto-emit");
        assert_eq!(path, dir.path().join("sindri.local.bom.spdx.json"));
        let body = std::fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["spdxVersion"], "SPDX-2.3");
        assert_eq!(v["packages"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn auto_emit_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let lf = lockfile_with(vec![comp(Backend::Mise, "nodejs", "22.0.0", None)]);
        let p1 = auto_emit_after_apply(&lf, dir.path()).unwrap();
        let p2 = auto_emit_after_apply(&lf, dir.path()).unwrap();
        assert_eq!(p1, p2);
        // Second run must successfully overwrite without erroring.
        assert!(p2.exists());
    }

    #[test]
    fn parse_format_accepts_aliases() {
        assert_eq!(BomFormat::parse("spdx").unwrap(), BomFormat::Spdx);
        assert_eq!(BomFormat::parse("spdx-json").unwrap(), BomFormat::Spdx);
        assert_eq!(BomFormat::parse("cyclonedx").unwrap(), BomFormat::CycloneDx);
        assert_eq!(BomFormat::parse("cdx").unwrap(), BomFormat::CycloneDx);
        assert!(BomFormat::parse("xml").is_err());
    }
}
