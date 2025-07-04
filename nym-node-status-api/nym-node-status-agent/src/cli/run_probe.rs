use crate::cli::{GwProbe, ServerConfig};
use anyhow::Context;
use rand::seq::SliceRandom;

pub(crate) async fn run_probe(
    servers: &[ServerConfig],
    probe_path: &str,
    probe_extra_args: &Vec<String>,
) -> anyhow::Result<()> {
    if servers.is_empty() {
        anyhow::bail!("No servers configured");
    }

    let probe = GwProbe::new(probe_path.to_string());

    let version = probe.version().await;
    tracing::info!("Probe version:\n{}", version);

    // Create indices and shuffle them for random selection
    let mut indices: Vec<usize> = (0..servers.len()).collect();
    indices.shuffle(&mut rand::thread_rng());

    for idx in indices {
        let server = &servers[idx];
        tracing::info!("Trying server: {}:{}", server.address, server.port);

        // Clone the auth key by converting to/from bytes
        let auth_key =
            nym_crypto::asymmetric::ed25519::PrivateKey::from_bytes(&server.auth_key.to_bytes())
                .expect("Failed to clone auth key");
        let ns_api_client =
            nym_node_status_client::NsApiClient::new(&server.address, server.port, auth_key);

        match ns_api_client.request_testrun().await {
            Ok(Some(testrun)) => {
                tracing::info!(
                    "Received testrun {} from {}:{}",
                    testrun.testrun_id,
                    server.address,
                    server.port
                );

                let log =
                    probe.run_and_get_log(&Some(testrun.gateway_identity_key), probe_extra_args);

                ns_api_client
                    .submit_results(testrun.testrun_id, log, testrun.assigned_at_utc)
                    .await
                    .context("Failed to submit results")?;

                tracing::info!(
                    "Successfully submitted results to {}:{}",
                    server.address,
                    server.port
                );
                return Ok(());
            }
            Ok(None) => {
                tracing::info!(
                    "No testruns available from {}:{}",
                    server.address,
                    server.port
                );
                continue;
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to contact {}:{} - {}",
                    server.address,
                    server.port,
                    e
                );
                continue;
            }
        }
    }

    tracing::info!("No testruns available from any API");
    Ok(())
}
