//! Well-known env-var → audience mappings for ambient credential discovery
//! (ADR-027 §"Phase 4").
//!
//! Used by built-in targets (`local`, `docker`, ...) to advertise
//! [`AuthCapability`] entries for credentials that operators have already
//! plumbed into their shell environment. The list is intentionally small and
//! conservative — only widely-recognised vendor variables that are safe to
//! probe by name. Users with bespoke env-vars should declare a `provides:`
//! entry on the target manifest instead of relying on this table.
//!
//! All detection is purely lexical (does `std::env::var` return `Ok` for the
//! variable name?). No subprocess is spawned and no value is read into memory
//! — only its presence is observed.

use sindri_core::auth::{AuthCapability, AuthSource};

/// One row in the env-var → audience table.
///
/// The capability `id` is derived deterministically from the variable name
/// (lower-cased) so multiple targets that surface the same env-var produce
/// the same capability id, simplifying lockfile diffs.
struct EnvAudience {
    /// Environment variable name, e.g. `ANTHROPIC_API_KEY`.
    var: &'static str,
    /// Audience the credential is intended for, e.g. `urn:anthropic:api`.
    audience: &'static str,
    /// Capability id (kept stable across targets advertising the same var).
    id: &'static str,
}

/// Static, well-known mapping. Keep this list short and well-justified.
///
/// New entries must satisfy:
/// 1. The vendor uses the same env-var name across docs and SDKs.
/// 2. The audience matches what `ComponentManifest.auth.tokens[*].audience`
///    declares for the same vendor in the registry-core component set.
/// 3. The credential is a static bearer token (OAuth flows belong elsewhere).
const TABLE: &[EnvAudience] = &[
    EnvAudience {
        var: "ANTHROPIC_API_KEY",
        audience: "urn:anthropic:api",
        id: "anthropic_api_key",
    },
    EnvAudience {
        var: "OPENAI_API_KEY",
        audience: "urn:openai:api",
        id: "openai_api_key",
    },
    EnvAudience {
        var: "GEMINI_API_KEY",
        audience: "urn:google:generative-language",
        id: "gemini_api_key",
    },
    EnvAudience {
        var: "GOOGLE_API_KEY",
        audience: "urn:google:generative-language",
        id: "google_api_key",
    },
    EnvAudience {
        var: "GROQ_API_KEY",
        audience: "urn:groq:api",
        id: "groq_api_key",
    },
    EnvAudience {
        var: "MISTRAL_API_KEY",
        audience: "urn:mistral:api",
        id: "mistral_api_key",
    },
    EnvAudience {
        var: "COHERE_API_KEY",
        audience: "urn:cohere:api",
        id: "cohere_api_key",
    },
    EnvAudience {
        var: "GITHUB_TOKEN",
        audience: "https://api.github.com",
        id: "github_token",
    },
    EnvAudience {
        var: "GH_TOKEN",
        audience: "https://api.github.com",
        id: "github_token",
    },
    EnvAudience {
        var: "GITLAB_TOKEN",
        audience: "https://gitlab.com/api/v4",
        id: "gitlab_token",
    },
    EnvAudience {
        var: "HF_TOKEN",
        audience: "https://huggingface.co",
        id: "huggingface_token",
    },
    EnvAudience {
        var: "HUGGING_FACE_HUB_TOKEN",
        audience: "https://huggingface.co",
        id: "huggingface_token",
    },
];

/// Process-wide lock guarding env mutation in tests. Exposed at
/// `pub(crate)` so per-target tests in this crate can serialise alongside
/// `well_known` tests without smashing each other's `set_var` /
/// `remove_var` calls. Production callers do not touch this.
#[cfg(test)]
pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Walk the well-known table and emit one [`AuthCapability`] per env-var that
/// is currently set in this process's environment.
///
/// `priority` is a small value (10 by default in callers) so that more
/// specific sources (CLI delegation, secrets stores, explicit `provides:`)
/// always win on ties. Callers that want to override may pass their own.
///
/// This function is **fast**: it is a `std::env::var` lookup per row, no
/// subprocess. Suitable for the resolver hot path.
pub fn ambient_env_capabilities(priority: i32) -> Vec<AuthCapability> {
    TABLE
        .iter()
        .filter(|row| std::env::var_os(row.var).is_some_and(|v| !v.is_empty()))
        .map(|row| AuthCapability {
            id: row.id.to_string(),
            audience: row.audience.to_string(),
            source: AuthSource::FromEnv {
                var: row.var.to_string(),
            },
            priority,
        })
        .collect()
}

/// Reduced form: only return capabilities whose env-var is currently set
/// **and** is in the allow-list. Used by `docker` (which won't pass through
/// every host env-var by default; we still advertise so the operator's
/// `provides:` entry can confirm which to forward).
pub fn ambient_env_capabilities_filtered(priority: i32, allow: &[&str]) -> Vec<AuthCapability> {
    ambient_env_capabilities(priority)
        .into_iter()
        .filter(|c| {
            if let AuthSource::FromEnv { var } = &c.source {
                allow.iter().any(|a| a.eq_ignore_ascii_case(var))
            } else {
                false
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: clear all known env vars for a hermetic test.
    fn clear_all() {
        for row in TABLE {
            // SAFETY: caller holds ENV_LOCK; no concurrent reads from env.
            unsafe { std::env::remove_var(row.var) };
        }
    }

    #[test]
    fn no_env_set_yields_empty() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_all();
        let caps = ambient_env_capabilities(10);
        assert!(caps.is_empty(), "expected no capabilities, got {:?}", caps);
    }

    #[test]
    fn anthropic_env_yields_capability() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_all();
        // SAFETY: caller holds ENV_LOCK.
        unsafe { std::env::set_var("ANTHROPIC_API_KEY", "sk-test") };
        let caps = ambient_env_capabilities(10);
        // SAFETY: caller holds ENV_LOCK.
        unsafe { std::env::remove_var("ANTHROPIC_API_KEY") };

        assert_eq!(caps.len(), 1, "expected single capability, got {:?}", caps);
        let c = &caps[0];
        assert_eq!(c.id, "anthropic_api_key");
        assert_eq!(c.audience, "urn:anthropic:api");
        assert_eq!(c.priority, 10);
        match &c.source {
            AuthSource::FromEnv { var } => assert_eq!(var, "ANTHROPIC_API_KEY"),
            other => panic!("expected FromEnv, got {:?}", other),
        }
    }

    #[test]
    fn empty_string_is_not_treated_as_set() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_all();
        // SAFETY: caller holds ENV_LOCK.
        unsafe { std::env::set_var("OPENAI_API_KEY", "") };
        let caps = ambient_env_capabilities(10);
        // SAFETY: caller holds ENV_LOCK.
        unsafe { std::env::remove_var("OPENAI_API_KEY") };
        assert!(caps.is_empty(), "empty value must not advertise");
    }

    #[test]
    fn filtered_only_returns_allow_listed() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_all();
        // SAFETY: caller holds ENV_LOCK.
        unsafe { std::env::set_var("GEMINI_API_KEY", "x") };
        unsafe { std::env::set_var("GROQ_API_KEY", "y") };
        let caps = ambient_env_capabilities_filtered(5, &["GEMINI_API_KEY"]);
        // SAFETY: caller holds ENV_LOCK.
        unsafe { std::env::remove_var("GEMINI_API_KEY") };
        unsafe { std::env::remove_var("GROQ_API_KEY") };
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].id, "gemini_api_key");
    }
}
