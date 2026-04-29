//! `--explain` formatter for registry source consultation order
//! (ADR-028 — Phase 4.2).
//!
//! Mirrors the layout of [`crate::backend_choice::explain_choice`]: a block
//! of plain-text lines that lists which sources the resolver consulted, in
//! declared order, and what the outcome was for the requested component.
//!
//! Outcomes per source:
//!
//! - **matched** — the source's `scope` admitted the component and the
//!   resolver recorded the source's [`SourceDescriptor`] in the lockfile.
//!   No subsequent source is consulted (first-match-wins, DDD-08
//!   §"Source ordering").
//! - **skipped: out of scope** — the source declared an explicit `scope:`
//!   list and the requested component was not in it.
//! - **skipped: not found** — the source had no scope filter (or the
//!   filter would have admitted the component) but the source did not
//!   yield a matching entry. Today the resolver's enum-level pick uses
//!   scope-only matching, so this outcome is reserved for future
//!   live-fetch source consultation; this module emits it when the
//!   pre-fetched registry index has no entry for the component.
//!
//! The output format is intentionally compact — a tabular per-component
//! block, one source per line, terminating with the recorded descriptor.

use sindri_core::registry::ComponentEntry;
use sindri_core::source_descriptor::SourceDescriptor;
use sindri_registry::source::{ComponentName, RegistrySource};

/// Outcome for a single source consulted while resolving one component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceOutcome {
    /// The source matched and contributed the recorded descriptor.
    Matched,
    /// The source declared a `scope` list and the component was not in it.
    /// The argument echoes the source's scope for the explain output.
    OutOfScope(Vec<String>),
    /// The source was eligible (no scope, or scope admitted the component)
    /// but no matching entry was found in the source's index.
    NotFound,
}

impl SourceOutcome {
    /// Short token used in the explain table.
    pub fn label(&self) -> &'static str {
        match self {
            SourceOutcome::Matched => "matched",
            SourceOutcome::OutOfScope(_) => "skipped",
            SourceOutcome::NotFound => "skipped",
        }
    }

    /// Human-readable detail.
    pub fn detail(&self) -> String {
        match self {
            SourceOutcome::Matched => String::new(),
            SourceOutcome::OutOfScope(scope) => {
                format!("out of scope (scope: [{}])", scope.join(", "))
            }
            SourceOutcome::NotFound => "not found".to_string(),
        }
    }
}

/// One line of the per-component source-consultation report.
#[derive(Debug, Clone)]
pub struct ConsultedSource {
    pub kind: &'static str,
    pub origin: String,
    pub outcome: SourceOutcome,
}

/// Compute the outcome for each source given a component entry.
///
/// Walks `sources` in declared order with first-match-wins semantics: the
/// first source whose scope admits the component is recorded as `Matched`
/// and subsequent sources are not considered. Sources before the match are
/// recorded as `OutOfScope` (when scope filtered the component out).
///
/// `entry` is the registry entry the resolver pre-loaded from the local
/// registry cache; it is used only to determine the component's name and
/// to format the recorded descriptor when no explicit source matched.
pub fn consultation_for(
    sources: &[RegistrySource],
    entry: &ComponentEntry,
) -> (Vec<ConsultedSource>, Option<SourceDescriptor>) {
    let cname = ComponentName::from(entry.name.as_str());
    let mut out = Vec::with_capacity(sources.len());
    let mut matched_descriptor: Option<SourceDescriptor> = None;

    for src in sources {
        let kind = src.kind();
        let origin = source_origin(src);
        if matched_descriptor.is_some() {
            // Once we have matched, subsequent sources aren't consulted —
            // they don't appear in the explain output.
            break;
        }
        if src.scope_matches(&cname) {
            out.push(ConsultedSource {
                kind,
                origin,
                outcome: SourceOutcome::Matched,
            });
            matched_descriptor = Some(src.dispatch_lockfile_descriptor());
        } else {
            let scope = src
                .scope()
                .map(|s| s.iter().map(|n| n.as_str().to_string()).collect())
                .unwrap_or_default();
            out.push(ConsultedSource {
                kind,
                origin,
                outcome: SourceOutcome::OutOfScope(scope),
            });
        }
    }

    if matched_descriptor.is_none() {
        // Fall back to the legacy OCI descriptor reconstructed from
        // `entry.oci_ref` — the same behaviour the resolver uses when no
        // explicit source matches.
        matched_descriptor =
            sindri_registry::source::oci_descriptor_from_legacy_ref(&entry.oci_ref);
    }

    (out, matched_descriptor)
}

/// Format a `--explain` block for one component, mirroring the layout of
/// [`crate::backend_choice::explain_choice`].
pub fn format_explain(address: &str, sources: &[RegistrySource], entry: &ComponentEntry) -> String {
    let (consulted, descriptor) = consultation_for(sources, entry);

    let mut lines = Vec::new();
    lines.push(format!("Component: {}", address));
    lines.push("Source consultation order:".to_string());

    if consulted.is_empty() {
        // No explicit `registry.sources:` declared — the resolver fell
        // through to the legacy single-OCI cache path.
        lines.push("  (no explicit registry.sources declared; using legacy OCI cache)".to_string());
    } else {
        for (idx, c) in consulted.iter().enumerate() {
            let detail = c.outcome.detail();
            let suffix = if detail.is_empty() {
                String::new()
            } else {
                format!(": {}", detail)
            };
            lines.push(format!(
                "  {}. {} ({})  {}{}",
                idx + 1,
                c.kind,
                c.origin,
                c.outcome.label(),
                suffix,
            ));
        }
    }

    if let Some(d) = descriptor {
        lines.push(format!("Recorded descriptor: {}", describe_descriptor(&d)));
    } else {
        lines.push("Recorded descriptor: (none)".to_string());
    }

    lines.join("\n")
}

