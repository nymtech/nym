use anyhow::{bail, Context};
use nym_common_models::ns_api::{SubmitResults, TestrunAssignment};
use nym_crypto::asymmetric::ed25519::{PrivateKey, PublicKey};
use tracing::instrument;

use crate::cli::GwProbe;

const URL_BASE: &str = "internal/testruns";

pub(crate) async fn run_probe(
    server_address: &str,
    server_port: u16,
    ns_api_auth_key: &str,
    probe_path: &str,
) -> anyhow::Result<()> {
    let server_address = format!("{}:{}", server_address, server_port);
    test_ns_api_conn(&server_address).await?;

    let auth_key = PrivateKey::from_base58_string(ns_api_auth_key)
        .context("Couldn't parse auth key, exiting")?;

    let probe = GwProbe::new(probe_path.to_string());

    let version = probe.version().await;
    tracing::info!("Probe version:\n{}", version);

    if let Some(testrun) = request_testrun(auth_key.public_key(), &server_address).await? {
        let log = probe.run_and_get_log(&Some(testrun.gateway_identity_key));

        submit_results(auth_key, &server_address, testrun.testrun_id, log).await?;
    } else {
        tracing::info!("No testruns available, exiting")
    }

    Ok(())
}

async fn test_ns_api_conn(server_addr: &str) -> anyhow::Result<()> {
    reqwest::get(server_addr)
        .await
        .map(|res| {
            tracing::info!(
                "Testing connection to NS API at {server_addr}: {}",
                res.status()
            );
        })
        .map_err(|err| anyhow::anyhow!("Couldn't connect to server on {}: {}", server_addr, err))
}

#[instrument(level = "debug", skip_all)]
pub(crate) async fn request_testrun(
    auth_key: PublicKey,
    server_addr: &str,
) -> anyhow::Result<Option<TestrunAssignment>> {
    let target_url = format!("{}/{}", server_addr, URL_BASE);
    let client = reqwest::Client::new();
    let res = client
        .get(target_url)
        .body(auth_key.to_base58_string())
        .send()
        .await?;
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

#[instrument(level = "debug", skip(auth_key, probe_outcome))]
pub(crate) async fn submit_results(
    auth_key: PrivateKey,
    server_addr: &str,
    testrun_id: i64,
    probe_outcome: String,
) -> anyhow::Result<()> {
    let target_url = format!("{}/{}/{}", server_addr, URL_BASE, testrun_id);

    let results = sign_message(auth_key, probe_outcome);

    let client = reqwest::Client::new();
    let res = client
        .post(target_url)
        .json(&results)
        .send()
        .await
        .and_then(|response| response.error_for_status())?;

    tracing::debug!("Submitted results: {})", res.status());
    Ok(())
}

fn sign_message(key: PrivateKey, probe_outcome: String) -> SubmitResults {
    let signature = key.sign(&probe_outcome);

    SubmitResults {
        message: probe_outcome,
        signature,
    }
}
