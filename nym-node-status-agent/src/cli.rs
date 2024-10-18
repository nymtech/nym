use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use std::sync::OnceLock;
use tracing::instrument;

use crate::probe::GwProbe;

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
    #[arg(short, long, env = "NODE_STATUS_AGENT_SERVER_ADDRESS")]
    pub(crate) server_address: String,

    #[arg(short = 'p', long, env = "NODE_STATUS_AGENT_SERVER_PORT")]
    pub(crate) server_port: u16,
    // TODO dz accept keypair for identification / auth
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    RunProbe {
        /// path of binary to run
        #[arg(long, env = "NODE_STATUS_AGENT_PROBE_PATH")]
        probe_path: String,
        #[arg(short, long, env = "NODE_STATUS_AGENT_GATEWAY_ID")]
        gateway_id: Option<String>,
    },
}

impl Args {
    pub(crate) async fn execute(&self) -> anyhow::Result<()> {
        match &self.command {
            Command::RunProbe {
                probe_path,
                gateway_id,
            } => self.run_probe(probe_path, gateway_id).await?,
        }

        Ok(())
    }

    async fn run_probe(&self, probe_path: &str, gateway_id: &Option<String>) -> anyhow::Result<()> {
        let probe = GwProbe::new(probe_path.to_string());

        let version = probe.print_version().await;
        tracing::info!("Probe version:\n{}", version);

        // TODO dz register to get a task with NSAPI

        let log = probe.run_and_get_log(gateway_id);

        let server_address = format!("{}:{}", &self.server_address, self.server_port);
        submit_results(&server_address, log).await?;

        Ok(())
    }
}

#[instrument(level = "debug", skip(probe_outcome))]
async fn submit_results(server_addr: &str, probe_outcome: String) -> anyhow::Result<()> {
    let target_url = format!("{}/internal/testruns", server_addr);
    let client = reqwest::Client::new();
    let res = client
        .post(target_url)
        .body(probe_outcome)
        .send()
        .await
        .and_then(|response| response.error_for_status())?;

    tracing::debug!("Submitted results: {})", res.status());
    Ok(())
}