/// Short origin string for a source (URL, path, ref) used in the explain
/// table's parenthesised second column.
fn source_origin(src: &RegistrySource) -> String {
    match src {
        RegistrySource::LocalPath(s) => s.path.display().to_string(),
        RegistrySource::Oci(s) => format!("{}:{}", s.url, s.tag),
        RegistrySource::LocalOci(s) => s.layout_path.display().to_string(),
        RegistrySource::Git(s) => {
            if let Some(sub) = &s.subdir {
                format!("{} @ {} ({})", s.url, s.git_ref, sub.display())
            } else {
                format!("{} @ {}", s.url, s.git_ref)
            }
        }
    }
}

/// One-line summary of a [`SourceDescriptor`] for the explain output.
fn describe_descriptor(d: &SourceDescriptor) -> String {
    match d {
        SourceDescriptor::Oci {
            url,
            tag,
            manifest_digest,
        } => match manifest_digest {
            Some(digest) => format!("oci {}:{} (digest {})", url, tag, digest),
            None => format!("oci {}:{}", url, tag),
        },
        SourceDescriptor::LocalOci {
            layout_path,
            manifest_digest,
        } => match manifest_digest {
            Some(digest) => format!("local-oci {} (digest {})", layout_path.display(), digest),
            None => format!("local-oci {}", layout_path.display()),
        },
        SourceDescriptor::LocalPath { path } => format!("local-path {}", path.display()),
        SourceDescriptor::Git {
            url,
            commit_sha,
            subdir,
        } => match subdir {
            Some(s) => format!("git {} @ {} ({})", url, commit_sha, s.display()),
            None => format!("git {} @ {}", url, commit_sha),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::registry::{ComponentEntry, ComponentKind};
    use sindri_registry::source::{GitSource, LocalPathSource, OciSourceConfig, RegistrySource};
    use std::path::PathBuf;

    fn entry(name: &str) -> ComponentEntry {
        ComponentEntry {
            name: name.into(),
            backend: "mise".into(),
            latest: "1.0.0".into(),
            versions: vec!["1.0.0".into()],
            description: "test".into(),
            kind: ComponentKind::Component,
            oci_ref: format!("ghcr.io/sindri-dev/registry-core/{}:1.0.0", name),
            license: "MIT".into(),
            depends_on: vec![],
        }
    }

    fn local_path_with_scope(path: &str, scope: Vec<&str>) -> RegistrySource {
        RegistrySource::LocalPath(LocalPathSource {
            path: PathBuf::from(path),
            scope: Some(scope.into_iter().map(ComponentName::from).collect()),
        })
    }

    fn oci(url: &str, tag: &str) -> RegistrySource {
        RegistrySource::Oci(OciSourceConfig {
            url: url.to_string(),
            tag: tag.to_string(),
            scope: None,
            registry_name: "sindri/core".to_string(),
        })
    }

    fn git(url: &str, gref: &str) -> RegistrySource {
        RegistrySource::Git(GitSource {
            url: url.to_string(),
            git_ref: gref.to_string(),
            subdir: None,
            scope: None,
            require_signed: false,
        })
    }

    #[test]
    fn explain_emits_each_source_in_declared_order() {
        let sources = vec![
            local_path_with_scope("./my-components", vec!["acme-internal"]),
            oci("ghcr.io/sindri-dev/registry-core", "2026.04"),
        ];
        let report = format_explain("mise:nodejs", &sources, &entry("nodejs"));
        // First source: skipped (scope mismatch).
        assert!(report.contains("local-path"), "{}", report);
        assert!(report.contains("out of scope"), "{}", report);
        // Second source: matched.
        assert!(report.contains("oci"), "{}", report);
        assert!(report.contains("matched"), "{}", report);
        // Recorded descriptor is for the OCI source.
        assert!(report.contains("Recorded descriptor: oci"), "{}", report);
    }

    #[test]
    fn explain_stops_after_match() {
        // Two unscoped sources; the first matches and the second is not
        // shown in the consultation output.
        let sources = vec![
            git("https://example.com/repo.git", "main"),
            oci("ghcr.io/sindri-dev/registry-core", "2026.04"),
        ];
        let report = format_explain("mise:nodejs", &sources, &entry("nodejs"));
        assert!(report.contains("git"), "{}", report);
        assert!(report.contains("matched"), "{}", report);
        // The OCI source is not consulted (first-match-wins).
        let oci_lines: Vec<&str> = report
            .lines()
            .filter(|l| l.contains("ghcr.io/sindri-dev/registry-core"))
            .collect();
        assert!(
            oci_lines.is_empty(),
            "OCI source should not appear: {}",
            report
        );
    }

    #[test]
    fn explain_with_no_sources_uses_legacy_fallback() {
        let report = format_explain("mise:nodejs", &[], &entry("nodejs"));
        assert!(report.contains("legacy OCI cache"), "{}", report);
        // Legacy descriptor reconstructed from entry.oci_ref.
        assert!(report.contains("Recorded descriptor: oci"), "{}", report);
    }

    #[test]
    fn explain_records_scope_in_skip_detail() {
        let sources = vec![local_path_with_scope(
            "./my-components",
            vec!["acme-internal"],
        )];
        let report = format_explain("mise:nodejs", &sources, &entry("nodejs"));
        assert!(report.contains("scope: [acme-internal]"), "{}", report);
    }
}
