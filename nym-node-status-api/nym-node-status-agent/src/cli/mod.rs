use crate::log_capture::LogCapture;
use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use nym_crypto::asymmetric::ed25519::PrivateKey;
use std::{env, sync::OnceLock};

pub(crate) mod generate_keypair;
pub(crate) mod run_probe;

#[derive(Debug)]
pub(crate) struct ServerConfig {
    pub(crate) address: String,
    pub(crate) port: u16,
    pub(crate) auth_key: PrivateKey,
}

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
}

fn parse_server_config(s: &str) -> Result<ServerConfig, String> {
    let parts: Vec<&str> = s.split('|').collect();
    if parts.len() != 2 {
        return Err("Server config must be in format 'address|port'".to_string());
    }

    let address = parts[0].to_string();
    let port = parts[1]
        .parse::<u16>()
        .map_err(|_| "Invalid port number".to_string())?;
    let auth_key =
        PrivateKey::from_base58_string(env::var("NODE_STATUS_AGENT_AUTH_KEY").unwrap()).unwrap();

    Ok(ServerConfig {
        address,
        port,
        auth_key,
    })
}

#[derive(Parser, Debug)]
#[clap(author = "Nymtech", version, long_version = pretty_build_info_static(), about)]
pub(crate) struct Args {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[allow(clippy::large_enum_variant)]
#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    RunProbe(RunProbeArgs),

    GenerateKeypair {
        #[arg(long)]
        path: Option<String>,
    },
}

#[derive(clap::Args, Debug)]
pub(crate) struct RunProbeArgs {
    /// Server configurations in format "address|port"
    /// Can be specified multiple times for multiple servers
    #[arg(short, long, required = true)]
    pub server: Vec<String>,

    /// Probe configuration overrides (netstack, socks5, etc.)
    /// Can also be set via PROBE_* environment variables.
    #[command(flatten)]
    pub probe_config: nym_gateway_probe::config::ProbeConfig,
}

impl Args {
    pub(crate) async fn execute(self, log_capture: LogCapture) -> anyhow::Result<()> {
        match self.command {
            Command::RunProbe(args) => {
                // Parse server configs
                let mut servers = Vec::new();
                for s in &args.server {
                    match parse_server_config(s) {
                        Ok(config) => servers.push(config),
                        Err(e) => {
                            tracing::error!("Invalid server config '{}': {}", s, e);
                            anyhow::bail!("Invalid server config '{}': {}", s, e);
                        }
                    }
                }

                run_probe::run_probe(&servers, args.probe_config, log_capture)
                    .await
                    .inspect_err(|err| {
                        tracing::error!("{err}");
                    })?
            }
            Command::GenerateKeypair { path } => {
                let path = path
                    .to_owned()
                    .unwrap_or_else(|| String::from("private-key"));
                generate_keypair::generate_key_pair(path)?
            }
        }

        Ok(())
    }
}
