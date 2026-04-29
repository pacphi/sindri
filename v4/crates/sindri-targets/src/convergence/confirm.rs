//! Pluggable confirmation prompt for destructive plan entries.
//!
//! `target update` calls into a `Confirm` impl before executing any
//! `Destroy` / `DestroyAndRecreate` action. `--auto-approve` short-
//! circuits to `AlwaysYesConfirm`; the default is [`StdinConfirm`].
//!
//! Tests inject [`ScriptedConfirm`] to simulate user input.

use std::cell::Cell;

/// Decide whether to proceed with a destructive change.
pub trait Confirm {
    /// Return true to proceed, false to abort. Implementations may
    /// print to stderr/stdout to display the prompt; the engine
    /// itself does not.
    fn confirm(&mut self, prompt: &str) -> bool;
}

/// Always answers yes — used by `--auto-approve`.
#[derive(Default)]
pub struct AlwaysYesConfirm;
impl Confirm for AlwaysYesConfirm {
    fn confirm(&mut self, _prompt: &str) -> bool {
        true
    }
}

/// Always answers no — useful in tests to assert the default-deny
/// behaviour of plain `target update`.
#[derive(Default)]
pub struct AlwaysNoConfirm;
impl Confirm for AlwaysNoConfirm {
    fn confirm(&mut self, _prompt: &str) -> bool {
        false
    }
}

/// Reads a single y/n line from stdin. Anything other than `y` or
/// `yes` (case-insensitive) returns false — fail-safe.
pub struct StdinConfirm {
    pub stream_isatty: bool,
}

impl Default for StdinConfirm {
    fn default() -> Self {
        StdinConfirm {
            // Best-effort tty check: stdin.is_terminal() is stable
            // since 1.70. Fall back to true if unavailable.
            stream_isatty: std::io::IsTerminal::is_terminal(&std::io::stdin()),
        }
    }
}

impl Confirm for StdinConfirm {
    fn confirm(&mut self, prompt: &str) -> bool {
        use std::io::{BufRead, Write};
        if !self.stream_isatty {
            // Non-interactive: refuse to destroy.
            eprintln!("{} (non-interactive — refusing)", prompt);
            return false;
        }
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        let _ = write!(out, "{} [y/N]: ", prompt);
        let _ = out.flush();
        let mut line = String::new();
        if std::io::stdin().lock().read_line(&mut line).is_err() {
            return false;
        }
        matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
    }
}

/// Returns a fixed sequence of pre-scripted answers. Once the queue is
/// drained subsequent calls return `false` (default-deny).
pub struct ScriptedConfirm {
    answers: Vec<bool>,
    cursor: Cell<usize>,
    pub prompts_seen: Vec<String>,
}

impl ScriptedConfirm {
    pub fn new(answers: Vec<bool>) -> Self {
        ScriptedConfirm {
            answers,
            cursor: Cell::new(0),
            prompts_seen: Vec::new(),
        }
    }
}

impl Confirm for ScriptedConfirm {
    fn confirm(&mut self, prompt: &str) -> bool {
        self.prompts_seen.push(prompt.to_string());
        let i = self.cursor.get();
        let ans = self.answers.get(i).copied().unwrap_or(false);
        self.cursor.set(i + 1);
        ans
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_yes_returns_true() {
        let mut c = AlwaysYesConfirm;
        assert!(c.confirm("destroy?"));
    }

    #[test]
    fn always_no_returns_false() {
        let mut c = AlwaysNoConfirm;
        assert!(!c.confirm("destroy?"));
    }

    #[test]
    fn scripted_consumes_answers_in_order() {
        let mut c = ScriptedConfirm::new(vec![true, false, true]);
        assert!(c.confirm("a"));
        assert!(!c.confirm("b"));
        assert!(c.confirm("c"));
        // exhausted → default-deny
        assert!(!c.confirm("d"));
        assert_eq!(c.prompts_seen, vec!["a", "b", "c", "d"]);
    }
}
