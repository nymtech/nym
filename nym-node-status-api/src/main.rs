use anyhow::anyhow;
use clap::Parser;
use nym_network_defaults::setup_env;

mod cli;
mod db;
mod logging;
mod monitor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::setup_tracing_logger();

    let args = cli::Cli::parse();
    // if dotenv file is present, load its values
    // otherwise, default to mainnet
    setup_env(args.config_env_file.as_ref());
    tracing::debug!("{:?}", std::env::var("NETWORK_NAME"));
    tracing::debug!("{:?}", std::env::var("EXPLORER_API"));
    tracing::debug!("{:?}", std::env::var("NYM_API"));

    let storage = db::Storage::init().await?;
    monitor::spawn_in_background(storage)
        .await
        .expect("Monitor task failed");
    tracing::info!("Started server");

    Ok(())
}

pub(crate) fn read_env_var(env_var: &str) -> anyhow::Result<String> {
    std::env::var(env_var)
        .map(|value| {
            tracing::trace!("{}={}", env_var, value);
            value
        })
        .map_err(|_| anyhow!("You need to set {}", env_var))
}
