/// StatusLedger — sindri log / sindri ledger (Sprint 12, ADR-007)
use serde::{Deserialize, Serialize};
use sindri_core::exit_codes::{EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LedgerEvent {
    pub timestamp: u64,
    pub event_type: String,
    pub component: String,
    pub version: String,
    pub target: String,
    pub success: bool,
    pub detail: Option<String>,
}

pub fn ledger_path() -> PathBuf {
    sindri_core::paths::home_dir()
        .unwrap_or_default()
        .join(".sindri")
        .join("ledger.jsonl")
}

pub fn append_event(event: &LedgerEvent) -> Result<(), std::io::Error> {
    use std::io::Write;
    let path = ledger_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    writeln!(f, "{}", serde_json::to_string(event).unwrap_or_default())
}

pub struct LogArgs {
    pub last: Option<usize>,
    pub json: bool,
}

pub fn run_log(args: LogArgs) -> i32 {
    let path = ledger_path();
    if !path.exists() {
        println!("No ledger entries yet. Run `sindri apply` to start logging.");
        return EXIT_SUCCESS;
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Cannot read ledger: {}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let mut events: Vec<LedgerEvent> = content
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();

    if let Some(n) = args.last {
        let start = events.len().saturating_sub(n);
        events = events[start..].to_vec();
    }

    if events.is_empty() {
        println!("No ledger entries.");
        return EXIT_SUCCESS;
    }

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&events).unwrap_or_default()
        );
    } else {
        for e in &events {
            let status = if e.success { "OK" } else { "FAIL" };
            println!(
                "[{}] {} {} {} v{} on {} — [{}]",
                e.timestamp, e.event_type, status, e.component, e.version, e.target, status
            );
        }
    }

    EXIT_SUCCESS
}
