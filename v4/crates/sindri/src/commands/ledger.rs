//! `sindri ledger {stats,export,compact}` — StatusLedger maintenance verbs
//! (ADR-007, ADR-011, implementation plan §12.3).
//!
//! All three sub-verbs operate on the JSONL file at `~/.sindri/ledger.jsonl`
//! (overridable in tests via the `path_override` field of each Args struct).

use crate::commands::log::{ledger_path, LedgerEvent};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::Serialize;
use sindri_core::exit_codes::{EXIT_ERROR, EXIT_SCHEMA_OR_RESOLVE_ERROR, EXIT_SUCCESS};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Arguments for `sindri ledger stats`.
pub struct StatsArgs {
    /// Optional ISO-8601 date (e.g. `2026-04-01`) — only count events with
    /// `timestamp >= since`.
    pub since: Option<String>,
    /// Emit JSON instead of a human-readable table.
    pub json: bool,
    /// Test override of the ledger path.
    pub path_override: Option<PathBuf>,
}

/// Arguments for `sindri ledger export`.
pub struct ExportArgs {
    /// `jsonl` (default, pass-through) or `csv` (with header row).
    pub format: String,
    /// Output file path. If `None`, write to stdout.
    pub output: Option<String>,
    /// Test override of the ledger path.
    pub path_override: Option<PathBuf>,
}

/// Arguments for `sindri ledger compact`.
pub struct CompactArgs {
    /// Number of most-recent events to keep in the active file.
    pub keep_last: usize,
    /// Test override of the ledger path.
    pub path_override: Option<PathBuf>,
    /// Test override of the directory in which archive files are written
    /// (defaults to the parent of the ledger file).
    pub archive_dir_override: Option<PathBuf>,
    /// Fixed timestamp suffix for archives (test-only; production uses
    /// the current time).
    pub timestamp_override: Option<String>,
}

/// Per-event-type counts emitted by `ledger stats`.
#[derive(Debug, Default, Serialize)]
pub struct LedgerStats {
    pub installs: usize,
    pub upgrades: usize,
    pub removes: usize,
    pub rollbacks: usize,
    pub other: usize,
    pub total: usize,
}

