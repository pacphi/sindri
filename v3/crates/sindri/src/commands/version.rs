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
