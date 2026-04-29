//! schema-gen — JSON Schema emitter for sindri-core v4 types.
//!
//! # Usage
//!
//! ```text
//! cargo run -p schema-gen               # (re)generate schemas in v4/schemas/
//! cargo run -p schema-gen -- --check    # exit non-zero if schemas are stale
//! ```
//!
//! Each schema is emitted with its canonical `$id` URL per ADR-013:
//! `https://schemas.sindri.dev/v4/{name}.json`.

use anyhow::{bail, Context, Result};
use clap::Parser;
use schemars::schema_for;
use serde_json::Value;
use sindri_core::{
    component::ComponentManifest, manifest::BomManifest, policy::InstallPolicy,
    registry::RegistryIndex,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// ADR-013 base URL for all v4 JSON schemas.
const SCHEMA_BASE_URL: &str = "https://schemas.sindri.dev/v4";

/// `$schema` dialect used in every emitted document.
const JSON_SCHEMA_DRAFT: &str = "http://json-schema.org/draft-07/schema#";

#[derive(Debug, Parser)]
#[command(
    name = "schema-gen",
    about = "Generate JSON Schemas for sindri-core v4 types",
    long_about = "Writes pretty-printed JSON Schema files to v4/schemas/.\n\
                  With --check, exits non-zero if any on-disk schema differs \
                  from what would be generated."
)]
struct Cli {
    /// Instead of writing, compare generated schemas to on-disk copies and
    /// exit non-zero if any differ (suitable for CI).
    #[arg(long)]
    check: bool,

    /// Override the output directory (default: v4/schemas/ relative to this binary's manifest).
    #[arg(long, value_name = "DIR")]
    out_dir: Option<PathBuf>,
}

/// A single schema descriptor.
struct SchemaSpec {
    /// File name without directory, e.g. `"bom.json"`.
    filename: &'static str,
    /// The generated schema value (already has `$id` embedded at root).
    schema: Value,
}

fn build_schemas() -> Vec<SchemaSpec> {
    vec![
        build_spec::<BomManifest>("bom.json"),
        build_spec::<ComponentManifest>("component.json"),
        build_spec::<InstallPolicy>("policy.json"),
        build_spec::<RegistryIndex>("registry-index.json"),
    ]
}

fn build_spec<T: schemars::JsonSchema>(filename: &'static str) -> SchemaSpec {
    let stem = filename.strip_suffix(".json").unwrap_or(filename);
    let id = format!("{}/{}", SCHEMA_BASE_URL, filename);

    let mut schema = serde_json::to_value(schema_for!(T))
        .expect("schema_for! always produces serialisable output");

    // Inject / overwrite `$id` and `$schema` at the root.
    if let Value::Object(ref mut map) = schema {
        map.insert("$id".to_string(), Value::String(id));
        map.insert(
            "$schema".to_string(),
            Value::String(JSON_SCHEMA_DRAFT.to_string()),
        );
        // schemars 1.x emits `$schema` inside `"$defs"` — we want it only at
        // the root.  Remove any nested `$schema` keys inside `"$defs"`.
        if let Some(Value::Object(defs)) = map.get_mut("$defs") {
            for def in defs.values_mut() {
                if let Value::Object(def_map) = def {
                    def_map.remove("$schema");
                }
            }
        }
        // schemars 1.x may emit `"title"` as the type name; keep it but also
        // ensure a minimal description is present when it's absent.
        map.entry("description")
            .or_insert_with(|| Value::String(format!("sindri v4 {} schema", stem)));
    }

    SchemaSpec { filename, schema }
}

fn schema_dir_from_manifest() -> PathBuf {
    // Resolve at compile time: CARGO_MANIFEST_DIR is tools/schema-gen.
    // Walk up two levels to reach v4/, then append schemas/.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // tools/
        .expect("tools/ parent missing")
        .parent() // v4/
        .expect("v4/ parent missing")
        .join("schemas")
}

fn pretty_json(value: &Value) -> String {
    // Produce deterministic output with a trailing newline.
    let mut s = serde_json::to_string_pretty(value).expect("value is always serialisable");
    s.push('\n');
    s
}

fn run(cli: &Cli) -> Result<()> {
    let out_dir = cli.out_dir.clone().unwrap_or_else(schema_dir_from_manifest);

    let specs = build_schemas();

    if cli.check {
        run_check(&out_dir, &specs)
    } else {
        run_generate(&out_dir, &specs)
    }
}

