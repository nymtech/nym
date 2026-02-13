use crate::cli::ServerConfig;
use crate::cli::common;
use crate::log_capture::LogCapture;
use nym_gateway_probe::RunPortsConfig;
use tracing::instrument;

pub(crate) async fn run_ports_check(
    servers: &[ServerConfig],
    min_gateway_mixnet_performance: Option<u8>,
    ignore_egress_epoch_role: bool,
    mut netstack_args: nym_gateway_probe::config::NetstackArgs,
    log_capture: LogCapture,
) -> anyhow::Result<()> {
    let primary_server = common::primary(servers)?;
    tracing::info!(
        "Requesting ports-check testrun from primary server: {}:{}",
        primary_server.address,
        primary_server.port
    );

    let ns_api_client = common::build_client(primary_server);

    let testrun = match ns_api_client.request_ports_check_testrun().await {
        Ok(Some(testrun)) => testrun,
        Ok(None) => {
            tracing::info!("No ports-check testruns available from primary server");
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

    tracing::info!(
        "Received ports-check testrun {testrun_id} for gateway {gateway_identity_key} from primary",
    );

    let network = nym_sdk::NymNetworkDetails::new_from_env();

    // Force full exit policy list for this job kind
    netstack_args.port_check_ports = nym_gateway_probe::config::EXIT_POLICY_PORTS.to_vec();

    let run_ports_config = RunPortsConfig {
        min_gateway_mixnet_performance,
        ignore_egress_epoch_role,
        netstack_args,
    };

    let credentials_args = common::credential_args_from(testrun.ticket_materials);

    log_capture.start();
    let port_check_result = nym_gateway_probe::Probe::run_ports_for_agent(
        gateway_identity_pubkey,
        network,
        &run_ports_config,
        credentials_args,
    )
    .await?;
    let probe_log = log_capture.stop_and_drain();

    submit_ports_check_results_to_servers(
        servers,
        testrun_id,
        testrun_assigned_at,
        &gateway_identity_key,
        port_check_result,
        probe_log,
    )
    .await;

    Ok(())
}

#[instrument(level = "info", skip_all, fields(gateway_id = %gateway_identity_key, testrun = testrun_id))]
async fn submit_ports_check_results_to_servers(
    servers: &[ServerConfig],
    testrun_id: i32,
    testrun_assigned_at: i64,
    gateway_identity_key: &str,
    port_check_result: nym_gateway_probe::PortCheckResult,
    probe_log: String,
) {
    let handles = servers
        .iter()
        .enumerate()
        .map(|(idx, server)| {
            let port_check_result = port_check_result.clone();
            let probe_log = probe_log.clone();
            let gateway_identity_key = gateway_identity_key.to_string();

            async move {
                let client = common::build_client(server);

                let result = client
                    .submit_ports_check_results_with_context(
                        testrun_id,
                        port_check_result,
                        probe_log,
                        testrun_assigned_at,
                        gateway_identity_key,
                    )
                    .await;

                (idx, server.address.clone(), server.port, result)
            }
        })
        .collect::<Vec<_>>();

    let results = futures::future::join_all(handles).await;

    for (index, server_address, server_port, result) in results {
        match result {
            Ok(()) => {
                tracing::info!(
                    "✅ Successfully submitted ports-check to server[{index}] {server_address}:{server_port}",
                );
            }
            Err(e) => {
                tracing::warn!(
                    "❌ Failed to submit ports-check to server[{index}] {server_address}:{server_port} - {e}"
                );
            }
        }
    }
}
