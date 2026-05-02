//! `target-doc-gen` — parses the [`Target`] trait surface in
//! `crates/sindri-targets/src/traits.rs` and emits a markdown summary at
//! `docs/_generated/target-trait.md`.
//!
//! The output is consumed by `docs/TARGETS.md` via fenced
//! `<!-- BEGIN AUTOGEN target-trait -->` / `<!-- END AUTOGEN target-trait -->`
//! markers. CI runs this binary with `--check` to fail if the generated
//! file is out of date with the trait source.
//!
//! Pattern mirrors `tools/schema-gen` (F-XCUT freshness check).

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::path::PathBuf;
use syn::{Item, ReturnType, TraitItem, Type};

#[derive(Parser, Debug)]
#[command(about = "Generate v4/docs/_generated/target-trait.md from the Target trait.")]
struct Args {
    /// Verify the generated file is in sync with the source trait. Exits
    /// non-zero (and prints a diff) when a regen would change the file.
    #[arg(long)]
    check: bool,
}

struct WorkspacePaths {
    trait_src: PathBuf,
    out_md: PathBuf,
    /// Hand-authored doc that hosts an inline summary table between the
    /// `<!-- BEGIN AUTOGEN target-trait -->` markers.
    targets_md: PathBuf,
}

/// Locations are computed from `CARGO_MANIFEST_DIR` so the binary works
/// regardless of where it was launched from.
fn workspace_paths() -> WorkspacePaths {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")); // tools/target-doc-gen
    let v4_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("crate is two levels under v4/")
        .to_path_buf();
    WorkspacePaths {
        trait_src: v4_root
            .join("crates")
            .join("sindri-targets")
            .join("src")
            .join("traits.rs"),
        out_md: v4_root
            .join("docs")
            .join("_generated")
            .join("target-trait.md"),
        targets_md: v4_root.join("docs").join("TARGETS.md"),
    }
}

const AUTOGEN_BEGIN: &str = "<!-- BEGIN AUTOGEN target-trait -->";
const AUTOGEN_END: &str = "<!-- END AUTOGEN target-trait -->";

fn rewrite_autogen_block(host: &str, body: &str) -> Result<String> {
    let begin = host
        .find(AUTOGEN_BEGIN)
        .ok_or_else(|| anyhow!("autogen begin marker not found in TARGETS.md"))?;
    let end = host
        .find(AUTOGEN_END)
        .ok_or_else(|| anyhow!("autogen end marker not found in TARGETS.md"))?;
    if end <= begin {
        return Err(anyhow!("autogen markers in wrong order in TARGETS.md"));
    }
    let mut out = String::with_capacity(host.len());
    out.push_str(&host[..begin]);
    out.push_str(AUTOGEN_BEGIN);
    out.push_str("\n\n");
    out.push_str(body);
    out.push_str("\n\n");
    out.push_str(&host[end..]);
    Ok(out)
}

fn main() -> Result<()> {
    let args = Args::parse();
    let paths = workspace_paths();

    let source = std::fs::read_to_string(&paths.trait_src)
        .with_context(|| format!("read {}", paths.trait_src.display()))?;
    let parsed: syn::File =
        syn::parse_file(&source).with_context(|| format!("parse {}", paths.trait_src.display()))?;

    let target_trait = parsed
        .items
        .iter()
        .find_map(|i| match i {
            Item::Trait(t) if t.ident == "Target" => Some(t),
            _ => None,
        })
        .ok_or_else(|| anyhow!("`Target` trait not found in {}", paths.trait_src.display()))?;

    let methods = extract_methods(target_trait);
    let full_doc = render_full(&methods);
    let summary = render_summary_table(&methods);

    let host = std::fs::read_to_string(&paths.targets_md)
        .with_context(|| format!("read {}", paths.targets_md.display()))?;
    let new_host = rewrite_autogen_block(&host, &summary)?;

    if args.check {
        let mut stale = false;
        let existing = std::fs::read_to_string(&paths.out_md).unwrap_or_default();
        if existing != full_doc {
            eprintln!("target-trait.md is out of date: {}", paths.out_md.display());
            stale = true;
        }
        if host != new_host {
            eprintln!(
                "TARGETS.md autogen block is out of date: {}",
                paths.targets_md.display()
            );
            stale = true;
        }
        if stale {
            eprintln!(
                "Run `cargo run -p target-doc-gen` to regenerate (source: {}).",
                paths.trait_src.display()
            );
            std::process::exit(1);
        }
        println!("ok     {}", paths.out_md.display());
        println!("ok     {}", paths.targets_md.display());
        return Ok(());
    }

    if let Some(parent) = paths.out_md.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    std::fs::write(&paths.out_md, &full_doc)
        .with_context(|| format!("write {}", paths.out_md.display()))?;
    std::fs::write(&paths.targets_md, &new_host)
        .with_context(|| format!("write {}", paths.targets_md.display()))?;
    println!("wrote  {}", paths.out_md.display());
    println!("wrote  {}", paths.targets_md.display());
    Ok(())
}