fn run_generate(out_dir: &Path, specs: &[SchemaSpec]) -> Result<()> {
    fs::create_dir_all(out_dir)
        .with_context(|| format!("create schemas dir: {}", out_dir.display()))?;

    for spec in specs {
        let path = out_dir.join(spec.filename);
        let content = pretty_json(&spec.schema);
        fs::write(&path, &content).with_context(|| format!("write {}", path.display()))?;
        println!("wrote  {}", path.display());
    }

    println!(
        "\nGenerated {} schemas with $id base {}",
        specs.len(),
        SCHEMA_BASE_URL
    );
    Ok(())
}

fn run_check(out_dir: &Path, specs: &[SchemaSpec]) -> Result<()> {
    let mut drift = Vec::new();

    for spec in specs {
        let path = out_dir.join(spec.filename);
        let generated = pretty_json(&spec.schema);

        match fs::read_to_string(&path) {
            Ok(on_disk) => {
                if on_disk != generated {
                    drift.push(format!(
                        "  {} — on-disk content differs from generated output",
                        path.display()
                    ));
                } else {
                    println!("ok     {}", path.display());
                }
            }
            Err(e) => {
                drift.push(format!("  {} — {}", path.display(), e));
            }
        }
    }

    if drift.is_empty() {
        println!("\nAll {} schemas are up to date.", specs.len());
        Ok(())
    } else {
        bail!(
            "Schema drift detected — run `cargo run -p schema-gen` to regenerate:\n{}",
            drift.join("\n")
        )
    }
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(&cli) {
        eprintln!("error: {:#}", e);
        std::process::exit(1);
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Verify that every spec carries the canonical ADR-013 `$id` URL.
    #[test]
    fn all_specs_have_canonical_id() {
        let specs = build_schemas();
        assert_eq!(specs.len(), 4, "expected 4 schemas");

        let expected_ids = [
            "https://schemas.sindri.dev/v4/bom.json",
            "https://schemas.sindri.dev/v4/component.json",
            "https://schemas.sindri.dev/v4/policy.json",
            "https://schemas.sindri.dev/v4/registry-index.json",
        ];

        for (spec, expected) in specs.iter().zip(expected_ids.iter()) {
            let id = spec
                .schema
                .get("$id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            assert_eq!(id, *expected, "wrong $id for {}", spec.filename);
        }
    }

    /// Verify schema filenames match expectations.
    #[test]
    fn spec_filenames_are_correct() {
        let specs = build_schemas();
        let names: Vec<&str> = specs.iter().map(|s| s.filename).collect();
        assert_eq!(
            names,
            &[
                "bom.json",
                "component.json",
                "policy.json",
                "registry-index.json"
            ]
        );
    }

    /// Round-trip: a minimal BOM document validates against the generated BOM schema.
    #[test]
    fn bom_schema_validates_minimal_document() {
        let specs = build_schemas();
        let bom_spec = specs.iter().find(|s| s.filename == "bom.json").unwrap();

        // Minimal valid BOM — matches BomManifest's required fields.
        let instance = json!({
            "components": []
        });

        let compiled = jsonschema::validator_for(&bom_spec.schema).expect("schema compiles");
        let result = compiled.validate(&instance);
        assert!(
            result.is_ok(),
            "minimal BOM failed schema validation: {:?}",
            result.err()
        );
    }

    /// Verify that --check mode returns an error when the schema dir is empty.
    #[test]
    fn check_mode_detects_missing_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let specs = build_schemas();
        let result = run_check(tmp.path(), &specs);
        assert!(result.is_err(), "expected drift error for missing files");
        let msg = format!("{:#}", result.unwrap_err());
        assert!(
            msg.contains("Schema drift detected"),
            "unexpected message: {msg}"
        );
    }

    /// Verify that --check mode passes when schemas are written then re-checked.
    #[test]
    fn check_mode_passes_after_generate() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let specs = build_schemas();

        // First pass: generate.
        run_generate(tmp.path(), &specs).expect("generate failed");

        // Second pass: check — should be clean.
        let result = run_check(tmp.path(), &specs);
        assert!(
            result.is_ok(),
            "check failed after generate: {:#?}",
            result.err()
        );
    }
}
