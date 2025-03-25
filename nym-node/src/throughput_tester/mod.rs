// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::upgrade_helpers::try_load_current_config;
use crate::node::NymNode;
use crate::throughput_tester::client::ThroughputTestingClient;
use crate::throughput_tester::stats::ClientStats;
use colored::Colorize;
use futures::future::join_all;
use human_repr::{HumanCount, HumanDuration, HumanThroughput};
use indicatif::{ProgressState, ProgressStyle};
use nym_crypto::asymmetric::x25519;
use nym_task::ShutdownToken;
use rand::{thread_rng, Rng};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::System;
use tokio::runtime::Runtime;
use tokio::time::{interval, sleep, Instant};
use tokio::{runtime, select};
use tracing::{info, info_span, instrument, Span};
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

async fn global_stats(
    header_span: Span,
    client_stats: Vec<ClientStats>,
    shutdown_token: ShutdownToken,
) {
    let mut update_interval = interval(Duration::from_millis(500));
    let mut system_info = System::new_all();

    fn update_stats_span(
        system: &mut System,
        header_span: &Span,
        stats: &[ClientStats],
        last_received: &mut usize,
        last_update: &mut Instant,
    ) {
        let mut all_received = 0;
        let mut all_sent = 0;
        let mut all_latencies = 0;
        for stat in stats {
            all_sent += stat.sent();
            all_received += stat.received();
            all_latencies += stat.average_latency_nanos();
        }

        let time_delta_secs = last_update.elapsed().as_secs_f64();
        let receive_rate = (all_received - *last_received) as f64 / time_delta_secs;
        let avg_rate = receive_rate.human_throughput("packets");
        let avg_latency = all_latencies as f64 / stats.len() as f64;

        system.refresh_cpu_usage();
        let cpu_usage = system.global_cpu_usage();
        let cpu_count = system.cpus().len();
        let usage_per_cpu = cpu_usage / cpu_count as f32;

        let formatted_usage = if usage_per_cpu < 0.3 {
            format!("{:.2}%", usage_per_cpu * 100.).green().bold()
        } else if usage_per_cpu < 0.7 {
            format!("{:.2}%", usage_per_cpu * 100.).yellow().bold()
        } else {
            format!("{:.2}%", usage_per_cpu * 100.).red().bold()
        };

        header_span.pb_set_message(&format!(
            "active_clients: {} | total received: {} total sent {} (avg packet latency: {}, total receive rate: {avg_rate}), avg core load: {formatted_usage}",
            stats.len(),
            all_received.human_count_bare(),
            all_sent.human_count_bare(),
            Duration::from_nanos(avg_latency as u64).human_duration()
        ));
        *last_received = all_received;
        *last_update = Instant::now();
    }

    let mut last_received = 0;
    let mut last_update = Instant::now();

    loop {
        select! {
            biased;
            _ = shutdown_token.cancelled() => {
                break;
            }
            _ = update_interval.tick() => {
                    update_stats_span(&mut system_info, &header_span, &client_stats, &mut last_received, &mut last_update);
            }
        }
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
    packet_latency_threshold: Duration,
    stats: ClientStats,
    shutdown_token: ShutdownToken,
) -> anyhow::Result<()> {
    let _ = sender_id;
    let client = ThroughputTestingClient::try_create(
        Duration::from_millis(50),
        100,
        packet_latency_threshold,
        &node_keys,
        node_listener,
        stats,
        shutdown_token,
    )
    .await?;

    // wait a random amount of time before actually starting to desync the clients a bit
    // (so they wouldn't update their rates at the same time)
    let delay = Duration::from_millis(thread_rng().gen_range(100..5000));
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

    let mut clients_handles = Vec::new();

    for (sender_id, stats) in stats.iter().enumerate() {
        let token = nym_node.shutdown_token(format!("dummy-load-client-{sender_id}"));

        let client_future = run_testing_client(
            sender_id,
            sphinx_keys.clone(),
            listener,
            packet_latency_threshold,
            stats.clone(),
            token,
        );
        let handle = tester.clients_runtime.spawn(client_future);
        clients_handles.push(handle);
    }

    tester.clients_runtime.spawn(global_stats(
        header_span,
        stats,
        nym_node.shutdown_token("global-stats"),
    ));

    tester
        .node_runtime
        .block_on(async move { nym_node.run_minimal_mixnet_processing().await })?;

    tester.clients_runtime.block_on(join_all(clients_handles));

    Ok(())
}
