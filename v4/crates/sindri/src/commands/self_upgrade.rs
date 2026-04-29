//! `sindri self-upgrade` — upgrade the `sindri` CLI binary itself (ADR-011).
//!
//! Distinct from `sindri upgrade`, which upgrades user components.
//!
//! Detection rules:
//! - Cargo: `sindri` resolved under `~/.cargo/bin/` →
//!   `cargo install --locked sindri@latest`.
//! - npm: `sindri` resolved under a directory whose path contains
//!   `node_modules/.bin`, `mise/installs/node`, `.nvm/`, `npm-global`, or
//!   `nodejs/bin` → `npm install -g @sindri-dev/cli@latest`.
//! - Anything else → print the GitHub releases URL and exit 0 with a clear
//!   "manual upgrade required" message.
//!
//! The detection is **honest**: if we can't classify the install, we never
//! pretend to know how to upgrade.

use sindri_core::exit_codes::{EXIT_ERROR, EXIT_SUCCESS};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Outcome of install-method detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallMethod {
    /// `cargo install`-managed (under `~/.cargo/bin`).
    Cargo,
    /// npm-global / mise-managed Node install.
    Npm,
    /// Unknown — manual upgrade required.
    Manual,
}

/// Arguments for `sindri self-upgrade`.
pub struct SelfUpgradeArgs {
    /// Print what would happen but do not execute the upgrade.
    pub dry_run: bool,
    /// Override the resolved binary path (test-only).
    pub binary_path_override: Option<PathBuf>,
}

/// Entry point for `sindri self-upgrade`.
pub fn run(args: SelfUpgradeArgs) -> i32 {
    let bin_path = match resolve_binary_path(args.binary_path_override.clone()) {
        Some(p) => p,
        None => {
            eprintln!(
                "Could not locate the running `sindri` binary on $PATH. \
                 Visit https://github.com/pacphi/sindri/releases for manual upgrade instructions."
            );
            return EXIT_SUCCESS;
        }
    };

    let method = classify_install(&bin_path);
    println!(
        "Detected install method: {} (binary: {})",
        method_label(&method),
        bin_path.display()
    );

    match method {
        InstallMethod::Cargo => {
            let cmd = ["cargo", "install", "--locked", "sindri@latest"];
            run_or_print(&cmd, args.dry_run)
        }
        InstallMethod::Npm => {
            let cmd = ["npm", "install", "-g", "@sindri-dev/cli@latest"];
            run_or_print(&cmd, args.dry_run)
        }
        InstallMethod::Manual => {
            println!(
                "Manual upgrade required. Download the latest release from:\n  \
                https://github.com/pacphi/sindri/releases"
            );
            EXIT_SUCCESS
        }
    }
}

/// Resolve the path of the running `sindri` binary by consulting `which sindri`.
/// Falls back to `std::env::current_exe`. Tests can pass an explicit override.
pub fn resolve_binary_path(override_path: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(p) = override_path {
        return Some(p);
    }
    if let Ok(out) = Command::new("which").arg("sindri").output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return Some(PathBuf::from(s));
            }
        }
    }
    std::env::current_exe().ok()
}

/// Classify a binary path into an [`InstallMethod`].
pub fn classify_install(bin_path: &Path) -> InstallMethod {
    let s = bin_path.to_string_lossy().to_string();

    // Cargo's bin directory is the most reliable signal.
    if s.contains("/.cargo/bin/") || s.ends_with("/.cargo/bin/sindri") {
        return InstallMethod::Cargo;
    }

    // npm / mise / nvm install layouts.
    let npm_markers = [
        "/node_modules/.bin/",
        "/mise/installs/node",
        "/.nvm/",
        "/npm-global/",
        "/nodejs/bin/",
        "/lib/node_modules/",
    ];
    if npm_markers.iter().any(|m| s.contains(m)) {
        return InstallMethod::Npm;
    }

    InstallMethod::Manual
}

fn method_label(m: &InstallMethod) -> &'static str {
    match m {
        InstallMethod::Cargo => "cargo",
        InstallMethod::Npm => "npm",
        InstallMethod::Manual => "manual",
    }
}

fn run_or_print(cmd: &[&str], dry_run: bool) -> i32 {
    if dry_run {
        println!("Would run: {}", cmd.join(" "));
        return EXIT_SUCCESS;
    }
    println!("Running: {}", cmd.join(" "));
    let mut iter = cmd.iter();
    let bin = match iter.next() {
        Some(b) => *b,
        None => return EXIT_ERROR,
    };
    let rest: Vec<&str> = iter.copied().collect();
    match Command::new(bin).args(&rest).status() {
        Ok(s) if s.success() => {
            println!("Self-upgrade complete.");
            EXIT_SUCCESS
        }
        Ok(s) => {
            eprintln!("Upgrade command exited with status: {}", s);
            EXIT_ERROR
        }
        Err(e) => {
            eprintln!("Failed to run upgrade command: {}", e);
            EXIT_ERROR
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_cargo_install() {
        let p = PathBuf::from("/Users/me/.cargo/bin/sindri");
        assert_eq!(classify_install(&p), InstallMethod::Cargo);
    }

    #[test]
    fn detects_npm_global() {
        let p = PathBuf::from("/usr/local/lib/node_modules/.bin/sindri");
        assert_eq!(classify_install(&p), InstallMethod::Npm);
    }

    #[test]
    fn detects_mise_node() {
        let p = PathBuf::from("/Users/me/.local/share/mise/installs/node/20.11.0/bin/sindri");
        assert_eq!(classify_install(&p), InstallMethod::Npm);
    }

    #[test]
    fn unknown_install_falls_through_to_manual() {
        let p = PathBuf::from("/opt/random/sindri");
        assert_eq!(classify_install(&p), InstallMethod::Manual);
    }

    #[test]
    fn cargo_dry_run_succeeds() {
        let code = run(SelfUpgradeArgs {
            dry_run: true,
            binary_path_override: Some(PathBuf::from("/Users/me/.cargo/bin/sindri")),
        });
        assert_eq!(code, EXIT_SUCCESS);
    }

    #[test]
    fn manual_dry_run_succeeds() {
        let code = run(SelfUpgradeArgs {
            dry_run: true,
            binary_path_override: Some(PathBuf::from("/opt/random/sindri")),
        });
        assert_eq!(code, EXIT_SUCCESS);
    }
}
