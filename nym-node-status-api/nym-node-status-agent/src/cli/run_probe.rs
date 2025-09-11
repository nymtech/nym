use crate::cli::{GwProbe, ServerConfig};

pub(crate) async fn run_probe(
    servers: &[ServerConfig],
    probe_path: &str,
    mnemonic: &str,
    probe_extra_args: &Vec<String>,
) -> anyhow::Result<()> {
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

    let testrun_id = testrun.testrun_id;
    let testrun_assigned_at = testrun.assigned_at_utc;
    let gateway_identity_key = testrun.gateway_identity_key;

    tracing::info!("Received testrun {testrun_id} for gateway {gateway_identity_key} from primary",);

    // Run the probe
    let log = probe.run_and_get_log(
        &Some(gateway_identity_key.clone()),
        mnemonic,
        probe_extra_args,
        testrun.ticket_materials,
    );

    // Submit to ALL servers in parallel
    let handles = servers
        .iter()
        .enumerate()
        .map(move |(idx, server)| {
            let log = log.clone();
            let gateway_identity_key = gateway_identity_key.clone();

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
                        .submit_results(testrun_id as i64, log, testrun_assigned_at)
                        .await
                } else {
                    // Other servers: submit results with context
                    client
                        .submit_results_with_context(
                            testrun_id,
                            log,
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
                tracing::info!("✅ Successfully submitted {method} to server[{index}] {server_address}:{server_port}");
            }
            Err(e) => {
                tracing::warn!("❌ Failed to submit {method} to server[{index}] {server_address}:{server_port} - {e}");
            }
        }
    }

    Ok(())
}