#[derive(Debug)]
struct Method {
    name: String,
    signature: String,
    doc: String,
    /// True when the trait body provides a default implementation. Distinguishes
    /// "every target must implement this" from "opt-in extension point."
    has_default: bool,
    /// Best-effort summary of the return type (e.g. `Result<…>`, `Vec<…>`,
    /// `&str`). Empty for `()` returns.
    returns: String,
}

fn extract_methods(t: &syn::ItemTrait) -> Vec<Method> {
    t.items
        .iter()
        .filter_map(|i| match i {
            TraitItem::Fn(f) => Some(f),
            _ => None,
        })
        .map(|f| {
            let name = f.sig.ident.to_string();
            let signature = render_signature(&f.sig);
            let doc = collect_doc(&f.attrs);
            let has_default = f.default.is_some();
            let returns = render_return(&f.sig.output);
            Method {
                name,
                signature,
                doc,
                has_default,
                returns,
            }
        })
        .collect()
}

fn render_signature(sig: &syn::Signature) -> String {
    // Reconstruct a clean one-line signature. We rely on `quote::ToTokens`
    // via syn's Display impl chain, then post-process to collapse whitespace.
    use proc_macro2::TokenStream;
    use quote_compat::quote_signature;
    let ts: TokenStream = quote_signature(sig);
    let raw = ts.to_string();
    collapse_ws(&raw)
}

fn render_return(ret: &ReturnType) -> String {
    match ret {
        ReturnType::Default => String::new(),
        ReturnType::Type(_, ty) => collapse_ws(&type_to_string(ty)),
    }
}

fn type_to_string(ty: &Type) -> String {
    use quote_compat::quote_type;
    quote_compat::ts_string(quote_type(ty))
}

fn collapse_ws(s: &str) -> String {
    let single_spaced: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    tidy_rust(&single_spaced)
}

/// Cosmetic post-pass: TokenStream::to_string() inserts spaces around every
/// punctuation token. The output is *legal* Rust but reads poorly in
/// markdown. These replacements make it match how a human would write the
/// same signature without losing fidelity.
fn tidy_rust(s: &str) -> String {
    let mut out = s.to_string();
    // Path separator: `std :: path :: Path` → `std::path::Path`.
    out = out.replace(" :: ", "::");
    out = out.replace(":: ", "::");
    out = out.replace(" ::", "::");
    // Reference forms.
    out = out.replace("& mut self", "&mut self");
    out = out.replace("& mut ", "&mut ");
    out = out.replace("& '", "&'");
    out = out.replace("& self", "&self");
    out = out.replace("& str", "&str");
    out = out.replace("& [", "&[");
    // Bare `& Foo` (e.g. `& std::path::Path`) — cover after path collapse.
    while let Some(pos) = out.find("& ") {
        // Don't touch sequences like "& &" or where the next char isn't ident-y.
        let next = out[pos + 2..].chars().next();
        if matches!(next, Some(c) if c.is_alphabetic() || c == '_') {
            out.replace_range(pos..pos + 2, "&");
        } else {
            break;
        }
    }
    // Generic-bracket spacing.
    out = out.replace(" <", "<");
    out = out.replace("< ", "<");
    out = out.replace(" >", ">");
    // Comma / colon spacing.
    out = out.replace(" ,", ",");
    out = out.replace(" :", ":");
    // Paren spacing.
    out = out.replace(" )", ")");
    out = out.replace("( ", "(");
    // Trailing comma in argument list: `..., u64,)` → `..., u64)`.
    out = out.replace(",)", ")");
    // Function-name to paren: `name (` → `name(`. We only want this for
    // ident+space+open-paren; collapse runs.
    out = collapse_ident_paren(&out);
    out
}

