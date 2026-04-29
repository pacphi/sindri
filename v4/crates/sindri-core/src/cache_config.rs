//! User-level cache configuration (ADR-028 §4.5 — Phase 4.5).
//!
//! Lives at `~/.sindri/config.yaml` under the top-level `cache:` key:
//!
//! ```yaml
//! cache:
//!   git:
//!     max_size: "10GB"   # default
//!     max_age:  "90d"    # default
//! ```
//!
//! Both fields are optional and default to the constants below when
//! absent.  The `GitSource` reads these values at the start of every
//! `fetch_index` call and evicts cache entries that exceed either
//! threshold.
//!
//! ## Why a custom parser
//!
//! The values are user-facing strings rather than raw bytes/seconds so
//! operators can write `"10GB"` or `"90d"`.  We parse them with two tiny
//! readers in this module rather than pulling in `humansize` or
//! `humantime` — both deps are MIT/Apache-2.0 and would work, but the
//! syntax we accept is small enough that a 50-line parser keeps the
//! workspace dependency graph minimal.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Default `cache.git.max_size` — 10 gibibytes.
pub const DEFAULT_GIT_MAX_SIZE_BYTES: u64 = 10 * 1024 * 1024 * 1024;
/// Default `cache.git.max_age` — 90 days.
pub const DEFAULT_GIT_MAX_AGE_SECS: u64 = 90 * 24 * 60 * 60;

/// Top-level cache config block (`~/.sindri/config.yaml#/cache`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CacheConfig {
    /// Git-source cache eviction thresholds.
    #[serde(default, skip_serializing_if = "GitCacheConfig::is_default")]
    pub git: GitCacheConfig,
}

impl CacheConfig {
    /// `true` when every nested field is at its default — used to elide
    /// the config from serialized output.
    pub fn is_default(&self) -> bool {
        self.git.is_default()
    }
}

/// `cache.git` block — caps for the per-URL / per-commit Git cache laid
/// out at `~/.sindri/cache/git/<sha256(url)>/<commit-sha>/`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GitCacheConfig {
    /// Maximum total bytes under the entire `git/` cache root. Eviction
    /// removes commit-sha directories oldest-mtime-first until the total
    /// is back under this cap. Accepts strings like `"10GB"`, `"500MB"`,
    /// `"1.5GB"` — see [`parse_size`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_size: Option<String>,

    /// Maximum directory mtime age. Any commit-sha directory older than
    /// this is evicted regardless of size. Accepts strings like `"90d"`,
    /// `"7d"`, `"24h"` — see [`parse_age`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_age: Option<String>,
}

impl GitCacheConfig {
    /// `true` when no thresholds are set; the resolver will substitute
    /// the default constants.
    pub fn is_default(&self) -> bool {
        self.max_size.is_none() && self.max_age.is_none()
    }

    /// Resolved `max_size` in bytes — user value if set, otherwise the
    /// default constant.
    pub fn max_size_bytes(&self) -> u64 {
        self.max_size
            .as_deref()
            .and_then(|s| parse_size(s).ok())
            .unwrap_or(DEFAULT_GIT_MAX_SIZE_BYTES)
    }

    /// Resolved `max_age` as a [`Duration`] — user value if set, otherwise
    /// the default constant.
    pub fn max_age_duration(&self) -> Duration {
        let secs = self
            .max_age
            .as_deref()
            .and_then(|s| parse_age(s).ok())
            .unwrap_or(DEFAULT_GIT_MAX_AGE_SECS);
        Duration::from_secs(secs)
    }
}