/// Run `sindri ledger stats`.
pub fn run_stats(args: StatsArgs) -> i32 {
    let path = args.path_override.clone().unwrap_or_else(ledger_path);
    let events = match read_events(&path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let since_ts = match args.since.as_deref().map(parse_iso_date).transpose() {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("{}", msg);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let stats = compute_stats(&events, since_ts);

    if args.json {
        match serde_json::to_string_pretty(&stats) {
            Ok(s) => println!("{}", s),
            Err(e) => {
                eprintln!("Failed to serialize stats: {}", e);
                return EXIT_ERROR;
            }
        }
    } else {
        println!(
            "Installs: {} / Upgrades: {} / Removes: {} / Rollbacks: {}",
            stats.installs, stats.upgrades, stats.removes, stats.rollbacks
        );
        if stats.other > 0 {
            println!("Other: {}", stats.other);
        }
        println!("Total: {}", stats.total);
    }
    EXIT_SUCCESS
}

/// Run `sindri ledger export`.
pub fn run_export(args: ExportArgs) -> i32 {
    let path = args.path_override.clone().unwrap_or_else(ledger_path);
    let events = match read_events(&path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    let body = match args.format.as_str() {
        "jsonl" | "" => events_to_jsonl(&events),
        "csv" => events_to_csv(&events),
        other => {
            eprintln!("Unknown export format '{}'. Valid: jsonl | csv.", other);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    match &args.output {
        None => {
            print!("{}", body);
            EXIT_SUCCESS
        }
        Some(p) => match std::fs::write(p, body) {
            Ok(_) => {
                println!("Wrote {} entries to {}", events.len(), p);
                EXIT_SUCCESS
            }
            Err(e) => {
                eprintln!("Failed to write {}: {}", p, e);
                EXIT_ERROR
            }
        },
    }
}

/// Run `sindri ledger compact`.
pub fn run_compact(args: CompactArgs) -> i32 {
    let path = args.path_override.clone().unwrap_or_else(ledger_path);
    let events = match read_events(&path) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{}", e);
            return EXIT_SCHEMA_OR_RESOLVE_ERROR;
        }
    };

    if events.len() <= args.keep_last {
        println!(
            "Nothing to compact ({} events, keep-last={}).",
            events.len(),
            args.keep_last
        );
        return EXIT_SUCCESS;
    }

    let split_at = events.len() - args.keep_last;
    let (to_archive, to_keep) = events.split_at(split_at);

    let archive_dir = args
        .archive_dir_override
        .clone()
        .unwrap_or_else(|| path.parent().map(PathBuf::from).unwrap_or_default());
    if let Err(e) = std::fs::create_dir_all(&archive_dir) {
        eprintln!("Cannot create archive dir {}: {}", archive_dir.display(), e);
        return EXIT_ERROR;
    }

    let ts = args
        .timestamp_override
        .clone()
        .unwrap_or_else(|| chrono::Utc::now().format("%Y%m%dT%H%M%S").to_string());
    let archive_path = archive_dir.join(format!("ledger-archive-{}.jsonl.gz", ts));

    if let Err(e) = write_gzipped_jsonl(&archive_path, to_archive) {
        eprintln!("Failed to write archive {}: {}", archive_path.display(), e);
        return EXIT_ERROR;
    }

    let active = events_to_jsonl(to_keep);
    let tmp = path.with_extension("jsonl.tmp");
    if let Err(e) = std::fs::write(&tmp, active) {
        eprintln!("Failed to write active ledger: {}", e);
        return EXIT_ERROR;
    }
    if let Err(e) = std::fs::rename(&tmp, &path) {
        eprintln!("Failed to atomically replace ledger: {}", e);
        return EXIT_ERROR;
    }

    println!(
        "Archived {} events to {}; {} events remain active.",
        to_archive.len(),
        archive_path.display(),
        to_keep.len()
    );
    EXIT_SUCCESS
}

// -- helpers -----------------------------------------------------------------

fn read_events(path: &Path) -> Result<Vec<LedgerEvent>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
    Ok(content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect())
}

fn events_to_jsonl(events: &[LedgerEvent]) -> String {
    let mut out = String::new();
    for e in events {
        if let Ok(s) = serde_json::to_string(e) {
            out.push_str(&s);
            out.push('\n');
        }
    }
    out
}

fn events_to_csv(events: &[LedgerEvent]) -> String {
    let mut out = String::new();
    out.push_str("timestamp,event_type,component,version,target,success,detail\n");
    for e in events {
        out.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            e.timestamp,
            csv_escape(&e.event_type),
            csv_escape(&e.component),
            csv_escape(&e.version),
            csv_escape(&e.target),
            e.success,
            csv_escape(e.detail.as_deref().unwrap_or("")),
        ));
    }
    out
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn write_gzipped_jsonl(path: &Path, events: &[LedgerEvent]) -> std::io::Result<()> {
    let f = std::fs::File::create(path)?;
    let mut gz = GzEncoder::new(f, Compression::default());
    for e in events {
        let line = serde_json::to_string(e)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        gz.write_all(line.as_bytes())?;
        gz.write_all(b"\n")?;
    }
    gz.finish()?;
    Ok(())
}

fn parse_iso_date(s: &str) -> Result<u64, String> {
    // Accept YYYY-MM-DD or full RFC-3339.
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.timestamp().max(0) as u64);
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let dt = Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).unwrap_or_default());
        return Ok(dt.timestamp().max(0) as u64);
    }
    Err(format!(
        "Cannot parse '{}' as an ISO-8601 date (try YYYY-MM-DD).",
        s
    ))
}

