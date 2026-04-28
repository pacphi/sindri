//! Plan rendering — Terraform-style human-readable output.
//!
//! No external `colored` crate dependency: the engine uses raw ANSI
//! escapes (the same approach the rest of the v4 CLI uses). Disable
//! with `RenderOptions::color = false`.

use super::plan::{Action, Plan};
use std::fmt::Write;

/// Rendering options.
#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    pub color: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions { color: true }
    }
}

const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const MAGENTA: &str = "\x1b[35m";
const DIM: &str = "\x1b[2m";

fn paint(s: &str, code: &str, color: bool) -> String {
    if color {
        format!("{}{}{}", code, s, RESET)
    } else {
        s.to_string()
    }
}

/// Render a plan as a multi-line string. The summary line at the
/// bottom matches Terraform's `Plan: X to add, Y to change, Z to
/// destroy` exactly so users have a familiar mental model.
pub fn render_plan(plan: &Plan, opts: RenderOptions) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Target: {} (kind: {})", plan.target_name, plan.kind);
    let _ = writeln!(out, "{}", "-".repeat(64));

    if plan.entries.is_empty() {
        let _ = writeln!(out, "No resources defined.");
        return out;
    }

    for entry in &plan.entries {
        let glyph = entry.action.glyph();
        let (code, label) = match &entry.action {
            Action::Noop => (DIM, "no change"),
            Action::Create => (GREEN, "create"),
            Action::Destroy => (RED, "destroy"),
            Action::InPlaceUpdate { .. } => (YELLOW, "update in-place"),
            Action::DestroyAndRecreate { .. } => (MAGENTA, "destroy + recreate"),
        };
        let lhs = format!("  {} {}", glyph, entry.name);
        let _ = writeln!(
            out,
            "{:<40} {}",
            paint(&lhs, code, opts.color),
            paint(label, code, opts.color)
        );
        match &entry.action {
            Action::InPlaceUpdate { changed_fields } => {
                for f in changed_fields {
                    let _ = writeln!(out, "      ~ {}", f);
                }
            }
            Action::DestroyAndRecreate { immutable_changes } => {
                for f in immutable_changes {
                    let line = format!("      ± {} (immutable — forces recreate)", f);
                    let _ = writeln!(out, "{}", paint(&line, MAGENTA, opts.color));
                }
            }
            _ => {}
        }
    }

    let c = plan.counts();
    let _ = writeln!(out, "{}", "-".repeat(64));
    let _ = writeln!(
        out,
        "Plan: {} to add, {} to change, {} to destroy, {} to recreate, {} unchanged",
        c.create, c.in_place, c.destroy, c.recreate, c.noop
    );
    out
}

#[cfg(test)]
mod tests {
    use super::super::plan::PlanEntry;
    use super::*;
    use serde_json::json;

    fn sample_plan() -> Plan {
        Plan {
            target_name: "edge".to_string(),
            kind: "fly".to_string(),
            entries: vec![
                PlanEntry {
                    name: "web".to_string(),
                    action: Action::Create,
                    desired: Some(json!({"app": "web"})),
                    recorded: None,
                },
                PlanEntry {
                    name: "cache".to_string(),
                    action: Action::InPlaceUpdate {
                        changed_fields: vec!["env.LOG".to_string()],
                    },
                    desired: Some(json!({})),
                    recorded: Some(json!({})),
                },
                PlanEntry {
                    name: "old".to_string(),
                    action: Action::Destroy,
                    desired: None,
                    recorded: Some(json!({})),
                },
                PlanEntry {
                    name: "db".to_string(),
                    action: Action::DestroyAndRecreate {
                        immutable_changes: vec!["regions".to_string()],
                    },
                    desired: Some(json!({})),
                    recorded: Some(json!({})),
                },
            ],
        }
    }

    #[test]
    fn render_includes_summary_line() {
        let r = render_plan(&sample_plan(), RenderOptions { color: false });
        assert!(r.contains("Plan: 1 to add, 1 to change, 1 to destroy, 1 to recreate, 0 unchanged"));
    }

    #[test]
    fn render_no_color_excludes_ansi_escapes() {
        let r = render_plan(&sample_plan(), RenderOptions { color: false });
        assert!(!r.contains("\x1b["), "expected no ANSI escapes: {:?}", r);
    }

    #[test]
    fn render_color_includes_ansi_escapes() {
        let r = render_plan(&sample_plan(), RenderOptions { color: true });
        assert!(r.contains("\x1b["));
    }

    #[test]
    fn render_empty_plan_says_no_resources() {
        let plan = Plan {
            target_name: "t".to_string(),
            kind: "docker".to_string(),
            entries: vec![],
        };
        let r = render_plan(&plan, RenderOptions { color: false });
        assert!(r.contains("No resources defined."));
    }
}
