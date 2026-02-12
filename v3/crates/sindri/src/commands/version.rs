//! Version command

use crate::cli::VersionArgs;
use crate::version::VersionInfo;
use anyhow::Result;

pub fn run(args: VersionArgs) -> Result<()> {
    let info = VersionInfo::current();

    if args.json {
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("{}", info.display());

        // Additional build info
        if let Some(commit) = &info.commit {
            println!("Commit:     {}", commit);
        }
        if let Some(date) = &info.build_date {
            println!("Build date: {}", date);
        }
        if let Some(target) = &info.target {
            println!("Target:     {}", target);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info_current_returns_non_empty_version() {
        let info = VersionInfo::current();
        assert!(
            !info.version.is_empty(),
            "version string should not be empty"
        );
    }

    #[test]
    fn test_version_info_current_is_valid_semver() {
        let info = VersionInfo::current();
        let parsed = semver::Version::parse(&info.version);
        assert!(
            parsed.is_ok(),
            "version should be valid semver, got: {}",
            info.version
        );
    }

    #[test]
    fn test_version_info_display_contains_version() {
        let info = VersionInfo::current();
        let display = info.display();
        assert!(
            display.contains(&info.version),
            "display should contain the version string"
        );
        assert!(
            display.starts_with("sindri "),
            "display should start with 'sindri '"
        );
    }

    #[test]
    fn test_version_info_display_trait() {
        let info = VersionInfo::current();
        let display_str = format!("{}", info);
        assert_eq!(display_str, info.display());
    }

    #[test]
    fn test_version_info_json_serialization() {
        let info = VersionInfo::current();
        let json = serde_json::to_string(&info).expect("should serialize to JSON");
        assert!(json.contains(&info.version));

        let deserialized: VersionInfo =
            serde_json::from_str(&json).expect("should deserialize from JSON");
        assert_eq!(deserialized.version, info.version);
    }

    #[test]
    fn test_version_info_display_with_all_fields() {
        let info = VersionInfo {
            version: "1.2.3".to_string(),
            commit: Some("abc1234".to_string()),
            build_date: Some("2026-01-01".to_string()),
            target: Some("x86_64-unknown-linux-gnu".to_string()),
        };
        let display = info.display();
        assert!(display.contains("sindri 1.2.3"));
        assert!(display.contains("(abc1234)"));
        assert!(display.contains("x86_64-unknown-linux-gnu"));
    }

    #[test]
    fn test_version_info_display_without_optional_fields() {
        let info = VersionInfo {
            version: "0.1.0".to_string(),
            commit: None,
            build_date: None,
            target: None,
        };
        assert_eq!(info.display(), "sindri 0.1.0");
    }
}
