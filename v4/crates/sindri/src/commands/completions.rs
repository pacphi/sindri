//! `sindri completions <shell>` — emit a shell completion script (ADR-011).
//!
//! Output is always written to stdout. The user is responsible for redirecting
//! it to the shell-specific install location:
//!
//! | shell      | typical install path                                   |
//! |------------|--------------------------------------------------------|
//! | bash       | `/etc/bash_completion.d/sindri` (or `~/.local/share/bash-completion/completions/sindri`) |
//! | zsh        | a directory in `$fpath`, e.g. `~/.zsh/completions/_sindri` |
//! | fish       | `~/.config/fish/completions/sindri.fish`               |
//! | powershell | source-script in `$PROFILE`                            |
//!
//! Example:
//! ```sh
//! sindri completions bash > ~/.local/share/bash-completion/completions/sindri
//! ```

use clap::Command;
use clap_complete::{generate, Shell};
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use std::io;

/// Arguments for `sindri completions`.
pub struct CompletionsArgs {
    /// One of `bash`, `zsh`, `fish`, `powershell`.
    pub shell: String,
}

/// Run with the live CLI definition.
pub fn run<F>(args: CompletionsArgs, build_cmd: F) -> i32
where
    F: FnOnce() -> Command,
{
    let shell = match parse_shell(&args.shell) {
        Some(s) => s,
        None => {
            eprintln!(
                "Unknown shell '{}'. Valid: bash, zsh, fish, powershell.",
                args.shell
            );
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let mut cmd = build_cmd();
    let bin_name = cmd.get_name().to_string();
    let mut out = io::stdout().lock();
    generate(shell, &mut cmd, bin_name, &mut out);
    EXIT_SUCCESS
}

/// Emit completions to a writer (test-friendly).
pub fn emit_to_writer<W: io::Write>(
    shell_str: &str,
    cmd: &mut Command,
    writer: &mut W,
) -> Result<(), String> {
    let shell = parse_shell(shell_str).ok_or_else(|| format!("Unknown shell '{}'", shell_str))?;
    let bin_name = cmd.get_name().to_string();
    generate(shell, cmd, bin_name, writer);
    Ok(())
}

fn parse_shell(s: &str) -> Option<Shell> {
    match s.to_lowercase().as_str() {
        "bash" => Some(Shell::Bash),
        "zsh" => Some(Shell::Zsh),
        "fish" => Some(Shell::Fish),
        "powershell" | "pwsh" => Some(Shell::PowerShell),
        "elvish" => Some(Shell::Elvish),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Command;

    fn dummy_cmd() -> Command {
        Command::new("sindri").subcommand(Command::new("validate"))
    }

    #[test]
    fn bash_emits_valid_completion_script() {
        let mut cmd = dummy_cmd();
        let mut buf: Vec<u8> = Vec::new();
        emit_to_writer("bash", &mut cmd, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        // Bash completions registered with clap_complete contain a `complete -F`
        // line for the binary name.
        assert!(
            out.contains("complete -F"),
            "bash output missing `complete -F`: {}",
            out
        );
        assert!(out.contains("sindri"));
    }

    #[test]
    fn zsh_emits_compdef_directive() {
        let mut cmd = dummy_cmd();
        let mut buf: Vec<u8> = Vec::new();
        emit_to_writer("zsh", &mut cmd, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("#compdef"));
    }

    #[test]
    fn unknown_shell_errors() {
        let mut cmd = dummy_cmd();
        let mut buf: Vec<u8> = Vec::new();
        let err = emit_to_writer("tcsh", &mut cmd, &mut buf);
        assert!(err.is_err());
    }
}