fn collapse_ident_paren(s: &str) -> String {
    // Walk byte-wise; whenever we see `<ident> (`, replace with `<ident>(`.
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == ' ' && i + 1 < bytes.len() && bytes[i + 1] == b'(' {
            // Look back to the previous non-space char.
            let prev = out.chars().last();
            if matches!(prev, Some(p) if p.is_alphanumeric() || p == '_' || p == '>' || p == ']') {
                // Drop this space.
                i += 1;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

fn collect_doc(attrs: &[syn::Attribute]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for a in attrs {
        if !a.path().is_ident("doc") {
            continue;
        }
        // syn 2: `#[doc = "..."]` — extract the string literal from the meta.
        let syn::Meta::NameValue(nv) = &a.meta else {
            continue;
        };
        let syn::Expr::Lit(lit) = &nv.value else {
            continue;
        };
        let syn::Lit::Str(s) = &lit.lit else {
            continue;
        };
        let raw = s.value();
        // Doc comment lines start with a leading space when written `///`. Trim
        // exactly one leading space so the markdown output is left-flush.
        let trimmed = raw.strip_prefix(' ').unwrap_or(&raw).to_string();
        lines.push(trimmed);
    }
    lines.join("\n")
}

/// Inline-inclusion fragment: just the method summary table. Embedded
/// between the `<!-- BEGIN AUTOGEN target-trait -->` markers in TARGETS.md
/// so a reader sees the surface at a glance.
fn render_summary_table(methods: &[Method]) -> String {
    let mut out = String::new();
    out.push_str(
        "_Auto-generated from \
         [`crates/sindri-targets/src/traits.rs`](../crates/sindri-targets/src/traits.rs); \
         see [`docs/_generated/target-trait.md`](_generated/target-trait.md) for per-method detail._\n\n",
    );
    out.push_str("| Method | Default impl? | Returns |\n");
    out.push_str("|--------|---------------|---------|\n");
    for m in methods {
        let returns = if m.returns.is_empty() {
            "()".to_string()
        } else {
            format!("`{}`", m.returns)
        };
        let default = if m.has_default { "yes" } else { "**required**" };
        out.push_str(&format!(
            "| `{name}` | {default} | {returns} |\n",
            name = m.name,
            default = default,
            returns = returns,
        ));
    }
    out
}

/// Full reference document. Lives at `docs/_generated/target-trait.md`.
fn render_full(methods: &[Method]) -> String {
    let mut out = String::new();
    out.push_str("# Target trait surface (auto-generated)\n\n");
    out.push_str(
        "**Source:** [`crates/sindri-targets/src/traits.rs`](../../crates/sindri-targets/src/traits.rs).\n\
         **Generator:** [`tools/target-doc-gen`](../../tools/target-doc-gen) — \
         do not edit this file by hand. Run `cargo run -p target-doc-gen` to refresh.\n\n",
    );

    out.push_str("## Method summary\n\n");
    out.push_str("| Method | Default impl? | Returns |\n");
    out.push_str("|--------|---------------|---------|\n");
    for m in methods {
        let returns = if m.returns.is_empty() {
            "()".to_string()
        } else {
            format!("`{}`", m.returns)
        };
        let default = if m.has_default { "yes" } else { "**required**" };
        out.push_str(&format!(
            "| [`{name}`](#{anchor}) | {default} | {returns} |\n",
            name = m.name,
            anchor = m.name.replace('_', "-"),
            default = default,
            returns = returns,
        ));
    }
    out.push('\n');

    out.push_str("## Method detail\n\n");
    for m in methods {
        out.push_str(&format!("### `{}`\n\n", m.name));
        out.push_str("```rust\n");
        out.push_str(&m.signature);
        out.push_str("\n```\n\n");
        if !m.doc.is_empty() {
            out.push_str(&m.doc);
            out.push_str("\n\n");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tidy_collapses_token_spacing() {
        let raw = "fn foo (& self , x : & str) -> Result < String , Error >";
        let tidy = tidy_rust(raw);
        assert_eq!(tidy, "fn foo(&self, x: &str) -> Result<String, Error>");
    }

    #[test]
    fn tidy_preserves_path_separators() {
        let raw = "x : & std :: path :: Path";
        let tidy = tidy_rust(raw);
        assert_eq!(tidy, "x: &std::path::Path");
    }

    #[test]
    fn tidy_strips_trailing_argument_comma() {
        let raw = "fn f (a : u8 , b : u8 ,)";
        let tidy = tidy_rust(raw);
        assert_eq!(tidy, "fn f(a: u8, b: u8)");
    }

    #[test]
    fn rewrite_autogen_replaces_block_contents() {
        let host = "before\n<!-- BEGIN AUTOGEN target-trait -->\nold\n<!-- END AUTOGEN target-trait -->\nafter\n";
        let new = rewrite_autogen_block(host, "NEW BODY").unwrap();
        assert!(new.contains("NEW BODY"));
        assert!(!new.contains("old"));
        assert!(new.contains("before"));
        assert!(new.contains("after"));
    }

    #[test]
    fn rewrite_autogen_errors_when_marker_missing() {
        let host = "no markers here\n";
        assert!(rewrite_autogen_block(host, "x").is_err());
    }
}

// --- Local quote shim -------------------------------------------------------
//
// We need to render a syn::Signature / syn::Type back to a TokenStream string.
// The `quote` crate is the standard way; pulling it in for two helpers is
// fine but the workspace doesn't already use it. To stay dep-light we lean on
// `proc-macro2`'s ToTokens forwarding (syn's types implement `ToTokens`
// directly under the `extra-traits` feature already enabled in Cargo.toml).
mod quote_compat {
    use proc_macro2::TokenStream;
    use syn::__private::ToTokens;

    pub fn quote_signature(sig: &syn::Signature) -> TokenStream {
        let mut ts = TokenStream::new();
        sig.to_tokens(&mut ts);
        ts
    }

    pub fn quote_type(ty: &syn::Type) -> TokenStream {
        let mut ts = TokenStream::new();
        ty.to_tokens(&mut ts);
        ts
    }

    pub fn ts_string(ts: TokenStream) -> String {
        ts.to_string()
    }
}
