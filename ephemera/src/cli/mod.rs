use crate::cli::crypto::GenerateKeypairCmd;
use clap::Parser;

pub mod config;
mod crypto;
pub mod init;
pub mod peers;
pub mod run_node;

pub const PEERS_CONFIG_FILE: &str = "peers.toml";

#[derive(Parser)]
#[command()]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(clap::Subcommand)]
pub enum Subcommand {
    InitConfig(init::Cmd),
    InitLocalPeersConfig(peers::CreateLocalPeersConfiguration),
    RunNode(run_node::RunExternalNodeCmd),
    GenerateKeypair(crypto::GenerateKeypairCmd),
    UpdateConfig(config::UpdateConfigCmd),
}

impl Cli {
    /// # Errors
    /// Returns an error if the subcommand fails.
    pub async fn execute(self) -> anyhow::Result<()> {
        match self.subcommand {
            Subcommand::InitConfig(init) => {
                init.execute(None);
            }
            Subcommand::InitLocalPeersConfig(add_local_peers) => {
                add_local_peers.execute();
            }
            Subcommand::RunNode(run_node) => run_node.execute().await?,
            Subcommand::GenerateKeypair(_) => {
                GenerateKeypairCmd::execute();
            }
            Subcommand::UpdateConfig(update_config) => {
                update_config.execute();
            }
        }
        Ok(())
    }
}
