//! `sindri prefer <os> <backend-order>` — write a per-OS backend preference
//! into `sindri.yaml` (ADR-011).
//!
//! Example:
//! ```sh
//! sindri prefer macos brew,mise,binary,script
//! ```
//!
//! Each token in `<backend-order>` must parse to a valid [`Backend`] from
//! `sindri-core`; an invalid token aborts the command without modifying the
//! file.

use crate::commands::manifest::{load_manifest, save_manifest};
use sindri_core::component::Backend;
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use sindri_core::manifest::Preferences;
use std::collections::HashMap;
use std::str::FromStr;

/// Arguments for `sindri prefer`.
pub struct PreferArgs {
    /// One of `linux`, `macos`, `windows`.
    pub os: String,
    /// Comma-separated backend names, e.g. `brew,mise,binary`.
    pub order: String,
    /// Manifest path. Defaults to `sindri.yaml`.
    pub manifest: String,
}

/// Entry point for `sindri prefer`.
pub fn run(args: PreferArgs) -> i32 {
    let os = match validate_os(&args.os) {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("{}", msg);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let backends = match parse_backend_order(&args.order) {
        Ok(b) => b,
        Err(msg) => {
            eprintln!("{}", msg);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let (mut manifest, _orig) = match load_manifest(&args.manifest) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let prefs = manifest.preferences.get_or_insert(Preferences {
        backend_order: None,
        default_target: None,
    });
    let order_map = prefs.backend_order.get_or_insert_with(HashMap::new);
    let stringified: Vec<String> = backends.iter().map(|b| b.as_str().to_string()).collect();
    order_map.insert(os.to_string(), stringified.clone());

    if let Err(e) = save_manifest(&args.manifest, &manifest) {
        eprintln!("Failed to write {}: {}", args.manifest, e);
        return EXIT_SCHEMA_OR_RESOLVE_ERROR;
    }

    println!(
        "Set preferences.backend_order.{} = [{}] in {}",
        os,
        stringified.join(", "),
        args.manifest
    );
    EXIT_SUCCESS
}

fn validate_os(os: &str) -> Result<&'static str, String> {
    match os {
        "linux" => Ok("linux"),
        "macos" => Ok("macos"),
        "windows" => Ok("windows"),
        other => Err(format!(
            "Unknown OS '{}'. Valid: linux | macos | windows.",
            other
        )),
    }
}

fn parse_backend_order(s: &str) -> Result<Vec<Backend>, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("Backend order cannot be empty.".into());
    }
    let mut out = Vec::new();
    for token in trimmed.split(',') {
        let t = token.trim();
        if t.is_empty() {
            return Err("Empty backend token in order list.".into());
        }
        let backend = Backend::from_str(t).map_err(|_| {
            format!(
                "Unknown backend '{}'. Run `sindri ls --backend` for valid backends.",
                t
            )
        })?;
        out.push(backend);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    fn write_basic_manifest(path: &Path) {
        std::fs::write(path, "name: demo\ncomponents: []\n").unwrap();
    }

    #[test]
    fn parse_valid_order() {
        let v = parse_backend_order("brew,mise,binary,script").unwrap();
        assert_eq!(v.len(), 4);
    }

    #[test]
    fn parse_rejects_unknown() {
        assert!(parse_backend_order("brew,foo").is_err());
    }

    #[test]
    fn writes_preference_for_macos() {
        let tmp = TempDir::new().unwrap();
        let manifest_path = tmp.path().join("sindri.yaml");
        write_basic_manifest(&manifest_path);

        let code = run(PreferArgs {
            os: "macos".into(),
            order: "brew,mise,binary".into(),
            manifest: manifest_path.to_string_lossy().into_owned(),
        });

        assert_eq!(code, EXIT_SUCCESS);
        let yaml = std::fs::read_to_string(&manifest_path).unwrap();
        assert!(yaml.contains("backend_order"), "yaml: {}", yaml);
        assert!(yaml.contains("macos"));
        assert!(yaml.contains("brew"));
        assert!(yaml.contains("mise"));
    }

    #[test]
    fn rejects_invalid_backend() {
        let tmp = TempDir::new().unwrap();
        let manifest_path = tmp.path().join("sindri.yaml");
        write_basic_manifest(&manifest_path);

        let code = run(PreferArgs {
            os: "macos".into(),
            order: "foo,bar".into(),
            manifest: manifest_path.to_string_lossy().into_owned(),
        });

        assert_eq!(code, EXIT_SCHEMA_OR_RESOLVE_ERROR);
    }

    #[test]
    fn rejects_invalid_os() {
        let tmp = TempDir::new().unwrap();
        let manifest_path = tmp.path().join("sindri.yaml");
        write_basic_manifest(&manifest_path);

        let code = run(PreferArgs {
            os: "freebsd".into(),
            order: "brew".into(),
            manifest: manifest_path.to_string_lossy().into_owned(),
        });

        assert_eq!(code, EXIT_SCHEMA_OR_RESOLVE_ERROR);
    }
}
