//! Build script for version info

fn main() {
    // Set build timestamp
    let now = chrono::Utc::now();
    println!("cargo:rustc-env=BUILD_DATE={}", now.format("%Y-%m-%d"));
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", now.to_rfc3339());

    // Try to get git info
    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
    {
        if output.status.success() {
            let sha = String::from_utf8_lossy(&output.stdout);
            println!("cargo:rustc-env=GIT_SHA={}", sha.trim());
        }
    }

    // Rerun if git changes
    println!("cargo:rerun-if-changed=.git/HEAD");
}
