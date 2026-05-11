use crate::cli::ServerConfig;
use crate::cli::common;
use crate::log_capture::LogCapture;
use tracing::instrument;

pub(crate) async fn run_probe(
    servers: &[ServerConfig],
    probe_config: nym_gateway_probe::config::ProbeConfig,
    log_capture: LogCapture,
) -> anyhow::Result<()> {
    let primary_server = common::primary(servers)?;
    tracing::info!(
        "Requesting testrun from primary server: {}:{}",
        primary_server.address,
        primary_server.port
    );

    let ns_api_client = common::build_client(primary_server);

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
    let gateway_identity_pubkey = common::parse_gateway_pubkey(&gateway_identity_key)?;

    tracing::info!("Received testrun {testrun_id} for gateway {gateway_identity_key} from primary",);

    let network = nym_sdk::NymNetworkDetails::new_from_env();
    let probe =
        nym_gateway_probe::Probe::new_for_agent(gateway_identity_pubkey, network, probe_config)
            .await?;

    // probe constructor might modify config to suit the testing mode, so log afterwards
    tracing::info!("Using probe config:\n{:#?}", &probe.config());

    let credentials_args = common::credential_args_from(testrun.ticket_materials);

    // Run the probe, capturing all tracing output
    log_capture.start();
    let probe_result_res = Box::pin(probe.probe_run_agent(credentials_args)).await;
    let probe_log = log_capture.stop_and_drain();
    let probe_result = probe_result_res?;

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
                let client = common::build_client(server);

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
                            probe_result,
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
