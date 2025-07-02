use crate::probe::GwProbe;
use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use std::sync::OnceLock;

pub(crate) mod generate_keypair;
pub(crate) mod run_probe;

// Helper for passing LONG_VERSION to clap
fn pretty_build_info_static() -> &'static str {
    static PRETTY_BUILD_INFORMATION: OnceLock<String> = OnceLock::new();
    PRETTY_BUILD_INFORMATION.get_or_init(|| bin_info!().pretty_print())
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
        #[arg(short, long, env = "NODE_STATUS_AGENT_SERVER_ADDRESS")]
        server_address: String,

        #[arg(short = 'p', long, env = "NODE_STATUS_AGENT_SERVER_PORT")]
        server_port: u16,

        /// base58-encoded private key
        #[arg(long, env = "NODE_STATUS_AGENT_AUTH_KEY")]
        ns_api_auth_key: String,

        /// path of binary to run
        #[arg(long, env = "NODE_STATUS_AGENT_PROBE_PATH")]
        probe_path: String,

        /// mnemonic for acquiring zk-nyms
        #[arg(long, env = "NODE_STATUS_AGENT_PROBE_MNEMONIC")]
        mnemonic: String,

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
                server_address,
                server_port,
                ns_api_auth_key,
                probe_path,
                mnemonic,
                probe_extra_args,
            } => run_probe::run_probe(
                server_address,
                server_port.to_owned(),
                ns_api_auth_key,
                probe_path,
                mnemonic,
                probe_extra_args,
            )
            .await
            .inspect_err(|err| {
                tracing::error!("{err}");
            })?,
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
