use crate::probe::GwProbe;
use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use nym_crypto::asymmetric::ed25519::PrivateKey;
use std::sync::OnceLock;

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
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return Err("Server config must be in format 'address:port:auth_key'".to_string());
    }

    let address = parts[0].to_string();
    let port = parts[1]
        .parse::<u16>()
        .map_err(|_| "Invalid port number".to_string())?;
    let auth_key =
        PrivateKey::from_base58_string(parts[2]).map_err(|_| "Invalid auth key".to_string())?;

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

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    RunProbe {
        /// Server configurations in format "address:port:auth_key"
        /// Can be specified multiple times for multiple servers
        #[arg(short, long, required = true)]
        server: Vec<String>,

        /// path of binary to run
        #[arg(long, env = "NODE_STATUS_AGENT_PROBE_PATH")]
        probe_path: String,

        #[arg(
            long,
            env = "NODE_STATUS_AGENT_PROBE_EXTRA_ARGS",
            value_delimiter = ','
        )]
        probe_extra_args: Vec<String>,
    },

    GenerateKeypair {
        #[arg(long)]
        path: Option<String>,
    },
}

impl Args {
    pub(crate) async fn execute(&self) -> anyhow::Result<()> {
        match &self.command {
            Command::RunProbe {
                server,
                probe_path,
                probe_extra_args,
            } => {
                // Parse server configs
                let mut servers = Vec::new();
                for s in server {
                    match parse_server_config(s) {
                        Ok(config) => servers.push(config),
                        Err(e) => {
                            tracing::error!("Invalid server config '{}': {}", s, e);
                            anyhow::bail!("Invalid server config '{}': {}", s, e);
                        }
                    }
                }

                run_probe::run_probe(&servers, probe_path, probe_extra_args)
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
