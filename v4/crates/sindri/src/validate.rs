use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_core::manifest::BomManifest;
use std::fs;
use std::path::Path;

/// Run `sindri validate <path>`.
///
/// Returns an exit code per ADR-012:
///   0 = valid
///   4 = file not found or schema/YAML error
pub fn run(path: &str, json_output: bool) -> i32 {
    let p = Path::new(path);

    if !p.exists() {
        if json_output {
            eprintln!(
                r#"{{"error":"FILE_NOT_FOUND","file":"{}","fix":"Create sindri.yaml in the current directory or specify a path."}}"#,
                path
            );
        } else {
            eprintln!("Error: {} not found", path);
            eprintln!("Hint: run `sindri init` to create a sindri.yaml");
        }
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    let content = match fs::read_to_string(p) {
        Ok(c) => c,
        Err(e) => {
            if json_output {
                eprintln!(
                    r#"{{"error":"READ_ERROR","file":"{}","detail":"{}"}}"#,
                    path, e
                );
            } else {
                eprintln!("Error reading {}: {}", path, e);
            }
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    match serde_yaml::from_str::<BomManifest>(&content) {
        Ok(_manifest) => {
            if json_output {
                println!(r#"{{"valid":true,"file":"{}"}}"#, path);
            } else {
                println!("{} is valid", path);
            }
            EXIT_SUCCESS
        }
        Err(e) => {
            if json_output {
                eprintln!(
                    r#"{{"error":"SCHEMA_ERROR","file":"{}","detail":"{}","fix":"Check the YAML syntax and required fields."}}"#,
                    path, e
                );
            } else {
                eprintln!("Validation error in {}: {}", path, e);
                eprintln!("Hint: Check https://schemas.sindri.dev/v4/bom.json for the schema");
            }
            EXIT_SCHEMA_OR_RESOLVE_ERROR
        }
    }
}
