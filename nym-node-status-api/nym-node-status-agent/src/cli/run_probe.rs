use crate::cli::{GwProbe, ServerConfig};
use crate::log_capture::LogCapture;
use nym_gateway_probe::config::{CredentialArgs, NetstackArgs, Socks5Args};
use nym_gateway_probe::types::{AttachedTicketMaterials, VersionedSerialise};
use nym_sdk::mixnet::ed25519::PublicKey;
use tracing::instrument;

pub(crate) async fn run_probe(
    servers: &[ServerConfig],
    probe_path: &str,
    probe_extra_args: &Vec<String>,
    log_capture: LogCapture,
) -> anyhow::Result<()> {
    // TODO dz how do we pass probe_extra_args now that we don't invoke probe as a CLI tool anymore?
    if servers.is_empty() {
        anyhow::bail!("No servers configured");
    }

    let probe = GwProbe::new(probe_path.to_string());

    let version = probe.version().await;
    tracing::info!("Probe version:\n{}", version);

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

    tracing::info!("Received testrun {testrun_id} for gateway {gateway_identity_key} from primary",);

    // TODO dz prettify this
    // ===== NEW PART: USING GW PROBE AS A LIB ======

    // TODO dz all of this preparation should already be a part of the function, either on gw probe side or here as a separate function
    let network = nym_sdk::NymNetworkDetails::new_from_env();
    let api_url = network
        .endpoints
        .first()
        .and_then(|ep| ep.api_url())
        .ok_or(anyhow::anyhow!("missing api url"))?;

    let directory = nym_gateway_probe::NymApiDirectory::new(api_url).await?;
    let gateway_identity_pubkey =
        PublicKey::from_base58_string(gateway_identity_key.clone()).unwrap();
    let entry_details = directory
        .entry_gateway(&gateway_identity_pubkey)?
        .to_testable_node()?;

    // TODO dz constructing ad hoc: in GW probe, this came as CLI arguments
    let probe_config = default_probe_config();
    let gw_probe = nym_gateway_probe::Probe::new(entry_details, None, network, probe_config);

    let serialized_ticket_materials = testrun.ticket_materials.to_serialised_string();
    let credentials_args = CredentialArgs {
        ticket_materials: serialized_ticket_materials,
        ticket_materials_revision:
            <AttachedTicketMaterials as VersionedSerialise>::CURRENT_SERIALISATION_REVISION,
    };

    // Run the probe, capturing all tracing output
    log_capture.start();
    let probe_result = Box::pin(gw_probe.probe_run_agent(credentials_args))
        .await
        .unwrap();
    let probe_log = log_capture.stop_and_drain();

    // Run the probe
    // let log = probe.run_and_get_log(
    //     gateway_identity_key.clone(),
    //     probe_extra_args,
    //     testrun.ticket_materials,
    // );

    // Inspect the probe output for socks5 field
    match probe_result.outcome.socks5.as_ref() {
        Some(socks5) => tracing::info!("🌐 socks5 field present: {:#?}", socks5),
        None => tracing::warn!("🌐⚠️ socks5 field is MISSING from probe output"),
    }
    // Extract JSON from log output (probe outputs logs followed by JSON)
    // TODO dz this should be part of parsed probeOutput so parsing the logs no longer necessary
    // extract to its own function
    // let json_str = extract_json_from_log(&log);
    // if json_str.is_empty() {
    //     tracing::warn!("Failed to extract JSON from probe output");
    // } else {
    //     match serde_json::from_str::<serde_json::Value>(&json_str) {
    //         Ok(json) => {
    //             if let Some(outcome) = json.get("outcome") {
    //                 match outcome.get("socks5") {
    //                     Some(socks5) if socks5.is_null() => {
    //                         tracing::warn!("🌐⚠️ socks5 field is NULL in probe output");
    //                     }
    //                     Some(socks5) => {
    //                         tracing::info!("🌐 socks5 field present: {}", socks5);
    //                     }
    //                     None => {
    //                         tracing::warn!("🌐⚠️ socks5 field is MISSING from probe output");
    //                     }
    //                 }
    //             } else {
    //                 tracing::warn!("🌐⚠️ outcome field is MISSING from probe output");
    //             }
    //         }
    //         Err(e) => {
    //             tracing::error!("Failed to parse probe output as JSON: {e}");
    //         }
    //     }
    // }

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

// TODO dz test with these values because they're based on CLI provided
// best practices, then delete this function and use Default impl
fn default_probe_config() -> nym_gateway_probe::config::ProbeConfig {
    nym_gateway_probe::config::ProbeConfig {
        min_gateway_mixnet_performance: None,
        test_mode: nym_gateway_probe::config::TestMode::All,
        ignore_egress_epoch_role: false,
        amnezia_args: None,
        netstack_args: NetstackArgs {
            // CLI defined overrides
            netstack_download_timeout_sec: 30,
            netstack_num_ping: 2,
            netstack_send_timeout_sec: 1,
            netstack_recv_timeout_sec: 1,
            ..Default::default()
        },
        socks5_args: Socks5Args::default(),
    }
}