fn compute_stats(events: &[LedgerEvent], since_ts: Option<u64>) -> LedgerStats {
    let mut stats = LedgerStats::default();
    for e in events {
        if let Some(min) = since_ts {
            if e.timestamp < min {
                continue;
            }
        }
        match e.event_type.as_str() {
            "Installed" | "Install" | "install_completed" | "install_started" => {
                stats.installs += 1
            }
            "Upgraded" | "Upgrade" | "upgrade_completed" => stats.upgrades += 1,
            "Removed" | "Remove" | "remove_completed" => stats.removes += 1,
            "RolledBack" | "Rollback" => stats.rollbacks += 1,
            _ => stats.other += 1,
        }
        stats.total += 1;
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_event(ts: u64, ev: &str, name: &str) -> LedgerEvent {
        LedgerEvent {
            timestamp: ts,
            event_type: ev.into(),
            component: name.into(),
            version: "1.0.0".into(),
            target: "local".into(),
            success: true,
            detail: None,
        }
    }

    fn write_jsonl(path: &Path, events: &[LedgerEvent]) {
        let body = events_to_jsonl(events);
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn stats_counts_correctly() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("ledger.jsonl");
        let events = vec![
            sample_event(100, "Installed", "git"),
            sample_event(200, "Upgraded", "git"),
            sample_event(300, "Removed", "git"),
            sample_event(400, "RolledBack", "git"),
            sample_event(500, "Installed", "node"),
        ];
        write_jsonl(&p, &events);

        let stats = compute_stats(&read_events(&p).unwrap(), None);
        assert_eq!(stats.installs, 2);
        assert_eq!(stats.upgrades, 1);
        assert_eq!(stats.removes, 1);
        assert_eq!(stats.rollbacks, 1);
        assert_eq!(stats.total, 5);
    }

    #[test]
    fn stats_since_filter() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("ledger.jsonl");
        let events = vec![
            sample_event(100, "Installed", "git"),
            sample_event(10_000_000_000, "Installed", "node"),
        ];
        write_jsonl(&p, &events);

        let s = compute_stats(&read_events(&p).unwrap(), Some(1_000_000_000));
        assert_eq!(s.installs, 1);
        assert_eq!(s.total, 1);
    }

    #[test]
    fn export_csv_has_header() {
        let events = vec![
            sample_event(100, "Installed", "git"),
            sample_event(200, "Upgraded", "node"),
        ];
        let csv = events_to_csv(&events);
        let mut lines = csv.lines();
        let header = lines.next().unwrap();
        assert!(header.starts_with("timestamp,event_type,"));
        // 1 header + 2 rows.
        assert_eq!(csv.lines().count(), 3);
    }

    #[test]
    fn export_jsonl_round_trips() {
        let events = vec![sample_event(100, "Installed", "git")];
        let body = events_to_jsonl(&events);
        let parsed: Vec<LedgerEvent> = body
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].component, "git");
    }

    #[test]
    fn parse_iso_date_accepts_yyyy_mm_dd() {
        let ts = parse_iso_date("2026-04-01").unwrap();
        // 2026-04-01 00:00:00 UTC == 1775001600
        assert_eq!(ts, 1_775_001_600);
    }

    #[test]
    fn compact_keeps_last_n_and_archives_rest() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("ledger.jsonl");
        let archive_dir = tmp.path().join("archives");
        let events: Vec<_> = (0..1500)
            .map(|i| sample_event(i as u64, "Installed", &format!("c{}", i)))
            .collect();
        write_jsonl(&p, &events);

        let code = run_compact(CompactArgs {
            keep_last: 1000,
            path_override: Some(p.clone()),
            archive_dir_override: Some(archive_dir.clone()),
            timestamp_override: Some("FIXED".into()),
        });

        assert_eq!(code, EXIT_SUCCESS);

        // Active file: 1000 entries.
        let active = read_events(&p).unwrap();
        assert_eq!(active.len(), 1000);
        // First active entry should be the 501st (0-indexed: 500).
        assert_eq!(active[0].component, "c500");

        // Archive file exists.
        let archive = archive_dir.join("ledger-archive-FIXED.jsonl.gz");
        assert!(archive.exists(), "archive missing at {}", archive.display());

        // Decompress and confirm 500 entries.
        let f = std::fs::File::open(&archive).unwrap();
        let mut decoder = flate2::read::GzDecoder::new(f);
        let mut s = String::new();
        std::io::Read::read_to_string(&mut decoder, &mut s).unwrap();
        let archived_count = s.lines().filter(|l| !l.trim().is_empty()).count();
        assert_eq!(archived_count, 500);
    }
}
