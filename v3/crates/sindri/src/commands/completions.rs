//! Shell completions generation

use crate::cli::{Cli, CompletionsArgs};
use anyhow::Result;
use clap::CommandFactory;
use clap_complete::generate;
use std::io;

pub fn run(args: CompletionsArgs) -> Result<()> {
    let mut cmd = Cli::command();
    generate(args.shell, &mut cmd, "sindri", &mut io::stdout());
    Ok(())
}
