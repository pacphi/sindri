//! `.sindri/insecure-plugins.yaml` — auditable record of plugins
//! that the operator opted to trust without a cosign signature.
//!
//! Phase 3 of the 2026-04-30 reconciliation plan (F-TGT-05, Q2=B+C).
//!
//! Pattern derived from Terraform's `dev_overrides` (which writes to
//! `~/.terraformrc` and prints a banner on every apply). We diverge by
//! storing the file in the project directory (`.sindri/insecure-plugins.yaml`)
//! so it lives with the project and `git diff` surfaces overrides at
//! code-review time. A mandatory `reason` field keeps each entry
//! self-documenting.
//!
//! ## Lifecycle
//!
//! - **Add**: `sindri target plugin trust <kind> --insecure --reason <text>`
//!   appends an entry. One-time stderr warn at trust-time.
//! - **Banner**: `sindri apply` reads the file at startup and prints a
//!   yellow stderr banner listing every active entry.
//! - **Remove**: edit the file by hand (or `sindri target plugin trust
//!   <kind> --signer ...` overwrites the entry to a real key — both
//!   signals are auditable in the file's history).
//!
//! ## File format
//!
//! ```yaml
//! apiVersion: sindri.dev/v4
//! kind: InsecurePlugins
//! plugins:
//!   - kind: my-dev-plugin
//!     reason: "Local debugging of issue #1234"
//!     timestamp: "2026-04-30T20:15:00Z"
//!     user: "alice"
//!     hostname: "alice-laptop"
//! ```

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const INSECURE_PLUGINS_API_VERSION: &str = "sindri.dev/v4";
pub const INSECURE_PLUGINS_KIND: &str = "InsecurePlugins";

/// Default file location relative to the project root. Created on
/// first write; the parent `.sindri/` directory is created lazily.
pub fn insecure_plugins_path() -> PathBuf {
    PathBuf::from(".sindri").join("insecure-plugins.yaml")
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InsecurePluginsFile {
    /// `sindri.dev/v4` — validated.
    #[serde(default = "default_api_version")]
    pub api_version: String,
    /// `InsecurePlugins` — validated.
    #[serde(default = "default_kind")]
    pub kind: String,
    /// Each plugin opted into without a cosign signature.
    #[serde(default)]
    pub plugins: Vec<InsecurePluginEntry>,
}

impl Default for InsecurePluginsFile {
    fn default() -> Self {
        InsecurePluginsFile {
            api_version: default_api_version(),
            kind: default_kind(),
            plugins: Vec::new(),
        }
    }
}

fn default_api_version() -> String {
    INSECURE_PLUGINS_API_VERSION.to_string()
}

fn default_kind() -> String {
    INSECURE_PLUGINS_KIND.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InsecurePluginEntry {
    /// Plugin kind name (matches `target.kind` in `sindri.yaml`).
    pub kind: String,
    /// Mandatory operator-supplied justification.
    pub reason: String,
    /// RFC3339 UTC timestamp when the override was added.
    pub timestamp: String,
    /// Username that ran the trust command (`$USER` or `whoami`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    /// Hostname that ran the trust command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
}

impl InsecurePluginsFile {
    /// Read the file at `path`. Returns an empty default if the file
    /// does not exist (no plugins are insecure-trusted yet).
    pub fn load(path: &Path) -> Result<Self, std::io::Error> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        // serde_yaml errors mapped through io::Error so callers can use `?`.
        serde_yaml::from_str(&content).map_err(std::io::Error::other)
    }

    /// Atomic write via `<path>.tmp` + rename.
    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let yaml = serde_yaml::to_string(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("yaml.tmp");
        std::fs::write(&tmp, yaml)?;
        std::fs::rename(&tmp, path)
    }

    /// Append (or replace) an entry for `kind`. Returns the previous
    /// entry if one existed.
    pub fn upsert(&mut self, entry: InsecurePluginEntry) -> Option<InsecurePluginEntry> {
        let existing = self
            .plugins
            .iter()
            .position(|p| p.kind == entry.kind)
            .map(|i| self.plugins.remove(i));
        self.plugins.push(entry);
        existing
    }

    /// True when there are zero entries — banner emitter uses this
    /// to skip work cheaply.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

/// Build a single `InsecurePluginEntry` with `timestamp` populated to
/// the current UTC instant and `user`/`hostname` populated from the
/// environment (best-effort; missing values become `None`).
pub fn new_entry(kind: &str, reason: &str) -> InsecurePluginEntry {
    InsecurePluginEntry {
        kind: kind.into(),
        reason: reason.into(),
        timestamp: rfc3339_now(),
        user: std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .ok(),
        hostname: hostname_lookup(),
    }
}

fn rfc3339_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, m, d, h, mi, s) = unix_to_ymdhms(secs);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn unix_to_ymdhms(secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let rem = (secs % 86_400) as u32;
    let hour = rem / 3600;
    let minute = (rem % 3600) / 60;
    let second = rem % 60;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year as u32, m as u32, d as u32, hour, minute, second)
}

