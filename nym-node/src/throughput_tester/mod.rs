// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::upgrade_helpers::try_load_current_config;
use crate::node::NymNode;
use crate::throughput_tester::client::ThroughputTestingClient;
use futures::future::join_all;
use human_repr::HumanDuration;
use indicatif::{ProgressState, ProgressStyle};
use nym_crypto::asymmetric::x25519;
use nym_task::ShutdownToken;
use rand::{thread_rng, Rng};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime;
use tokio::runtime::Runtime;
use tokio::time::sleep;
use tracing::{info, info_span, instrument};
use tracing_indicatif::span_ext::IndicatifSpanExt;

pub(crate) mod client;
mod stats;

pub struct ThroughputTest {
    node_runtime: Runtime,
    clients_runtime: Runtime,
}

impl ThroughputTest {
    fn new(senders: usize) -> anyhow::Result<Self> {
        Ok(ThroughputTest {
            node_runtime: runtime::Builder::new_multi_thread()
                .enable_all()
                .thread_name("nym-node-pool")
                .build()?,
            clients_runtime: runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(senders)
                .thread_name("testing-clients-pool")
                .build()?,
        })
    }

    fn prepare_nymnode(&self, config_path: PathBuf) -> anyhow::Result<NymNode> {
        self.node_runtime.block_on(async {
            let config = try_load_current_config(config_path).await?;

            let nym_node = NymNode::new(config).await?;
            Ok(nym_node)
        })
    }
}

#[instrument(
    skip_all,
    fields(
        sender_id = %sender_id
    )

)]
async fn run_testing_client(
    sender_id: usize,
    node_keys: Arc<x25519::KeyPair>,
    node_listener: SocketAddr,
    shutdown_token: ShutdownToken,
) -> anyhow::Result<()> {
    let _ = sender_id;
    let client = ThroughputTestingClient::try_create(
        Duration::from_millis(50),
        50,
        Duration::from_millis(10),
        &node_keys,
        node_listener,
        shutdown_token,
    )
    .await?;

    // wait a random amount of time before actually starting to desync the clients a bit
    // (so they wouldn't update their rates at the same time)
    let delay = Duration::from_millis(thread_rng().gen_range(100..10000));
    info!(
        "waiting for {} before attempting to start the processing loop",
        delay.human_duration()
    );
    sleep(delay).await;

    client.run().await
}

pub(crate) fn test_mixing_throughput(config_path: PathBuf, senders: usize) -> anyhow::Result<()> {
    let tester = ThroughputTest::new(senders)?;

    let nym_node = tester.prepare_nymnode(config_path)?;
    let listener = nym_node.config().mixnet.bind_address;
    let sphinx_keys = nym_node.x25519_sphinx_keys();

    let header_span = info_span!("header");
    header_span.pb_set_style(
        &ProgressStyle::with_template(
            "Testing mixing throughput of this machine. {wide_msg} {elapsed}\n{wide_bar}",
        )?
        .with_key(
            "elapsed",
            |state: &ProgressState, writer: &mut dyn std::fmt::Write| {
                let _ = writer.write_str(&format!("{}", state.elapsed().human_duration()));
            },
        )
        .progress_chars("---"),
    );
    header_span.pb_start();

    // Bit of a hack to show a full "-----" line underneath the header.
    header_span.pb_set_length(1);
    header_span.pb_set_position(1);

    // for now try one client

    // temp
    let senders = 1;

    let mut clients_handles = Vec::new();

    for sender_id in 0..senders {
        let token = nym_node.shutdown_token(format!("dummy-load-client-{sender_id}"));

        let client_future = run_testing_client(sender_id, sphinx_keys.clone(), listener, token);
        let handle = tester.clients_runtime.spawn(client_future);
        clients_handles.push(handle);
    }

    tester
        .node_runtime
        .block_on(async move { nym_node.run_minimal_mixnet_processing().await })?;

    tester.clients_runtime.block_on(join_all(clients_handles));

    println!("listening on {}", listener);

    Ok(())
}
