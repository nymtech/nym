use anyhow::bail;
use clap::{Parser, Subcommand};
use nym_bin_common::bin_info;
use nym_common_models::ns_api::TestrunAssignment;
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
    },
}

impl Args {
    pub(crate) async fn execute(&self) -> anyhow::Result<()> {
        match &self.command {
            Command::RunProbe { probe_path } => self.run_probe(probe_path).await?,
        }

        Ok(())
    }

    async fn run_probe(&self, probe_path: &str) -> anyhow::Result<()> {
        let server_address = format!("{}:{}", &self.server_address, self.server_port);

        let probe = GwProbe::new(probe_path.to_string());

        let version = probe.version().await;
        tracing::info!("Probe version:\n{}", version);

        if let Some(testrun) = request_testrun(&server_address).await? {
            let log = probe.run_and_get_log(&Some(testrun.gateway_identity_key));

            submit_results(&server_address, testrun.testrun_id, log).await?;
        } else {
            tracing::info!("No testruns available, exiting")
        }

        Ok(())
    }
}

const URL_BASE: &str = "internal/testruns";

#[instrument(level = "debug", skip_all)]
async fn request_testrun(server_addr: &str) -> anyhow::Result<Option<TestrunAssignment>> {
    let target_url = format!("{}/{}", server_addr, URL_BASE);
    let client = reqwest::Client::new();
    let res = client.get(target_url).send().await?;
    let status = res.status();
    let response_text = res.text().await?;

    if status.is_client_error() {
        bail!("{}: {}", status, response_text);
    } else if status.is_server_error() {
        if matches!(status, reqwest::StatusCode::SERVICE_UNAVAILABLE)
            && response_text.contains("No testruns available")
        {
            return Ok(None);
        } else {
            bail!("{}: {}", status, response_text);
        }
    }

    serde_json::from_str(&response_text)
        .map(|testrun| {
            tracing::info!("Received testrun assignment: {:?}", testrun);
            testrun
        })
        .map_err(|err| {
            tracing::error!("err");
            err.into()
        })
}

#[instrument(level = "debug", skip(probe_outcome))]
async fn submit_results(
    server_addr: &str,
    testrun_id: i64,
    probe_outcome: String,
) -> anyhow::Result<()> {
    let target_url = format!("{}/{}/{}", server_addr, URL_BASE, testrun_id);
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