fn hostname_lookup() -> Option<String> {
    // Best-effort, no extra dep. Falls back to `HOSTNAME` env then None.
    std::env::var("HOSTNAME").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("insecure-plugins.yaml");
        let f = InsecurePluginsFile::default();
        f.save(&path).unwrap();
        let loaded = InsecurePluginsFile::load(&path).unwrap();
        assert!(loaded.plugins.is_empty());
        assert_eq!(loaded.api_version, INSECURE_PLUGINS_API_VERSION);
        assert_eq!(loaded.kind, INSECURE_PLUGINS_KIND);
    }

    #[test]
    fn round_trip_with_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("insecure-plugins.yaml");
        let mut f = InsecurePluginsFile::default();
        f.upsert(new_entry("foo", "debugging"));
        f.upsert(new_entry("bar", "ci sandbox"));
        f.save(&path).unwrap();
        let loaded = InsecurePluginsFile::load(&path).unwrap();
        assert_eq!(loaded.plugins.len(), 2);
        assert!(loaded.plugins.iter().any(|p| p.kind == "foo"));
        assert!(loaded.plugins.iter().any(|p| p.kind == "bar"));
    }

    #[test]
    fn upsert_replaces_existing_entry() {
        let mut f = InsecurePluginsFile::default();
        f.upsert(new_entry("foo", "old reason"));
        let prev = f.upsert(new_entry("foo", "new reason"));
        assert!(prev.is_some());
        assert_eq!(prev.unwrap().reason, "old reason");
        assert_eq!(f.plugins.len(), 1);
        assert_eq!(f.plugins[0].reason, "new reason");
    }

    #[test]
    fn load_missing_file_is_empty_default() {
        let f = InsecurePluginsFile::load(Path::new("/no/such/file/xyz.yaml")).unwrap();
        assert!(f.plugins.is_empty());
    }

    #[test]
    fn deny_unknown_fields_rejects_typos() {
        let yaml = r#"
apiVersion: sindri.dev/v4
kind: InsecurePlugins
plugins:
  - kind: foo
    reason: bar
    timestamp: "2026-04-30T00:00:00Z"
    extraField: oops
"#;
        let r: Result<InsecurePluginsFile, _> = serde_yaml::from_str(yaml);
        assert!(r.is_err(), "deny_unknown_fields should reject typos");
    }

    #[test]
    fn rfc3339_now_round_trips() {
        let s = rfc3339_now();
        // Format check: YYYY-MM-DDTHH:MM:SSZ
        assert_eq!(s.len(), 20);
        assert!(s.ends_with('Z'));
        assert!(s.chars().nth(4).unwrap() == '-');
        assert!(s.chars().nth(10).unwrap() == 'T');
    }
}