/// Parse a human size string like `"10GB"`, `"500MB"`, `"1.5GB"`.
///
/// Accepted suffixes (case-insensitive):
///
/// - `B`  — bytes
/// - `KB` / `K` — 1024 bytes
/// - `MB` / `M` — 1024² bytes
/// - `GB` / `G` — 1024³ bytes
/// - `TB` / `T` — 1024⁴ bytes
///
/// Decimal fractions are honoured (`"1.5GB"` → `1610612736` bytes).
/// Whitespace between the number and the suffix is tolerated.
pub fn parse_size(s: &str) -> Result<u64, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("empty size string".to_string());
    }
    // Split into the longest leading numeric run + remainder.
    let mut split_idx = 0usize;
    for (i, c) in trimmed.char_indices() {
        if c.is_ascii_digit() || c == '.' {
            split_idx = i + c.len_utf8();
        } else {
            break;
        }
    }
    if split_idx == 0 {
        return Err(format!("no leading number in size string: {s:?}"));
    }
    let (num_str, suffix) = trimmed.split_at(split_idx);
    let num: f64 = num_str
        .parse()
        .map_err(|e| format!("invalid number {num_str:?}: {e}"))?;
    if num.is_nan() || num.is_sign_negative() {
        return Err(format!("size must be non-negative: {s:?}"));
    }
    let mult: f64 = match suffix.trim().to_ascii_uppercase().as_str() {
        "" | "B" => 1.0,
        "K" | "KB" => 1024.0,
        "M" | "MB" => 1024.0 * 1024.0,
        "G" | "GB" => 1024.0 * 1024.0 * 1024.0,
        "T" | "TB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        other => return Err(format!("unknown size suffix: {other:?}")),
    };
    Ok((num * mult) as u64)
}

/// Parse a human age string like `"90d"`, `"7d"`, `"24h"`, `"30m"`,
/// `"45s"` and return the equivalent number of seconds.
///
/// Accepted suffixes (case-sensitive — lowercase only, matching common
/// Unix idiom):
///
/// - `s` — seconds
/// - `m` — minutes
/// - `h` — hours
/// - `d` — days
pub fn parse_age(s: &str) -> Result<u64, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("empty age string".to_string());
    }
    let mut split_idx = 0usize;
    for (i, c) in trimmed.char_indices() {
        if c.is_ascii_digit() || c == '.' {
            split_idx = i + c.len_utf8();
        } else {
            break;
        }
    }
    if split_idx == 0 {
        return Err(format!("no leading number in age string: {s:?}"));
    }
    let (num_str, suffix) = trimmed.split_at(split_idx);
    let num: f64 = num_str
        .parse()
        .map_err(|e| format!("invalid number {num_str:?}: {e}"))?;
    if num.is_nan() || num.is_sign_negative() {
        return Err(format!("age must be non-negative: {s:?}"));
    }
    let mult: u64 = match suffix.trim() {
        "s" => 1,
        "m" => 60,
        "h" => 60 * 60,
        "d" => 24 * 60 * 60,
        other => return Err(format!("unknown age suffix: {other:?}")),
    };
    Ok((num * mult as f64) as u64)
}

/// Resolve `~/.sindri/config.yaml` and return the `cache:` block, or a
/// default config when the file is missing or unparseable.
///
/// Errors during read/parse are converted to defaults — eviction must
/// never block the resolver. The config file is end-user-editable and
/// should not be a hard requirement.
pub fn load_user_cache_config() -> CacheConfig {
    let Some(home) = crate::paths::home_dir() else {
        return CacheConfig::default();
    };
    load_cache_config_from(&home.join(".sindri").join("config.yaml"))
}

/// Read a specific config file path and return the `cache:` block; used
/// by tests to point at a tempdir.
pub fn load_cache_config_from(path: &std::path::Path) -> CacheConfig {
    let raw = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return CacheConfig::default(),
    };
    parse_cache_config_from_yaml(&raw)
}

/// Parse just the `cache:` block from a `~/.sindri/config.yaml` document.
/// Unknown keys at the top level are tolerated (the user may have set
/// other config). Returns the default config if the file has no `cache:`
/// key.
pub fn parse_cache_config_from_yaml(raw: &str) -> CacheConfig {
    #[derive(Deserialize)]
    struct OuterConfig {
        #[serde(default)]
        cache: CacheConfig,
    }
    serde_yaml::from_str::<OuterConfig>(raw)
        .map(|o| o.cache)
        .unwrap_or_default()
}

