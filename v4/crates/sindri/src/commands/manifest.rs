//! Shared helpers for reading/modifying sindri.yaml

use std::fs;
use sindri_core::manifest::BomManifest;

pub fn load_manifest(path: &str) -> Result<(BomManifest, String), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {}: {}", path, e))?;
    let manifest = serde_yaml::from_str::<BomManifest>(&content)
        .map_err(|e| format!("Parse error in {}: {}", path, e))?;
    Ok((manifest, content))
}

pub fn save_manifest(path: &str, manifest: &BomManifest) -> Result<(), String> {
    let yaml = serde_yaml::to_string(manifest)
        .map_err(|e| format!("Serialization error: {}", e))?;

    // Preserve the YAML-LSP pragma if the original had one
    let existing = fs::read_to_string(path).unwrap_or_default();
    let pragma = extract_pragma(&existing);

    let output = if pragma.is_empty() {
        yaml
    } else {
        format!("{}\n{}", pragma, yaml)
    };

    let tmp = format!("{}.tmp", path);
    fs::write(&tmp, &output).map_err(|e| format!("Write error: {}", e))?;
    fs::rename(&tmp, path).map_err(|e| format!("Rename error: {}", e))?;
    Ok(())
}

fn extract_pragma(content: &str) -> String {
    let lines: Vec<&str> = content
        .lines()
        .take_while(|l| l.starts_with('#'))
        .collect();
    if lines.is_empty() {
        String::new()
    } else {
        lines.join("\n")
    }
}

pub fn find_entry_index(manifest: &BomManifest, address: &str) -> Option<usize> {
    let (backend, name) = split_address(address)?;
    manifest.components.iter().position(|c| {
        let (cb, cn) = split_address(&c.address).unwrap_or_default();
        cb == backend && cn == name
    })
}

fn split_address(addr: &str) -> Option<(String, String)> {
    let (backend, rest) = addr.split_once(':')?;
    let name = rest.split('@').next()?.to_string();
    Some((backend.to_string(), name))
}

pub fn address_without_version(addr: &str) -> String {
    addr.split('@').next().unwrap_or(addr).to_string()
}

pub fn address_version(addr: &str) -> Option<String> {
    addr.split('@').nth(1).map(|s| s.to_string())
}
