pub mod cli;
pub mod commands;
use std::process::ExitCode;

use clap::Parser;

#[tokio::main]
async fn main() -> ExitCode {
    cli::Cli::parse().run().await
}
