use crate::cli::GwProbe;
use anyhow::Context;
use nym_crypto::asymmetric::ed25519::PrivateKey;

pub(crate) async fn run_probe(
    server_ip: &str,
    server_port: u16,
    ns_api_auth_key: &str,
    probe_path: &str,
    probe_extra_args: &Vec<String>,
) -> anyhow::Result<()> {
    let auth_key = PrivateKey::from_base58_string(ns_api_auth_key)
        .context("Couldn't parse auth key, exiting")?;

    let ns_api_client = nym_node_status_client::NsApiClient::new(server_ip, server_port, auth_key);

    let probe = GwProbe::new(probe_path.to_string());

    let version = probe.version().await;
    tracing::info!("Probe version:\n{}", version);

    if let Some(testrun) = ns_api_client.request_testrun().await? {
        let log = probe.run_and_get_log(&Some(testrun.gateway_identity_key), probe_extra_args);

        ns_api_client
            .submit_results(testrun.testrun_id, log, testrun.assigned_at_utc)
            .await?;
    } else {
        tracing::info!("No testruns available, exiting")
    }

    Ok(())
}