/// On-disk cache root for git sources, derived from `~/.sindri/cache/git/`.
/// Pure helper used by the eviction routine and the runtime cache code.
pub fn git_cache_root() -> Option<PathBuf> {
    crate::paths::home_dir().map(|h| h.join(".sindri").join("cache").join("git"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_size_accepts_common_suffixes() {
        assert_eq!(parse_size("0").unwrap(), 0);
        assert_eq!(parse_size("100B").unwrap(), 100);
        assert_eq!(parse_size("1KB").unwrap(), 1024);
        assert_eq!(parse_size("1MB").unwrap(), 1024 * 1024);
        assert_eq!(parse_size("1GB").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size("10GB").unwrap(), 10u64 * 1024 * 1024 * 1024);
        assert_eq!(parse_size("1TB").unwrap(), 1024u64.pow(4));
    }

    #[test]
    fn parse_size_accepts_fractions_and_lowercase() {
        let v = parse_size("1.5gb").unwrap();
        assert_eq!(v, (1.5 * 1024.0 * 1024.0 * 1024.0) as u64);
        assert_eq!(parse_size("500mb").unwrap(), 500 * 1024 * 1024);
    }

    #[test]
    fn parse_size_rejects_garbage() {
        assert!(parse_size("").is_err());
        assert!(parse_size("abc").is_err());
        assert!(parse_size("-5GB").is_err());
        assert!(parse_size("10ZZ").is_err());
    }

    #[test]
    fn parse_age_accepts_canonical_suffixes() {
        assert_eq!(parse_age("90d").unwrap(), 90 * 24 * 60 * 60);
        assert_eq!(parse_age("7d").unwrap(), 7 * 24 * 60 * 60);
        assert_eq!(parse_age("24h").unwrap(), 24 * 60 * 60);
        assert_eq!(parse_age("30m").unwrap(), 30 * 60);
        assert_eq!(parse_age("45s").unwrap(), 45);
    }

    #[test]
    fn parse_age_rejects_garbage() {
        assert!(parse_age("").is_err());
        assert!(parse_age("five-days").is_err());
        assert!(parse_age("10y").is_err());
        assert!(parse_age("-1d").is_err());
    }

    #[test]
    fn defaults_apply_when_config_absent() {
        let cfg = parse_cache_config_from_yaml("");
        assert!(cfg.is_default());
        assert_eq!(cfg.git.max_size_bytes(), DEFAULT_GIT_MAX_SIZE_BYTES);
        assert_eq!(
            cfg.git.max_age_duration(),
            Duration::from_secs(DEFAULT_GIT_MAX_AGE_SECS)
        );
    }

    #[test]
    fn defaults_apply_when_cache_block_missing() {
        // Non-cache top-level keys are tolerated; cache block defaults.
        let cfg = parse_cache_config_from_yaml("registry:\n  policy:\n    strict_oci: true\n");
        assert!(cfg.is_default());
    }

    #[test]
    fn custom_values_are_honoured() {
        let raw = "cache:\n  git:\n    max_size: \"500MB\"\n    max_age:  \"7d\"\n";
        let cfg = parse_cache_config_from_yaml(raw);
        assert!(!cfg.is_default());
        assert_eq!(cfg.git.max_size_bytes(), 500 * 1024 * 1024);
        assert_eq!(cfg.git.max_age_duration(), Duration::from_secs(7 * 86400));
    }

    #[test]
    fn invalid_strings_fall_back_to_defaults() {
        // The runtime helpers swallow parse errors and return the
        // defaults so a typo in `~/.sindri/config.yaml` never blocks
        // resolution.
        let raw = "cache:\n  git:\n    max_size: \"oops\"\n    max_age:  \"five-days\"\n";
        let cfg = parse_cache_config_from_yaml(raw);
        assert_eq!(cfg.git.max_size_bytes(), DEFAULT_GIT_MAX_SIZE_BYTES);
        assert_eq!(
            cfg.git.max_age_duration(),
            Duration::from_secs(DEFAULT_GIT_MAX_AGE_SECS)
        );
    }
}
