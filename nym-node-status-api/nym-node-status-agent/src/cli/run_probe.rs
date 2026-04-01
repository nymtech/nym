use crate::cli::ServerConfig;
use crate::log_capture::LogCapture;
use anyhow::anyhow;
use nym_gateway_probe::config::CredentialArgs;
use nym_gateway_probe::types::{AttachedTicketMaterials, VersionedSerialise};
use nym_sdk::mixnet::ed25519::PublicKey;
use tracing::instrument;

pub(crate) async fn run_probe(
    servers: &[ServerConfig],
    probe_config: nym_gateway_probe::config::ProbeConfig,
    log_capture: LogCapture,
) -> anyhow::Result<()> {
    if servers.is_empty() {
        anyhow::bail!("No servers configured");
    }

    // Always use first server as primary
    let primary_server = &servers[0];
    tracing::info!(
        "Requesting testrun from primary server: {}:{}",
        primary_server.address,
        primary_server.port
    );

    let auth_key = nym_crypto::asymmetric::ed25519::PrivateKey::from_bytes(
        &primary_server.auth_key.to_bytes(),
    )
    .expect("Failed to clone auth key");
    let ns_api_client = nym_node_status_client::NsApiClient::new(
        &primary_server.address,
        primary_server.port,
        auth_key,
    );

    let testrun = match ns_api_client.request_testrun().await {
        Ok(Some(testrun)) => testrun,
        Ok(None) => {
            tracing::info!("No testruns available from primary server");
            return Ok(());
        }
        Err(err) => {
            tracing::error!("Failed to contact primary server: {err}");
            return Err(err);
        }
    };

    let testrun_id = testrun.assignment.testrun_id;
    let testrun_assigned_at = testrun.assignment.assigned_at_utc;
    let gateway_identity_key = testrun.assignment.gateway_identity_key.clone();
    let gateway_identity_pubkey = PublicKey::from_base58_string(gateway_identity_key.clone())
        .map_err(|e| anyhow!("Failed to parse GW identity key: {e}"))?;

    tracing::info!("Received testrun {testrun_id} for gateway {gateway_identity_key} from primary",);

    let network = nym_sdk::NymNetworkDetails::new_from_env();
    let probe =
        nym_gateway_probe::Probe::new_for_agent(gateway_identity_pubkey, network, probe_config)
            .await?;

    // probe constructor might modify config to suit the testing mode, so log afterwards
    tracing::info!("Using probe config::\n{:#?}", &probe.config());

    let serialized_ticket_materials = testrun.ticket_materials.to_serialised_string();
    let credentials_args = CredentialArgs {
        ticket_materials: serialized_ticket_materials,
        ticket_materials_revision:
            <AttachedTicketMaterials as VersionedSerialise>::CURRENT_SERIALISATION_REVISION,
    };

    // Run the probe, capturing all tracing output
    log_capture.start();
    let probe_result = Box::pin(probe.probe_run_agent(credentials_args))
        .await
        .unwrap();
    let probe_log = log_capture.stop_and_drain();

    // Inspect the probe output for socks5 field
    match probe_result.outcome.socks5.as_ref() {
        Some(socks5) => tracing::info!("🌐 socks5 field present: {:#?}", socks5),
        None => tracing::warn!("🌐⚠️ socks5 field is MISSING from probe output"),
    }

    submit_results_to_servers(
        servers,
        testrun_id,
        testrun_assigned_at,
        &gateway_identity_key,
        probe_result,
        probe_log,
    )
    .await;

    Ok(())
}

#[instrument(level = "info", skip_all, fields(gateway_id = %gateway_identity_key, testrun = testrun_id))]
async fn submit_results_to_servers(
    servers: &[ServerConfig],
    testrun_id: i32,
    testrun_assigned_at: i64,
    gateway_identity_key: &str,
    probe_result: nym_gateway_probe::ProbeResult,
    probe_log: String,
) {
    let handles = servers
        .iter()
        .enumerate()
        .map(|(idx, server)| {
            let probe_result = probe_result.clone();
            let probe_log = probe_log.clone();
            let gateway_identity_key = gateway_identity_key.to_string();

            async move {
                let auth_key = nym_crypto::asymmetric::ed25519::PrivateKey::from_bytes(
                    &server.auth_key.to_bytes(),
                )
                .expect("Failed to clone auth key");
                let client = nym_node_status_client::NsApiClient::new(
                    &server.address,
                    server.port,
                    auth_key,
                );

                let result = if idx == 0 {
                    // Primary server: submit regular results without context
                    client
                        .submit_results(
                            testrun_id as i64,
                            probe_result,
                            probe_log,
                            testrun_assigned_at,
                        )
                        .await
                } else {
                    // Other servers: submit results with context
                    client
                        .submit_results_with_context(
                            testrun_id,
                            probe_log,
                            testrun_assigned_at,
                            gateway_identity_key,
                        )
                        .await
                };

                (idx, server.address.clone(), server.port, result)
            }
        })
        .collect::<Vec<_>>();

    let results = futures::future::join_all(handles).await;

    for (index, server_address, server_port, result) in results {
        let method = if index == 0 {
            "regular"
        } else {
            "with context"
        };
        match result {
            Ok(()) => {
                tracing::info!(
                    "✅ Successfully submitted {method} to server[{index}] {server_address}:{server_port}",
                );
            }
            Err(e) => {
                tracing::warn!(
                    "❌ Failed to submit {method} to server[{index}] {server_address}:{server_port} - {e}"
                );
            }
        }
    }
}
