use crate::cli::{GwProbe, ServerConfig};

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

    match ns_api_client.request_testrun().await {
        Ok(Some(testrun)) => {
            tracing::info!(
                "Received testrun {} for gateway {} from primary",
                testrun.testrun_id,
                testrun.gateway_identity_key
            );

            // Run the probe
            let log = probe.run_and_get_log(
                &Some(testrun.gateway_identity_key.clone()),
                probe_extra_args,
            );

            // Submit to ALL servers in parallel
            let handles = servers
                .iter()
                .enumerate()
                .map(|(idx, server)| {
                    let testrun = testrun.clone();
                    let log = log.clone();

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
                                    testrun.testrun_id as i64,
                                    log,
                                    testrun.assigned_at_utc,
                                )
                                .await
                        } else {
                            // Other servers: submit results with context
                            client
                                .submit_results_with_context(
                                    testrun.testrun_id,
                                    log,
                                    testrun.assigned_at_utc,
                                    testrun.gateway_identity_key,
                                )
                                .await
                        };

                        (idx, server.address.clone(), server.port, result)
                    }
                })
                .collect::<Vec<_>>();

            let results = futures::future::join_all(handles).await;

            for result in results {
                match result.3 {
                    Ok(()) => {
                        let method = if result.0 == 0 {
                            "regular"
                        } else {
                            "with context"
                        };
                        tracing::info!(
                            "✅ Successfully submitted {} to server[{}] {}:{}",
                            method,
                            result.0,
                            result.1,
                            result.2
                        );
                    }
                    Err(e) => {
                        let method = if result.0 == 0 {
                            "regular"
                        } else {
                            "with context"
                        };
                        tracing::warn!(
                            "❌ Failed to submit {} to server[{}] {}:{} - {}",
                            method,
                            result.0,
                            result.1,
                            result.2,
                            e
                        );
                    }
                }
            }

            Ok(())
        }
        Ok(None) => {
            tracing::info!("No testruns available from primary server");
            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to contact primary server: {}", e);
            Err(e)
        }
    }
}
