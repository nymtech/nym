// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::upgrade_helpers::try_load_current_config;
use crate::node::NymNode;
use crate::throughput_tester::client::ThroughputTestingClient;
use crate::throughput_tester::global_stats::GlobalStatsUpdater;
use crate::throughput_tester::stats::ClientStats;
use futures::future::join_all;
use human_repr::HumanDuration;
use indicatif::{ProgressState, ProgressStyle};
use nym_crypto::asymmetric::x25519;
use nym_task::ShutdownToken;
use rand::{thread_rng, Rng};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime;
use tokio::runtime::Runtime;
use tokio::time::sleep;
use tracing::{info, info_span, instrument};
use tracing_indicatif::span_ext::IndicatifSpanExt;

pub(crate) mod client;
pub(crate) mod global_stats;
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
            let mut config = try_load_current_config(config_path).await?;

            // make sure to change bind address to localhost!
            config
                .mixnet
                .bind_address
                .set_ip(IpAddr::V4(Ipv4Addr::LOCALHOST));

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
#[allow(clippy::too_many_arguments)]
async fn run_testing_client(
    sender_id: usize,
    node_keys: Arc<x25519::KeyPair>,
    node_listener: SocketAddr,
    packet_latency_threshold: Duration,
    starting_sending_batch_size: usize,
    starting_sending_delay: Duration,
    stats: ClientStats,
    shutdown_token: ShutdownToken,
) -> anyhow::Result<()> {
    let _ = sender_id;
    let client = ThroughputTestingClient::try_create(
        starting_sending_delay,
        starting_sending_batch_size,
        packet_latency_threshold,
        &node_keys,
        node_listener,
        stats,
        shutdown_token,
    )
    .await?;

    // wait a random amount of time before actually starting to desync the clients a bit
    // (so they wouldn't update their rates at the same time)
    let delay = Duration::from_millis(thread_rng().gen_range(10..200));
    info!(
        "waiting for {} before attempting to start the processing loop",
        delay.human_duration()
    );
    sleep(delay).await;

    client.run().await
}

pub(crate) fn test_mixing_throughput(
    config_path: PathBuf,
    senders: usize,
    packet_latency_threshold: Duration,
    starting_sending_batch_size: usize,
    starting_sending_delay: Duration,
    output_directory: PathBuf,
) -> anyhow::Result<()> {
    let tester = ThroughputTest::new(senders)?;

    let nym_node = tester.prepare_nymnode(config_path)?;
    let listener = nym_node.config().mixnet.bind_address;

    let sphinx_keys = nym_node.x25519_sphinx_keys();

    let mut stats = Vec::with_capacity(senders);
    for _ in 0..senders {
        stats.push(ClientStats::default())
    }

    let header_span = info_span!("header");
    header_span.pb_set_style(
        &ProgressStyle::with_template(
            "testing mixing throughput of this machine... {wide_msg} {elapsed}\n{wide_bar}",
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

    let mut tasks_handles = Vec::new();

    for (sender_id, stats) in stats.iter().enumerate() {
        let token = nym_node.shutdown_token(format!("dummy-load-client-{sender_id}"));

        let client_future = run_testing_client(
            sender_id,
            sphinx_keys.clone(),
            listener,
            packet_latency_threshold,
            starting_sending_batch_size,
            starting_sending_delay,
            stats.clone(),
            token,
        );
        let handle = tester.clients_runtime.spawn(client_future);
        tasks_handles.push(handle);
    }

    let mut global_stats = GlobalStatsUpdater::new(
        header_span,
        stats,
        output_directory,
        nym_node.shutdown_token("global-stats"),
    );

    let stats_handle = tester.clients_runtime.spawn(async move {
        global_stats.run().await;
        Ok(())
    });
    tasks_handles.push(stats_handle);

    tester
        .node_runtime
        .block_on(async move { nym_node.run_minimal_mixnet_processing().await })?;

    tester.clients_runtime.block_on(join_all(tasks_handles));

    Ok(())
}
