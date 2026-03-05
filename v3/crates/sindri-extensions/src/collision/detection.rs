//! Version marker detection for collision handling

use sindri_core::types::{DetectionMethod, VersionMarker};
use std::path::Path;

/// A detected version from evaluating workspace markers
#[derive(Debug, Clone)]
pub struct DetectedVersion {
    /// The marker path that matched
    pub marker_path: String,
    /// The version label from the marker definition
    pub version_label: String,
}

/// Evaluates version markers against the workspace filesystem
pub struct VersionDetector;

impl VersionDetector {
    /// Evaluate version markers against workspace.
    /// Checks FileExists, DirectoryExists, ContentMatch with match_any and exclude_if.
    pub fn detect(workspace: &Path, markers: &[VersionMarker]) -> Vec<DetectedVersion> {
        let mut detected = Vec::new();

        for marker in markers {
            let target = workspace.join(&marker.path);

            // Check exclude_if first — if any exclude path exists, skip this marker
            let excluded = marker
                .detection
                .exclude_if
                .iter()
                .any(|exc| workspace.join(exc).exists());

            if excluded {
                continue;
            }

            let matched = match marker.detection.method {
                DetectionMethod::FileExists => target.is_file(),
                DetectionMethod::DirectoryExists => target.is_dir(),
                DetectionMethod::ContentMatch => {
                    if !target.is_file() {
                        false
                    } else {
                        match std::fs::read_to_string(&target) {
                            Ok(content) => {
                                if marker.detection.match_any {
                                    marker
                                        .detection
                                        .patterns
                                        .iter()
                                        .any(|p| content.contains(p))
                                } else {
                                    marker
                                        .detection
                                        .patterns
                                        .iter()
                                        .all(|p| content.contains(p))
                                }
                            }
                            Err(_) => false,
                        }
                    }
                }
            };

            if matched {
                detected.push(DetectedVersion {
                    marker_path: marker.path.clone(),
                    version_label: marker.version.clone(),
                });
            }
        }

        detected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sindri_core::types::{StateMarkerType, VersionDetection};
    use tempfile::TempDir;

    fn make_marker(
        path: &str,
        version: &str,
        method: DetectionMethod,
        patterns: Vec<&str>,
        match_any: bool,
        exclude_if: Vec<&str>,
    ) -> VersionMarker {
        VersionMarker {
            path: path.to_string(),
            r#type: StateMarkerType::File,
            version: version.to_string(),
            detection: VersionDetection {
                method,
                patterns: patterns.into_iter().map(|s| s.to_string()).collect(),
                match_any,
                exclude_if: exclude_if.into_iter().map(|s| s.to_string()).collect(),
            },
        }
    }

    #[test]
    fn test_file_exists_positive() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("marker.txt"), "exists").unwrap();

        let markers = vec![make_marker(
            "marker.txt",
            "v1",
            DetectionMethod::FileExists,
            vec![],
            false,
            vec![],
        )];
        let detected = VersionDetector::detect(tmp.path(), &markers);
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].version_label, "v1");
    }

    #[test]
    fn test_file_exists_negative() {
        let tmp = TempDir::new().unwrap();
        let markers = vec![make_marker(
            "missing.txt",
            "v1",
            DetectionMethod::FileExists,
            vec![],
            false,
            vec![],
        )];
        let detected = VersionDetector::detect(tmp.path(), &markers);
        assert!(detected.is_empty());
    }

    #[test]
    fn test_directory_exists_positive() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join(".agentic-qe")).unwrap();

        let markers = vec![make_marker(
            ".agentic-qe",
            "agentic-qe",
            DetectionMethod::DirectoryExists,
            vec![],
            false,
            vec![],
        )];
        let detected = VersionDetector::detect(tmp.path(), &markers);
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].version_label, "agentic-qe");
    }

    #[test]
    fn test_directory_exists_negative() {
        let tmp = TempDir::new().unwrap();
        let markers = vec![make_marker(
            ".missing",
            "v1",
            DetectionMethod::DirectoryExists,
            vec![],
            false,
            vec![],
        )];
        let detected = VersionDetector::detect(tmp.path(), &markers);
        assert!(detected.is_empty());
    }

    #[test]
    fn test_content_match_single_pattern() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("config.json"),
            r#"{"swarm": true, "sona": false}"#,
        )
        .unwrap();

        let markers = vec![make_marker(
            "config.json",
            "ruflo",
            DetectionMethod::ContentMatch,
            vec![r#""swarm""#],
            false,
            vec![],
        )];
        let detected = VersionDetector::detect(tmp.path(), &markers);
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].version_label, "ruflo");
    }

    #[test]
    fn test_content_match_any() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("config.json"), r#"{"sona": true}"#).unwrap();

        let markers = vec![make_marker(
            "config.json",
            "ruflo",
            DetectionMethod::ContentMatch,
            vec![r#""swarm""#, r#""sona""#],
            true, // match_any
            vec![],
        )];
        let detected = VersionDetector::detect(tmp.path(), &markers);
        assert_eq!(detected.len(), 1);
    }

    #[test]
    fn test_content_match_all() {
        let tmp = TempDir::new().unwrap();
        // Only one pattern present, match_any=false requires all
        std::fs::write(tmp.path().join("config.json"), r#"{"sona": true}"#).unwrap();

        let markers = vec![make_marker(
            "config.json",
            "ruflo",
            DetectionMethod::ContentMatch,
            vec![r#""swarm""#, r#""sona""#],
            false, // match all
            vec![],
        )];
        let detected = VersionDetector::detect(tmp.path(), &markers);
        assert!(detected.is_empty());
    }

    #[test]
    fn test_exclude_if_suppresses() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join(".claude")).unwrap();
        std::fs::create_dir(tmp.path().join(".agentic-qe")).unwrap();

        let markers = vec![make_marker(
            ".claude",
            "unknown",
            DetectionMethod::DirectoryExists,
            vec![],
            false,
            vec![".agentic-qe"], // exclude if agentic-qe present
        )];
        let detected = VersionDetector::detect(tmp.path(), &markers);
        assert!(detected.is_empty());
    }
}
