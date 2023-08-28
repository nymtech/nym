use clap::Parser;

use ephemera::cli::Cli;
use ephemera::logging;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init();

    Cli::parse().execute().await?;
    Ok(())
}
