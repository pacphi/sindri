/// SBOM generation (ADR-007, Sprint 12)
///
/// Emits SPDX 2.3 JSON or CycloneDX 1.6 XML from a resolved lockfile.
use std::path::PathBuf;
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_STALE_LOCKFILE, EXIT_SUCCESS};
use sindri_core::lockfile::Lockfile;

pub struct BomArgs {
    pub format: String, // "spdx" | "cyclonedx"
    pub target: String,
    pub output: Option<String>,
}

pub fn run(args: BomArgs) -> i32 {
    let lock_name = if args.target == "local" {
        "sindri.lock".to_string()
    } else {
        format!("sindri.{}.lock", args.target)
    };

    let lockfile_path = PathBuf::from(&lock_name);
    if !lockfile_path.exists() {
        eprintln!("Lockfile '{}' not found. Run `sindri resolve` first.", lock_name);
        return EXIT_STALE_LOCKFILE;
    }

    let content = match std::fs::read_to_string(&lockfile_path) {
        Ok(c) => c,
        Err(e) => { eprintln!("Cannot read lockfile: {}", e); return EXIT_STALE_LOCKFILE; }
    };

    let lockfile: Lockfile = match serde_json::from_str(&content) {
        Ok(l) => l,
        Err(e) => { eprintln!("Malformed lockfile: {}", e); return EXIT_STALE_LOCKFILE; }
    };

    let sbom = match args.format.as_str() {
        "cyclonedx" => emit_cyclonedx(&lockfile),
        _ => emit_spdx(&lockfile),
    };

    let output_path = args.output.as_deref().unwrap_or_else(|| {
        if args.format == "cyclonedx" { "sindri.bom.cdx.xml" } else { "sindri.bom.spdx.json" }
    });

    match std::fs::write(output_path, &sbom) {
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

fn emit_spdx(lockfile: &Lockfile) -> String {
    let packages: Vec<serde_json::Value> = lockfile.components.iter().map(|c| {
        serde_json::json!({
            "SPDXID": format!("SPDXRef-{}-{}", c.id.backend.as_str(), c.id.name.replace('/', "-")),
            "name": c.id.name,
            "versionInfo": c.version.0,
            "downloadLocation": c.oci_digest.as_deref().unwrap_or("NOASSERTION"),
            "filesAnalyzed": false,
            "externalRefs": [{
                "referenceCategory": "PACKAGE-MANAGER",
                "referenceType": "purl",
                "referenceLocator": format!("pkg:{}/{}@{}", c.id.backend.as_str(), c.id.name, c.version.0),
            }],
        })
    }).collect();

    serde_json::to_string_pretty(&serde_json::json!({
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": format!("sindri-bom-{}", lockfile.target),
        "documentNamespace": format!("https://sindri.dev/bom/{}", lockfile.bom_hash),
        "documentDescribes": packages.iter()
            .map(|p| p["SPDXID"].as_str().unwrap_or("").to_string())
            .collect::<Vec<_>>(),
        "packages": packages,
    }))
    .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

fn emit_cyclonedx(lockfile: &Lockfile) -> String {
    let components: Vec<String> = lockfile.components.iter().map(|c| {
        format!(
            r#"    <component type="library">
      <name>{}</name>
      <version>{}</version>
      <purl>pkg:{}/{}@{}</purl>
    </component>"#,
            xml_escape(&c.id.name),
            xml_escape(&c.version.0),
            xml_escape(c.id.backend.as_str()),
            xml_escape(&c.id.name),
            xml_escape(&c.version.0),
        )
    }).collect();

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<bom xmlns="http://cyclonedx.org/schema/bom/1.6" version="1">
  <metadata>
    <component type="application">
      <name>sindri-bom-{}</name>
      <version>{}</version>
    </component>
  </metadata>
  <components>
{}
  </components>
</bom>"#,
        lockfile.target,
        lockfile.bom_hash,
        components.join("\n"),
    )
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
}
