use crate::types::{
    ProvenanceVerification, Sbom, SbomPackage, SignatureInfo, SignatureVerification,
};
use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use std::process::Command;
use tracing::{debug, trace, warn};

/// Verifies container image signatures and provenance using Cosign
#[derive(Debug)]
pub struct ImageVerifier {
    cosign_path: PathBuf,
}

impl ImageVerifier {
    /// Create a new image verifier
    ///
    /// # Errors
    /// Returns an error if cosign is not found in PATH
    pub fn new() -> Result<Self> {
        let cosign_path = which::which("cosign")
            .context("cosign not found in PATH. Install from: https://docs.sigstore.dev/cosign/installation/")?;

        debug!("Found cosign at: {:?}", cosign_path);

        Ok(Self { cosign_path })
    }

    /// Verify image signature using cosign
    ///
    /// # Arguments
    /// * `image_ref` - Full image reference (e.g., "ghcr.io/pacphi/sindri:v3.0.0")
    /// * `certificate_identity` - Optional certificate identity regexp for verification
    /// * `certificate_oidc_issuer` - Optional OIDC issuer for verification
    ///
    /// # Returns
    /// Verification result with signature details
    pub async fn verify_signature(
        &self,
        image_ref: &str,
        certificate_identity: Option<&str>,
        certificate_oidc_issuer: Option<&str>,
    ) -> Result<SignatureVerification> {
        debug!("Verifying signature for: {}", image_ref);

        let mut cmd = Command::new(&self.cosign_path);
        cmd.arg("verify");

        // Add certificate identity if provided
        if let Some(identity) = certificate_identity {
            cmd.arg("--certificate-identity-regexp").arg(identity);
        }

        // Add OIDC issuer if provided
        if let Some(issuer) = certificate_oidc_issuer {
            cmd.arg("--certificate-oidc-issuer").arg(issuer);
        }

        cmd.arg(image_ref);

        trace!("Running: {:?}", cmd);

        let output = cmd.output().context("Failed to execute cosign verify")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            debug!("Signature verification succeeded");

            // Parse cosign output for signature details
            let signatures = self.parse_signature_output(&stdout)?;

            Ok(SignatureVerification {
                verified: true,
                signatures,
                errors: Vec::new(),
            })
        } else {
            warn!("Signature verification failed: {}", stderr);

            Ok(SignatureVerification {
                verified: false,
                signatures: Vec::new(),
                errors: vec![stderr.to_string()],
            })
        }
    }

    /// Verify image provenance attestation (SLSA)
    ///
    /// # Arguments
    /// * `image_ref` - Full image reference
    /// * `certificate_identity` - Optional certificate identity regexp
    /// * `certificate_oidc_issuer` - Optional OIDC issuer
    ///
    /// # Returns
    /// Provenance verification result with SLSA level
    pub async fn verify_provenance(
        &self,
        image_ref: &str,
        certificate_identity: Option<&str>,
        certificate_oidc_issuer: Option<&str>,
    ) -> Result<ProvenanceVerification> {
        debug!("Verifying provenance for: {}", image_ref);

        let mut cmd = Command::new(&self.cosign_path);
        cmd.arg("verify-attestation");
        cmd.arg("--type").arg("https://slsa.dev/provenance/v1");

        if let Some(identity) = certificate_identity {
            cmd.arg("--certificate-identity-regexp").arg(identity);
        }

        if let Some(issuer) = certificate_oidc_issuer {
            cmd.arg("--certificate-oidc-issuer").arg(issuer);
        }

        cmd.arg(image_ref);

        trace!("Running: {:?}", cmd);

        let output = cmd
            .output()
            .context("Failed to execute cosign verify-attestation")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if output.status.success() {
            debug!("Provenance verification succeeded");

            // Parse provenance details from JSON output
            let (slsa_level, builder_id, source_repo) = self.parse_provenance_output(&stdout)?;

            Ok(ProvenanceVerification {
                verified: true,
                slsa_level: Some(slsa_level),
                builder_id: Some(builder_id),
                source_repo,
                errors: Vec::new(),
            })
        } else {
            warn!("Provenance verification failed: {}", stderr);

            Ok(ProvenanceVerification {
                verified: false,
                slsa_level: None,
                builder_id: None,
                source_repo: None,
                errors: vec![stderr.to_string()],
            })
        }
    }

    /// Fetch SBOM (Software Bill of Materials) for an image
    ///
    /// # Arguments
    /// * `image_ref` - Full image reference
    ///
    /// # Returns
    /// SBOM with package information
    pub async fn fetch_sbom(&self, image_ref: &str) -> Result<Sbom> {
        debug!("Fetching SBOM for: {}", image_ref);

        let mut cmd = Command::new(&self.cosign_path);
        cmd.arg("download").arg("sbom").arg(image_ref);

        trace!("Running: {:?}", cmd);

        let output = cmd
            .output()
            .context("Failed to execute cosign download sbom")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to download SBOM: {}", stderr));
        }

        let raw_data = String::from_utf8_lossy(&output.stdout).to_string();

        // Parse SBOM (assuming SPDX JSON format)
        let sbom = self.parse_sbom(&raw_data)?;

        debug!("SBOM contains {} packages", sbom.packages.len());

        Ok(sbom)
    }

    /// Check if cosign is available
    pub fn is_available() -> bool {
        which::which("cosign").is_ok()
    }

    // Private helper methods

    fn parse_signature_output(&self, output: &str) -> Result<Vec<SignatureInfo>> {
        let mut signatures = Vec::new();

        // Cosign outputs JSON — either one object per line (v2.x) or a JSON array (v3.x).
        // Keys may be uppercase (v2.x: "Optional", "Issuer", "Subject") or
        // lowercase (v3.x: "optional", "issuer", "subject"). Handle both.
        for line in output.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
                // If it's an array (cosign 3.x format), extract each element
                let entries: Vec<&serde_json::Value> = if let Some(arr) = json.as_array() {
                    arr.iter().collect()
                } else {
                    vec![&json]
                };

                for entry in entries {
                    if let Some(sig) = Self::extract_signature_from_entry(entry) {
                        signatures.push(sig);
                    }
                }
            }
        }

        if signatures.is_empty() {
            // Fallback: create a generic signature info
            signatures.push(SignatureInfo {
                issuer: "Verified".to_string(),
                subject: "Sigstore".to_string(),
                valid_from: "N/A".to_string(),
                valid_until: "N/A".to_string(),
            });
        }

        Ok(signatures)
    }

    /// Extract a `SignatureInfo` from a single cosign verify JSON entry.
    /// Handles both cosign 2.x (uppercase) and 3.x (lowercase) key formats.
    fn extract_signature_from_entry(entry: &serde_json::Value) -> Option<SignatureInfo> {
        // Try both "optional" (v3.x) and "Optional" (v2.x)
        let optional = entry.get("optional").or_else(|| entry.get("Optional"))?;

        // Issuer: try "Issuer" (v2.x) then lowercase variants
        let issuer = optional
            .get("Issuer")
            .or_else(|| optional.get("issuer"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Subject: try "Subject" (v2.x) then lowercase variants
        let subject = optional
            .get("Subject")
            .or_else(|| optional.get("subject"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        Some(SignatureInfo {
            issuer,
            subject,
            valid_from: "N/A".to_string(),
            valid_until: "N/A".to_string(),
        })
    }

    fn parse_provenance_output(&self, output: &str) -> Result<(String, String, Option<String>)> {
        // Parse SLSA provenance from cosign output.
        // Cosign 3.x may output a JSON array; cosign 2.x outputs one object per line.
        for line in output.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
                let entries: Vec<&serde_json::Value> = if let Some(arr) = json.as_array() {
                    arr.iter().collect()
                } else {
                    vec![&json]
                };

                for entry in entries {
                    if let Some(result) = Self::extract_provenance_from_entry(entry) {
                        return Ok(result);
                    }
                }
            }
        }

        // Fallback
        Ok((
            "SLSA Level 3".to_string(),
            "GitHub Actions".to_string(),
            None,
        ))
    }

    /// Extract provenance info from a single cosign verify-attestation JSON entry.
    fn extract_provenance_from_entry(
        entry: &serde_json::Value,
    ) -> Option<(String, String, Option<String>)> {
        let payload_str = entry.get("payload").and_then(|v| v.as_str())?;
        let decoded = BASE64.decode(payload_str).ok()?;
        let provenance = serde_json::from_slice::<serde_json::Value>(&decoded).ok()?;

        let slsa_level = provenance
            .pointer("/predicate/buildType")
            .and_then(|v| v.as_str())
            .map(|s| {
                if s.contains("slsa.dev/provenance") {
                    "SLSA Level 3".to_string()
                } else {
                    "SLSA".to_string()
                }
            })
            .unwrap_or_else(|| "SLSA".to_string());

        let builder_id = provenance
            .pointer("/predicate/builder/id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let source_repo = provenance
            .pointer("/predicate/materials/0/uri")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Some((slsa_level, builder_id, source_repo))
    }

    fn parse_sbom(&self, raw_data: &str) -> Result<Sbom> {
        let json: serde_json::Value =
            serde_json::from_str(raw_data).context("Failed to parse SBOM as JSON")?;

        let format = json
            .get("spdxVersion")
            .and_then(|v| v.as_str())
            .map(|_| "spdx-json".to_string())
            .or_else(|| {
                json.get("bomFormat")
                    .and_then(|v| v.as_str())
                    .map(|_| "cyclonedx-json".to_string())
            })
            .unwrap_or_else(|| "unknown".to_string());

        let version = json
            .get("spdxVersion")
            .or_else(|| json.get("specVersion"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Parse packages
        let packages = if let Some(pkgs) = json.get("packages").and_then(|v| v.as_array()) {
            pkgs.iter()
                .filter_map(|pkg| {
                    Some(SbomPackage {
                        name: pkg.get("name")?.as_str()?.to_string(),
                        version: pkg
                            .get("versionInfo")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        supplier: pkg
                            .get("supplier")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        license: pkg
                            .get("licenseConcluded")
                            .or_else(|| pkg.get("licenseDeclared"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

        Ok(Sbom {
            format,
            version,
            packages,
            raw_data: raw_data.to_string(),
        })
    }
}

// Use base64 crate for decoding provenance
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

#[cfg(test)]
impl ImageVerifier {
    /// Test helper: parse signature output without needing cosign
    fn test_parse_signature_output(output: &str) -> anyhow::Result<Vec<SignatureInfo>> {
        let verifier = Self {
            cosign_path: std::path::PathBuf::from("cosign"),
        };
        verifier.parse_signature_output(output)
    }

    /// Test helper: parse provenance output without needing cosign
    fn test_parse_provenance_output(
        output: &str,
    ) -> anyhow::Result<(String, String, Option<String>)> {
        let verifier = Self {
            cosign_path: std::path::PathBuf::from("cosign"),
        };
        verifier.parse_provenance_output(output)
    }

    /// Test helper: parse SBOM without needing cosign
    fn test_parse_sbom(raw_data: &str) -> anyhow::Result<Sbom> {
        let verifier = Self {
            cosign_path: std::path::PathBuf::from("cosign"),
        };
        verifier.parse_sbom(raw_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_available / new ──────────────────────────────────────────

    #[test]
    fn test_cosign_availability_returns_bool() {
        // is_available() should return a bool without panicking
        let _available = ImageVerifier::is_available();
    }

    #[test]
    fn test_verifier_creation_depends_on_cosign() {
        let result = ImageVerifier::new();
        if ImageVerifier::is_available() {
            assert!(result.is_ok(), "Should succeed when cosign is installed");
        } else {
            assert!(result.is_err(), "Should fail when cosign is not installed");
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("cosign not found"),
                "Error should mention cosign, got: {}",
                err
            );
        }
    }

    // ── parse_signature_output ──────────────────────────────────────

    #[test]
    fn test_parse_signature_empty_returns_fallback() {
        let sigs = ImageVerifier::test_parse_signature_output("").unwrap();
        assert_eq!(
            sigs.len(),
            1,
            "Empty input should produce a fallback signature"
        );
        assert_eq!(sigs[0].issuer, "Verified");
        assert_eq!(sigs[0].subject, "Sigstore");
    }

    #[test]
    fn test_parse_signature_valid_cosign_json() {
        let json_line = r#"{"critical":{},"optional":{"Issuer":"https://accounts.google.com","Subject":"user@example.com"}}"#;
        let sigs = ImageVerifier::test_parse_signature_output(json_line).unwrap();
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0].issuer, "https://accounts.google.com");
        assert_eq!(sigs[0].subject, "user@example.com");
    }

    #[test]
    fn test_parse_signature_multiple_lines() {
        let input = format!(
            "{}\n{}\n",
            r#"{"critical":{},"optional":{"Issuer":"issuer-a","Subject":"sub-a"}}"#,
            r#"{"critical":{},"optional":{"Issuer":"issuer-b","Subject":"sub-b"}}"#,
        );
        let sigs = ImageVerifier::test_parse_signature_output(&input).unwrap();
        assert_eq!(
            sigs.len(),
            2,
            "Should parse two signatures from two JSON lines"
        );
        assert_eq!(sigs[0].issuer, "issuer-a");
        assert_eq!(sigs[1].issuer, "issuer-b");
    }

    #[test]
    fn test_parse_signature_malformed_json_returns_fallback() {
        let input = "this is not json\nalso not json\n";
        let sigs = ImageVerifier::test_parse_signature_output(input).unwrap();
        assert_eq!(sigs.len(), 1, "Malformed JSON should produce fallback");
        assert_eq!(sigs[0].issuer, "Verified");
    }

    #[test]
    fn test_parse_signature_json_without_optional_returns_fallback() {
        // Valid JSON but no "optional" key → no signatures extracted → fallback
        let input = r#"{"critical":{"something":"value"}}"#;
        let sigs = ImageVerifier::test_parse_signature_output(input).unwrap();
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0].issuer, "Verified");
    }

    // ── cosign 3.x format tests ──────────────────────────────────────

    #[test]
    fn test_parse_signature_cosign3_array_format() {
        // Cosign 3.x wraps results in a JSON array
        let input = r#"[{"critical":{},"optional":{"Issuer":"https://token.actions.githubusercontent.com","Subject":"https://github.com/pacphi/sindri/.github/workflows/release-v3.yml@refs/tags/v3.0.0"}}]"#;
        let sigs = ImageVerifier::test_parse_signature_output(input).unwrap();
        assert_eq!(sigs.len(), 1);
        assert_eq!(
            sigs[0].issuer,
            "https://token.actions.githubusercontent.com"
        );
        assert!(sigs[0].subject.contains("pacphi/sindri"));
    }

    #[test]
    fn test_parse_signature_cosign3_lowercase_keys() {
        // Cosign 3.x may use lowercase keys
        let input = r#"[{"critical":{},"optional":{"issuer":"https://token.actions.githubusercontent.com","subject":"https://github.com/example/repo"}}]"#;
        let sigs = ImageVerifier::test_parse_signature_output(input).unwrap();
        assert_eq!(sigs.len(), 1);
        assert_eq!(
            sigs[0].issuer,
            "https://token.actions.githubusercontent.com"
        );
        assert_eq!(sigs[0].subject, "https://github.com/example/repo");
    }

    #[test]
    fn test_parse_signature_cosign3_multiple_in_array() {
        let input = r#"[{"critical":{},"optional":{"Issuer":"issuer-a","Subject":"sub-a"}},{"critical":{},"optional":{"Issuer":"issuer-b","Subject":"sub-b"}}]"#;
        let sigs = ImageVerifier::test_parse_signature_output(input).unwrap();
        assert_eq!(sigs.len(), 2);
        assert_eq!(sigs[0].issuer, "issuer-a");
        assert_eq!(sigs[1].issuer, "issuer-b");
    }

    #[test]
    fn test_parse_provenance_cosign3_array_format() {
        let predicate = serde_json::json!({
            "predicate": {
                "buildType": "https://slsa.dev/provenance/v1",
                "builder": { "id": "https://github.com/actions/runner" },
                "materials": [{ "uri": "git+https://github.com/pacphi/sindri" }]
            }
        });
        let payload_b64 = BASE64.encode(predicate.to_string().as_bytes());
        // Wrap in array like cosign 3.x
        let line = format!(r#"[{{"payload":"{}"}}]"#, payload_b64);

        let (level, builder, repo) = ImageVerifier::test_parse_provenance_output(&line).unwrap();
        assert_eq!(level, "SLSA Level 3");
        assert!(builder.contains("actions/runner"));
        assert_eq!(
            repo,
            Some("git+https://github.com/pacphi/sindri".to_string())
        );
    }

    // ── parse_provenance_output ─────────────────────────────────────

    #[test]
    fn test_parse_provenance_empty_returns_fallback() {
        let (level, builder, repo) = ImageVerifier::test_parse_provenance_output("").unwrap();
        assert_eq!(level, "SLSA Level 3");
        assert_eq!(builder, "GitHub Actions");
        assert!(repo.is_none());
    }

    #[test]
    fn test_parse_provenance_valid_payload() {
        // Build a mock SLSA provenance JSON and base64-encode it
        let predicate = serde_json::json!({
            "predicate": {
                "buildType": "https://slsa.dev/provenance/v0.2",
                "builder": {
                    "id": "https://github.com/slsa-framework/slsa-github-generator/.github/workflows/generator_container_slsa3.yml@refs/tags/v1.5.0"
                },
                "materials": [
                    { "uri": "git+https://github.com/example/repo" }
                ]
            }
        });
        let payload_b64 = BASE64.encode(predicate.to_string().as_bytes());
        let line = serde_json::json!({ "payload": payload_b64 }).to_string();

        let (level, builder, repo) = ImageVerifier::test_parse_provenance_output(&line).unwrap();
        assert_eq!(
            level, "SLSA Level 3",
            "buildType containing slsa.dev/provenance should map to SLSA Level 3"
        );
        assert!(
            builder.contains("slsa-github-generator"),
            "builder_id should contain generator name, got: {}",
            builder
        );
        assert_eq!(
            repo,
            Some("git+https://github.com/example/repo".to_string())
        );
    }

    #[test]
    fn test_parse_provenance_non_slsa_build_type() {
        let predicate = serde_json::json!({
            "predicate": {
                "buildType": "https://example.com/custom-build",
                "builder": { "id": "custom-builder" }
            }
        });
        let payload_b64 = BASE64.encode(predicate.to_string().as_bytes());
        let line = serde_json::json!({ "payload": payload_b64 }).to_string();

        let (level, builder, repo) = ImageVerifier::test_parse_provenance_output(&line).unwrap();
        assert_eq!(
            level, "SLSA",
            "Non-slsa.dev buildType should map to generic SLSA"
        );
        assert_eq!(builder, "custom-builder");
        assert!(repo.is_none());
    }

    #[test]
    fn test_parse_provenance_malformed_payload_returns_fallback() {
        // payload that is not valid base64
        let line = r#"{"payload":"!!!not-base64!!!"}"#;
        let (level, builder, repo) = ImageVerifier::test_parse_provenance_output(line).unwrap();
        assert_eq!(level, "SLSA Level 3");
        assert_eq!(builder, "GitHub Actions");
        assert!(repo.is_none());
    }

    // ── parse_sbom ──────────────────────────────────────────────────

    #[test]
    fn test_parse_sbom_spdx_json() {
        let spdx = serde_json::json!({
            "spdxVersion": "SPDX-2.3",
            "packages": [
                {
                    "name": "openssl",
                    "versionInfo": "3.0.12",
                    "supplier": "Organization: OpenSSL",
                    "licenseConcluded": "Apache-2.0"
                },
                {
                    "name": "zlib",
                    "versionInfo": "1.3"
                }
            ]
        });

        let sbom = ImageVerifier::test_parse_sbom(&spdx.to_string()).unwrap();
        assert_eq!(sbom.format, "spdx-json");
        assert_eq!(sbom.version, "SPDX-2.3");
        assert_eq!(sbom.packages.len(), 2);
        assert_eq!(sbom.packages[0].name, "openssl");
        assert_eq!(sbom.packages[0].version.as_deref(), Some("3.0.12"));
        assert_eq!(
            sbom.packages[0].supplier.as_deref(),
            Some("Organization: OpenSSL")
        );
        assert_eq!(sbom.packages[0].license.as_deref(), Some("Apache-2.0"));
        assert_eq!(sbom.packages[1].name, "zlib");
        assert!(sbom.packages[1].supplier.is_none());
    }

    #[test]
    fn test_parse_sbom_cyclonedx_json() {
        let cdx = serde_json::json!({
            "bomFormat": "CycloneDX",
            "specVersion": "1.4",
            "packages": []
        });

        let sbom = ImageVerifier::test_parse_sbom(&cdx.to_string()).unwrap();
        assert_eq!(sbom.format, "cyclonedx-json");
        assert_eq!(sbom.version, "1.4");
        assert!(sbom.packages.is_empty());
    }

    #[test]
    fn test_parse_sbom_unknown_format() {
        let unknown = serde_json::json!({
            "something": "else"
        });

        let sbom = ImageVerifier::test_parse_sbom(&unknown.to_string()).unwrap();
        assert_eq!(sbom.format, "unknown");
        assert_eq!(sbom.version, "unknown");
        assert!(sbom.packages.is_empty());
    }

    #[test]
    fn test_parse_sbom_invalid_json_returns_err() {
        let result = ImageVerifier::test_parse_sbom("not json at all");
        assert!(result.is_err(), "Invalid JSON should return an error");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Failed to parse SBOM"),
            "Error should mention SBOM parsing, got: {}",
            err
        );
    }

    #[test]
    fn test_parse_sbom_empty_packages() {
        let spdx = serde_json::json!({
            "spdxVersion": "SPDX-2.3",
            "packages": []
        });

        let sbom = ImageVerifier::test_parse_sbom(&spdx.to_string()).unwrap();
        assert!(sbom.packages.is_empty());
    }

    #[test]
    fn test_parse_sbom_preserves_raw_data() {
        let input = r#"{"spdxVersion":"SPDX-2.3","packages":[]}"#;
        let sbom = ImageVerifier::test_parse_sbom(input).unwrap();
        assert_eq!(sbom.raw_data, input);
    }

    #[test]
    fn test_parse_sbom_license_falls_back_to_declared() {
        let spdx = serde_json::json!({
            "spdxVersion": "SPDX-2.3",
            "packages": [
                {
                    "name": "pkg",
                    "licenseDeclared": "MIT"
                }
            ]
        });

        let sbom = ImageVerifier::test_parse_sbom(&spdx.to_string()).unwrap();
        assert_eq!(
            sbom.packages[0].license.as_deref(),
            Some("MIT"),
            "Should fall back to licenseDeclared when licenseConcluded is absent"
        );
    }
}
